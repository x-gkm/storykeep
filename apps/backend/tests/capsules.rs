//! Part 8: time capsules and server-enforced unlocks.

mod common;

use chrono::{Duration, Utc};
use common::{TestApp, TestResponse, assert_error};
use serde_json::{Value, json};
use sqlx::PgPool;
use warp::http::StatusCode;

const SECRET: &str = "the secret is out";

fn in_days(days: i64) -> String {
	(Utc::now() + Duration::days(days)).to_rfc3339()
}

/// Creates a capsule unlocking in 30 days; returns its JSON.
async fn create_capsule(app: &TestApp, token: &str, relationship_id: i64, title: &str) -> Value {
	let response = app
		.post(
			&format!("/api/relationships/{relationship_id}/capsules"),
			token,
			json!({ "title": title, "message": SECRET, "unlock_at": in_days(30) }),
		)
		.await;
	assert_eq!(response.status, StatusCode::CREATED, "{}", response.body);
	response.body
}

/// Simulates time passing: moves the capsule's creation and unlock into the past.
async fn unlock(app: &TestApp, id: i64) {
	sqlx::query(
		"UPDATE time_capsules
		SET created_at = now() - interval '2 days', unlock_at = now() - interval '1 day'
		WHERE id = $1",
	)
	.bind(id)
	.execute(&app.state.db)
	.await
	.unwrap();
}

/// Attaches a media row directly (uploads are Part 6); returns the media id.
async fn attach_media(app: &TestApp, capsule: &Value) -> i64 {
	let media_id: i64 = sqlx::query_scalar(
		"INSERT INTO media (media_type_id, storage_path, file_name, mime_type, file_size, uploaded_by)
		VALUES (1, $1, 'first-steps.jpg', 'image/jpeg', 1234, $2) RETURNING id",
	)
	.bind(uuid::Uuid::new_v4().to_string())
	.bind(capsule["created_by"].as_i64().unwrap())
	.fetch_one(&app.state.db)
	.await
	.unwrap();
	sqlx::query("INSERT INTO capsule_media (capsule_id, media_id) VALUES ($1, $2)")
		.bind(capsule["id"].as_i64().unwrap())
		.bind(media_id)
		.execute(&app.state.db)
		.await
		.unwrap();
	media_id
}

/// Asserts that a capsule object carries no protected content at all: the keys are
/// absent (not merely null) and the secret appears nowhere in the raw response.
#[track_caller]
fn assert_sealed(capsule: &Value) {
	let object = capsule.as_object().expect("capsule object");
	assert!(!object.contains_key("message"), "{capsule}");
	assert!(!object.contains_key("media"), "{capsule}");
}

#[track_caller]
fn assert_no_secret(response: &TestResponse) {
	let raw = String::from_utf8_lossy(&response.raw);
	assert!(!raw.contains(SECRET), "{raw}");
	assert!(!raw.contains("first-steps.jpg"), "{raw}");
	assert!(!raw.contains("/api/media/"), "{raw}");
}

async fn stored_status(app: &TestApp, id: i64) -> i16 {
	sqlx::query_scalar("SELECT status_id FROM time_capsules WHERE id = $1")
		.bind(id)
		.fetch_one(&app.state.db)
		.await
		.unwrap()
}

struct Circle {
	owner: String,
	parent: String,
	member: String,
	viewer: String,
	stranger: String,
	relationship_id: i64,
}

async fn circle(app: &TestApp) -> Circle {
	let owner = app.register("owner@example.com").await;
	let parent = app.register("parent@example.com").await;
	let member = app.register("member@example.com").await;
	let viewer = app.register("viewer@example.com").await;
	let stranger = app.register("stranger@example.com").await;
	let (_, relationship_id) = app.child(&owner, "Mira").await;
	for (email, role) in [
		("parent@example.com", "PARENT"),
		("member@example.com", "MEMBER"),
		("viewer@example.com", "VIEWER"),
	] {
		app.add_member(&owner, relationship_id, email, role).await;
	}
	Circle {
		owner,
		parent,
		member,
		viewer,
		stranger,
		relationship_id,
	}
}

#[sqlx::test]
async fn capsules_can_be_created_listed_and_fetched(pool: PgPool) {
	let app = TestApp::new(pool);
	let token = app.register("ada@example.com").await;
	let (_, relationship_id) = app.child(&token, "Mira").await;
	let path = format!("/api/relationships/{relationship_id}/capsules");

	let later = app
		.post(
			&path,
			&token,
			json!({ "title": "  For your 18th  ", "message": SECRET, "unlock_at": in_days(365) }),
		)
		.await;
	assert_eq!(later.status, StatusCode::CREATED, "{}", later.body);
	let capsule = &later.body;
	assert_eq!(capsule["title"], "For your 18th");
	assert_eq!(capsule["status"], "LOCKED");
	assert_eq!(capsule["relationship_id"], relationship_id);
	assert_eq!(capsule["media_count"], 0);
	assert!(capsule["created_by"].is_i64());
	assert!(capsule["unlock_at"].is_string());
	assert_sealed(capsule);
	assert_no_secret(&later);

	let sooner = create_capsule(&app, &token, relationship_id, "First birthday").await;

	let list = app.get(&path, &token).await;
	assert_eq!(list.status, StatusCode::OK, "{}", list.body);
	let titles: Vec<_> = list
		.body
		.as_array()
		.unwrap()
		.iter()
		.map(|c| c["title"].as_str().unwrap())
		.collect();
	assert_eq!(titles, ["First birthday", "For your 18th"]);

	let fetched = app
		.get(&format!("/api/capsules/{}", sooner["id"]), &token)
		.await;
	assert_eq!(fetched.status, StatusCode::OK, "{}", fetched.body);
	assert_eq!(fetched.body, sooner);

	// Status filter.
	unlock(&app, sooner["id"].as_i64().unwrap()).await;
	let locked = app.get(&format!("{path}?status=LOCKED"), &token).await;
	assert_eq!(locked.body.as_array().unwrap().len(), 1);
	assert_eq!(locked.body[0]["title"], "For your 18th");
	let available = app.get(&format!("{path}?status=AVAILABLE"), &token).await;
	assert_eq!(available.body.as_array().unwrap().len(), 1);
	assert_eq!(available.body[0]["title"], "First birthday");
	assert_error(
		&app.get(&format!("{path}?status=SOON"), &token).await,
		StatusCode::BAD_REQUEST,
		"bad_request",
	);
}

#[sqlx::test]
async fn capsule_input_is_validated(pool: PgPool) {
	let app = TestApp::new(pool);
	let token = app.register("ada@example.com").await;
	let (_, relationship_id) = app.child(&token, "Mira").await;
	let path = format!("/api/relationships/{relationship_id}/capsules");

	for body in [
		json!({ "title": "Past", "unlock_at": in_days(-1) }),
		json!({ "title": "Now-ish", "unlock_at": Utc::now().to_rfc3339() }),
		json!({ "title": "Too far", "unlock_at": in_days(365 * 101) }),
		json!({ "title": "   ", "unlock_at": in_days(30) }),
		json!({ "title": "x".repeat(201), "unlock_at": in_days(30) }),
		json!({ "title": "No date" }),
		json!({ "title": "Bad date", "unlock_at": "next tuesday" }),
	] {
		let response = app.post(&path, &token, body.clone()).await;
		assert_error(&response, StatusCode::BAD_REQUEST, "bad_request");
	}

	let capsule = create_capsule(&app, &token, relationship_id, "Ok").await;
	let capsule_path = format!("/api/capsules/{}", capsule["id"]);
	for body in [
		json!({ "unlock_at": in_days(-1) }),
		json!({ "title": "" }),
		json!({ "unlock_at": in_days(365 * 101) }),
	] {
		let response = app.put(&capsule_path, &token, body).await;
		assert_error(&response, StatusCode::BAD_REQUEST, "bad_request");
	}
}

#[sqlx::test]
async fn locked_content_is_never_returned_to_anyone(pool: PgPool) {
	let app = TestApp::new(pool);
	let c = circle(&app).await;
	// The creator is a MEMBER, so every role (including the creator) is checked.
	let capsule = create_capsule(&app, &c.member, c.relationship_id, "Sealed").await;
	let id = capsule["id"].as_i64().unwrap();
	assert_sealed(&capsule);
	attach_media(&app, &capsule).await;

	let list_path = format!("/api/relationships/{}/capsules", c.relationship_id);
	let path = format!("/api/capsules/{id}");
	for token in [&c.member, &c.owner, &c.parent, &c.viewer] {
		let list = app.get(&list_path, token).await;
		assert_eq!(list.status, StatusCode::OK, "{}", list.body);
		assert_sealed(&list.body[0]);
		assert_eq!(list.body[0]["media_count"], 1);
		assert_no_secret(&list);

		let fetched = app.get(&path, token).await;
		assert_eq!(fetched.status, StatusCode::OK, "{}", fetched.body);
		assert_eq!(fetched.body["status"], "LOCKED");
		assert_sealed(&fetched.body);
		assert_no_secret(&fetched);

		let opened = app.post(&format!("{path}/open"), token, json!({})).await;
		assert_error(&opened, StatusCode::CONFLICT, "conflict");
		assert_no_secret(&opened);
	}

	// Editing returns metadata only, even to the creator who just wrote the message.
	let updated = app
		.put(
			&path,
			&c.member,
			json!({ "message": SECRET, "title": "Sealed!" }),
		)
		.await;
	assert_eq!(updated.status, StatusCode::OK, "{}", updated.body);
	assert_eq!(updated.body["title"], "Sealed!");
	assert_sealed(&updated.body);
	assert_no_secret(&updated);

	assert_eq!(stored_status(&app, id).await, 1);
}

#[sqlx::test]
async fn unlocked_capsules_become_available_and_open_once(pool: PgPool) {
	let app = TestApp::new(pool);
	let c = circle(&app).await;
	let capsule = create_capsule(&app, &c.owner, c.relationship_id, "Hello future").await;
	let id = capsule["id"].as_i64().unwrap();
	let media_id = attach_media(&app, &capsule).await;
	unlock(&app, id).await;

	// AVAILABLE without any write; content still requires opening.
	let path = format!("/api/capsules/{id}");
	let fetched = app.get(&path, &c.viewer).await;
	assert_eq!(fetched.body["status"], "AVAILABLE", "{}", fetched.body);
	assert_sealed(&fetched.body);
	assert_no_secret(&fetched);
	let list = app
		.get(
			&format!("/api/relationships/{}/capsules", c.relationship_id),
			&c.viewer,
		)
		.await;
	assert_eq!(list.body[0]["status"], "AVAILABLE");
	assert_eq!(stored_status(&app, id).await, 1);

	// Viewers have read access, so they may open it.
	let opened = app
		.post(&format!("{path}/open"), &c.viewer, json!({}))
		.await;
	assert_eq!(opened.status, StatusCode::OK, "{}", opened.body);
	assert_eq!(opened.body["status"], "OPENED");
	assert_eq!(opened.body["message"], SECRET);
	assert_eq!(
		opened.body["media"],
		json!([{
			"id": media_id,
			"media_type": "IMAGE",
			"file_name": "first-steps.jpg",
			"mime_type": "image/jpeg",
			"file_size": 1234,
			"uploaded_by": { "id": opened.body["created_by"], "first_name": "Test", "last_name": "owner" },
			"created_at": opened.body["media"][0]["created_at"],
			"content_url": format!("/api/media/{media_id}/content"),
		}])
	);
	assert_eq!(stored_status(&app, id).await, 3);

	// Idempotent, and GET now includes the content for every member.
	let again = app
		.post(&format!("{path}/open"), &c.member, json!({}))
		.await;
	assert_eq!(again.status, StatusCode::OK, "{}", again.body);
	assert_eq!(again.body, opened.body);
	let fetched = app.get(&path, &c.owner).await;
	assert_eq!(fetched.body, opened.body);

	// The list stays metadata-only.
	let list = app
		.get(
			&format!("/api/relationships/{}/capsules", c.relationship_id),
			&c.owner,
		)
		.await;
	assert_eq!(list.body[0]["status"], "OPENED");
	assert_sealed(&list.body[0]);
}

#[sqlx::test]
async fn capsules_can_only_change_while_locked(pool: PgPool) {
	let app = TestApp::new(pool);
	let token = app.register("ada@example.com").await;
	let (_, relationship_id) = app.child(&token, "Mira").await;
	let capsule = create_capsule(&app, &token, relationship_id, "Later").await;
	let id = capsule["id"].as_i64().unwrap();
	let path = format!("/api/capsules/{id}");

	// Partial updates keep omitted fields.
	let new_unlock = in_days(60);
	let updated = app
		.put(
			&path,
			&token,
			json!({ "title": "Much later", "unlock_at": new_unlock }),
		)
		.await;
	assert_eq!(updated.status, StatusCode::OK, "{}", updated.body);
	assert_eq!(updated.body["title"], "Much later");
	assert_ne!(updated.body["unlock_at"], capsule["unlock_at"]);

	unlock(&app, id).await;
	assert_error(
		&app.put(&path, &token, json!({ "title": "Too late" })).await,
		StatusCode::CONFLICT,
		"conflict",
	);
	assert_error(
		&app.post(&format!("{path}/cancel"), &token, json!({})).await,
		StatusCode::CONFLICT,
		"conflict",
	);

	let opened = app.post(&format!("{path}/open"), &token, json!({})).await;
	assert_eq!(opened.status, StatusCode::OK, "{}", opened.body);
	assert_eq!(opened.body["title"], "Much later");
	// The message survived the title-only update.
	assert_eq!(opened.body["message"], SECRET);
	assert_eq!(opened.body["media"], json!([]));
	assert_error(
		&app.put(&path, &token, json!({ "title": "Too late" })).await,
		StatusCode::CONFLICT,
		"conflict",
	);
	assert_error(
		&app.post(&format!("{path}/cancel"), &token, json!({})).await,
		StatusCode::CONFLICT,
		"conflict",
	);

	// Clearing the message with an explicit null.
	let other = create_capsule(&app, &token, relationship_id, "Blank").await;
	let other_path = format!("/api/capsules/{}", other["id"]);
	let cleared = app
		.put(&other_path, &token, json!({ "message": null }))
		.await;
	assert_eq!(cleared.status, StatusCode::OK, "{}", cleared.body);
	unlock(&app, other["id"].as_i64().unwrap()).await;
	let opened = app
		.post(&format!("{other_path}/open"), &token, json!({}))
		.await;
	assert_eq!(opened.body["message"], Value::Null);
	assert!(opened.body.as_object().unwrap().contains_key("message"));
}

#[sqlx::test]
async fn cancelled_capsules_never_reveal_content(pool: PgPool) {
	let app = TestApp::new(pool);
	let token = app.register("ada@example.com").await;
	let (_, relationship_id) = app.child(&token, "Mira").await;
	let capsule = create_capsule(&app, &token, relationship_id, "Never mind").await;
	let id = capsule["id"].as_i64().unwrap();
	attach_media(&app, &capsule).await;
	let path = format!("/api/capsules/{id}");

	let cancelled = app.post(&format!("{path}/cancel"), &token, json!({})).await;
	assert_eq!(cancelled.status, StatusCode::OK, "{}", cancelled.body);
	assert_eq!(cancelled.body["status"], "CANCELLED");
	assert_sealed(&cancelled.body);
	assert_no_secret(&cancelled);

	// Even after the unlock date has passed.
	unlock(&app, id).await;
	let fetched = app.get(&path, &token).await;
	assert_eq!(fetched.body["status"], "CANCELLED");
	assert_sealed(&fetched.body);
	assert_no_secret(&fetched);
	let opened = app.post(&format!("{path}/open"), &token, json!({})).await;
	assert_error(&opened, StatusCode::CONFLICT, "conflict");
	assert_no_secret(&opened);
	for response in [
		app.put(&path, &token, json!({ "title": "Back on" })).await,
		app.post(&format!("{path}/cancel"), &token, json!({})).await,
	] {
		assert_error(&response, StatusCode::CONFLICT, "conflict");
	}
	let list = app
		.get(
			&format!("/api/relationships/{relationship_id}/capsules?status=CANCELLED"),
			&token,
		)
		.await;
	assert_eq!(list.body.as_array().unwrap().len(), 1);
	assert_no_secret(&list);
	assert_eq!(stored_status(&app, id).await, 4);

	// Cancelled capsules can still be deleted.
	assert_eq!(
		app.delete(&path, &token).await.status,
		StatusCode::NO_CONTENT
	);
	assert_error(
		&app.get(&path, &token).await,
		StatusCode::NOT_FOUND,
		"not_found",
	);
}

#[sqlx::test]
async fn roles_control_who_may_change_capsules(pool: PgPool) {
	let app = TestApp::new(pool);
	let c = circle(&app).await;
	let list_path = format!("/api/relationships/{}/capsules", c.relationship_id);
	let body = json!({ "title": "Viewer's", "unlock_at": in_days(30) });

	// Strangers see nothing, and can't tell capsules from missing ids.
	let owners = create_capsule(&app, &c.owner, c.relationship_id, "Owner's").await;
	let path = format!("/api/capsules/{}", owners["id"]);
	for response in [
		app.get(&list_path, &c.stranger).await,
		app.post(&list_path, &c.stranger, body.clone()).await,
		app.get(&path, &c.stranger).await,
		app.put(&path, &c.stranger, json!({ "title": "x" })).await,
		app.post(&format!("{path}/cancel"), &c.stranger, json!({}))
			.await,
		app.post(&format!("{path}/open"), &c.stranger, json!({}))
			.await,
		app.delete(&path, &c.stranger).await,
		app.get("/api/capsules/999999", &c.owner).await,
	] {
		assert_error(&response, StatusCode::NOT_FOUND, "not_found");
	}

	// Viewers read metadata but can't write.
	let viewed = app.get(&path, &c.viewer).await;
	assert_eq!(viewed.status, StatusCode::OK);
	assert_eq!(app.get(&list_path, &c.viewer).await.status, StatusCode::OK);
	for response in [
		app.post(&list_path, &c.viewer, body.clone()).await,
		app.put(&path, &c.viewer, json!({ "title": "x" })).await,
		app.post(&format!("{path}/cancel"), &c.viewer, json!({}))
			.await,
		app.delete(&path, &c.viewer).await,
	] {
		assert_error(&response, StatusCode::FORBIDDEN, "forbidden");
	}

	// Members edit their own capsules, not others'.
	let members = create_capsule(&app, &c.member, c.relationship_id, "Member's").await;
	let member_path = format!("/api/capsules/{}", members["id"]);
	let renamed = app
		.put(&member_path, &c.member, json!({ "title": "Mine" }))
		.await;
	assert_eq!(renamed.status, StatusCode::OK, "{}", renamed.body);
	for response in [
		app.put(&path, &c.member, json!({ "title": "x" })).await,
		app.post(&format!("{path}/cancel"), &c.member, json!({}))
			.await,
		app.delete(&path, &c.member).await,
	] {
		assert_error(&response, StatusCode::FORBIDDEN, "forbidden");
	}

	// Parents (manage access) edit and cancel anyone's.
	let renamed = app
		.put(
			&member_path,
			&c.parent,
			json!({ "title": "Edited by parent" }),
		)
		.await;
	assert_eq!(renamed.status, StatusCode::OK, "{}", renamed.body);
	let cancelled = app
		.post(&format!("{member_path}/cancel"), &c.parent, json!({}))
		.await;
	assert_eq!(cancelled.status, StatusCode::OK, "{}", cancelled.body);
	assert_eq!(
		app.delete(&path, &c.parent).await.status,
		StatusCode::NO_CONTENT
	);

	// A creator who is demoted to VIEWER loses write access to their own capsule.
	let another = create_capsule(&app, &c.member, c.relationship_id, "Before demotion").await;
	let member_id = another["created_by"].as_i64().unwrap();
	let demoted = app
		.put(
			&format!(
				"/api/relationships/{}/members/{member_id}",
				c.relationship_id
			),
			&c.owner,
			json!({ "role": "VIEWER" }),
		)
		.await;
	assert_eq!(demoted.status, StatusCode::OK, "{}", demoted.body);
	assert_error(
		&app.put(
			&format!("/api/capsules/{}", another["id"]),
			&c.member,
			json!({ "title": "x" }),
		)
		.await,
		StatusCode::FORBIDDEN,
		"forbidden",
	);
}
