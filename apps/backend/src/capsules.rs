//! Time capsules: content sealed until a future date.
//!
//! The backend is the security boundary for the lock. A capsule's protected
//! content (`message` and its media list) is only ever read from the database
//! by [`fetch_content`], whose query itself requires the capsule to be `OPENED`
//! and past `unlock_at`; every other query selects metadata only. All time
//! comparisons use the database clock (`now()`), never the application's.
//!
//! Lifecycle (statuses from `time_capsule_statuses`):
//!
//! ```text
//!   create ──> LOCKED ──(unlock_at passes)──> AVAILABLE ──(open)──> OPENED
//!                │
//!                └──(cancel)──> CANCELLED
//! ```
//!
//! `AVAILABLE` is never written: it's the effective status of a stored `LOCKED`
//! capsule whose `unlock_at` has passed, computed in SQL by `effective_status!`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use sqlx::{PgConnection, PgExecutor};
use warp::{Filter, reply::Response};

use crate::{
	AppState, Routes,
	auth::{CurrentUser, authenticated},
	authz::{self, Access},
	created,
	error::{ApiError, ApiResult},
	json_body,
	media::{self, LinkScope},
	no_content, ok,
	reference::{CapsuleStatus, Role},
	respond, validate, with_state,
};

const MAX_TITLE_CHARS: usize = 200;
const MAX_MESSAGE_CHARS: usize = 20_000;
/// How far in the future `unlock_at` may be.
const MAX_UNLOCK_YEARS: i32 = 100;

/// The status clients see; ids match `CapsuleStatus` (1 LOCKED, 2 AVAILABLE, 3 OPENED).
macro_rules! effective_status {
	() => {
		"(CASE WHEN c.status_id = 1 AND c.unlock_at <= now() THEN 2 ELSE c.status_id END)::smallint"
	};
}

/// Capsule metadata only — deliberately no `message` column. `$tail` adds filters and ordering.
macro_rules! select_capsules {
	($tail:literal) => {
		concat!(
			"SELECT c.id, c.relationship_id, c.title, ",
			effective_status!(),
			" AS status, c.unlock_at, c.created_by, c.created_at, c.updated_at,
				(SELECT count(*) FROM capsule_media cm WHERE cm.capsule_id = c.id) AS media_count
			FROM time_capsules c ",
			$tail
		)
	};
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Capsule {
	pub id: i64,
	pub relationship_id: i64,
	pub title: String,
	pub status: CapsuleStatus,
	pub unlock_at: DateTime<Utc>,
	pub created_by: i64,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
	pub media_count: i64,
	/// Present only once the capsule has been opened; absent (not `null`) otherwise.
	#[sqlx(skip)]
	#[serde(flatten)]
	pub content: Option<CapsuleContent>,
}

#[derive(Debug, Serialize)]
pub struct CapsuleContent {
	pub message: Option<String>,
	pub media: Vec<media::Media>,
}

/// Capsule metadata, without content.
async fn fetch_metadata<'e>(db: impl PgExecutor<'e>, id: i64) -> ApiResult<Capsule> {
	sqlx::query_as(select_capsules!("WHERE c.id = $1"))
		.bind(id)
		.fetch_optional(db)
		.await?
		.ok_or(ApiError::NotFound("capsule"))
}

/// The protected content, or `None` unless the capsule is opened and past its unlock time.
/// This is the only place capsule content is read.
async fn fetch_content(conn: &mut PgConnection, id: i64) -> ApiResult<Option<CapsuleContent>> {
	let message: Option<Option<String>> = sqlx::query_scalar(
		"SELECT message FROM time_capsules WHERE id = $1 AND status_id = $2 AND unlock_at <= now()",
	)
	.bind(id)
	.bind(CapsuleStatus::Opened)
	.fetch_optional(&mut *conn)
	.await?;
	let Some(message) = message else {
		return Ok(None);
	};

	let media = media::list_for_capsule(&mut *conn, id).await?;
	Ok(Some(CapsuleContent { message, media }))
}

/// The capsule as any member may see it: content is included only once opened.
async fn fetch(conn: &mut PgConnection, id: i64) -> ApiResult<Capsule> {
	let mut capsule = fetch_metadata(&mut *conn, id).await?;
	if capsule.status == CapsuleStatus::Opened {
		capsule.content = fetch_content(conn, id).await?;
	}
	Ok(capsule)
}

/// Requires `access` to the capsule's relationship; returns the capsule's creator and the
/// user's role. Capsules in relationships the user isn't a member of are `404`.
async fn authorize<'e>(
	db: impl PgExecutor<'e> + Copy,
	user: CurrentUser,
	id: i64,
	access: Access,
) -> ApiResult<(i64, Role)> {
	let (relationship_id, created_by): (i64, i64) =
		sqlx::query_as("SELECT relationship_id, created_by FROM time_capsules WHERE id = $1")
			.bind(id)
			.fetch_optional(db)
			.await?
			.ok_or(ApiError::NotFound("capsule"))?;
	let role = authz::require_relationship(db, user, relationship_id, access)
		.await
		.map_err(|err| match err {
			ApiError::NotFound(_) => ApiError::NotFound("capsule"),
			err => err,
		})?;
	Ok((created_by, role))
}

/// Content edit rule: the creator (while they have write access) or anyone with manage access.
async fn authorize_edit(state: &AppState, user: CurrentUser, id: i64) -> ApiResult<()> {
	let (created_by, role) = authorize(&state.db, user, id, Access::Write).await?;
	authz::require_content_editor(role, user, created_by)
}

/// Locks the capsule row for the rest of the transaction and requires it to still be locked.
async fn lock_sealed(conn: &mut PgConnection, id: i64) -> ApiResult<()> {
	let sealed: bool = sqlx::query_scalar(
		"SELECT status_id = $2 AND unlock_at > now() FROM time_capsules WHERE id = $1 FOR UPDATE",
	)
	.bind(id)
	.bind(CapsuleStatus::Locked)
	.fetch_optional(&mut *conn)
	.await?
	.ok_or(ApiError::NotFound("capsule"))?;
	if !sealed {
		return Err(ApiError::conflict(
			"a capsule can only be changed while it is locked",
		));
	}
	Ok(())
}

/// Checks `unlock_at` against the database clock (constant within a transaction).
async fn validate_unlock_at(conn: &mut PgConnection, unlock_at: DateTime<Utc>) -> ApiResult<()> {
	let (future, in_range): (bool, bool) =
		sqlx::query_as("SELECT $1 > now(), $1 <= now() + make_interval(years => $2)")
			.bind(unlock_at)
			.bind(MAX_UNLOCK_YEARS)
			.fetch_one(conn)
			.await?;
	if !future {
		return Err(ApiError::bad_request("unlock_at must be in the future"));
	}
	if !in_range {
		return Err(ApiError::bad_request(format!(
			"unlock_at must be at most {MAX_UNLOCK_YEARS} years in the future"
		)));
	}
	Ok(())
}

pub fn routes(state: &AppState) -> Routes {
	let list = warp::path!("relationships" / i64 / "capsules")
		.and(warp::get())
		.and(with_state(state))
		.and(authenticated(state))
		.and(warp::query())
		.then(|id, state, user, query| async move { respond(list(state, user, id, query).await) });

	let create = warp::path!("relationships" / i64 / "capsules")
		.and(warp::post())
		.and(with_state(state))
		.and(authenticated(state))
		.and(json_body())
		.then(|id, state, user, body| async move { respond(create(state, user, id, body).await) });

	let get = warp::path!("capsules" / i64)
		.and(warp::get())
		.and(with_state(state))
		.and(authenticated(state))
		.then(|id, state, user| async move { respond(get(state, user, id).await) });

	let update = warp::path!("capsules" / i64)
		.and(warp::put())
		.and(with_state(state))
		.and(authenticated(state))
		.and(json_body())
		.then(|id, state, user, body| async move { respond(update(state, user, id, body).await) });

	let delete = warp::path!("capsules" / i64)
		.and(warp::delete())
		.and(with_state(state))
		.and(authenticated(state))
		.then(|id, state, user| async move { respond(delete(state, user, id).await) });

	let cancel = warp::path!("capsules" / i64 / "cancel")
		.and(warp::post())
		.and(with_state(state))
		.and(authenticated(state))
		.then(|id, state, user| async move { respond(cancel(state, user, id).await) });

	let open = warp::path!("capsules" / i64 / "open")
		.and(warp::post())
		.and(with_state(state))
		.and(authenticated(state))
		.then(|id, state, user| async move { respond(open(state, user, id).await) });

	list.or(create)
		.unify()
		.or(get)
		.unify()
		.or(update)
		.unify()
		.or(delete)
		.unify()
		.or(cancel)
		.unify()
		.or(open)
		.unify()
		.boxed()
}

#[derive(Deserialize)]
struct ListQuery {
	status: Option<CapsuleStatus>,
}

/// Metadata of a relationship's capsules, soonest unlock first. Never includes content.
async fn list(
	state: AppState,
	user: CurrentUser,
	relationship_id: i64,
	query: ListQuery,
) -> ApiResult<Response> {
	authz::require_relationship(&state.db, user, relationship_id, Access::Read).await?;
	let capsules: Vec<Capsule> = sqlx::query_as(concat!(
		select_capsules!("WHERE c.relationship_id = $1 AND ($2::smallint IS NULL OR "),
		effective_status!(),
		" = $2) ORDER BY c.unlock_at, c.id"
	))
	.bind(relationship_id)
	.bind(query.status)
	.fetch_all(&state.db)
	.await?;
	ok(&capsules)
}

#[derive(Deserialize)]
struct CreateCapsuleRequest {
	title: String,
	message: Option<String>,
	unlock_at: DateTime<Utc>,
}

async fn create(
	state: AppState,
	user: CurrentUser,
	relationship_id: i64,
	body: CreateCapsuleRequest,
) -> ApiResult<Response> {
	authz::require_relationship(&state.db, user, relationship_id, Access::Write).await?;
	let title = validate::required_text("title", &body.title, MAX_TITLE_CHARS)?;
	let message = validate::optional_text("message", body.message.as_deref(), MAX_MESSAGE_CHARS)?;

	let mut tx = state.db.begin().await?;
	validate_unlock_at(&mut tx, body.unlock_at).await?;
	let id: i64 = sqlx::query_scalar(
		"INSERT INTO time_capsules (relationship_id, created_by, title, message, unlock_at, status_id)
		VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
	)
	.bind(relationship_id)
	.bind(user.id)
	.bind(title)
	.bind(message)
	.bind(body.unlock_at)
	.bind(CapsuleStatus::Locked)
	.fetch_one(&mut *tx)
	.await?;
	let capsule = fetch_metadata(&mut *tx, id).await?;
	tx.commit().await?;
	created(&capsule)
}

async fn get(state: AppState, user: CurrentUser, id: i64) -> ApiResult<Response> {
	authorize(&state.db, user, id, Access::Read).await?;
	let mut conn = state.db.acquire().await?;
	ok(&fetch(&mut conn, id).await?)
}

/// Partial update: omitted fields are kept. `message` is write-only while sealed, so a
/// client that can't read it back must be able to change the title without resending it.
#[derive(Deserialize)]
struct UpdateCapsuleRequest {
	title: Option<String>,
	/// Absent: keep. `null` or blank: clear. Otherwise: replace.
	#[serde(default, deserialize_with = "present")]
	message: Option<Option<String>>,
	unlock_at: Option<DateTime<Utc>>,
}

/// Distinguishes an explicit `null` (`Some(None)`) from an absent field (`None`).
fn present<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
	deserializer: D,
) -> Result<Option<T>, D::Error> {
	T::deserialize(deserializer).map(Some)
}

async fn update(
	state: AppState,
	user: CurrentUser,
	id: i64,
	body: UpdateCapsuleRequest,
) -> ApiResult<Response> {
	authorize_edit(&state, user, id).await?;
	let title = body
		.title
		.as_deref()
		.map(|title| validate::required_text("title", title, MAX_TITLE_CHARS))
		.transpose()?;
	let message = body
		.message
		.as_ref()
		.map(|message| validate::optional_text("message", message.as_deref(), MAX_MESSAGE_CHARS))
		.transpose()?;

	let mut tx = state.db.begin().await?;
	lock_sealed(&mut tx, id).await?;
	if let Some(unlock_at) = body.unlock_at {
		validate_unlock_at(&mut tx, unlock_at).await?;
	}
	sqlx::query(
		"UPDATE time_capsules SET
			title = coalesce($2, title),
			message = CASE WHEN $3 THEN $4 ELSE message END,
			unlock_at = coalesce($5, unlock_at)
		WHERE id = $1",
	)
	.bind(id)
	.bind(title)
	.bind(message.is_some())
	.bind(message.flatten())
	.bind(body.unlock_at)
	.execute(&mut *tx)
	.await?;
	let capsule = fetch_metadata(&mut *tx, id).await?;
	tx.commit().await?;
	ok(&capsule)
}

/// Cancels a still-locked capsule. Cancelled capsules never reveal their content.
async fn cancel(state: AppState, user: CurrentUser, id: i64) -> ApiResult<Response> {
	authorize_edit(&state, user, id).await?;

	let mut tx = state.db.begin().await?;
	lock_sealed(&mut tx, id).await?;
	sqlx::query("UPDATE time_capsules SET status_id = $2 WHERE id = $1")
		.bind(id)
		.bind(CapsuleStatus::Cancelled)
		.execute(&mut *tx)
		.await?;
	let capsule = fetch_metadata(&mut *tx, id).await?;
	tx.commit().await?;
	ok(&capsule)
}

/// Deletes a capsule in any status (edit rule). Its media links go with it.
async fn delete(state: AppState, user: CurrentUser, id: i64) -> ApiResult<Response> {
	authorize_edit(&state, user, id).await?;
	let media = media::linked_media(&state.db, LinkScope::Capsule(id)).await?;
	sqlx::query("DELETE FROM time_capsules WHERE id = $1")
		.bind(id)
		.execute(&state.db)
		.await?;
	media::delete_orphaned(&state.db, &state.media_dir, &media).await?;
	no_content()
}

/// Opens an unlocked capsule (any member, including viewers) and returns its content.
/// Opening an already opened capsule just returns it again.
async fn open(state: AppState, user: CurrentUser, id: i64) -> ApiResult<Response> {
	authorize(&state.db, user, id, Access::Read).await?;

	let mut tx = state.db.begin().await?;
	sqlx::query(
		"UPDATE time_capsules SET status_id = $2
		WHERE id = $1 AND status_id = $3 AND unlock_at <= now()",
	)
	.bind(id)
	.bind(CapsuleStatus::Opened)
	.bind(CapsuleStatus::Locked)
	.execute(&mut *tx)
	.await?;
	let capsule = fetch(&mut tx, id).await?;
	tx.commit().await?;

	match capsule.status {
		CapsuleStatus::Opened => ok(&capsule),
		CapsuleStatus::Cancelled => Err(ApiError::conflict(
			"this capsule was cancelled and can't be opened",
		)),
		_ => Err(ApiError::conflict(format!(
			"this capsule is locked until {}",
			capsule.unlock_at.to_rfc3339()
		))),
	}
}
