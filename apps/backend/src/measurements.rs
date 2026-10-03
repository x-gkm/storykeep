//! Dated numeric measurements (height, weight, ...) for any profile, plus chart series.
//!
//! Values are stored and returned as recorded; there is no percentile or other
//! interpretation.

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use warp::{Filter, reply::Response};

use crate::{
	AppState, Routes,
	auth::{CurrentUser, authenticated},
	authz::Access,
	created,
	development::{Author, profile_access, require_editor, validate_event_date},
	error::{ApiError, ApiResult},
	json_body, no_content, ok,
	reference::MeasurementType,
	respond, validate, with_state,
};

/// `NUMERIC(10, 3)`: at most 7 integer digits and 3 decimal places.
const MAX_SCALE: u32 = 3;
const VALUE_LIMIT: i64 = 10_000_000;

#[derive(Debug, Serialize)]
pub struct Measurement {
	pub id: i64,
	pub profile_id: i64,
	pub measurement_type: MeasurementType,
	pub unit: String,
	/// Sent as a JSON number.
	#[serde(serialize_with = "rust_decimal::serde::float::serialize")]
	pub value: Decimal,
	pub measurement_date: NaiveDate,
	pub created_by: Author,
	pub created_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct MeasurementRow {
	id: i64,
	profile_id: i64,
	measurement_type: MeasurementType,
	unit: String,
	value: Decimal,
	measurement_date: NaiveDate,
	created_at: DateTime<Utc>,
	author_id: i64,
	author_first_name: String,
	author_last_name: String,
}

impl From<MeasurementRow> for Measurement {
	fn from(row: MeasurementRow) -> Self {
		Self {
			id: row.id,
			profile_id: row.profile_id,
			measurement_type: row.measurement_type,
			unit: row.unit,
			value: row.value,
			measurement_date: row.measurement_date,
			created_by: Author {
				id: row.author_id,
				first_name: row.author_first_name,
				last_name: row.author_last_name,
			},
			created_at: row.created_at,
		}
	}
}

/// Measurements joined with their type's unit and author; `$tail` adds filters and ordering.
macro_rules! select_measurements {
	($tail:literal) => {
		concat!(
			"SELECT m.id, m.profile_id, m.measurement_type_id AS measurement_type, t.unit, m.value,
				m.measurement_date, m.created_at,
				u.id AS author_id, u.first_name AS author_first_name, u.last_name AS author_last_name
			FROM measurements m
			JOIN measurement_types t ON t.id = m.measurement_type_id
			JOIN users u ON u.id = m.created_by ",
			$tail
		)
	};
}

#[derive(Serialize)]
struct Series {
	measurement_type: MeasurementType,
	unit: String,
	points: Vec<Point>,
}

#[derive(Serialize)]
struct Point {
	date: NaiveDate,
	#[serde(serialize_with = "rust_decimal::serde::float::serialize")]
	value: Decimal,
}

pub fn routes(state: &AppState) -> Routes {
	let list = warp::path!("profiles" / i64 / "measurements")
		.and(warp::get())
		.and(with_state(state))
		.and(authenticated(state))
		.and(warp::query())
		.then(|profile_id, state, user, query| async move {
			respond(list(state, user, profile_id, query).await)
		});

	let create = warp::path!("profiles" / i64 / "measurements")
		.and(warp::post())
		.and(with_state(state))
		.and(authenticated(state))
		.and(json_body())
		.then(|profile_id, state, user, body| async move {
			respond(create(state, user, profile_id, body).await)
		});

	let series = warp::path!("profiles" / i64 / "measurements" / "series")
		.and(warp::get())
		.and(with_state(state))
		.and(authenticated(state))
		.and(warp::query())
		.then(|profile_id, state, user, query| async move {
			respond(series(state, user, profile_id, query).await)
		});

	let get = warp::path!("measurements" / i64)
		.and(warp::get())
		.and(with_state(state))
		.and(authenticated(state))
		.then(|id, state, user| async move { respond(get(state, user, id).await) });

	let update = warp::path!("measurements" / i64)
		.and(warp::put())
		.and(with_state(state))
		.and(authenticated(state))
		.and(json_body())
		.then(|id, state, user, body| async move { respond(update(state, user, id, body).await) });

	let delete = warp::path!("measurements" / i64)
		.and(warp::delete())
		.and(with_state(state))
		.and(authenticated(state))
		.then(|id, state, user| async move { respond(delete(state, user, id).await) });

	list.or(create)
		.unify()
		.or(series)
		.unify()
		.or(get)
		.unify()
		.or(update)
		.unify()
		.or(delete)
		.unify()
		.boxed()
}

/// Loads a measurement without any access check.
async fn fetch(db: &PgPool, id: i64) -> ApiResult<Measurement> {
	let row: MeasurementRow = sqlx::query_as(select_measurements!("WHERE m.id = $1"))
		.bind(id)
		.fetch_optional(db)
		.await?
		.ok_or(ApiError::NotFound("measurement"))?;
	Ok(row.into())
}

/// The owning profile and creator of a measurement, for authorization.
async fn measurement_owner(db: &PgPool, id: i64) -> ApiResult<(i64, i64)> {
	sqlx::query_as("SELECT profile_id, created_by FROM measurements WHERE id = $1")
		.bind(id)
		.fetch_optional(db)
		.await?
		.ok_or(ApiError::NotFound("measurement"))
}

#[derive(Deserialize)]
struct ListQuery {
	#[serde(rename = "type")]
	measurement_type: Option<MeasurementType>,
	from: Option<NaiveDate>,
	to: Option<NaiveDate>,
}

async fn list(
	state: AppState,
	user: CurrentUser,
	profile_id: i64,
	query: ListQuery,
) -> ApiResult<Response> {
	profile_access(&state.db, user, profile_id, Access::Read, "profile").await?;
	validate::date_order("from", query.from, "to", query.to)?;

	let rows: Vec<MeasurementRow> = sqlx::query_as(select_measurements!(
		"WHERE m.profile_id = $1
			AND ($2::smallint IS NULL OR m.measurement_type_id = $2)
			AND ($3::date IS NULL OR m.measurement_date >= $3)
			AND ($4::date IS NULL OR m.measurement_date <= $4)
		ORDER BY m.measurement_date, m.measurement_type_id, m.id"
	))
	.bind(profile_id)
	.bind(query.measurement_type)
	.bind(query.from)
	.bind(query.to)
	.fetch_all(&state.db)
	.await?;
	ok(&rows.into_iter().map(Measurement::from).collect::<Vec<_>>())
}

/// With `type`: one series (possibly with no points). Without: one series per type that has data.
async fn series(
	state: AppState,
	user: CurrentUser,
	profile_id: i64,
	query: ListQuery,
) -> ApiResult<Response> {
	profile_access(&state.db, user, profile_id, Access::Read, "profile").await?;
	validate::date_order("from", query.from, "to", query.to)?;

	let rows: Vec<(MeasurementType, String, NaiveDate, Decimal)> = sqlx::query_as(
		"SELECT m.measurement_type_id, t.unit, m.measurement_date, m.value
		FROM measurements m
		JOIN measurement_types t ON t.id = m.measurement_type_id
		WHERE m.profile_id = $1
			AND ($2::smallint IS NULL OR m.measurement_type_id = $2)
			AND ($3::date IS NULL OR m.measurement_date >= $3)
			AND ($4::date IS NULL OR m.measurement_date <= $4)
		ORDER BY m.measurement_type_id, m.measurement_date, m.id",
	)
	.bind(profile_id)
	.bind(query.measurement_type)
	.bind(query.from)
	.bind(query.to)
	.fetch_all(&state.db)
	.await?;

	let mut all: Vec<Series> = Vec::new();
	for (measurement_type, unit, date, value) in rows {
		match all.last_mut() {
			Some(series) if series.measurement_type == measurement_type => {}
			_ => all.push(Series {
				measurement_type,
				unit,
				points: Vec::new(),
			}),
		}
		all.last_mut()
			.expect("a series was just ensured")
			.points
			.push(Point { date, value });
	}

	let Some(measurement_type) = query.measurement_type else {
		return ok(&all);
	};
	match all.pop() {
		Some(series) => ok(&series),
		None => {
			let unit: String =
				sqlx::query_scalar("SELECT unit FROM measurement_types WHERE id = $1")
					.bind(measurement_type)
					.fetch_one(&state.db)
					.await?;
			ok(&Series {
				measurement_type,
				unit,
				points: Vec::new(),
			})
		}
	}
}

#[derive(Deserialize)]
struct MeasurementRequest {
	measurement_type: MeasurementType,
	value: Decimal,
	measurement_date: NaiveDate,
}

fn validate_value(value: Decimal) -> ApiResult<Decimal> {
	if value <= Decimal::ZERO {
		return Err(ApiError::bad_request("value must be greater than 0"));
	}
	let value = value.normalize();
	if value.scale() > MAX_SCALE {
		return Err(ApiError::bad_request(format!(
			"value must have at most {MAX_SCALE} decimal places"
		)));
	}
	if value >= Decimal::from(VALUE_LIMIT) {
		return Err(ApiError::bad_request(format!(
			"value must be less than {VALUE_LIMIT}"
		)));
	}
	Ok(value)
}

async fn create(
	state: AppState,
	user: CurrentUser,
	profile_id: i64,
	body: MeasurementRequest,
) -> ApiResult<Response> {
	let profile = profile_access(&state.db, user, profile_id, Access::Write, "profile").await?;
	let value = validate_value(body.value)?;
	validate_event_date(
		"measurement_date",
		body.measurement_date,
		profile.date_of_birth,
	)?;

	let id: i64 = sqlx::query_scalar(
		"INSERT INTO measurements (profile_id, measurement_type_id, value, measurement_date, created_by)
		VALUES ($1, $2, $3, $4, $5) RETURNING id",
	)
	.bind(profile_id)
	.bind(body.measurement_type)
	.bind(value)
	.bind(body.measurement_date)
	.bind(user.id)
	.fetch_one(&state.db)
	.await?;
	created(&fetch(&state.db, id).await?)
}

async fn get(state: AppState, user: CurrentUser, id: i64) -> ApiResult<Response> {
	let (profile_id, _) = measurement_owner(&state.db, id).await?;
	profile_access(&state.db, user, profile_id, Access::Read, "measurement").await?;
	ok(&fetch(&state.db, id).await?)
}

async fn update(
	state: AppState,
	user: CurrentUser,
	id: i64,
	body: MeasurementRequest,
) -> ApiResult<Response> {
	let (profile_id, created_by) = measurement_owner(&state.db, id).await?;
	let profile = profile_access(&state.db, user, profile_id, Access::Write, "measurement").await?;
	require_editor(&profile, user, created_by)?;
	let value = validate_value(body.value)?;
	validate_event_date(
		"measurement_date",
		body.measurement_date,
		profile.date_of_birth,
	)?;

	sqlx::query(
		"UPDATE measurements SET measurement_type_id = $1, value = $2, measurement_date = $3
		WHERE id = $4",
	)
	.bind(body.measurement_type)
	.bind(value)
	.bind(body.measurement_date)
	.bind(id)
	.execute(&state.db)
	.await?;
	ok(&fetch(&state.db, id).await?)
}

async fn delete(state: AppState, user: CurrentUser, id: i64) -> ApiResult<Response> {
	let (profile_id, created_by) = measurement_owner(&state.db, id).await?;
	let profile = profile_access(&state.db, user, profile_id, Access::Write, "measurement").await?;
	require_editor(&profile, user, created_by)?;

	sqlx::query("DELETE FROM measurements WHERE id = $1")
		.bind(id)
		.execute(&state.db)
		.await?;
	no_content()
}
