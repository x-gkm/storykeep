//! Profiles: the people or animals that relationships are about.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgExecutor;
use warp::{Filter, reply::Response};

use crate::{
	AppState, Routes,
	auth::{CurrentUser, authenticated},
	authz::{self, Access},
	created,
	error::{ApiError, ApiResult},
	json_body, no_content, ok,
	reference::{ProfileType, RelationshipType, Role},
	relationships::{self, Relationship},
	respond, validate, with_state,
};

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Profile {
	pub id: i64,
	pub profile_type: ProfileType,
	pub name: String,
	pub date_of_birth: Option<NaiveDate>,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
	/// The caller's strongest role across their relationships with this profile.
	pub role: Role,
}

/// Profiles visible to user `$1`, with their strongest role; `$tail` adds filters and grouping.
macro_rules! select_profiles {
	($tail:literal) => {
		concat!(
			"SELECT p.id, p.profile_type_id AS profile_type, p.name, p.date_of_birth,
				p.created_at, p.updated_at, min(m.role_id) AS role
			FROM profiles p
			JOIN relationships r ON r.profile_id = p.id
			JOIN relationship_members m ON m.relationship_id = r.id
			WHERE m.user_id = $1",
			$tail
		)
	};
}

pub async fn fetch<'e>(db: impl PgExecutor<'e>, user: CurrentUser, id: i64) -> ApiResult<Profile> {
	sqlx::query_as(select_profiles!(" AND p.id = $2 GROUP BY p.id"))
		.bind(user.id)
		.bind(id)
		.fetch_optional(db)
		.await?
		.ok_or(ApiError::NotFound("profile"))
}

pub fn routes(state: &AppState) -> Routes {
	let list = warp::path!("profiles")
		.and(warp::get())
		.and(with_state(state))
		.and(authenticated(state))
		.then(|state, user| async move { respond(list(state, user).await) });

	let create = warp::path!("profiles")
		.and(warp::post())
		.and(with_state(state))
		.and(authenticated(state))
		.and(json_body())
		.then(|state, user, body| async move { respond(create(state, user, body).await) });

	let get = warp::path!("profiles" / i64)
		.and(warp::get())
		.and(with_state(state))
		.and(authenticated(state))
		.then(|id, state: AppState, user| async move {
			respond(
				fetch(&state.db, user, id)
					.await
					.and_then(|profile| ok(&profile)),
			)
		});

	let update = warp::path!("profiles" / i64)
		.and(warp::put())
		.and(with_state(state))
		.and(authenticated(state))
		.and(json_body())
		.then(|id, state, user, body| async move { respond(update(state, user, id, body).await) });

	let delete = warp::path!("profiles" / i64)
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

async fn list(state: AppState, user: CurrentUser) -> ApiResult<Response> {
	let profiles: Vec<Profile> =
		sqlx::query_as(select_profiles!(" GROUP BY p.id ORDER BY p.name, p.id"))
			.bind(user.id)
			.fetch_all(&state.db)
			.await?;
	ok(&profiles)
}

#[derive(Deserialize)]
struct CreateProfileRequest {
	profile_type: ProfileType,
	name: String,
	date_of_birth: Option<NaiveDate>,
	/// The caller's relationship to the new profile; they become its OWNER.
	relationship_type: RelationshipType,
	started_at: Option<NaiveDate>,
}

#[derive(Serialize)]
struct CreateProfileResponse {
	profile: Profile,
	relationship: Relationship,
}

async fn create(
	state: AppState,
	user: CurrentUser,
	body: CreateProfileRequest,
) -> ApiResult<Response> {
	let name = validate::required_text("name", &body.name, 100)?;

	let mut tx = state.db.begin().await?;
	let profile_id: i64 = sqlx::query_scalar(
		"INSERT INTO profiles (profile_type_id, name, date_of_birth) VALUES ($1, $2, $3) RETURNING id",
	)
	.bind(body.profile_type)
	.bind(name)
	.bind(body.date_of_birth)
	.fetch_one(&mut *tx)
	.await?;
	let relationship_id = relationships::insert(
		&mut tx,
		user,
		profile_id,
		body.relationship_type,
		body.started_at,
		None,
	)
	.await?;

	let response = CreateProfileResponse {
		profile: fetch(&mut *tx, user, profile_id).await?,
		relationship: relationships::fetch(&mut *tx, user, relationship_id).await?,
	};
	tx.commit().await?;
	created(&response)
}

#[derive(Deserialize)]
struct UpdateProfileRequest {
	profile_type: ProfileType,
	name: String,
	date_of_birth: Option<NaiveDate>,
}

async fn update(
	state: AppState,
	user: CurrentUser,
	id: i64,
	body: UpdateProfileRequest,
) -> ApiResult<Response> {
	authz::require_profile(&state.db, user, id, Access::Manage).await?;
	let name = validate::required_text("name", &body.name, 100)?;

	sqlx::query(
		"UPDATE profiles SET profile_type_id = $1, name = $2, date_of_birth = $3 WHERE id = $4",
	)
	.bind(body.profile_type)
	.bind(name)
	.bind(body.date_of_birth)
	.bind(id)
	.execute(&state.db)
	.await?;
	ok(&fetch(&state.db, user, id).await?)
}

/// Deleting a profile removes every relationship with it, so the caller must own all of them.
async fn delete(state: AppState, user: CurrentUser, id: i64) -> ApiResult<Response> {
	authz::require_profile(&state.db, user, id, Access::Own).await?;

	let owns_all: bool = sqlx::query_scalar(
		"SELECT NOT EXISTS (
			SELECT 1 FROM relationships r
			WHERE r.profile_id = $1
				AND NOT EXISTS (
					SELECT 1 FROM relationship_members m
					WHERE m.relationship_id = r.id AND m.user_id = $2 AND m.role_id = $3
				)
		)",
	)
	.bind(id)
	.bind(user.id)
	.bind(Role::Owner)
	.fetch_one(&state.db)
	.await?;
	if !owns_all {
		return Err(ApiError::Forbidden);
	}

	sqlx::query("DELETE FROM profiles WHERE id = $1")
		.bind(id)
		.execute(&state.db)
		.await?;
	no_content()
}
