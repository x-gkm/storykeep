//! Part 3: registration, login, sessions and the current user.

mod common;

use common::{TestApp, assert_error};
use serde_json::json;
use sqlx::PgPool;
use warp::http::StatusCode;

#[sqlx::test]
async fn register_returns_a_session_and_hides_the_password(pool: PgPool) {
	let app = TestApp::new(pool);
	let response = app
		.call(
			"POST",
			"/api/auth/register",
			None,
			Some(json!({
				"email": "  Ada@Example.com ",
				"password": "correct horse battery",
				"first_name": "Ada",
				"last_name": "Lovelace",
				"date_of_birth": "1990-12-10",
			})),
		)
		.await;

	assert_eq!(response.status, StatusCode::CREATED, "{}", response.body);
	assert_eq!(response.body["token"].as_str().unwrap().len(), 64);
	assert!(response.body["expires_at"].is_string());
	let user = &response.body["user"];
	assert_eq!(user["email"], "ada@example.com");
	assert_eq!(user["first_name"], "Ada");
	assert_eq!(user["date_of_birth"], "1990-12-10");
	assert!(user.get("password_hash").is_none());

	let stored: String = sqlx::query_scalar("SELECT password_hash FROM users")
		.fetch_one(&app.state.db)
		.await
		.unwrap();
	assert!(stored.starts_with("$argon2id$"), "{stored}");
}

#[sqlx::test]
async fn register_rejects_duplicate_emails_ignoring_case(pool: PgPool) {
	let app = TestApp::new(pool);
	app.register("ada@example.com").await;
	let response = app
		.call(
			"POST",
			"/api/auth/register",
			None,
			Some(json!({
				"email": "ADA@example.com",
				"password": "another password",
				"first_name": "Ada",
				"last_name": "Again",
			})),
		)
		.await;
	assert_error(&response, StatusCode::CONFLICT, "conflict");
}

#[sqlx::test]
async fn register_validates_input(pool: PgPool) {
	let app = TestApp::new(pool);
	let valid = json!({
		"email": "ada@example.com",
		"password": "correct horse battery",
		"first_name": "Ada",
		"last_name": "Lovelace",
	});
	for (field, value) in [
		("email", json!("not-an-email")),
		("password", json!("short")),
		("first_name", json!("   ")),
		("date_of_birth", json!("2999-01-01")),
	] {
		let mut body = valid.clone();
		body[field] = value;
		let response = app
			.call("POST", "/api/auth/register", None, Some(body))
			.await;
		assert_error(&response, StatusCode::BAD_REQUEST, "bad_request");
	}

	let mut missing = valid.clone();
	missing.as_object_mut().unwrap().remove("last_name");
	let response = app
		.call("POST", "/api/auth/register", None, Some(missing))
		.await;
	assert_error(&response, StatusCode::BAD_REQUEST, "bad_request");
}

#[sqlx::test]
async fn login_checks_credentials(pool: PgPool) {
	let app = TestApp::new(pool);
	app.register("ada@example.com").await;

	let login = |email: &'static str, password: &'static str| {
		app.call(
			"POST",
			"/api/auth/login",
			None,
			Some(json!({ "email": email, "password": password })),
		)
	};

	let response = login("ADA@example.com", "correct horse battery").await;
	assert_eq!(response.status, StatusCode::OK, "{}", response.body);
	let token = response.body["token"].as_str().unwrap();
	assert_eq!(app.get("/api/users/me", token).await.status, StatusCode::OK);

	assert_error(
		&login("ada@example.com", "wrong password").await,
		StatusCode::UNAUTHORIZED,
		"invalid_credentials",
	);
	assert_error(
		&login("nobody@example.com", "correct horse battery").await,
		StatusCode::UNAUTHORIZED,
		"invalid_credentials",
	);
}

#[sqlx::test]
async fn protected_routes_require_a_valid_token(pool: PgPool) {
	let app = TestApp::new(pool);
	let token = app.register("ada@example.com").await;

	let anonymous = app.call("GET", "/api/users/me", None, None).await;
	assert_error(&anonymous, StatusCode::UNAUTHORIZED, "unauthorized");
	assert_error(
		&app.get("/api/users/me", "not-a-real-token").await,
		StatusCode::UNAUTHORIZED,
		"unauthorized",
	);
	let wrong_scheme = app
		.send(
			app.request("GET", "/api/users/me", None)
				.header("authorization", format!("Basic {token}")),
		)
		.await;
	assert_error(&wrong_scheme, StatusCode::UNAUTHORIZED, "unauthorized");

	// Unknown routes are 404 even without a token.
	let unknown = app.call("GET", "/api/does-not-exist", None, None).await;
	assert_error(&unknown, StatusCode::NOT_FOUND, "not_found");
}

#[sqlx::test]
async fn expired_sessions_are_rejected(pool: PgPool) {
	let app = TestApp::new(pool);
	let token = app.register("ada@example.com").await;
	sqlx::query("UPDATE sessions SET created_at = now() - interval '2 days', expires_at = now() - interval '1 day'")
		.execute(&app.state.db)
		.await
		.unwrap();
	assert_error(
		&app.get("/api/users/me", &token).await,
		StatusCode::UNAUTHORIZED,
		"unauthorized",
	);
}

#[sqlx::test]
async fn logout_ends_only_the_current_session(pool: PgPool) {
	let app = TestApp::new(pool);
	let first = app.register("ada@example.com").await;
	let second = app
		.call(
			"POST",
			"/api/auth/login",
			None,
			Some(json!({ "email": "ada@example.com", "password": "correct horse battery" })),
		)
		.await
		.body["token"]
		.as_str()
		.unwrap()
		.to_owned();

	let response = app
		.call("POST", "/api/auth/logout", Some(&first), None)
		.await;
	assert_eq!(response.status, StatusCode::NO_CONTENT);
	assert_error(
		&app.get("/api/users/me", &first).await,
		StatusCode::UNAUTHORIZED,
		"unauthorized",
	);
	assert_eq!(
		app.get("/api/users/me", &second).await.status,
		StatusCode::OK
	);
}

#[sqlx::test]
async fn users_can_update_their_profile(pool: PgPool) {
	let app = TestApp::new(pool);
	let token = app.register("ada@example.com").await;

	let response = app
		.put(
			"/api/users/me",
			&token,
			json!({ "first_name": " Augusta ", "last_name": "King", "date_of_birth": "1990-12-10" }),
		)
		.await;
	assert_eq!(response.status, StatusCode::OK, "{}", response.body);
	assert_eq!(response.body["first_name"], "Augusta");
	assert_eq!(response.body["last_name"], "King");

	let me = app.get("/api/users/me", &token).await;
	assert_eq!(me.body["first_name"], "Augusta");
	assert_eq!(me.body["email"], "ada@example.com");

	let invalid = app
		.put(
			"/api/users/me",
			&token,
			json!({ "first_name": "", "last_name": "King" }),
		)
		.await;
	assert_error(&invalid, StatusCode::BAD_REQUEST, "bad_request");
}

#[sqlx::test]
async fn changing_the_password_signs_out_other_sessions(pool: PgPool) {
	let app = TestApp::new(pool);
	let current = app.register("ada@example.com").await;
	let other = app
		.call(
			"POST",
			"/api/auth/login",
			None,
			Some(json!({ "email": "ada@example.com", "password": "correct horse battery" })),
		)
		.await
		.body["token"]
		.as_str()
		.unwrap()
		.to_owned();

	let wrong = app
		.put(
			"/api/users/me/password",
			&current,
			json!({ "current_password": "nope nope nope", "new_password": "a brand new password" }),
		)
		.await;
	assert_error(&wrong, StatusCode::UNAUTHORIZED, "invalid_credentials");

	let response = app
		.put(
			"/api/users/me/password",
			&current,
			json!({ "current_password": "correct horse battery", "new_password": "a brand new password" }),
		)
		.await;
	assert_eq!(response.status, StatusCode::NO_CONTENT, "{}", response.body);

	assert_eq!(
		app.get("/api/users/me", &current).await.status,
		StatusCode::OK
	);
	assert_error(
		&app.get("/api/users/me", &other).await,
		StatusCode::UNAUTHORIZED,
		"unauthorized",
	);

	let login = app
		.call(
			"POST",
			"/api/auth/login",
			None,
			Some(json!({ "email": "ada@example.com", "password": "a brand new password" })),
		)
		.await;
	assert_eq!(login.status, StatusCode::OK);
}

#[sqlx::test]
async fn reference_data_is_public(pool: PgPool) {
	let app = TestApp::new(pool);
	let response = app.call("GET", "/api/reference", None, None).await;
	assert_eq!(response.status, StatusCode::OK);
	assert_eq!(
		response.body["profile_types"],
		json!(["CHILD", "PET", "PERSON", "OTHER"])
	);
	assert_eq!(
		response.body["measurement_types"][0],
		json!({ "name": "HEIGHT", "unit": "cm" })
	);
}
