//! Tags label memories. The `tags` table is global and names are unique
//! regardless of case, but listings only ever show tags used on memories the
//! caller can read, so one family's tags never leak to another.

use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgExecutor};
use warp::{Filter, reply::Response};

use crate::{
	AppState, Routes,
	auth::{CurrentUser, authenticated},
	authz::{self, Access},
	error::{ApiError, ApiResult},
	json_body, ok, respond, validate, with_state,
};

pub const NAME_MAX: usize = 50;
/// Most tags a single memory may have.
pub const PER_MEMORY_MAX: usize = 20;
const DEFAULT_LIMIT: i64 = 50;
const MAX_LIMIT: i64 = 200;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Tag {
	pub name: String,
	/// Memories with this tag that the caller can read (within the listing's scope).
	pub memory_count: i64,
}

/// Trims and checks tag names, dropping case-insensitive duplicates (first spelling wins).
pub fn validate_names(names: &[String]) -> ApiResult<Vec<String>> {
	let mut valid: Vec<String> = Vec::with_capacity(names.len());
	for name in names {
		let name = validate::required_text("tag name", name, NAME_MAX)?;
		if !valid
			.iter()
			.any(|seen| seen.to_lowercase() == name.to_lowercase())
		{
			valid.push(name);
		}
	}
	if valid.len() > PER_MEMORY_MAX {
		return Err(ApiError::bad_request(format!(
			"a memory may have at most {PER_MEMORY_MAX} tags"
		)));
	}
	Ok(valid)
}

/// Creates any of `names` that don't exist yet (compared case-insensitively).
pub async fn ensure(conn: &mut PgConnection, names: &[String]) -> ApiResult<()> {
	// A consistent insertion order keeps concurrent writers from deadlocking on the unique index.
	let mut sorted = names.to_vec();
	sorted.sort_by_key(|name| name.to_lowercase());
	sqlx::query(
		"INSERT INTO tags (name)
		SELECT name FROM unnest($1::text[]) WITH ORDINALITY AS input(name, position)
		ORDER BY position
		ON CONFLICT ((lower(name))) DO NOTHING",
	)
	.bind(&sorted)
	.execute(conn)
	.await?;
	Ok(())
}

/// Tags on memories that user `$1` can read, filtered by relationship `$2` and
/// search text `$3` (both optional), limited to `$4`.
async fn visible<'e>(
	db: impl PgExecutor<'e>,
	user: CurrentUser,
	relationship_id: Option<i64>,
	q: Option<&str>,
	limit: i64,
) -> ApiResult<Vec<Tag>> {
	Ok(sqlx::query_as(
		"SELECT t.name, count(*) AS memory_count
		FROM tags t
		JOIN memory_tags mt ON mt.tag_id = t.id
		JOIN memories m ON m.id = mt.memory_id
		JOIN relationship_members rm ON rm.relationship_id = m.relationship_id AND rm.user_id = $1
		WHERE ($2::bigint IS NULL OR m.relationship_id = $2)
			AND ($3::text IS NULL OR strpos(lower(t.name), lower($3)) > 0)
		GROUP BY t.id, t.name
		ORDER BY lower(t.name), t.id
		LIMIT $4",
	)
	.bind(user.id)
	.bind(relationship_id)
	.bind(q)
	.bind(limit)
	.fetch_all(db)
	.await?)
}

pub fn routes(state: &AppState) -> Routes {
	let list = warp::path!("tags")
		.and(warp::get())
		.and(with_state(state))
		.and(authenticated(state))
		.and(warp::query::<TagQuery>())
		.then(
			|state, user, query| async move { respond(list_tags(state, user, None, query).await) },
		);

	let list_for_relationship = warp::path!("relationships" / i64 / "tags")
		.and(warp::get())
		.and(with_state(state))
		.and(authenticated(state))
		.and(warp::query::<TagQuery>())
		.then(|id, state, user, query| async move {
			respond(list_tags(state, user, Some(id), query).await)
		});

	let create = warp::path!("tags")
		.and(warp::post())
		.and(with_state(state))
		.and(authenticated(state))
		.and(json_body())
		.then(|state, user, body| async move { respond(create(state, user, body).await) });

	list.or(list_for_relationship)
		.unify()
		.or(create)
		.unify()
		.boxed()
}

#[derive(Deserialize)]
struct TagQuery {
	q: Option<String>,
	limit: Option<i64>,
}

async fn list_tags(
	state: AppState,
	user: CurrentUser,
	relationship_id: Option<i64>,
	query: TagQuery,
) -> ApiResult<Response> {
	if let Some(id) = relationship_id {
		authz::require_relationship(&state.db, user, id, Access::Read).await?;
	}
	let q = validate::optional_text("q", query.q.as_deref(), NAME_MAX)?;
	let limit = query.limit.unwrap_or(DEFAULT_LIMIT);
	if !(1..=MAX_LIMIT).contains(&limit) {
		return Err(ApiError::bad_request(format!(
			"limit must be between 1 and {MAX_LIMIT}"
		)));
	}
	ok(&visible(&state.db, user, relationship_id, q.as_deref(), limit).await?)
}

#[derive(Deserialize)]
struct CreateTagRequest {
	name: String,
}

/// Creates a tag, or returns the existing one with the same name ignoring case.
/// Always `200`, so the response doesn't reveal whether another user created it.
async fn create(state: AppState, user: CurrentUser, body: CreateTagRequest) -> ApiResult<Response> {
	let name = validate::required_text("name", &body.name, NAME_MAX)?;

	let mut conn = state.db.acquire().await?;
	ensure(&mut conn, std::slice::from_ref(&name)).await?;
	let memory_count: i64 = sqlx::query_scalar(
		"SELECT count(*)
		FROM memory_tags mt
		JOIN tags t ON t.id = mt.tag_id
		JOIN memories m ON m.id = mt.memory_id
		JOIN relationship_members rm ON rm.relationship_id = m.relationship_id AND rm.user_id = $1
		WHERE lower(t.name) = lower($2)",
	)
	.bind(user.id)
	.bind(&name)
	.fetch_one(&mut *conn)
	.await?;
	// Echo the caller's spelling unless they can already see the tag in use, so the
	// stored spelling of someone else's tag isn't revealed.
	let name = if memory_count > 0 {
		sqlx::query_scalar("SELECT name FROM tags WHERE lower(name) = lower($1)")
			.bind(&name)
			.fetch_one(&mut *conn)
			.await?
	} else {
		name
	};
	ok(&Tag { name, memory_count })
}
