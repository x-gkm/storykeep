//! Media files attached to memories and time capsules.
//!
//! Files are stored under [`AppState::media_dir`] as `<uuid>.<ext>`; the client's
//! file name is only kept as metadata. Uploads are typed by sniffing their bytes,
//! never by trusting the declared `Content-Type`.
//!
//! A user may read a media item if it's linked to a memory in a relationship they
//! belong to, or to an *opened* time capsule in such a relationship (`OPENED` and
//! past its unlock time). Media only in a capsule that isn't open is hidden from everyone,
//! including its uploader. Capsule media can only be added or removed while the capsule
//! is `LOCKED` and its unlock time is still in the future.

use std::{
	collections::HashMap,
	io,
	path::{Component, Path, PathBuf},
};

use anyhow::{Context, anyhow};
use bytes::{Buf, Bytes, BytesMut};
use chrono::{DateTime, Utc};
use futures_util::{Stream, StreamExt, TryStreamExt};
use serde::Serialize;
use sqlx::{PgConnection, PgExecutor, PgPool};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use warp::{
	Filter,
	http::{HeaderValue, header},
	multipart::{FormData, Part},
	reply::{Reply, Response},
};

use crate::{
	AppState, Routes,
	auth::{CurrentUser, authenticated},
	authz::{self, Access},
	created,
	error::{ApiError, ApiResult},
	no_content, ok,
	reference::{CapsuleStatus, MediaType},
	respond,
	users::UserSummary,
	with_state,
};

/// Largest accepted file, in bytes.
pub const MAX_FILE_SIZE: u64 = 50 * 1024 * 1024;
/// Most files accepted in one upload request.
pub const MAX_FILES_PER_REQUEST: usize = 20;
/// Longest stored original file name, in characters (the column is `VARCHAR(255)`).
const MAX_FILE_NAME_CHARS: usize = 255;
/// How many leading bytes are read before deciding a file's type.
const SNIFF_LEN: usize = 4096;
/// The multipart field that carries files.
const FILE_FIELD: &str = "file";

/// Media metadata as returned by the API.
#[derive(Debug, Serialize)]
pub struct Media {
	pub id: i64,
	pub media_type: MediaType,
	pub file_name: String,
	pub mime_type: String,
	pub file_size: Option<i64>,
	pub uploaded_by: UserSummary,
	pub created_at: DateTime<Utc>,
	/// Where the file itself can be downloaded.
	pub content_url: String,
}

#[derive(sqlx::FromRow)]
struct MediaRow {
	id: i64,
	media_type: MediaType,
	file_name: String,
	mime_type: String,
	file_size: Option<i64>,
	uploader_id: i64,
	uploader_first_name: String,
	uploader_last_name: String,
	created_at: DateTime<Utc>,
}

impl From<MediaRow> for Media {
	fn from(row: MediaRow) -> Self {
		Self {
			id: row.id,
			media_type: row.media_type,
			file_name: row.file_name,
			mime_type: row.mime_type,
			file_size: row.file_size,
			uploaded_by: UserSummary {
				id: row.uploader_id,
				first_name: row.uploader_first_name,
				last_name: row.uploader_last_name,
			},
			created_at: row.created_at,
			content_url: content_url(row.id),
		}
	}
}

/// The URL that serves a media item's bytes.
pub fn content_url(id: i64) -> String {
	format!("/api/media/{id}/content")
}

/// Media rows with their uploader; `$tail` adds filters and ordering.
macro_rules! select_media {
	($tail:literal) => {
		concat!(
			"SELECT m.id, m.media_type_id AS media_type, m.file_name, m.mime_type, m.file_size,
				u.id AS uploader_id, u.first_name AS uploader_first_name,
				u.last_name AS uploader_last_name, m.created_at
			FROM media m
			JOIN users u ON u.id = m.uploaded_by ",
			$tail
		)
	};
}

/// SQL condition: user `$2` may read media `m` (see the module docs).
macro_rules! readable_by_user_2 {
	() => {
		"(EXISTS (
			SELECT 1 FROM memory_media mm
			JOIN memories me ON me.id = mm.memory_id
			JOIN relationship_members rm ON rm.relationship_id = me.relationship_id
			WHERE mm.media_id = m.id AND rm.user_id = $2
		) OR EXISTS (
			SELECT 1 FROM capsule_media cm
			JOIN time_capsules c ON c.id = cm.capsule_id
			JOIN relationship_members rm ON rm.relationship_id = c.relationship_id
			WHERE cm.media_id = m.id AND rm.user_id = $2
				AND c.status_id = 3 AND c.unlock_at <= now()
		))"
	};
}

/// A media item the user may read, or `404`.
pub async fn fetch<'e>(db: impl PgExecutor<'e>, user: CurrentUser, id: i64) -> ApiResult<Media> {
	let row: MediaRow = sqlx::query_as(concat!(
		select_media!("WHERE m.id = $1 AND "),
		readable_by_user_2!()
	))
	.bind(id)
	.bind(user.id)
	.fetch_optional(db)
	.await?
	.ok_or(ApiError::NotFound("media"))?;
	Ok(row.into())
}

/// Media attached to each of `memory_ids`, oldest first; memories without media
/// are absent from the map. The caller must have checked read access.
pub async fn list_for_memories<'e>(
	db: impl PgExecutor<'e>,
	memory_ids: &[i64],
) -> ApiResult<HashMap<i64, Vec<Media>>> {
	#[derive(sqlx::FromRow)]
	struct LinkedRow {
		memory_id: i64,
		#[sqlx(flatten)]
		media: MediaRow,
	}

	let rows: Vec<LinkedRow> = sqlx::query_as(
		"SELECT mm.memory_id, m.id, m.media_type_id AS media_type, m.file_name, m.mime_type,
			m.file_size, u.id AS uploader_id, u.first_name AS uploader_first_name,
			u.last_name AS uploader_last_name, m.created_at
		FROM memory_media mm
		JOIN media m ON m.id = mm.media_id
		JOIN users u ON u.id = m.uploaded_by
		WHERE mm.memory_id = ANY($1)
		ORDER BY m.created_at, m.id",
	)
	.bind(memory_ids)
	.fetch_all(db)
	.await?;

	let mut media: HashMap<i64, Vec<Media>> = HashMap::new();
	for row in rows {
		media
			.entry(row.memory_id)
			.or_default()
			.push(row.media.into());
	}
	Ok(media)
}

/// Media attached to a memory, oldest first. The caller must have checked read access.
pub async fn list_for_memory<'e>(db: impl PgExecutor<'e>, memory_id: i64) -> ApiResult<Vec<Media>> {
	Ok(list_for_memories(db, &[memory_id])
		.await?
		.remove(&memory_id)
		.unwrap_or_default())
}

/// Media attached to a time capsule, oldest first. The caller must have checked
/// read access **and** that the capsule's content may be revealed.
pub async fn list_for_capsule<'e>(
	db: impl PgExecutor<'e>,
	capsule_id: i64,
) -> ApiResult<Vec<Media>> {
	let rows: Vec<MediaRow> = sqlx::query_as(select_media!(
		"JOIN capsule_media cm ON cm.media_id = m.id WHERE cm.capsule_id = $1
		ORDER BY m.created_at, m.id"
	))
	.bind(capsule_id)
	.fetch_all(db)
	.await?;
	Ok(rows.into_iter().map(Media::from).collect())
}

/// Something whose deletion cascades to media links.
#[derive(Debug, Clone, Copy)]
pub enum LinkScope {
	Memory(i64),
	Capsule(i64),
	Relationship(i64),
	Profile(i64),
}

/// Ids of media linked to anything within `scope`. Collect them before deleting
/// the scope, then pass them to [`delete_orphaned`].
pub async fn linked_media<'e>(db: impl PgExecutor<'e>, scope: LinkScope) -> ApiResult<Vec<i64>> {
	let (sql, id) = match scope {
		LinkScope::Memory(id) => ("SELECT media_id FROM memory_media WHERE memory_id = $1", id),
		LinkScope::Capsule(id) => (
			"SELECT media_id FROM capsule_media WHERE capsule_id = $1",
			id,
		),
		LinkScope::Relationship(id) => (
			"SELECT mm.media_id FROM memory_media mm JOIN memories m ON m.id = mm.memory_id
				WHERE m.relationship_id = $1
			UNION
			SELECT cm.media_id FROM capsule_media cm JOIN time_capsules c ON c.id = cm.capsule_id
				WHERE c.relationship_id = $1",
			id,
		),
		LinkScope::Profile(id) => (
			"SELECT mm.media_id FROM memory_media mm
				JOIN memories m ON m.id = mm.memory_id
				JOIN relationships r ON r.id = m.relationship_id
				WHERE r.profile_id = $1
			UNION
			SELECT cm.media_id FROM capsule_media cm
				JOIN time_capsules c ON c.id = cm.capsule_id
				JOIN relationships r ON r.id = c.relationship_id
				WHERE r.profile_id = $1",
			id,
		),
	};
	Ok(sqlx::query_scalar(sql).bind(id).fetch_all(db).await?)
}

/// Deletes those of `candidates` that are no longer attached to anything, with
/// their files; returns how many. Deleting a memory, capsule, relationship or
/// profile only removes the links (by cascade), so call this after such a delete.
pub async fn delete_orphaned(db: &PgPool, media_dir: &Path, candidates: &[i64]) -> ApiResult<u64> {
	if candidates.is_empty() {
		return Ok(0);
	}
	let paths: Vec<String> = sqlx::query_scalar(
		"DELETE FROM media m
		WHERE m.id = ANY($1)
			AND NOT EXISTS (SELECT 1 FROM memory_media WHERE media_id = m.id)
			AND NOT EXISTS (SELECT 1 FROM capsule_media WHERE media_id = m.id)
		RETURNING storage_path",
	)
	.bind(candidates)
	.fetch_all(db)
	.await?;
	for path in &paths {
		remove_file(media_dir, path).await;
	}
	Ok(paths.len() as u64)
}

pub fn routes(state: &AppState) -> Routes {
	// Sizes are enforced per file while streaming, so no overall length (or
	// `Content-Length` header) is required up front.
	let form = || warp::multipart::form().max_length(None);

	let upload_to_memory = warp::path!("memories" / i64 / "media")
		.and(warp::post())
		.and(with_state(state))
		.and(authenticated(state))
		.and(form())
		.then(|id, state, user, form| async move {
			respond(upload(state, user, Owner::Memory(id), form).await)
		});

	let upload_to_capsule = warp::path!("capsules" / i64 / "media")
		.and(warp::post())
		.and(with_state(state))
		.and(authenticated(state))
		.and(form())
		.then(|id, state, user, form| async move {
			respond(upload(state, user, Owner::Capsule(id), form).await)
		});

	let detach_from_memory = warp::path!("memories" / i64 / "media" / i64)
		.and(warp::delete())
		.and(with_state(state))
		.and(authenticated(state))
		.then(|id, media_id, state, user| async move {
			respond(detach(state, user, Owner::Memory(id), media_id).await)
		});

	let detach_from_capsule = warp::path!("capsules" / i64 / "media" / i64)
		.and(warp::delete())
		.and(with_state(state))
		.and(authenticated(state))
		.then(|id, media_id, state, user| async move {
			respond(detach(state, user, Owner::Capsule(id), media_id).await)
		});

	let get = warp::path!("media" / i64)
		.and(warp::get())
		.and(with_state(state))
		.and(authenticated(state))
		.then(|id, state: AppState, user| async move {
			respond(
				fetch(&state.db, user, id)
					.await
					.and_then(|media| ok(&media)),
			)
		});

	let content = warp::path!("media" / i64 / "content")
		.and(warp::get())
		.and(with_state(state))
		.and(authenticated(state))
		.then(|id, state, user| async move { respond(content(state, user, id).await) });

	upload_to_memory
		.or(upload_to_capsule)
		.unify()
		.or(detach_from_memory)
		.unify()
		.or(detach_from_capsule)
		.unify()
		.or(get)
		.unify()
		.or(content)
		.unify()
		.boxed()
}

/// What media is attached to.
#[derive(Debug, Clone, Copy)]
enum Owner {
	Memory(i64),
	Capsule(i64),
}

/// The owner's relationship and, for capsules, whether it's still locked.
struct OwnerInfo {
	relationship_id: i64,
	/// `Some(true)` for a capsule that is LOCKED with a future unlock time, `Some(false)` otherwise.
	capsule_locked: Option<bool>,
}

impl Owner {
	fn not_found(self) -> ApiError {
		match self {
			Owner::Memory(_) => ApiError::NotFound("memory"),
			Owner::Capsule(_) => ApiError::NotFound("time capsule"),
		}
	}

	/// Looks the owner up; with `for_update` the row is locked for the transaction.
	async fn info(self, conn: &mut PgConnection, for_update: bool) -> ApiResult<OwnerInfo> {
		let row: Option<(i64, Option<bool>)> = match (self, for_update) {
			(Owner::Memory(id), _) => {
				sqlx::query_as("SELECT relationship_id, NULL::boolean FROM memories WHERE id = $1")
					.bind(id)
					.fetch_optional(&mut *conn)
					.await?
			}
			(Owner::Capsule(id), false) => {
				sqlx::query_as(
					"SELECT relationship_id, status_id = $2 AND unlock_at > now()
					FROM time_capsules WHERE id = $1",
				)
				.bind(id)
				.bind(CapsuleStatus::Locked)
				.fetch_optional(&mut *conn)
				.await?
			}
			(Owner::Capsule(id), true) => {
				sqlx::query_as(
					"SELECT relationship_id, status_id = $2 AND unlock_at > now()
					FROM time_capsules WHERE id = $1 FOR UPDATE",
				)
				.bind(id)
				.bind(CapsuleStatus::Locked)
				.fetch_optional(&mut *conn)
				.await?
			}
		};
		let (relationship_id, capsule_locked) = row.ok_or_else(|| self.not_found())?;
		Ok(OwnerInfo {
			relationship_id,
			capsule_locked,
		})
	}

	/// Requires the user to be allowed to add media; returns the owner's info.
	async fn authorize_upload(
		self,
		conn: &mut PgConnection,
		user: CurrentUser,
		for_update: bool,
	) -> ApiResult<OwnerInfo> {
		let info = self.info(conn, for_update).await?;
		authz::require_relationship(&mut *conn, user, info.relationship_id, Access::Write)
			.await
			.map_err(|err| match err {
				ApiError::NotFound(_) => self.not_found(),
				err => err,
			})?;
		if info.capsule_locked == Some(false) {
			return Err(ApiError::conflict(
				"media can only be added while the time capsule is locked",
			));
		}
		Ok(info)
	}
}

async fn upload(
	state: AppState,
	user: CurrentUser,
	owner: Owner,
	form: FormData,
) -> ApiResult<Response> {
	// Fail fast, before receiving any file data.
	owner
		.authorize_upload(&mut *state.db.acquire().await?, user, false)
		.await?;

	let mut pending = PendingFiles::new(&state.media_dir);
	let files = receive_files(form, &mut pending).await?;

	let mut tx = state.db.begin().await?;
	// Check again with the capsule locked, in case it unlocked during the upload.
	owner.authorize_upload(&mut tx, user, true).await?;
	let mut ids = Vec::with_capacity(files.len());
	for file in &files {
		let id: i64 = sqlx::query_scalar(
			"INSERT INTO media (media_type_id, storage_path, file_name, mime_type, file_size, uploaded_by)
			VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
		)
		.bind(file.kind.media_type)
		.bind(&file.storage_path)
		.bind(&file.file_name)
		.bind(file.kind.mime_type)
		.bind(file.size as i64)
		.bind(user.id)
		.fetch_one(&mut *tx)
		.await?;
		match owner {
			Owner::Memory(memory_id) => {
				sqlx::query("INSERT INTO memory_media (memory_id, media_id) VALUES ($1, $2)")
					.bind(memory_id)
					.bind(id)
					.execute(&mut *tx)
					.await?;
			}
			Owner::Capsule(capsule_id) => {
				sqlx::query("INSERT INTO capsule_media (capsule_id, media_id) VALUES ($1, $2)")
					.bind(capsule_id)
					.bind(id)
					.execute(&mut *tx)
					.await?;
			}
		}
		ids.push(id);
	}
	let rows: Vec<MediaRow> = sqlx::query_as(select_media!("WHERE m.id = ANY($1) ORDER BY m.id"))
		.bind(&ids)
		.fetch_all(&mut *tx)
		.await?;
	tx.commit().await?;
	pending.keep();

	let media: Vec<Media> = rows.into_iter().map(Media::from).collect();
	created(&media)
}

/// Removes media from a memory or capsule, deleting it once nothing uses it.
async fn detach(
	state: AppState,
	user: CurrentUser,
	owner: Owner,
	media_id: i64,
) -> ApiResult<Response> {
	let mut tx = state.db.begin().await?;
	// Locks a capsule row, so it can't be opened while media is being removed.
	let info = owner.info(&mut tx, true).await?;
	let role = authz::relationship_role(&mut *tx, user, info.relationship_id)
		.await?
		.ok_or_else(|| owner.not_found())?;

	// Locking the media row serialises concurrent detaches, so exactly one of
	// them sees the last link go and deletes the row.
	let uploaded_by: Option<i64> = match owner {
		Owner::Memory(id) => {
			sqlx::query_scalar(
				"SELECT m.uploaded_by FROM media m
				JOIN memory_media l ON l.media_id = m.id
				WHERE l.memory_id = $1 AND m.id = $2
				FOR UPDATE OF m",
			)
			.bind(id)
			.bind(media_id)
			.fetch_optional(&mut *tx)
			.await?
		}
		Owner::Capsule(id) => {
			sqlx::query_scalar(
				"SELECT m.uploaded_by FROM media m
				JOIN capsule_media l ON l.media_id = m.id
				WHERE l.capsule_id = $1 AND m.id = $2
				FOR UPDATE OF m",
			)
			.bind(id)
			.bind(media_id)
			.fetch_optional(&mut *tx)
			.await?
		}
	};
	let uploaded_by = uploaded_by.ok_or(ApiError::NotFound("media"))?;

	let own_upload = uploaded_by == user.id;
	if info.capsule_locked.is_some() && !own_upload && !role.allows(Access::Manage) {
		// Don't reveal what's in a capsule to members who can't act on it.
		return Err(ApiError::NotFound("media"));
	}
	authz::require_content_editor(role, user, uploaded_by)?;
	if info.capsule_locked == Some(false) {
		return Err(ApiError::conflict(
			"media can only be removed while the time capsule is locked",
		));
	}

	match owner {
		Owner::Memory(id) => {
			sqlx::query("DELETE FROM memory_media WHERE memory_id = $1 AND media_id = $2")
				.bind(id)
				.bind(media_id)
				.execute(&mut *tx)
				.await?;
		}
		Owner::Capsule(id) => {
			sqlx::query("DELETE FROM capsule_media WHERE capsule_id = $1 AND media_id = $2")
				.bind(id)
				.bind(media_id)
				.execute(&mut *tx)
				.await?;
		}
	}
	let orphaned_path: Option<String> = sqlx::query_scalar(
		"DELETE FROM media m WHERE m.id = $1
			AND NOT EXISTS (SELECT 1 FROM memory_media WHERE media_id = m.id)
			AND NOT EXISTS (SELECT 1 FROM capsule_media WHERE media_id = m.id)
		RETURNING storage_path",
	)
	.bind(media_id)
	.fetch_optional(&mut *tx)
	.await?;
	tx.commit().await?;

	if let Some(path) = orphaned_path {
		remove_file(&state.media_dir, &path).await;
	}
	no_content()
}

/// Streams a media file the user may read.
async fn content(state: AppState, user: CurrentUser, id: i64) -> ApiResult<Response> {
	let (storage_path, file_name, mime_type): (String, String, String) = sqlx::query_as(concat!(
		"SELECT m.storage_path, m.file_name, m.mime_type FROM media m WHERE m.id = $1 AND ",
		readable_by_user_2!()
	))
	.bind(id)
	.bind(user.id)
	.fetch_optional(&state.db)
	.await?
	.ok_or(ApiError::NotFound("media"))?;

	let path = resolve_storage_path(&state.media_dir, &storage_path)?;
	let file = tokio::fs::File::open(&path)
		.await
		.with_context(|| format!("failed to open media file {}", path.display()))?;
	let length = file
		.metadata()
		.await
		.with_context(|| format!("failed to stat media file {}", path.display()))?
		.len();

	let mut response = warp::reply::stream(file_stream(file)).into_response();
	let headers = response.headers_mut();
	headers.insert(
		header::CONTENT_TYPE,
		HeaderValue::from_str(&mime_type)
			.unwrap_or(HeaderValue::from_static("application/octet-stream")),
	);
	headers.insert(header::CONTENT_LENGTH, HeaderValue::from(length));
	headers.insert(
		header::CONTENT_DISPOSITION,
		HeaderValue::from_str(&content_disposition(&file_name))
			.context("invalid content-disposition header")?,
	);
	headers.insert(
		header::X_CONTENT_TYPE_OPTIONS,
		HeaderValue::from_static("nosniff"),
	);
	headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("private"));
	Ok(response)
}

fn file_stream(
	file: tokio::fs::File,
) -> impl Stream<Item = io::Result<Bytes>> + Send + Sync + 'static {
	futures_util::stream::try_unfold(file, |mut file| async move {
		let mut buf = BytesMut::with_capacity(64 * 1024);
		let read = file.read_buf(&mut buf).await?;
		Ok(if read == 0 {
			None
		} else {
			Some((buf.freeze(), file))
		})
	})
}

// ===== Receiving uploads =====

/// A file written to disk but not yet recorded in the database.
struct ReceivedFile {
	storage_path: String,
	file_name: String,
	kind: FileKind,
	size: u64,
}

/// Files written during an upload; removed on drop unless [`PendingFiles::keep`] is called,
/// so a failed upload (or a dropped connection) leaves nothing behind.
struct PendingFiles {
	paths: Vec<PathBuf>,
	media_dir: PathBuf,
}

impl PendingFiles {
	fn new(media_dir: &Path) -> Self {
		Self {
			paths: Vec::new(),
			media_dir: media_dir.to_owned(),
		}
	}

	fn add(&mut self, storage_path: &str) -> PathBuf {
		let path = self.media_dir.join(storage_path);
		self.paths.push(path.clone());
		path
	}

	fn keep(mut self) {
		self.paths.clear();
	}
}

impl Drop for PendingFiles {
	fn drop(&mut self) {
		for path in &self.paths {
			if let Err(err) = std::fs::remove_file(path)
				&& err.kind() != io::ErrorKind::NotFound
			{
				tracing::error!("failed to remove {}: {err}", path.display());
			}
		}
	}
}

fn bad_multipart(err: warp::Error) -> ApiError {
	ApiError::bad_request(format!("invalid multipart body: {err}"))
}

async fn receive_files(
	mut form: FormData,
	pending: &mut PendingFiles,
) -> ApiResult<Vec<ReceivedFile>> {
	let mut files = Vec::new();
	while let Some(part) = form.try_next().await.map_err(bad_multipart)? {
		if part.name() != FILE_FIELD {
			return Err(ApiError::bad_request(format!(
				"unexpected form field \"{}\"; send files in fields named \"{FILE_FIELD}\"",
				part.name()
			)));
		}
		if files.len() == MAX_FILES_PER_REQUEST {
			return Err(ApiError::bad_request(format!(
				"at most {MAX_FILES_PER_REQUEST} files can be uploaded at once"
			)));
		}
		files.push(receive_file(part, pending).await?);
	}
	if files.is_empty() {
		return Err(ApiError::bad_request(format!(
			"no files uploaded; send them in multipart fields named \"{FILE_FIELD}\""
		)));
	}
	Ok(files)
}

async fn receive_file(part: Part, pending: &mut PendingFiles) -> ApiResult<ReceivedFile> {
	let original_name = part.filename().and_then(sanitize_file_name);
	let label = original_name.clone().unwrap_or_else(|| "file".to_owned());
	let declared = part.content_type().map(str::to_owned);
	let mut chunks = std::pin::pin!(
		part.stream()
			.map(|chunk| chunk.map(|mut buf| buf.copy_to_bytes(buf.remaining())))
	);

	// Read enough to identify the file before writing anything.
	let mut head = BytesMut::new();
	while head.len() < SNIFF_LEN {
		match chunks.try_next().await.map_err(bad_multipart)? {
			Some(chunk) => head.extend_from_slice(&chunk),
			None => break,
		}
	}
	if head.is_empty() {
		return Err(ApiError::bad_request(format!("{label} is empty")));
	}
	if head.len() as u64 > MAX_FILE_SIZE {
		return Err(ApiError::PayloadTooLarge);
	}
	let kind = detect(declared.as_deref(), &head).map_err(|err| match err {
		DetectError::NotAllowed(mime) => {
			ApiError::bad_request(format!("{label}: file type {mime} is not allowed"))
		}
		DetectError::Unrecognized => {
			ApiError::bad_request(format!("{label}: unsupported or unrecognised file type"))
		}
		DetectError::Mismatch(mime) => ApiError::bad_request(format!(
			"{label}: file content does not match its declared type {mime}"
		)),
	})?;

	let storage_path = format!("{}.{}", uuid::Uuid::new_v4(), kind.extension);
	let path = pending.add(&storage_path);
	let mut file = tokio::fs::OpenOptions::new()
		.write(true)
		.create_new(true)
		.open(&path)
		.await
		.with_context(|| format!("failed to create {}", path.display()))?;
	let write_error = || format!("failed to write {}", path.display());

	let mut size = head.len() as u64;
	file.write_all(&head).await.with_context(write_error)?;
	while let Some(chunk) = chunks.try_next().await.map_err(bad_multipart)? {
		size += chunk.len() as u64;
		if size > MAX_FILE_SIZE {
			return Err(ApiError::PayloadTooLarge);
		}
		file.write_all(&chunk).await.with_context(write_error)?;
	}
	file.flush().await.with_context(write_error)?;

	Ok(ReceivedFile {
		storage_path,
		file_name: original_name.unwrap_or_else(|| format!("upload.{}", kind.extension)),
		kind,
		size,
	})
}

/// Keeps only the last path segment, without control characters, trimmed and
/// shortened (keeping the extension) to fit the column. `None` if nothing is left.
fn sanitize_file_name(raw: &str) -> Option<String> {
	let base = raw.rsplit(['/', '\\']).next().unwrap_or_default();
	let cleaned: String = base.chars().filter(|c| !c.is_control()).collect();
	let cleaned = cleaned.trim();
	if cleaned.is_empty() {
		return None;
	}
	if cleaned.chars().count() <= MAX_FILE_NAME_CHARS {
		return Some(cleaned.to_owned());
	}
	let (stem, extension) = match cleaned.rsplit_once('.') {
		Some((stem, extension)) if !stem.is_empty() && extension.chars().count() <= 16 => {
			(stem, Some(extension))
		}
		_ => (cleaned, None),
	};
	let extension_chars = extension.map_or(0, |ext| ext.chars().count() + 1);
	let mut name: String = stem
		.chars()
		.take(MAX_FILE_NAME_CHARS - extension_chars)
		.collect();
	if let Some(extension) = extension {
		name.push('.');
		name.push_str(extension);
	}
	Some(name)
}

/// `inline` with an ASCII fallback name and the exact name per RFC 6266 / 5987.
fn content_disposition(file_name: &str) -> String {
	let fallback: String = file_name
		.chars()
		.map(|c| match c {
			' '..='~' if c != '"' && c != '\\' => c,
			_ => '_',
		})
		.collect();
	let mut encoded = String::new();
	for byte in file_name.bytes() {
		match byte {
			b'A'..=b'Z'
			| b'a'..=b'z'
			| b'0'..=b'9'
			| b'!'
			| b'#'
			| b'$'
			| b'&'
			| b'+'
			| b'-'
			| b'.'
			| b'^'
			| b'_'
			| b'`'
			| b'|'
			| b'~' => encoded.push(byte as char),
			_ => encoded.push_str(&format!("%{byte:02X}")),
		}
	}
	format!("inline; filename=\"{fallback}\"; filename*=UTF-8''{encoded}")
}

/// Joins a stored relative path onto the media directory, refusing anything but a
/// single plain file name so a tampered row can't escape the directory.
fn resolve_storage_path(media_dir: &Path, storage_path: &str) -> ApiResult<PathBuf> {
	let relative = Path::new(storage_path);
	let mut components = relative.components();
	match (components.next(), components.next()) {
		(Some(Component::Normal(_)), None) => Ok(media_dir.join(relative)),
		_ => Err(anyhow!("refusing unsafe media storage path {storage_path:?}").into()),
	}
}

async fn remove_file(media_dir: &Path, storage_path: &str) {
	let path = match resolve_storage_path(media_dir, storage_path) {
		Ok(path) => path,
		Err(err) => {
			tracing::error!("{err}");
			return;
		}
	};
	if let Err(err) = tokio::fs::remove_file(&path).await
		&& err.kind() != io::ErrorKind::NotFound
	{
		tracing::error!("failed to remove {}: {err}", path.display());
	}
}

// ===== File type detection =====

/// An allowed file type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileKind {
	mime_type: &'static str,
	media_type: MediaType,
	extension: &'static str,
}

const fn kind(mime_type: &'static str, media_type: MediaType, extension: &'static str) -> FileKind {
	FileKind {
		mime_type,
		media_type,
		extension,
	}
}

const JPEG: FileKind = kind("image/jpeg", MediaType::Image, "jpg");
const PNG: FileKind = kind("image/png", MediaType::Image, "png");
const GIF: FileKind = kind("image/gif", MediaType::Image, "gif");
const WEBP: FileKind = kind("image/webp", MediaType::Image, "webp");
const HEIC: FileKind = kind("image/heic", MediaType::Image, "heic");
const MP4_VIDEO: FileKind = kind("video/mp4", MediaType::Video, "mp4");
const QUICKTIME: FileKind = kind("video/quicktime", MediaType::Video, "mov");
const WEBM_VIDEO: FileKind = kind("video/webm", MediaType::Video, "webm");
const MP3: FileKind = kind("audio/mpeg", MediaType::Audio, "mp3");
const MP4_AUDIO: FileKind = kind("audio/mp4", MediaType::Audio, "m4a");
const OGG: FileKind = kind("audio/ogg", MediaType::Audio, "ogg");
const WAV: FileKind = kind("audio/wav", MediaType::Audio, "wav");
const WEBM_AUDIO: FileKind = kind("audio/webm", MediaType::Audio, "weba");
const PDF: FileKind = kind("application/pdf", MediaType::Document, "pdf");

/// Every accepted type. SVG, HTML and other active content is deliberately absent.
const ALLOWED: &[FileKind] = &[
	JPEG, PNG, GIF, WEBP, HEIC, MP4_VIDEO, QUICKTIME, WEBM_VIDEO, MP3, MP4_AUDIO, OGG, WAV,
	WEBM_AUDIO, PDF,
];

#[derive(Debug, PartialEq, Eq)]
enum DetectError {
	/// The declared type isn't on the allowlist.
	NotAllowed(String),
	/// The bytes aren't any allowed format.
	Unrecognized,
	/// The bytes are an allowed format, but not the declared one.
	Mismatch(String),
}

/// Determines a file's type from its leading bytes. A declared type (other than
/// `application/octet-stream`) must agree with the content; it only picks between
/// interpretations of the same container, e.g. `audio/mp4` vs `video/mp4`.
fn detect(declared: Option<&str>, head: &[u8]) -> Result<FileKind, DetectError> {
	let candidates = sniff(head);
	let declared = declared
		.map(|mime| {
			mime.split(';')
				.next()
				.unwrap_or_default()
				.trim()
				.to_ascii_lowercase()
		})
		.filter(|mime| !mime.is_empty() && mime != "application/octet-stream");

	let Some(declared) = declared else {
		return candidates.first().copied().ok_or(DetectError::Unrecognized);
	};
	let canonical = canonical_mime(&declared);
	if !ALLOWED.iter().any(|kind| kind.mime_type == canonical) {
		return Err(DetectError::NotAllowed(declared));
	}
	candidates
		.iter()
		.find(|kind| kind.mime_type == canonical)
		.copied()
		.ok_or(DetectError::Mismatch(declared))
}

/// Maps common aliases onto the MIME types in [`ALLOWED`].
fn canonical_mime(mime: &str) -> &str {
	match mime {
		"image/jpg" | "image/pjpeg" => "image/jpeg",
		"image/heif" | "image/heic-sequence" | "image/heif-sequence" => "image/heic",
		"audio/mp3" | "audio/x-mp3" | "audio/mpeg3" | "audio/x-mpeg" => "audio/mpeg",
		"audio/x-m4a" | "audio/m4a" | "audio/aac" => "audio/mp4",
		"video/x-m4v" => "video/mp4",
		"audio/x-wav" | "audio/wave" | "audio/vnd.wave" => "audio/wav",
		"audio/vorbis" | "audio/opus" | "application/ogg" => "audio/ogg",
		other => other,
	}
}

/// The allowed types `head` could be, most likely first; empty if none.
fn sniff(head: &[u8]) -> &'static [FileKind] {
	let at = |offset: usize, magic: &[u8]| head.get(offset..offset + magic.len()) == Some(magic);

	if at(0, b"\xFF\xD8\xFF") {
		&[JPEG]
	} else if at(0, b"\x89PNG\r\n\x1A\n") {
		&[PNG]
	} else if at(0, b"GIF87a") || at(0, b"GIF89a") {
		&[GIF]
	} else if at(0, b"RIFF") && at(8, b"WEBP") {
		&[WEBP]
	} else if at(0, b"RIFF") && at(8, b"WAVE") {
		&[WAV]
	} else if at(0, b"%PDF-") {
		&[PDF]
	} else if at(0, b"OggS") {
		&[OGG]
	} else if at(0, b"\x1A\x45\xDF\xA3") {
		// Matroska/EBML; only the WebM profile is allowed.
		let header = &head[..head.len().min(64)];
		if header.windows(4).any(|window| window == b"webm") {
			&[WEBM_VIDEO, WEBM_AUDIO]
		} else {
			&[]
		}
	} else if at(4, b"ftyp") {
		sniff_iso_media(head)
	} else if at(4, b"moov") || at(4, b"mdat") || at(4, b"wide") {
		// Older QuickTime files have no `ftyp` box.
		&[QUICKTIME]
	} else if at(0, b"ID3") || is_mpeg_audio_frame(head) {
		&[MP3]
	} else {
		&[]
	}
}

/// ISO base media files (MP4, MOV, HEIC, M4A), told apart by their `ftyp` brands.
fn sniff_iso_media(head: &[u8]) -> &'static [FileKind] {
	let box_len = head.get(0..4).map_or(0, |len| {
		u32::from_be_bytes(len.try_into().unwrap()) as usize
	});
	let ftyp = &head[..head.len().min(box_len.max(16))];
	let major = &ftyp[8..12.min(ftyp.len())];
	// Compatible brands follow the major brand and minor version.
	let (compatible, _) = ftyp.get(16..).unwrap_or_default().as_chunks::<4>();

	const HEIC_BRANDS: &[&[u8]] = &[b"heic", b"heix", b"hevc", b"hevx", b"heim", b"heis"];
	match major {
		b"qt  " => &[QUICKTIME],
		b"M4A " | b"M4B " => &[MP4_AUDIO],
		b"mif1" | b"msf1" => {
			if compatible
				.iter()
				.any(|brand| HEIC_BRANDS.contains(&brand.as_slice()))
			{
				&[HEIC]
			} else {
				&[]
			}
		}
		major if HEIC_BRANDS.contains(&major) => &[HEIC],
		b"isom" | b"iso2" | b"iso3" | b"iso4" | b"iso5" | b"iso6" | b"mp41" | b"mp42" | b"avc1"
		| b"dash" | b"mmp4" | b"M4V " | b"MSNV" | b"f4v " => &[MP4_VIDEO, MP4_AUDIO],
		_ => &[],
	}
}

/// An MPEG-1/2 audio frame header (MP3 without an ID3 tag). Excludes AAC ADTS,
/// whose "layer" bits are zero.
fn is_mpeg_audio_frame(head: &[u8]) -> bool {
	match head {
		[0xFF, second, third, ..] => {
			second & 0xE0 == 0xE0
				&& second & 0x06 != 0
				&& second & 0x18 != 0x08
				&& third & 0xF0 != 0xF0
				&& third & 0x0C != 0x0C
		}
		_ => false,
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	const PNG_BYTES: &[u8] = b"\x89PNG\r\n\x1A\n\0\0\0\rIHDR";

	#[test]
	fn detects_types_from_content() {
		assert_eq!(detect(None, PNG_BYTES), Ok(PNG));
		assert_eq!(detect(Some("image/png"), PNG_BYTES), Ok(PNG));
		assert_eq!(detect(Some("application/octet-stream"), PNG_BYTES), Ok(PNG));
		assert_eq!(detect(Some("application/pdf"), b"%PDF-1.4\n"), Ok(PDF));
		assert_eq!(detect(None, b"\xFF\xD8\xFF\xE0\0\x10JFIF"), Ok(JPEG));
		assert_eq!(detect(Some("image/jpg"), b"\xFF\xD8\xFF\xE0"), Ok(JPEG));
		assert_eq!(detect(None, b"RIFF\0\0\0\0WAVEfmt "), Ok(WAV));
		assert_eq!(detect(None, b"ID3\x04\0\0"), Ok(MP3));
		assert_eq!(detect(None, b"\xFF\xFB\x90\x64"), Ok(MP3));
		assert_eq!(detect(None, b"OggS\0\x02"), Ok(OGG));
	}

	#[test]
	fn declared_type_picks_between_container_interpretations() {
		let mp4 = b"\0\0\0\x18ftypisom\0\0\x02\0isomiso2";
		assert_eq!(detect(None, mp4), Ok(MP4_VIDEO));
		assert_eq!(detect(Some("audio/mp4"), mp4), Ok(MP4_AUDIO));
		let m4a = b"\0\0\0\x14ftypM4A \0\0\0\0M4A ";
		assert_eq!(detect(Some("audio/x-m4a"), m4a), Ok(MP4_AUDIO));
		let mov = b"\0\0\0\x14ftypqt  \0\0\0\0qt  ";
		assert_eq!(detect(Some("video/quicktime"), mov), Ok(QUICKTIME));
		let heic = b"\0\0\0\x18ftypmif1\0\0\0\0mif1heic";
		assert_eq!(detect(Some("image/heic"), heic), Ok(HEIC));
		let avif = b"\0\0\0\x18ftypavif\0\0\0\0avifmif1";
		assert_eq!(detect(None, avif), Err(DetectError::Unrecognized));
		let webm = b"\x1A\x45\xDF\xA3\x9F\x42\x86\x81\x01\x42\x82\x84webm";
		assert_eq!(detect(Some("audio/webm"), webm), Ok(WEBM_AUDIO));
		assert_eq!(detect(None, webm), Ok(WEBM_VIDEO));
	}

	#[test]
	fn rejects_spoofed_and_disallowed_types() {
		assert_eq!(
			detect(Some("image/png"), b"%PDF-1.4"),
			Err(DetectError::Mismatch("image/png".into()))
		);
		assert_eq!(
			detect(Some("image/png"), b"not an image"),
			Err(DetectError::Mismatch("image/png".into()))
		);
		assert_eq!(
			detect(Some("image/svg+xml"), b"<svg/>"),
			Err(DetectError::NotAllowed("image/svg+xml".into()))
		);
		assert_eq!(
			detect(Some("text/html"), PNG_BYTES),
			Err(DetectError::NotAllowed("text/html".into()))
		);
		assert_eq!(detect(None, b"<html>"), Err(DetectError::Unrecognized));
		assert_eq!(
			detect(None, b"\xFF\xF1\x50\x80"),
			Err(DetectError::Unrecognized)
		);
	}

	#[test]
	fn sanitizes_file_names() {
		assert_eq!(
			sanitize_file_name("photo.png").as_deref(),
			Some("photo.png")
		);
		assert_eq!(
			sanitize_file_name("../../etc/passwd").as_deref(),
			Some("passwd")
		);
		assert_eq!(
			sanitize_file_name("C:\\Users\\ada\\cat.jpg").as_deref(),
			Some("cat.jpg")
		);
		assert_eq!(
			sanitize_file_name(" a\r\nb\0.pdf ").as_deref(),
			Some("ab.pdf")
		);
		assert_eq!(sanitize_file_name("dir/"), None);
		assert_eq!(sanitize_file_name("  "), None);
		let long = format!("{}.jpeg", "é".repeat(300));
		let name = sanitize_file_name(&long).unwrap();
		assert_eq!(name.chars().count(), MAX_FILE_NAME_CHARS);
		assert!(name.ends_with("é.jpeg"));
	}

	#[test]
	fn content_disposition_is_escaped() {
		assert_eq!(
			content_disposition("photo.png"),
			"inline; filename=\"photo.png\"; filename*=UTF-8''photo.png"
		);
		assert_eq!(
			content_disposition("a \"b\"\\ç.pdf"),
			"inline; filename=\"a _b___.pdf\"; filename*=UTF-8''a%20%22b%22%5C%C3%A7.pdf"
		);
	}

	#[test]
	fn storage_paths_cannot_escape_the_media_dir() {
		let dir = Path::new("/srv/media");
		assert_eq!(
			resolve_storage_path(dir, "abc.png").unwrap(),
			Path::new("/srv/media/abc.png")
		);
		for bad in ["../secret", "/etc/passwd", "a/b.png", "", ".", ".."] {
			assert!(resolve_storage_path(dir, bad).is_err(), "{bad}");
		}
	}
}
