//! Part 6: media upload, metadata, download and access control.

mod common;

use common::{TestApp, TestResponse, assert_error};
use serde_json::{Value, json};
use sqlx::PgPool;
use storykeep_backend::media::{self, MAX_FILE_SIZE};
use warp::http::StatusCode;

/// A valid 1×1 transparent PNG.
const PNG: &[u8] = &[
	0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
	0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4,
	0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
	0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE,
	0x42, 0x60, 0x82,
];

/// A minimal (empty) PDF document.
const PDF: &[u8] = b"%PDF-1.4\n1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n\
2 0 obj<</Type/Pages/Kids[]/Count 0>>endobj\ntrailer<</Root 1 0 R>>\n%%EOF\n";

const BOUNDARY: &str = "storykeep-test-boundary-7MA4YWxkTrZu0gW";

/// One multipart part: field name, file name, declared content type ("" omits it), bytes.
type FormPart<'a> = (&'a str, &'a str, &'a str, &'a [u8]);

fn multipart_body(parts: &[FormPart]) -> Vec<u8> {
	let mut body = Vec::new();
	for (field, file_name, content_type, data) in parts {
		body.extend_from_slice(format!("--{BOUNDARY}\r\n").as_bytes());
		body.extend_from_slice(
			format!(
				"Content-Disposition: form-data; name=\"{field}\"; filename=\"{file_name}\"\r\n"
			)
			.as_bytes(),
		);
		if !content_type.is_empty() {
			body.extend_from_slice(format!("Content-Type: {content_type}\r\n").as_bytes());
		}
		body.extend_from_slice(b"\r\n");
		body.extend_from_slice(data);
		body.extend_from_slice(b"\r\n");
	}
	body.extend_from_slice(format!("--{BOUNDARY}--\r\n").as_bytes());
	body
}

async fn upload(app: &TestApp, path: &str, token: &str, parts: &[FormPart<'_>]) -> TestResponse {
	let request = app
		.request("POST", path, Some(token))
		.header(
			"content-type",
			format!("multipart/form-data; boundary={BOUNDARY}"),
		)
		.body(multipart_body(parts));
	app.send(request).await
}

/// Uploads a single PNG and returns its media id.
async fn upload_png(app: &TestApp, path: &str, token: &str) -> i64 {
	let response = upload(app, path, token, &[("file", "pic.png", "image/png", PNG)]).await;
	assert_eq!(response.status, StatusCode::CREATED, "{}", response.body);
	response.body[0]["id"].as_i64().unwrap()
}

async fn memory(app: &TestApp, relationship_id: i64) -> i64 {
	sqlx::query_scalar(
		"INSERT INTO memories (relationship_id, category_id, title, memory_date, created_by)
		SELECT $1, 1, 'First steps', '2026-05-01', user_id
		FROM relationship_members WHERE relationship_id = $1 AND role_id = 1 LIMIT 1
		RETURNING id",
	)
	.bind(relationship_id)
	.fetch_one(&app.state.db)
	.await
	.unwrap()
}

/// A LOCKED capsule unlocking tomorrow.
async fn capsule(app: &TestApp, relationship_id: i64) -> i64 {
	sqlx::query_scalar(
		"INSERT INTO time_capsules (relationship_id, created_by, title, unlock_at, status_id)
		SELECT $1, user_id, 'For your 18th', now() + interval '1 day', 1
		FROM relationship_members WHERE relationship_id = $1 AND role_id = 1 LIMIT 1
		RETURNING id",
	)
	.bind(relationship_id)
	.fetch_one(&app.state.db)
	.await
	.unwrap()
}

/// Marks a capsule OPENED (status 3), as Part 8's open action does after unlock.
async fn open_capsule(app: &TestApp, capsule_id: i64) {
	unlock(app, capsule_id).await;
	sqlx::query("UPDATE time_capsules SET status_id = 3 WHERE id = $1")
		.bind(capsule_id)
		.execute(&app.state.db)
		.await
		.unwrap();
}

/// Moves a capsule's dates into the past, so its unlock time has passed.
async fn unlock(app: &TestApp, capsule_id: i64) {
	sqlx::query(
		"UPDATE time_capsules
		SET created_at = now() - interval '2 days', unlock_at = now() - interval '1 day'
		WHERE id = $1",
	)
	.bind(capsule_id)
	.execute(&app.state.db)
	.await
	.unwrap();
}

async fn storage_path(app: &TestApp, media_id: i64) -> Option<String> {
	sqlx::query_scalar("SELECT storage_path FROM media WHERE id = $1")
		.bind(media_id)
		.fetch_optional(&app.state.db)
		.await
		.unwrap()
}

async fn media_count(app: &TestApp) -> i64 {
	sqlx::query_scalar("SELECT count(*) FROM media")
		.fetch_one(&app.state.db)
		.await
		.unwrap()
}

fn files_on_disk(app: &TestApp) -> usize {
	std::fs::read_dir(app.media_dir()).unwrap().count()
}

#[sqlx::test]
async fn uploaded_media_can_be_described_and_downloaded(pool: PgPool) {
	let app = TestApp::new(pool);
	let owner = app.register("ada@example.com").await;
	let viewer = app.register("vic@example.com").await;
	let (_, relationship_id) = app.child(&owner, "Mira").await;
	app.add_member(&owner, relationship_id, "vic@example.com", "VIEWER")
		.await;
	let memory_id = memory(&app, relationship_id).await;

	let response = upload(
		&app,
		&format!("/api/memories/{memory_id}/media"),
		&owner,
		&[("file", "Erste Schritte ä.png", "image/png", PNG)],
	)
	.await;
	assert_eq!(response.status, StatusCode::CREATED, "{}", response.body);
	let created = &response.body[0];
	let id = created["id"].as_i64().unwrap();
	assert_eq!(created["media_type"], "IMAGE");
	assert_eq!(created["file_name"], "Erste Schritte ä.png");
	assert_eq!(created["mime_type"], "image/png");
	assert_eq!(created["file_size"], PNG.len());
	assert_eq!(created["uploaded_by"]["first_name"], "Test");
	assert_eq!(created["uploaded_by"]["last_name"], "ada");
	assert!(created["uploaded_by"]["id"].is_i64());
	assert_eq!(created["content_url"], format!("/api/media/{id}/content"));

	// Stored under a generated name, never the client's.
	let stored = storage_path(&app, id).await.unwrap();
	assert!(
		stored.ends_with(".png") && !stored.contains('/'),
		"{stored}"
	);
	assert_eq!(std::fs::read(app.media_dir().join(&stored)).unwrap(), PNG);

	for token in [&owner, &viewer] {
		let metadata = app.get(&format!("/api/media/{id}"), token).await;
		assert_eq!(metadata.status, StatusCode::OK, "{}", metadata.body);
		assert_eq!(&metadata.body, created);

		let content = app.get(&format!("/api/media/{id}/content"), token).await;
		assert_eq!(content.status, StatusCode::OK);
		assert_eq!(&content.raw[..], PNG);
		assert_eq!(content.headers["content-type"], "image/png");
		assert_eq!(content.headers["content-length"], PNG.len().to_string());
		assert_eq!(
			content.headers["content-disposition"],
			"inline; filename=\"Erste Schritte _.png\"; filename*=UTF-8''Erste%20Schritte%20%C3%A4.png"
		);
		assert_eq!(content.headers["x-content-type-options"], "nosniff");
		assert_eq!(content.headers["cache-control"], "private");
	}

	// Authentication is required.
	let anonymous = app
		.call("GET", &format!("/api/media/{id}/content"), None, None)
		.await;
	assert_error(&anonymous, StatusCode::UNAUTHORIZED, "unauthorized");
}

#[sqlx::test]
async fn several_files_can_be_uploaded_at_once(pool: PgPool) {
	let app = TestApp::new(pool);
	let owner = app.register("ada@example.com").await;
	let (_, relationship_id) = app.child(&owner, "Mira").await;
	let memory_id = memory(&app, relationship_id).await;

	let response = upload(
		&app,
		&format!("/api/memories/{memory_id}/media"),
		&owner,
		&[
			("file", "pic.png", "image/png", PNG),
			// Path components are stripped; a generic declared type is fine.
			("file", "../../report.pdf", "application/octet-stream", PDF),
		],
	)
	.await;
	assert_eq!(response.status, StatusCode::CREATED, "{}", response.body);
	let media = response.body.as_array().unwrap();
	assert_eq!(media.len(), 2);
	assert_eq!(media[0]["media_type"], "IMAGE");
	assert_eq!(media[1]["media_type"], "DOCUMENT");
	assert_eq!(media[1]["mime_type"], "application/pdf");
	assert_eq!(media[1]["file_name"], "report.pdf");

	let linked: i64 = sqlx::query_scalar("SELECT count(*) FROM memory_media WHERE memory_id = $1")
		.bind(memory_id)
		.fetch_one(&app.state.db)
		.await
		.unwrap();
	assert_eq!(linked, 2);
	assert_eq!(files_on_disk(&app), 2);

	let listed = media::list_for_memory(&app.state.db, memory_id)
		.await
		.unwrap();
	assert_eq!(
		listed.iter().map(|m| m.id).collect::<Vec<_>>(),
		media
			.iter()
			.map(|m| m["id"].as_i64().unwrap())
			.collect::<Vec<_>>()
	);
}

#[sqlx::test]
async fn invalid_files_are_rejected(pool: PgPool) {
	let app = TestApp::new(pool);
	let owner = app.register("ada@example.com").await;
	let (_, relationship_id) = app.child(&owner, "Mira").await;
	let memory_id = memory(&app, relationship_id).await;
	let path = format!("/api/memories/{memory_id}/media");

	let cases: &[&[FormPart]] = &[
		// Not on the allowlist.
		&[(
			"file",
			"page.html",
			"text/html",
			b"<html><script></script></html>",
		)],
		&[(
			"file",
			"logo.svg",
			"image/svg+xml",
			b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>",
		)],
		// Declared as an image, but the bytes aren't one.
		&[("file", "pic.png", "image/png", PDF)],
		&[("file", "pic.png", "image/png", b"definitely not a png")],
		// Undeclared and unrecognisable.
		&[("file", "data.bin", "", b"\x00\x01\x02\x03")],
		// Empty.
		&[("file", "empty.png", "image/png", b"")],
		// Wrong field name, or no files at all.
		&[("photo", "pic.png", "image/png", PNG)],
		&[],
		// One bad file fails the whole request.
		&[
			("file", "pic.png", "image/png", PNG),
			("file", "evil.png", "image/png", b"MZ\x90\x00"),
		],
	];
	for parts in cases {
		let response = upload(&app, &path, &owner, parts).await;
		assert_error(&response, StatusCode::BAD_REQUEST, "bad_request");
	}

	// Not a multipart request.
	let json_request = app.post(&path, &owner, json!({ "file": "x" })).await;
	assert_error(&json_request, StatusCode::BAD_REQUEST, "bad_request");

	assert_eq!(media_count(&app).await, 0);
	assert_eq!(files_on_disk(&app), 0);
}

#[sqlx::test]
async fn oversized_files_are_rejected(pool: PgPool) {
	let app = TestApp::new(pool);
	let owner = app.register("ada@example.com").await;
	let (_, relationship_id) = app.child(&owner, "Mira").await;
	let memory_id = memory(&app, relationship_id).await;

	let mut big = PNG.to_vec();
	big.resize(MAX_FILE_SIZE as usize + 1, 0);
	let response = upload(
		&app,
		&format!("/api/memories/{memory_id}/media"),
		&owner,
		&[("file", "big.png", "image/png", &big)],
	)
	.await;
	assert_error(
		&response,
		StatusCode::PAYLOAD_TOO_LARGE,
		"payload_too_large",
	);
	assert_eq!(media_count(&app).await, 0);
	assert_eq!(files_on_disk(&app), 0);
}

#[sqlx::test]
async fn roles_control_who_may_upload_and_remove_media(pool: PgPool) {
	let app = TestApp::new(pool);
	let owner = app.register("owner@example.com").await;
	let parent = app.register("parent@example.com").await;
	let member = app.register("member@example.com").await;
	let viewer = app.register("viewer@example.com").await;
	let stranger = app.register("eve@example.com").await;
	let (_, relationship_id) = app.child(&owner, "Mira").await;
	app.add_member(&owner, relationship_id, "parent@example.com", "PARENT")
		.await;
	app.add_member(&owner, relationship_id, "member@example.com", "MEMBER")
		.await;
	app.add_member(&owner, relationship_id, "viewer@example.com", "VIEWER")
		.await;
	let memory_id = memory(&app, relationship_id).await;
	let upload_path = format!("/api/memories/{memory_id}/media");
	let png = [("file", "pic.png", "image/png", PNG)];

	// Strangers can't tell the memory or its media exist.
	let response = upload(&app, &upload_path, &stranger, &png).await;
	assert_error(&response, StatusCode::NOT_FOUND, "not_found");
	assert_eq!(response.body["error"]["message"], "memory not found");
	let response = upload(&app, "/api/memories/999999/media", &owner, &png).await;
	assert_error(&response, StatusCode::NOT_FOUND, "not_found");

	let response = upload(&app, &upload_path, &viewer, &png).await;
	assert_error(&response, StatusCode::FORBIDDEN, "forbidden");

	let owners = upload_png(&app, &upload_path, &owner).await;
	let members = upload_png(&app, &upload_path, &member).await;
	let members_second = upload_png(&app, &upload_path, &member).await;
	let parents = upload_png(&app, &upload_path, &parent).await;

	for path in [
		format!("/api/media/{owners}"),
		format!("/api/media/{owners}/content"),
	] {
		assert_error(
			&app.get(&path, &stranger).await,
			StatusCode::NOT_FOUND,
			"not_found",
		);
	}
	let detach = |id: i64| format!("/api/memories/{memory_id}/media/{id}");
	assert_error(
		&app.delete(&detach(owners), &stranger).await,
		StatusCode::NOT_FOUND,
		"not_found",
	);
	assert_error(
		&app.delete(&detach(owners), &viewer).await,
		StatusCode::FORBIDDEN,
		"forbidden",
	);
	// MEMBERs may remove only their own uploads.
	assert_error(
		&app.delete(&detach(owners), &member).await,
		StatusCode::FORBIDDEN,
		"forbidden",
	);
	assert_error(
		&app.delete(&detach(parents), &member).await,
		StatusCode::FORBIDDEN,
		"forbidden",
	);
	assert_eq!(
		app.delete(&detach(members), &member).await.status,
		StatusCode::NO_CONTENT
	);
	// PARENTs (and OWNERs) may remove anyone's.
	assert_eq!(
		app.delete(&detach(owners), &parent).await.status,
		StatusCode::NO_CONTENT
	);
	// Gone now, and media must be detached via the memory it belongs to.
	assert_error(
		&app.delete(&detach(owners), &owner).await,
		StatusCode::NOT_FOUND,
		"not_found",
	);
	assert_error(
		&app.get(&format!("/api/media/{owners}"), &owner).await,
		StatusCode::NOT_FOUND,
		"not_found",
	);

	// A demoted uploader loses the right to remove their own uploads.
	let member_id: i64 =
		sqlx::query_scalar("SELECT id FROM users WHERE email = 'member@example.com'")
			.fetch_one(&app.state.db)
			.await
			.unwrap();
	let demote = app
		.put(
			&format!("/api/relationships/{relationship_id}/members/{member_id}"),
			&owner,
			json!({ "role": "VIEWER" }),
		)
		.await;
	assert_eq!(demote.status, StatusCode::OK, "{}", demote.body);
	assert_error(
		&app.delete(&detach(members_second), &member).await,
		StatusCode::FORBIDDEN,
		"forbidden",
	);
	assert_eq!(
		app.delete(&detach(members_second), &owner).await.status,
		StatusCode::NO_CONTENT
	);
}

#[sqlx::test]
async fn capsule_media_is_hidden_until_the_capsule_is_opened(pool: PgPool) {
	let app = TestApp::new(pool);
	let owner = app.register("ada@example.com").await;
	let viewer = app.register("vic@example.com").await;
	let stranger = app.register("eve@example.com").await;
	let (_, relationship_id) = app.child(&owner, "Mira").await;
	app.add_member(&owner, relationship_id, "vic@example.com", "VIEWER")
		.await;
	let capsule_id = capsule(&app, relationship_id).await;
	let path = format!("/api/capsules/{capsule_id}/media");

	assert_error(
		&upload(&app, &path, &viewer, &[("file", "a.png", "image/png", PNG)]).await,
		StatusCode::FORBIDDEN,
		"forbidden",
	);
	assert_error(
		&upload(
			&app,
			&path,
			&stranger,
			&[("file", "a.png", "image/png", PNG)],
		)
		.await,
		StatusCode::NOT_FOUND,
		"not_found",
	);
	let id = upload_png(&app, &path, &owner).await;

	// Not even the uploader can read it while it's locked.
	for token in [&owner, &viewer, &stranger] {
		for url in [
			format!("/api/media/{id}"),
			format!("/api/media/{id}/content"),
		] {
			assert_error(
				&app.get(&url, token).await,
				StatusCode::NOT_FOUND,
				"not_found",
			);
		}
	}

	// Past its unlock time but not yet opened (AVAILABLE): still hidden, and sealed.
	unlock(&app, capsule_id).await;
	assert_error(
		&app.get(&format!("/api/media/{id}/content"), &owner).await,
		StatusCode::NOT_FOUND,
		"not_found",
	);
	let sealed = [
		upload(&app, &path, &owner, &[("file", "b.png", "image/png", PNG)]).await,
		app.delete(&format!("{path}/{id}"), &owner).await,
	];
	for response in &sealed {
		assert_error(response, StatusCode::CONFLICT, "conflict");
	}

	open_capsule(&app, capsule_id).await;
	for token in [&owner, &viewer] {
		let metadata = app.get(&format!("/api/media/{id}"), token).await;
		assert_eq!(metadata.status, StatusCode::OK, "{}", metadata.body);
		let content = app.get(&format!("/api/media/{id}/content"), token).await;
		assert_eq!(content.status, StatusCode::OK);
		assert_eq!(&content.raw[..], PNG);
	}
	assert_error(
		&app.get(&format!("/api/media/{id}"), &stranger).await,
		StatusCode::NOT_FOUND,
		"not_found",
	);

	// Its contents stay fixed once opened.
	let sealed = [
		upload(&app, &path, &owner, &[("file", "b.png", "image/png", PNG)]).await,
		app.delete(&format!("{path}/{id}"), &owner).await,
	];
	for response in &sealed {
		assert_error(response, StatusCode::CONFLICT, "conflict");
	}
}

#[sqlx::test]
async fn cancelled_capsules_accept_no_media_and_never_reveal_it(pool: PgPool) {
	let app = TestApp::new(pool);
	let owner = app.register("ada@example.com").await;
	let (_, relationship_id) = app.child(&owner, "Mira").await;
	let capsule_id = capsule(&app, relationship_id).await;
	let path = format!("/api/capsules/{capsule_id}/media");
	let id = upload_png(&app, &path, &owner).await;

	sqlx::query("UPDATE time_capsules SET status_id = 4 WHERE id = $1")
		.bind(capsule_id)
		.execute(&app.state.db)
		.await
		.unwrap();
	assert_error(
		&upload(&app, &path, &owner, &[("file", "b.png", "image/png", PNG)]).await,
		StatusCode::CONFLICT,
		"conflict",
	);
	unlock(&app, capsule_id).await;
	assert_error(
		&app.get(&format!("/api/media/{id}/content"), &owner).await,
		StatusCode::NOT_FOUND,
		"not_found",
	);
	assert_error(
		&upload(
			&app,
			"/api/capsules/999999/media",
			&owner,
			&[("file", "b.png", "image/png", PNG)],
		)
		.await,
		StatusCode::NOT_FOUND,
		"not_found",
	);
}

#[sqlx::test]
async fn locked_capsule_media_can_be_removed_by_its_uploader_or_a_manager(pool: PgPool) {
	let app = TestApp::new(pool);
	let owner = app.register("ada@example.com").await;
	let member = app.register("max@example.com").await;
	let (_, relationship_id) = app.child(&owner, "Mira").await;
	app.add_member(&owner, relationship_id, "max@example.com", "MEMBER")
		.await;
	let capsule_id = capsule(&app, relationship_id).await;
	let path = format!("/api/capsules/{capsule_id}/media");
	let owners = upload_png(&app, &path, &owner).await;
	let members = upload_png(&app, &path, &member).await;
	let detach = |id: i64| format!("/api/capsules/{capsule_id}/media/{id}");

	// Other members can't even learn what's sealed inside.
	assert_error(
		&app.delete(&detach(owners), &member).await,
		StatusCode::NOT_FOUND,
		"not_found",
	);
	assert_eq!(
		app.delete(&detach(members), &member).await.status,
		StatusCode::NO_CONTENT
	);
	assert_eq!(
		app.delete(&detach(owners), &owner).await.status,
		StatusCode::NO_CONTENT
	);
	assert_eq!(media_count(&app).await, 0);
	assert_eq!(files_on_disk(&app), 0);
}

#[sqlx::test]
async fn media_is_deleted_when_its_last_link_is_removed(pool: PgPool) {
	let app = TestApp::new(pool);
	let owner = app.register("ada@example.com").await;
	let (_, relationship_id) = app.child(&owner, "Mira").await;
	let first = memory(&app, relationship_id).await;
	let second = memory(&app, relationship_id).await;
	let id = upload_png(&app, &format!("/api/memories/{first}/media"), &owner).await;
	sqlx::query("INSERT INTO memory_media (memory_id, media_id) VALUES ($1, $2)")
		.bind(second)
		.bind(id)
		.execute(&app.state.db)
		.await
		.unwrap();
	let file = app.media_dir().join(storage_path(&app, id).await.unwrap());

	let response = app
		.delete(&format!("/api/memories/{first}/media/{id}"), &owner)
		.await;
	assert_eq!(response.status, StatusCode::NO_CONTENT);
	assert!(storage_path(&app, id).await.is_some());
	assert!(file.exists());
	assert_eq!(
		app.get(&format!("/api/media/{id}"), &owner).await.status,
		StatusCode::OK
	);

	let response = app
		.delete(&format!("/api/memories/{second}/media/{id}"), &owner)
		.await;
	assert_eq!(response.status, StatusCode::NO_CONTENT);
	assert!(storage_path(&app, id).await.is_none());
	assert!(!file.exists());
}

#[sqlx::test]
async fn orphaned_media_can_be_cleaned_up(pool: PgPool) {
	let app = TestApp::new(pool);
	let owner = app.register("ada@example.com").await;
	let (_, relationship_id) = app.child(&owner, "Mira").await;
	let kept = memory(&app, relationship_id).await;
	let deleted = memory(&app, relationship_id).await;
	let kept_id = upload_png(&app, &format!("/api/memories/{kept}/media"), &owner).await;
	upload_png(&app, &format!("/api/memories/{deleted}/media"), &owner).await;

	let candidates = media::linked_media(&app.state.db, media::LinkScope::Memory(deleted))
		.await
		.unwrap();
	sqlx::query("DELETE FROM memories WHERE id = $1")
		.bind(deleted)
		.execute(&app.state.db)
		.await
		.unwrap();
	assert_eq!(
		media::delete_orphaned(&app.state.db, app.media_dir(), &candidates)
			.await
			.unwrap(),
		1
	);
	assert_eq!(media_count(&app).await, 1);
	assert_eq!(files_on_disk(&app), 1);
	let body: Value = app.get(&format!("/api/media/{kept_id}"), &owner).await.body;
	assert_eq!(body["id"], kept_id);
}

#[sqlx::test]
async fn deleting_a_relationship_removes_its_media_files(pool: PgPool) {
	let app = TestApp::new(pool);
	let owner = app.register("ada@example.com").await;
	let (_, relationship_id) = app.child(&owner, "Mira").await;
	let (_, other_relationship) = app.child(&owner, "Theo").await;
	let gone = memory(&app, relationship_id).await;
	let kept = memory(&app, other_relationship).await;
	upload_png(&app, &format!("/api/memories/{gone}/media"), &owner).await;
	let kept_id = upload_png(&app, &format!("/api/memories/{kept}/media"), &owner).await;
	assert_eq!(files_on_disk(&app), 2);

	let response = app
		.delete(&format!("/api/relationships/{relationship_id}"), &owner)
		.await;
	assert_eq!(response.status, StatusCode::NO_CONTENT);
	assert_eq!(media_count(&app).await, 1);
	assert_eq!(files_on_disk(&app), 1);
	assert_eq!(
		app.get(&format!("/api/media/{kept_id}"), &owner)
			.await
			.status,
		StatusCode::OK
	);
}
