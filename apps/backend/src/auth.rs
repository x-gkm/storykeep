//! Registration, login and bearer-token sessions.

use std::sync::LazyLock;

use anyhow::Context;
use argon2::{
	Argon2,
	password_hash::{PasswordHasher, PasswordVerifier, phc::PasswordHash},
};
use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, PgPool};
use warp::{Filter, Rejection, reply::Response};

use crate::{
	AppState, Routes, created,
	error::{ApiError, ApiResult},
	json_body, no_content, ok, respond,
	users::{self, User},
	validate, with_state,
};

const SESSION_LIFETIME: Duration = Duration::days(30);
const MIN_PASSWORD_CHARS: usize = 8;
const MAX_PASSWORD_CHARS: usize = 128;

/// The authenticated caller, extracted by [`authenticated`].
#[derive(Debug, Clone, Copy)]
pub struct CurrentUser {
	pub id: i64,
	session_id: i64,
}

/// Requires a valid `Authorization: Bearer <token>` header. Place it after the
/// path filters so unknown routes still return 404 rather than 401.
pub fn authenticated(
	state: &AppState,
) -> impl Filter<Extract = (CurrentUser,), Error = Rejection> + Clone + use<> {
	warp::header::optional::<String>("authorization")
		.and(with_state(state))
		.and_then(|header: Option<String>, state: AppState| async move {
			let token = header
				.as_deref()
				.and_then(|value| value.split_once(' '))
				.filter(|(scheme, _)| scheme.eq_ignore_ascii_case("bearer"))
				.map(|(_, token)| token.trim())
				.ok_or(ApiError::Unauthorized)?;

			let session: Option<(i64, i64)> = sqlx::query_as(
				"SELECT id, user_id FROM sessions WHERE token_hash = $1 AND expires_at > now()",
			)
			.bind(hash_token(token))
			.fetch_optional(&state.db)
			.await
			.map_err(ApiError::from)?;

			let (session_id, id) = session.ok_or(ApiError::Unauthorized)?;
			Ok::<_, Rejection>(CurrentUser { id, session_id })
		})
}

pub fn routes(state: &AppState) -> Routes {
	let register = warp::path!("auth" / "register")
		.and(warp::post())
		.and(with_state(state))
		.and(json_body())
		.then(|state, body| async move { respond(register(state, body).await) });

	let login = warp::path!("auth" / "login")
		.and(warp::post())
		.and(with_state(state))
		.and(json_body())
		.then(|state, body| async move { respond(login(state, body).await) });

	let logout = warp::path!("auth" / "logout")
		.and(warp::post())
		.and(with_state(state))
		.and(authenticated(state))
		.then(|state, user| async move { respond(logout(state, user).await) });

	register.or(login).unify().or(logout).unify().boxed()
}

#[derive(Deserialize)]
struct RegisterRequest {
	email: String,
	password: String,
	first_name: String,
	last_name: String,
	date_of_birth: Option<NaiveDate>,
}

#[derive(Deserialize)]
struct LoginRequest {
	email: String,
	password: String,
}

#[derive(Serialize)]
struct SessionResponse {
	token: String,
	expires_at: DateTime<Utc>,
	user: User,
}

async fn register(state: AppState, body: RegisterRequest) -> ApiResult<Response> {
	let email = normalize_email(&body.email)?;
	validate_password(&body.password)?;
	let first_name = validate::required_text("first_name", &body.first_name, 100)?;
	let last_name = validate::required_text("last_name", &body.last_name, 100)?;
	validate::not_in_future("date_of_birth", body.date_of_birth)?;

	let password_hash = hash_password(body.password).await?;

	let mut tx = state.db.begin().await?;
	let user_id: i64 = sqlx::query_scalar(
		"INSERT INTO users (email, password_hash, first_name, last_name, date_of_birth)
		VALUES ($1, $2, $3, $4, $5)
		ON CONFLICT DO NOTHING
		RETURNING id",
	)
	.bind(&email)
	.bind(&password_hash)
	.bind(&first_name)
	.bind(&last_name)
	.bind(body.date_of_birth)
	.fetch_optional(&mut *tx)
	.await?
	.ok_or_else(|| ApiError::conflict("an account with this email already exists"))?;

	let session = start_session(&mut tx, user_id).await?;
	tx.commit().await?;
	created(&session)
}

async fn login(state: AppState, body: LoginRequest) -> ApiResult<Response> {
	let row: Option<(i64, String)> =
		sqlx::query_as("SELECT id, password_hash FROM users WHERE lower(email) = lower($1)")
			.bind(body.email.trim())
			.fetch_optional(&state.db)
			.await?;

	// Verify against a dummy hash when the email is unknown so response timing
	// doesn't reveal which emails have accounts.
	let (user_id, stored_hash) = match row {
		Some((id, hash)) => (Some(id), hash),
		None => (None, DUMMY_HASH.clone()),
	};
	let valid = verify_password(body.password, stored_hash).await?;

	match user_id {
		Some(user_id) if valid => {
			ok(&start_session(&mut *state.db.acquire().await?, user_id).await?)
		}
		_ => Err(ApiError::InvalidCredentials),
	}
}

async fn logout(state: AppState, user: CurrentUser) -> ApiResult<Response> {
	sqlx::query("DELETE FROM sessions WHERE id = $1")
		.bind(user.session_id)
		.execute(&state.db)
		.await?;
	no_content()
}

async fn start_session(conn: &mut PgConnection, user_id: i64) -> ApiResult<SessionResponse> {
	let token = hex::encode(rand::random::<[u8; 32]>());
	let expires_at = Utc::now() + SESSION_LIFETIME;
	sqlx::query("INSERT INTO sessions (user_id, token_hash, expires_at) VALUES ($1, $2, $3)")
		.bind(user_id)
		.bind(hash_token(&token))
		.bind(expires_at)
		.execute(&mut *conn)
		.await?;
	Ok(SessionResponse {
		token,
		expires_at,
		user: users::fetch(&mut *conn, user_id).await?,
	})
}

/// Ends every session of `user` except the current one (e.g. after a password change).
pub async fn end_other_sessions(db: &PgPool, user: CurrentUser) -> ApiResult<()> {
	sqlx::query("DELETE FROM sessions WHERE user_id = $1 AND id <> $2")
		.bind(user.id)
		.bind(user.session_id)
		.execute(db)
		.await?;
	Ok(())
}

fn hash_token(token: &str) -> Vec<u8> {
	Sha256::digest(token.as_bytes()).to_vec()
}

fn normalize_email(email: &str) -> ApiResult<String> {
	let email = email.trim().to_lowercase();
	let valid = email.len() <= 254
		&& email.split_once('@').is_some_and(|(local, domain)| {
			!local.is_empty() && !domain.is_empty() && !domain.contains('@')
		});
	if !valid {
		return Err(ApiError::bad_request("email is not a valid email address"));
	}
	Ok(email)
}

pub fn validate_password(password: &str) -> ApiResult<()> {
	let chars = password.chars().count();
	if !(MIN_PASSWORD_CHARS..=MAX_PASSWORD_CHARS).contains(&chars) {
		return Err(ApiError::bad_request(format!(
			"password must be between {MIN_PASSWORD_CHARS} and {MAX_PASSWORD_CHARS} characters"
		)));
	}
	Ok(())
}

static DUMMY_HASH: LazyLock<String> = LazyLock::new(|| {
	Argon2::default()
		.hash_password(b"dummy password for timing")
		.expect("hashing a constant password succeeds")
		.to_string()
});

/// Argon2id hash in PHC format. Runs on a blocking thread since hashing is deliberately slow.
pub async fn hash_password(password: String) -> ApiResult<String> {
	let hash = tokio::task::spawn_blocking(move || {
		Argon2::default()
			.hash_password(password.as_bytes())
			.map(|hash| hash.to_string())
			.map_err(|err| anyhow::anyhow!("failed to hash password: {err}"))
	})
	.await
	.context("password hashing task failed")??;
	Ok(hash)
}

pub async fn verify_password(password: String, hash: String) -> ApiResult<bool> {
	let valid = tokio::task::spawn_blocking(move || {
		let hash = PasswordHash::new(&hash)
			.map_err(|err| anyhow::anyhow!("stored password hash is invalid: {err}"))?;
		Ok::<_, anyhow::Error>(
			Argon2::default()
				.verify_password(password.as_bytes(), &hash)
				.is_ok(),
		)
	})
	.await
	.context("password verification task failed")??;
	Ok(valid)
}
