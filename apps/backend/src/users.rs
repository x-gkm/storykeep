//! The authenticated user's own account.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgExecutor;
use warp::{Filter, reply::Response};

use crate::{
	AppState, Routes,
	auth::{self, CurrentUser, authenticated},
	error::{ApiError, ApiResult},
	json_body, no_content, ok, respond, validate, with_state,
};

/// Public view of a user account; never includes the password hash.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct User {
	pub id: i64,
	pub email: String,
	pub first_name: String,
	pub last_name: String,
	pub date_of_birth: Option<NaiveDate>,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}

pub async fn fetch<'e>(db: impl PgExecutor<'e>, id: i64) -> ApiResult<User> {
	sqlx::query_as(
		"SELECT id, email, first_name, last_name, date_of_birth, created_at, updated_at
		FROM users WHERE id = $1",
	)
	.bind(id)
	.fetch_optional(db)
	.await?
	.ok_or(ApiError::NotFound("user"))
}

pub fn routes(state: &AppState) -> Routes {
	let me = warp::path!("users" / "me");

	let get_me = me
		.and(warp::get())
		.and(with_state(state))
		.and(authenticated(state))
		.then(|state: AppState, user: CurrentUser| async move {
			respond(fetch(&state.db, user.id).await.and_then(|user| ok(&user)))
		});

	let update_me = me
		.and(warp::put())
		.and(with_state(state))
		.and(authenticated(state))
		.and(json_body())
		.then(|state, user, body| async move { respond(update_me(state, user, body).await) });

	let change_password = warp::path!("users" / "me" / "password")
		.and(warp::put())
		.and(with_state(state))
		.and(authenticated(state))
		.and(json_body())
		.then(|state, user, body| async move { respond(change_password(state, user, body).await) });

	get_me
		.or(update_me)
		.unify()
		.or(change_password)
		.unify()
		.boxed()
}

#[derive(Deserialize)]
struct UpdateMeRequest {
	first_name: String,
	last_name: String,
	date_of_birth: Option<NaiveDate>,
}

async fn update_me(
	state: AppState,
	user: CurrentUser,
	body: UpdateMeRequest,
) -> ApiResult<Response> {
	let first_name = validate::required_text("first_name", &body.first_name, 100)?;
	let last_name = validate::required_text("last_name", &body.last_name, 100)?;
	validate::not_in_future("date_of_birth", body.date_of_birth)?;

	sqlx::query(
		"UPDATE users SET first_name = $1, last_name = $2, date_of_birth = $3 WHERE id = $4",
	)
	.bind(first_name)
	.bind(last_name)
	.bind(body.date_of_birth)
	.bind(user.id)
	.execute(&state.db)
	.await?;
	ok(&fetch(&state.db, user.id).await?)
}

#[derive(Deserialize)]
struct ChangePasswordRequest {
	current_password: String,
	new_password: String,
}

async fn change_password(
	state: AppState,
	user: CurrentUser,
	body: ChangePasswordRequest,
) -> ApiResult<Response> {
	auth::validate_password(&body.new_password)?;

	let stored_hash: String = sqlx::query_scalar("SELECT password_hash FROM users WHERE id = $1")
		.bind(user.id)
		.fetch_one(&state.db)
		.await?;
	if !auth::verify_password(body.current_password, stored_hash).await? {
		return Err(ApiError::InvalidCredentials);
	}

	let new_hash = auth::hash_password(body.new_password).await?;
	sqlx::query("UPDATE users SET password_hash = $1 WHERE id = $2")
		.bind(new_hash)
		.bind(user.id)
		.execute(&state.db)
		.await?;
	// Anyone holding an old session (e.g. on a lost device) is signed out.
	auth::end_other_sessions(&state.db, user).await?;
	no_content()
}
