//! Child development: dated records of what caregivers observed, grouped by domain.
//!
//! Records exist for CHILD profiles only and are purely descriptive: the API stores
//! observations as written and never interprets, scores or evaluates them.

use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use warp::{Filter, reply::Response};

use crate::{
	AppState, Routes,
	auth::{CurrentUser, authenticated},
	authz::{self, Access},
	created,
	error::{ApiError, ApiResult},
	json_body, no_content, ok,
	reference::{DevelopmentDomain, ProfileType, Role},
	respond, validate, with_state,
};

const MAX_NOTES_CHARS: usize = 10_000;
const MAX_OBSERVATION_CHARS: usize = 5_000;
const MAX_OBSERVATIONS: usize = 50;

/// The user who created a record or measurement.
#[derive(Debug, Clone, Serialize)]
pub struct Author {
	pub id: i64,
	pub first_name: String,
	pub last_name: String,
}

#[derive(Debug, Serialize)]
pub struct DevelopmentRecord {
	pub id: i64,
	pub profile_id: i64,
	pub record_date: NaiveDate,
	pub notes: Option<String>,
	pub created_by: Author,
	pub created_at: DateTime<Utc>,
	pub observations: Vec<Observation>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Observation {
	#[serde(skip)]
	development_record_id: i64,
	pub id: i64,
	pub domain: DevelopmentDomain,
	pub observation: String,
	pub created_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct RecordRow {
	id: i64,
	profile_id: i64,
	record_date: NaiveDate,
	notes: Option<String>,
	created_at: DateTime<Utc>,
	author_id: i64,
	author_first_name: String,
	author_last_name: String,
}

/// Development records joined with their author; `$tail` adds filters and ordering.
macro_rules! select_records {
	($tail:literal) => {
		concat!(
			"SELECT r.id, r.profile_id, r.record_date, r.notes, r.created_at,
				u.id AS author_id, u.first_name AS author_first_name, u.last_name AS author_last_name
			FROM development_records r
			JOIN users u ON u.id = r.created_by ",
			$tail
		)
	};
}

/// What a content handler needs to know about the profile it acts on.
pub(crate) struct ProfileAccess {
	pub role: Role,
	pub profile_type: ProfileType,
	pub date_of_birth: Option<NaiveDate>,
}

/// Requires `access` to a profile. Callers that reached the profile through a
/// record pass that record's name as `resource`, so a 404 names what was requested.
pub(crate) async fn profile_access(
	db: &PgPool,
	user: CurrentUser,
	profile_id: i64,
	access: Access,
	resource: &'static str,
) -> ApiResult<ProfileAccess> {
	let role = authz::profile_role(db, user, profile_id)
		.await?
		.ok_or(ApiError::NotFound(resource))?;
	if !role.allows(access) {
		return Err(ApiError::Forbidden);
	}
	let (profile_type, date_of_birth) =
		sqlx::query_as("SELECT profile_type_id, date_of_birth FROM profiles WHERE id = $1")
			.bind(profile_id)
			.fetch_one(db)
			.await?;
	Ok(ProfileAccess {
		role,
		profile_type,
		date_of_birth,
	})
}

/// Content edit rule: the creator may edit their own items while they keep write
/// access; anyone with manage access may edit everyone's.
pub(crate) fn require_editor(
	profile: &ProfileAccess,
	user: CurrentUser,
	created_by: i64,
) -> ApiResult<()> {
	if created_by == user.id || profile.role.allows(Access::Manage) {
		Ok(())
	} else {
		Err(ApiError::Forbidden)
	}
}

/// Event dates must not be in the future nor before the profile's date of birth.
pub(crate) fn validate_event_date(
	field: &str,
	date: NaiveDate,
	date_of_birth: Option<NaiveDate>,
) -> ApiResult<()> {
	validate::not_in_future(field, Some(date))?;
	match date_of_birth {
		Some(birth) if date < birth => Err(ApiError::bad_request(format!(
			"{field} must not be before the profile's date_of_birth ({birth})"
		))),
		_ => Ok(()),
	}
}

fn require_child(profile: &ProfileAccess) -> ApiResult<()> {
	if profile.profile_type != ProfileType::Child {
		return Err(ApiError::bad_request(
			"development records are only available for CHILD profiles",
		));
	}
	Ok(())
}

pub fn routes(state: &AppState) -> Routes {
	let list = warp::path!("profiles" / i64 / "development-records")
		.and(warp::get())
		.and(with_state(state))
		.and(authenticated(state))
		.and(warp::query())
		.then(|profile_id, state, user, query| async move {
			respond(list(state, user, profile_id, query).await)
		});

	let create = warp::path!("profiles" / i64 / "development-records")
		.and(warp::post())
		.and(with_state(state))
		.and(authenticated(state))
		.and(json_body())
		.then(|profile_id, state, user, body| async move {
			respond(create(state, user, profile_id, body).await)
		});

	let get = warp::path!("development-records" / i64)
		.and(warp::get())
		.and(with_state(state))
		.and(authenticated(state))
		.then(|id, state, user| async move { respond(get(state, user, id).await) });

	let update = warp::path!("development-records" / i64)
		.and(warp::put())
		.and(with_state(state))
		.and(authenticated(state))
		.and(json_body())
		.then(|id, state, user, body| async move { respond(update(state, user, id, body).await) });

	let delete = warp::path!("development-records" / i64)
		.and(warp::delete())
		.and(with_state(state))
		.and(authenticated(state))
		.then(|id, state, user| async move { respond(delete(state, user, id).await) });

	list.or(create)
		.unify()
		.or(get)
		.unify()
		.or(update)
		.unify()
		.or(delete)
		.unify()
		.boxed()
}

/// Attaches observations to records with one query for all of them.
async fn with_observations(db: &PgPool, rows: Vec<RecordRow>) -> ApiResult<Vec<DevelopmentRecord>> {
	let ids: Vec<i64> = rows.iter().map(|row| row.id).collect();
	let observations: Vec<Observation> = sqlx::query_as(
		"SELECT development_record_id, id, domain_id AS domain, observation, created_at
		FROM development_observations
		WHERE development_record_id = ANY($1)
		ORDER BY id",
	)
	.bind(&ids)
	.fetch_all(db)
	.await?;

	let mut by_record: HashMap<i64, Vec<Observation>> = HashMap::new();
	for observation in observations {
		by_record
			.entry(observation.development_record_id)
			.or_default()
			.push(observation);
	}

	Ok(rows
		.into_iter()
		.map(|row| DevelopmentRecord {
			observations: by_record.remove(&row.id).unwrap_or_default(),
			id: row.id,
			profile_id: row.profile_id,
			record_date: row.record_date,
			notes: row.notes,
			created_by: Author {
				id: row.author_id,
				first_name: row.author_first_name,
				last_name: row.author_last_name,
			},
			created_at: row.created_at,
		})
		.collect())
}

/// Loads a record without any access check.
async fn fetch(db: &PgPool, id: i64) -> ApiResult<DevelopmentRecord> {
	let row: RecordRow = sqlx::query_as(select_records!("WHERE r.id = $1"))
		.bind(id)
		.fetch_optional(db)
		.await?
		.ok_or(ApiError::NotFound("development record"))?;
	Ok(with_observations(db, vec![row]).await?.remove(0))
}

/// The owning profile and creator of a record, for authorization.
async fn record_owner(db: &PgPool, id: i64) -> ApiResult<(i64, i64)> {
	sqlx::query_as("SELECT profile_id, created_by FROM development_records WHERE id = $1")
		.bind(id)
		.fetch_optional(db)
		.await?
		.ok_or(ApiError::NotFound("development record"))
}

#[derive(Deserialize)]
struct ListQuery {
	from: Option<NaiveDate>,
	to: Option<NaiveDate>,
	domain: Option<DevelopmentDomain>,
}

async fn list(
	state: AppState,
	user: CurrentUser,
	profile_id: i64,
	query: ListQuery,
) -> ApiResult<Response> {
	let profile = profile_access(&state.db, user, profile_id, Access::Read, "profile").await?;
	require_child(&profile)?;
	validate::date_order("from", query.from, "to", query.to)?;

	let rows: Vec<RecordRow> = sqlx::query_as(select_records!(
		"WHERE r.profile_id = $1
			AND ($2::date IS NULL OR r.record_date >= $2)
			AND ($3::date IS NULL OR r.record_date <= $3)
			AND ($4::smallint IS NULL OR EXISTS (
				SELECT 1 FROM development_observations o
				WHERE o.development_record_id = r.id AND o.domain_id = $4
			))
		ORDER BY r.record_date DESC, r.id DESC"
	))
	.bind(profile_id)
	.bind(query.from)
	.bind(query.to)
	.bind(query.domain)
	.fetch_all(&state.db)
	.await?;
	ok(&with_observations(&state.db, rows).await?)
}

#[derive(Deserialize)]
struct RecordRequest {
	record_date: NaiveDate,
	notes: Option<String>,
	#[serde(default)]
	observations: Vec<ObservationRequest>,
}

#[derive(Deserialize)]
struct ObservationRequest {
	domain: DevelopmentDomain,
	observation: String,
}

struct ValidRecord {
	record_date: NaiveDate,
	notes: Option<String>,
	domains: Vec<i16>,
	observations: Vec<String>,
}

fn validate_record(body: RecordRequest, profile: &ProfileAccess) -> ApiResult<ValidRecord> {
	validate_event_date("record_date", body.record_date, profile.date_of_birth)?;
	let notes = validate::optional_text("notes", body.notes.as_deref(), MAX_NOTES_CHARS)?;
	if body.observations.len() > MAX_OBSERVATIONS {
		return Err(ApiError::bad_request(format!(
			"a record may have at most {MAX_OBSERVATIONS} observations"
		)));
	}
	if notes.is_none() && body.observations.is_empty() {
		return Err(ApiError::bad_request(
			"a record needs notes or at least one observation",
		));
	}

	let mut domains = Vec::with_capacity(body.observations.len());
	let mut observations = Vec::with_capacity(body.observations.len());
	for item in &body.observations {
		domains.push(item.domain as i16);
		observations.push(validate::required_text(
			"observation",
			&item.observation,
			MAX_OBSERVATION_CHARS,
		)?);
	}
	Ok(ValidRecord {
		record_date: body.record_date,
		notes,
		domains,
		observations,
	})
}

async fn insert_observations(
	conn: &mut sqlx::PgConnection,
	record_id: i64,
	record: &ValidRecord,
) -> ApiResult<()> {
	sqlx::query(
		"INSERT INTO development_observations (development_record_id, domain_id, observation)
		SELECT $1, domain_id, observation
		FROM UNNEST($2::smallint[], $3::text[]) WITH ORDINALITY AS o (domain_id, observation, n)
		ORDER BY n",
	)
	.bind(record_id)
	.bind(&record.domains)
	.bind(&record.observations)
	.execute(conn)
	.await?;
	Ok(())
}

async fn create(
	state: AppState,
	user: CurrentUser,
	profile_id: i64,
	body: RecordRequest,
) -> ApiResult<Response> {
	let profile = profile_access(&state.db, user, profile_id, Access::Write, "profile").await?;
	require_child(&profile)?;
	let record = validate_record(body, &profile)?;

	let mut tx = state.db.begin().await?;
	let id: i64 = sqlx::query_scalar(
		"INSERT INTO development_records (profile_id, record_date, created_by, notes)
		VALUES ($1, $2, $3, $4) RETURNING id",
	)
	.bind(profile_id)
	.bind(record.record_date)
	.bind(user.id)
	.bind(&record.notes)
	.fetch_one(&mut *tx)
	.await?;
	insert_observations(&mut tx, id, &record).await?;
	tx.commit().await?;

	created(&fetch(&state.db, id).await?)
}

async fn get(state: AppState, user: CurrentUser, id: i64) -> ApiResult<Response> {
	let (profile_id, _) = record_owner(&state.db, id).await?;
	profile_access(
		&state.db,
		user,
		profile_id,
		Access::Read,
		"development record",
	)
	.await?;
	ok(&fetch(&state.db, id).await?)
}

/// Replaces the date, notes and the whole list of observations.
async fn update(
	state: AppState,
	user: CurrentUser,
	id: i64,
	body: RecordRequest,
) -> ApiResult<Response> {
	let (profile_id, created_by) = record_owner(&state.db, id).await?;
	let profile = profile_access(
		&state.db,
		user,
		profile_id,
		Access::Write,
		"development record",
	)
	.await?;
	require_editor(&profile, user, created_by)?;
	let record = validate_record(body, &profile)?;

	let mut tx = state.db.begin().await?;
	sqlx::query("UPDATE development_records SET record_date = $1, notes = $2 WHERE id = $3")
		.bind(record.record_date)
		.bind(&record.notes)
		.bind(id)
		.execute(&mut *tx)
		.await?;
	sqlx::query("DELETE FROM development_observations WHERE development_record_id = $1")
		.bind(id)
		.execute(&mut *tx)
		.await?;
	insert_observations(&mut tx, id, &record).await?;
	tx.commit().await?;

	ok(&fetch(&state.db, id).await?)
}

async fn delete(state: AppState, user: CurrentUser, id: i64) -> ApiResult<Response> {
	let (profile_id, created_by) = record_owner(&state.db, id).await?;
	let profile = profile_access(
		&state.db,
		user,
		profile_id,
		Access::Write,
		"development record",
	)
	.await?;
	require_editor(&profile, user, created_by)?;

	sqlx::query("DELETE FROM development_records WHERE id = $1")
		.bind(id)
		.execute(&state.db)
		.await?;
	no_content()
}
