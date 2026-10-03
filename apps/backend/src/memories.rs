//! Memories and the relationship timeline.
//!
//! Any member with write access may add memories. A memory can be edited or
//! deleted by its creator (while they still have write access) or by anyone who
//! manages the relationship.

use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
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
	reference::{MediaType, MemoryCategory, Role},
	respond, tags,
	users::UserSummary,
	validate, with_state,
};

const TITLE_MAX: usize = 200;
const DESCRIPTION_MAX: usize = 10_000;
const SEARCH_MAX: usize = 200;
const DEFAULT_LIMIT: i64 = 50;
const MAX_LIMIT: i64 = 200;

#[derive(Debug, Serialize)]
pub struct Memory {
	pub id: i64,
	pub relationship_id: i64,
	pub category: MemoryCategory,
	pub title: String,
	pub description: Option<String>,
	/// When the remembered event happened.
	pub memory_date: NaiveDate,
	pub created_by: UserSummary,
	/// When the record was created, as opposed to `memory_date`.
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
	/// Tag names, sorted case-insensitively.
	pub tags: Vec<String>,
	pub media: Vec<MemoryMedia>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct MemoryMedia {
	pub id: i64,
	pub media_type: MediaType,
	pub file_name: String,
	pub mime_type: String,
	pub file_size: Option<i64>,
	pub created_at: DateTime<Utc>,
	#[sqlx(skip)]
	pub content_url: String,
}

#[derive(sqlx::FromRow)]
struct MemoryRow {
	id: i64,
	relationship_id: i64,
	category: MemoryCategory,
	title: String,
	description: Option<String>,
	memory_date: NaiveDate,
	author_id: i64,
	author_first_name: String,
	author_last_name: String,
	created_at: DateTime<Utc>,
	updated_at: DateTime<Utc>,
}

/// Memories with their author; `$tail` adds filters and ordering.
macro_rules! select_memories {
	($tail:expr) => {
		concat!(
			"SELECT m.id, m.relationship_id, m.category_id AS category, m.title, m.description,
				m.memory_date, u.id AS author_id, u.first_name AS author_first_name,
				u.last_name AS author_last_name, m.created_at, m.updated_at
			FROM memories m
			JOIN users u ON u.id = m.created_by",
			$tail
		)
	};
}

/// Timeline filters on memories `m`: relationship `$1`, from `$2`, to `$3`,
/// category `$4`, tag name `$5`, search text `$6`.
macro_rules! timeline_filter {
	() => {
		" WHERE m.relationship_id = $1
			AND ($2::date IS NULL OR m.memory_date >= $2)
			AND ($3::date IS NULL OR m.memory_date <= $3)
			AND ($4::smallint IS NULL OR m.category_id = $4)
			AND ($5::text IS NULL OR EXISTS (
				SELECT 1 FROM memory_tags mt JOIN tags t ON t.id = mt.tag_id
				WHERE mt.memory_id = m.id AND lower(t.name) = lower($5)))
			AND ($6::text IS NULL
				OR strpos(lower(m.title), lower($6)) > 0
				OR strpos(lower(coalesce(m.description, '')), lower($6)) > 0)"
	};
}

/// Loads memories by id, in the given order, with their tags and media.
pub async fn fetch_many(db: &PgPool, ids: &[i64]) -> ApiResult<Vec<Memory>> {
	let rows: Vec<MemoryRow> = sqlx::query_as(select_memories!(
		" WHERE m.id = ANY($1) ORDER BY array_position($1, m.id)"
	))
	.bind(ids)
	.fetch_all(db)
	.await?;
	attach(db, rows).await
}

/// Loads one memory; the caller must already have checked access.
pub async fn fetch(db: &PgPool, id: i64) -> ApiResult<Memory> {
	fetch_many(db, &[id])
		.await?
		.pop()
		.ok_or(ApiError::NotFound("memory"))
}

/// Batch-loads tags and media for a page of memories (two queries, not one per memory).
async fn attach(db: &PgPool, rows: Vec<MemoryRow>) -> ApiResult<Vec<Memory>> {
	let ids: Vec<i64> = rows.iter().map(|row| row.id).collect();

	let mut tags: HashMap<i64, Vec<String>> = HashMap::new();
	let tag_rows: Vec<(i64, String)> = sqlx::query_as(
		"SELECT mt.memory_id, t.name
		FROM memory_tags mt
		JOIN tags t ON t.id = mt.tag_id
		WHERE mt.memory_id = ANY($1)
		ORDER BY lower(t.name), t.name",
	)
	.bind(&ids)
	.fetch_all(db)
	.await?;
	for (memory_id, name) in tag_rows {
		tags.entry(memory_id).or_default().push(name);
	}

	#[derive(sqlx::FromRow)]
	struct MediaRow {
		memory_id: i64,
		#[sqlx(flatten)]
		media: MemoryMedia,
	}
	let mut media: HashMap<i64, Vec<MemoryMedia>> = HashMap::new();
	let media_rows: Vec<MediaRow> = sqlx::query_as(
		"SELECT mm.memory_id, md.id, md.media_type_id AS media_type, md.file_name, md.mime_type,
			md.file_size, md.created_at
		FROM memory_media mm
		JOIN media md ON md.id = mm.media_id
		WHERE mm.memory_id = ANY($1)
		ORDER BY md.created_at, md.id",
	)
	.bind(&ids)
	.fetch_all(db)
	.await?;
	for MediaRow {
		memory_id,
		media: mut item,
	} in media_rows
	{
		item.content_url = format!("/api/media/{}/content", item.id);
		media.entry(memory_id).or_default().push(item);
	}

	Ok(rows
		.into_iter()
		.map(|row| Memory {
			tags: tags.remove(&row.id).unwrap_or_default(),
			media: media.remove(&row.id).unwrap_or_default(),
			id: row.id,
			relationship_id: row.relationship_id,
			category: row.category,
			title: row.title,
			description: row.description,
			memory_date: row.memory_date,
			created_by: UserSummary {
				id: row.author_id,
				first_name: row.author_first_name,
				last_name: row.author_last_name,
			},
			created_at: row.created_at,
			updated_at: row.updated_at,
		})
		.collect())
}

/// Requires `access` to the relationship holding a memory; returns the caller's
/// role and the memory's creator. Memories the caller can't read are "not found".
pub async fn require_memory(
	db: &PgPool,
	user: CurrentUser,
	id: i64,
	access: Access,
) -> ApiResult<(Role, i64)> {
	let (relationship_id, created_by): (i64, i64) =
		sqlx::query_as("SELECT relationship_id, created_by FROM memories WHERE id = $1")
			.bind(id)
			.fetch_optional(db)
			.await?
			.ok_or(ApiError::NotFound("memory"))?;
	let role = authz::relationship_role(db, user, relationship_id)
		.await?
		.ok_or(ApiError::NotFound("memory"))?;
	if !role.allows(access) {
		return Err(ApiError::Forbidden);
	}
	Ok((role, created_by))
}

/// Content edit rule: the creator (with write access) or anyone with manage access.
async fn require_editable(db: &PgPool, user: CurrentUser, id: i64) -> ApiResult<()> {
	let (role, created_by) = require_memory(db, user, id, Access::Write).await?;
	authz::require_content_editor(role, user, created_by)
}

pub fn routes(state: &AppState) -> Routes {
	let timeline = warp::path!("relationships" / i64 / "memories")
		.and(warp::get())
		.and(with_state(state))
		.and(authenticated(state))
		.and(warp::query::<TimelineQuery>())
		.then(
			|id, state, user, query| async move { respond(timeline(state, user, id, query).await) },
		);

	let create = warp::path!("relationships" / i64 / "memories")
		.and(warp::post())
		.and(with_state(state))
		.and(authenticated(state))
		.and(json_body())
		.then(|id, state, user, body| async move { respond(create(state, user, id, body).await) });

	let get = warp::path!("memories" / i64)
		.and(warp::get())
		.and(with_state(state))
		.and(authenticated(state))
		.then(|id, state, user| async move { respond(get(state, user, id).await) });

	let update = warp::path!("memories" / i64)
		.and(warp::put())
		.and(with_state(state))
		.and(authenticated(state))
		.and(json_body())
		.then(|id, state, user, body| async move { respond(update(state, user, id, body).await) });

	let delete = warp::path!("memories" / i64)
		.and(warp::delete())
		.and(with_state(state))
		.and(authenticated(state))
		.then(|id, state, user| async move { respond(delete(state, user, id).await) });

	timeline
		.or(create)
		.unify()
		.or(get)
		.unify()
		.or(update)
		.unify()
		.or(delete)
		.unify()
		.boxed()
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
enum SortOrder {
	Asc,
	#[default]
	Desc,
}

#[derive(Deserialize)]
struct TimelineQuery {
	from: Option<NaiveDate>,
	to: Option<NaiveDate>,
	category: Option<MemoryCategory>,
	tag: Option<String>,
	q: Option<String>,
	#[serde(default)]
	order: SortOrder,
	limit: Option<i64>,
	offset: Option<i64>,
}

#[derive(Serialize)]
struct TimelinePage {
	memories: Vec<Memory>,
	/// Number of memories matching the filters, ignoring `limit`/`offset`.
	total: i64,
	limit: i64,
	offset: i64,
}

async fn timeline(
	state: AppState,
	user: CurrentUser,
	relationship_id: i64,
	query: TimelineQuery,
) -> ApiResult<Response> {
	authz::require_relationship(&state.db, user, relationship_id, Access::Read).await?;

	validate::date_order("from", query.from, "to", query.to)?;
	let tag = validate::optional_text("tag", query.tag.as_deref(), tags::NAME_MAX)?;
	let q = validate::optional_text("q", query.q.as_deref(), SEARCH_MAX)?;
	let limit = query.limit.unwrap_or(DEFAULT_LIMIT);
	if !(1..=MAX_LIMIT).contains(&limit) {
		return Err(ApiError::bad_request(format!(
			"limit must be between 1 and {MAX_LIMIT}"
		)));
	}
	let offset = query.offset.unwrap_or(0);
	if offset < 0 {
		return Err(ApiError::bad_request("offset must not be negative"));
	}

	let sql = match query.order {
		SortOrder::Asc => concat!(
			"SELECT m.id FROM memories m",
			timeline_filter!(),
			" ORDER BY m.memory_date, m.id LIMIT $7 OFFSET $8"
		),
		SortOrder::Desc => concat!(
			"SELECT m.id FROM memories m",
			timeline_filter!(),
			" ORDER BY m.memory_date DESC, m.id DESC LIMIT $7 OFFSET $8"
		),
	};
	let ids: Vec<i64> = sqlx::query_scalar(sql)
		.bind(relationship_id)
		.bind(query.from)
		.bind(query.to)
		.bind(query.category)
		.bind(&tag)
		.bind(&q)
		.bind(limit)
		.bind(offset)
		.fetch_all(&state.db)
		.await?;
	let total: i64 = sqlx::query_scalar(concat!(
		"SELECT count(*) FROM memories m",
		timeline_filter!()
	))
	.bind(relationship_id)
	.bind(query.from)
	.bind(query.to)
	.bind(query.category)
	.bind(&tag)
	.bind(&q)
	.fetch_one(&state.db)
	.await?;

	ok(&TimelinePage {
		memories: fetch_many(&state.db, &ids).await?,
		total,
		limit,
		offset,
	})
}

/// Body of both create and update; update replaces every field, including tags.
#[derive(Deserialize)]
struct MemoryRequest {
	category: MemoryCategory,
	title: String,
	description: Option<String>,
	memory_date: NaiveDate,
	#[serde(default)]
	tags: Vec<String>,
}

struct ValidMemory {
	category: MemoryCategory,
	title: String,
	description: Option<String>,
	memory_date: NaiveDate,
	tags: Vec<String>,
}

impl MemoryRequest {
	fn validate(self) -> ApiResult<ValidMemory> {
		Ok(ValidMemory {
			category: self.category,
			title: validate::required_text("title", &self.title, TITLE_MAX)?,
			description: validate::optional_text(
				"description",
				self.description.as_deref(),
				DESCRIPTION_MAX,
			)?,
			memory_date: self.memory_date,
			tags: tags::validate_names(&self.tags)?,
		})
	}
}

async fn create(
	state: AppState,
	user: CurrentUser,
	relationship_id: i64,
	body: MemoryRequest,
) -> ApiResult<Response> {
	authz::require_relationship(&state.db, user, relationship_id, Access::Write).await?;
	let memory = body.validate()?;

	let mut tx = state.db.begin().await?;
	let id: i64 = sqlx::query_scalar(
		"INSERT INTO memories (relationship_id, category_id, title, description, memory_date, created_by)
		VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
	)
	.bind(relationship_id)
	.bind(memory.category)
	.bind(&memory.title)
	.bind(&memory.description)
	.bind(memory.memory_date)
	.bind(user.id)
	.fetch_one(&mut *tx)
	.await?;
	set_tags(&mut tx, id, &memory.tags).await?;
	tx.commit().await?;

	created(&fetch(&state.db, id).await?)
}

async fn get(state: AppState, user: CurrentUser, id: i64) -> ApiResult<Response> {
	require_memory(&state.db, user, id, Access::Read).await?;
	ok(&fetch(&state.db, id).await?)
}

async fn update(
	state: AppState,
	user: CurrentUser,
	id: i64,
	body: MemoryRequest,
) -> ApiResult<Response> {
	require_editable(&state.db, user, id).await?;
	let memory = body.validate()?;

	let mut tx = state.db.begin().await?;
	sqlx::query(
		"UPDATE memories SET category_id = $1, title = $2, description = $3, memory_date = $4
		WHERE id = $5",
	)
	.bind(memory.category)
	.bind(&memory.title)
	.bind(&memory.description)
	.bind(memory.memory_date)
	.bind(id)
	.execute(&mut *tx)
	.await?;
	set_tags(&mut tx, id, &memory.tags).await?;
	tx.commit().await?;

	ok(&fetch(&state.db, id).await?)
}

/// Deletes the memory and its tag/media links. Media rows and files are left for Part 6 to manage.
async fn delete(state: AppState, user: CurrentUser, id: i64) -> ApiResult<Response> {
	require_editable(&state.db, user, id).await?;
	let media = media::linked_media(&state.db, LinkScope::Memory(id)).await?;
	sqlx::query("DELETE FROM memories WHERE id = $1")
		.bind(id)
		.execute(&state.db)
		.await?;
	media::delete_orphaned(&state.db, &state.media_dir, &media).await?;
	no_content()
}

/// Replaces a memory's tags with `names` (already validated and deduplicated).
async fn set_tags(conn: &mut PgConnection, memory_id: i64, names: &[String]) -> ApiResult<()> {
	sqlx::query("DELETE FROM memory_tags WHERE memory_id = $1")
		.bind(memory_id)
		.execute(&mut *conn)
		.await?;
	if names.is_empty() {
		return Ok(());
	}
	tags::ensure(&mut *conn, names).await?;
	sqlx::query(
		"INSERT INTO memory_tags (memory_id, tag_id)
		SELECT $1, t.id FROM tags t
		WHERE lower(t.name) IN (SELECT lower(name) FROM unnest($2::text[]) AS name)",
	)
	.bind(memory_id)
	.bind(names)
	.execute(&mut *conn)
	.await?;
	Ok(())
}
