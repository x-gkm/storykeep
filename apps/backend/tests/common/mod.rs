//! Test harness: drives the full warp app in-process against the per-test
//! database that `#[sqlx::test]` provides.

#![allow(dead_code)] // Each test binary uses a different subset of helpers.

use std::path::PathBuf;

use serde_json::{Value, json};
use sqlx::PgPool;
use storykeep_backend::AppState;
use warp::http::StatusCode;

pub struct TestApp {
	pub state: AppState,
}

pub struct TestResponse {
	pub status: StatusCode,
	pub body: Value,
	pub raw: bytes::Bytes,
	pub headers: warp::http::HeaderMap,
}

impl TestApp {
	pub fn new(db: PgPool) -> Self {
		let media_dir =
			std::env::temp_dir().join(format!("storykeep-test-media-{}", uuid::Uuid::new_v4()));
		std::fs::create_dir_all(&media_dir).unwrap();
		Self {
			state: AppState { db, media_dir },
		}
	}

	pub fn media_dir(&self) -> &PathBuf {
		&self.state.media_dir
	}

	pub fn request(
		&self,
		method: &str,
		path: &str,
		token: Option<&str>,
	) -> warp::test::RequestBuilder {
		let mut request = warp::test::request().method(method).path(path);
		if let Some(token) = token {
			request = request.header("authorization", format!("Bearer {token}"));
		}
		request
	}

	pub async fn send(&self, request: warp::test::RequestBuilder) -> TestResponse {
		let response = request
			.reply(&storykeep_backend::app(self.state.clone()))
			.await;
		let status = response.status();
		let headers = response.headers().clone();
		let raw = response.into_body();
		let body = if raw.is_empty() {
			Value::Null
		} else {
			serde_json::from_slice(&raw).unwrap_or(Value::Null)
		};
		TestResponse {
			status,
			body,
			raw,
			headers,
		}
	}

	pub async fn call(
		&self,
		method: &str,
		path: &str,
		token: Option<&str>,
		body: Option<Value>,
	) -> TestResponse {
		let mut request = self.request(method, path, token);
		if let Some(body) = body {
			request = request.json(&body);
		}
		self.send(request).await
	}

	pub async fn get(&self, path: &str, token: &str) -> TestResponse {
		self.call("GET", path, Some(token), None).await
	}

	pub async fn post(&self, path: &str, token: &str, body: Value) -> TestResponse {
		self.call("POST", path, Some(token), Some(body)).await
	}

	pub async fn put(&self, path: &str, token: &str, body: Value) -> TestResponse {
		self.call("PUT", path, Some(token), Some(body)).await
	}

	pub async fn delete(&self, path: &str, token: &str) -> TestResponse {
		self.call("DELETE", path, Some(token), None).await
	}

	/// Registers a user and returns their bearer token.
	pub async fn register(&self, email: &str) -> String {
		let response = self
			.call(
				"POST",
				"/api/auth/register",
				None,
				Some(json!({
					"email": email,
					"password": "correct horse battery",
					"first_name": "Test",
					"last_name": email.split('@').next().unwrap(),
				})),
			)
			.await;
		assert_eq!(response.status, StatusCode::CREATED, "{}", response.body);
		response.body["token"].as_str().unwrap().to_owned()
	}

	/// Creates a CHILD profile owned by `token`'s user; returns `(profile_id, relationship_id)`.
	pub async fn child(&self, token: &str, name: &str) -> (i64, i64) {
		let response = self
			.post(
				"/api/profiles",
				token,
				json!({
					"profile_type": "CHILD",
					"name": name,
					"date_of_birth": "2024-03-01",
					"relationship_type": "PARENT_CHILD",
				}),
			)
			.await;
		assert_eq!(response.status, StatusCode::CREATED, "{}", response.body);
		(
			response.body["profile"]["id"].as_i64().unwrap(),
			response.body["relationship"]["id"].as_i64().unwrap(),
		)
	}

	/// Adds the user with `email` to a relationship with `role`, acting as `token`'s user.
	pub async fn add_member(&self, token: &str, relationship_id: i64, email: &str, role: &str) {
		let response = self
			.post(
				&format!("/api/relationships/{relationship_id}/members"),
				token,
				json!({ "email": email, "role": role }),
			)
			.await;
		assert_eq!(response.status, StatusCode::CREATED, "{}", response.body);
	}
}

impl Drop for TestApp {
	fn drop(&mut self) {
		let _ = std::fs::remove_dir_all(&self.state.media_dir);
	}
}

/// Asserts the response status and the error `code`, printing the body on failure.
#[track_caller]
pub fn assert_error(response: &TestResponse, status: StatusCode, code: &str) {
	assert_eq!(response.status, status, "{}", response.body);
	assert_eq!(response.body["error"]["code"], code, "{}", response.body);
}
