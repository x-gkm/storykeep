//! Part 5: memories, tags and the timeline.

mod common;

use common::{TestApp, assert_error};
use serde_json::{Value, json};
use sqlx::PgPool;
use warp::http::StatusCode;

impl TestApp {
	/// Creates a memory and returns its JSON.
	async fn memory(&self, token: &str, relationship_id: i64, body: Value) -> Value {
		let response = self
			.post(
				&format!("/api/relationships/{relationship_id}/memories"),
				token,
				body,
			)
			.await;
		assert_eq!(response.status, StatusCode::CREATED, "{}", response.body);
		response.body
	}

	/// Titles on a timeline page, in order.
	async fn titles(&self, token: &str, path: &str) -> Vec<String> {
		let response = self.get(path, token).await;
		assert_eq!(response.status, StatusCode::OK, "{}", response.body);
		response.body["memories"]
			.as_array()
			.unwrap()
			.iter()
			.map(|memory| memory["title"].as_str().unwrap().to_owned())
			.collect()
	}
}

fn simple(title: &str, date: &str) -> Value {
	json!({ "category": "GENERAL", "title": title, "memory_date": date })
}

#[sqlx::test]
async fn memory_crud(pool: PgPool) {
	let app = TestApp::new(pool);
	let token = app.register("ada@example.com").await;
	let (_, relationship_id) = app.child(&token, "Mira").await;

	let memory = app
		.memory(
			&token,
			relationship_id,
			json!({
				"category": "MILESTONE",
				"title": "  First steps  ",
				"description": "Across the living room",
				"memory_date": "2025-03-14",
				"tags": ["walking", "Home", "WALKING"],
			}),
		)
		.await;
	let id = memory["id"].as_i64().unwrap();
	assert_eq!(memory["relationship_id"], relationship_id);
	assert_eq!(memory["category"], "MILESTONE");
	assert_eq!(memory["title"], "First steps");
	assert_eq!(memory["description"], "Across the living room");
	assert_eq!(memory["memory_date"], "2025-03-14");
	assert_eq!(memory["created_by"]["first_name"], "Test");
	assert_eq!(memory["created_by"]["last_name"], "ada");
	assert!(memory["created_by"]["id"].is_i64());
	assert!(memory["created_by"].get("email").is_none());
	assert!(memory["created_at"].is_string());
	assert!(memory["updated_at"].is_string());
	assert_eq!(memory["tags"], json!(["Home", "walking"]));
	assert_eq!(memory["media"], json!([]));

	let path = format!("/api/memories/{id}");
	let fetched = app.get(&path, &token).await;
	assert_eq!(fetched.status, StatusCode::OK);
	assert_eq!(fetched.body, memory);

	let updated = app
		.put(
			&path,
			&token,
			json!({
				"category": "EVERYDAY",
				"title": "First real steps",
				"memory_date": "2025-03-15",
				"tags": ["walking", "proud"],
			}),
		)
		.await;
	assert_eq!(updated.status, StatusCode::OK, "{}", updated.body);
	assert_eq!(updated.body["category"], "EVERYDAY");
	assert_eq!(updated.body["title"], "First real steps");
	assert_eq!(updated.body["description"], Value::Null);
	assert_eq!(updated.body["memory_date"], "2025-03-15");
	assert_eq!(updated.body["tags"], json!(["proud", "walking"]));
	assert_eq!(updated.body["created_at"], memory["created_at"]);

	// Omitting tags on a full replace clears them.
	let cleared = app
		.put(&path, &token, simple("First real steps", "2025-03-15"))
		.await;
	assert_eq!(cleared.body["tags"], json!([]));

	let deleted = app.delete(&path, &token).await;
	assert_eq!(deleted.status, StatusCode::NO_CONTENT);
	assert_error(
		&app.get(&path, &token).await,
		StatusCode::NOT_FOUND,
		"not_found",
	);
	assert_error(
		&app.delete(&path, &token).await,
		StatusCode::NOT_FOUND,
		"not_found",
	);
}

#[sqlx::test]
async fn memory_input_is_validated(pool: PgPool) {
	let app = TestApp::new(pool);
	let token = app.register("ada@example.com").await;
	let (_, relationship_id) = app.child(&token, "Mira").await;
	let path = format!("/api/relationships/{relationship_id}/memories");
	let long_tag = "x".repeat(51);
	let many_tags: Vec<String> = (0..21).map(|i| format!("tag{i}")).collect();

	for body in [
		json!({ "category": "PICNIC", "title": "Hi", "memory_date": "2025-01-01" }),
		json!({ "category": "GENERAL", "title": "   ", "memory_date": "2025-01-01" }),
		json!({ "category": "GENERAL", "title": "x".repeat(201), "memory_date": "2025-01-01" }),
		json!({ "category": "GENERAL", "title": "Hi", "memory_date": "2025-13-01" }),
		json!({ "category": "GENERAL", "title": "Hi" }),
		json!({ "category": "GENERAL", "title": "Hi", "memory_date": "2025-01-01", "tags": [" "] }),
		json!({ "category": "GENERAL", "title": "Hi", "memory_date": "2025-01-01", "tags": [long_tag] }),
		json!({ "category": "GENERAL", "title": "Hi", "memory_date": "2025-01-01", "tags": many_tags }),
	] {
		let response = app.post(&path, &token, body.clone()).await;
		assert_error(&response, StatusCode::BAD_REQUEST, "bad_request");
	}

	let ok = app
		.memory(
			&token,
			relationship_id,
			simple(&"x".repeat(200), "2025-01-01"),
		)
		.await;
	let update = app
		.put(
			&format!("/api/memories/{}", ok["id"]),
			&token,
			json!({ "category": "GENERAL", "title": "", "memory_date": "2025-01-01" }),
		)
		.await;
	assert_error(&update, StatusCode::BAD_REQUEST, "bad_request");
}

#[sqlx::test]
async fn timeline_orders_filters_and_paginates(pool: PgPool) {
	let app = TestApp::new(pool);
	let token = app.register("ada@example.com").await;
	let (_, relationship_id) = app.child(&token, "Mira").await;
	let (_, other_relationship) = app.child(&token, "Noah").await;
	let base = format!("/api/relationships/{relationship_id}/memories");

	for (category, title, description, date, tags) in [
		(
			"BIRTHDAY",
			"First birthday",
			"Cake everywhere",
			"2025-03-01",
			json!(["Cake", "family"]),
		),
		(
			"TRAVEL",
			"Beach trip",
			"Sand and SUN",
			"2025-07-10",
			json!(["Family"]),
		),
		(
			"MILESTONE",
			"First word",
			"She said cat",
			"2024-11-20",
			json!([]),
		),
		("GENERAL", "Park day", "", "2025-07-10", json!(["park"])),
		(
			"BIRTHDAY",
			"Second birthday",
			"More cake",
			"2026-03-01",
			json!(["cake"]),
		),
	] {
		app.memory(
			&token,
			relationship_id,
			json!({
				"category": category,
				"title": title,
				"description": description,
				"memory_date": date,
				"tags": tags,
			}),
		)
		.await;
	}
	app.memory(
		&token,
		other_relationship,
		simple("Noah's day", "2025-05-05"),
	)
	.await;

	// Newest first by default; same-day memories by id.
	assert_eq!(
		app.titles(&token, &base).await,
		[
			"Second birthday",
			"Park day",
			"Beach trip",
			"First birthday",
			"First word"
		]
	);
	assert_eq!(
		app.titles(&token, &format!("{base}?order=asc")).await,
		[
			"First word",
			"First birthday",
			"Beach trip",
			"Park day",
			"Second birthday"
		]
	);

	let page = app.get(&base, &token).await.body;
	assert_eq!(page["total"], 5);
	assert_eq!(page["limit"], 50);
	assert_eq!(page["offset"], 0);

	// Inclusive date range.
	assert_eq!(
		app.titles(
			&token,
			&format!("{base}?from=2025-03-01&to=2025-07-10&order=asc")
		)
		.await,
		["First birthday", "Beach trip", "Park day"]
	);
	assert_eq!(
		app.titles(&token, &format!("{base}?from=2025-07-11")).await,
		["Second birthday"]
	);
	assert_eq!(
		app.titles(&token, &format!("{base}?to=2024-12-31")).await,
		["First word"]
	);
	assert_eq!(
		app.titles(&token, &format!("{base}?category=BIRTHDAY"))
			.await,
		["Second birthday", "First birthday"]
	);
	// Tag names match case-insensitively.
	assert_eq!(
		app.titles(&token, &format!("{base}?tag=CAKE")).await,
		["Second birthday", "First birthday"]
	);
	assert_eq!(
		app.titles(&token, &format!("{base}?tag=family&order=asc"))
			.await,
		["First birthday", "Beach trip"]
	);
	// Search covers title and description, case-insensitively; `%` is literal.
	assert_eq!(
		app.titles(&token, &format!("{base}?q=sun")).await,
		["Beach trip"]
	);
	assert_eq!(
		app.titles(&token, &format!("{base}?q=BIRTH")).await,
		["Second birthday", "First birthday"]
	);
	assert!(
		app.titles(&token, &format!("{base}?q=%25"))
			.await
			.is_empty()
	);
	// Filters combine.
	let combined = app
		.get(
			&format!("{base}?category=BIRTHDAY&tag=cake&from=2025-01-01&to=2025-12-31"),
			&token,
		)
		.await;
	assert_eq!(combined.body["total"], 1);
	assert_eq!(combined.body["memories"][0]["title"], "First birthday");

	// Pagination; `total` ignores limit/offset.
	let page = app
		.get(&format!("{base}?order=asc&limit=2&offset=2"), &token)
		.await;
	assert_eq!(page.body["total"], 5);
	assert_eq!(page.body["limit"], 2);
	assert_eq!(page.body["offset"], 2);
	let titles: Vec<&str> = page.body["memories"]
		.as_array()
		.unwrap()
		.iter()
		.map(|m| m["title"].as_str().unwrap())
		.collect();
	assert_eq!(titles, ["Beach trip", "Park day"]);
	let past_end = app.get(&format!("{base}?offset=10"), &token).await;
	assert_eq!(past_end.body["memories"], json!([]));
	assert_eq!(past_end.body["total"], 5);

	// Each memory in the list carries its tags.
	let first = app
		.get(&format!("{base}?order=asc&limit=2&offset=1"), &token)
		.await;
	assert_eq!(first.body["memories"][0]["tags"], json!(["Cake", "family"]));
}

#[sqlx::test]
async fn timeline_query_is_validated(pool: PgPool) {
	let app = TestApp::new(pool);
	let token = app.register("ada@example.com").await;
	let (_, relationship_id) = app.child(&token, "Mira").await;
	let base = format!("/api/relationships/{relationship_id}/memories");

	for query in [
		"from=yesterday",
		"to=2025-02-30",
		"from=2025-05-01&to=2025-04-01",
		"category=PICNIC",
		"order=sideways",
		"limit=0",
		"limit=201",
		"limit=ten",
		"offset=-1",
	] {
		let response = app.get(&format!("{base}?{query}"), &token).await;
		assert_error(&response, StatusCode::BAD_REQUEST, "bad_request");
	}
	let response = app.get(&format!("{base}?limit=200"), &token).await;
	assert_eq!(response.status, StatusCode::OK);
}

#[sqlx::test]
async fn tags_are_shared_case_insensitively_but_listed_per_reader(pool: PgPool) {
	let app = TestApp::new(pool.clone());
	let ada = app.register("ada@example.com").await;
	let eve = app.register("eve@example.com").await;
	let (_, ada_rel) = app.child(&ada, "Mira").await;
	let (_, ada_rel2) = app.child(&ada, "Noah").await;
	let (_, eve_rel) = app.child(&eve, "Zoe").await;

	app.memory(
		&ada,
		ada_rel,
		json!({ "category": "GENERAL", "title": "A", "memory_date": "2025-01-01", "tags": ["Beach", "family"] }),
	)
	.await;
	app.memory(
		&ada,
		ada_rel,
		json!({ "category": "GENERAL", "title": "B", "memory_date": "2025-01-02", "tags": ["beach"] }),
	)
	.await;
	app.memory(
		&ada,
		ada_rel2,
		json!({ "category": "GENERAL", "title": "C", "memory_date": "2025-01-03", "tags": ["BEACH", "zoo"] }),
	)
	.await;
	let eve_memory = app
		.memory(
			&eve,
			eve_rel,
			json!({ "category": "GENERAL", "title": "D", "memory_date": "2025-01-04", "tags": ["beach", "Secret Diagnosis"] }),
		)
		.await;
	// The existing spelling is reused.
	assert_eq!(eve_memory["tags"], json!(["Beach", "Secret Diagnosis"]));

	let tag_rows: i64 = sqlx::query_scalar("SELECT count(*) FROM tags")
		.fetch_one(&pool)
		.await
		.unwrap();
	assert_eq!(tag_rows, 4);

	let ada_tags = app.get("/api/tags", &ada).await;
	assert_eq!(ada_tags.status, StatusCode::OK);
	assert_eq!(
		ada_tags.body,
		json!([
			{ "name": "Beach", "memory_count": 3 },
			{ "name": "family", "memory_count": 1 },
			{ "name": "zoo", "memory_count": 1 },
		])
	);
	let eve_tags = app.get("/api/tags", &eve).await;
	assert_eq!(
		eve_tags.body,
		json!([
			{ "name": "Beach", "memory_count": 1 },
			{ "name": "Secret Diagnosis", "memory_count": 1 },
		])
	);
	assert_eq!(app.get("/api/tags?q=SEC", &ada).await.body, json!([]));
	assert_eq!(
		app.get("/api/tags?q=AM", &ada).await.body,
		json!([{ "name": "family", "memory_count": 1 }])
	);
	assert_eq!(
		app.get("/api/tags?limit=1", &ada).await.body,
		json!([{ "name": "Beach", "memory_count": 3 }])
	);
	assert_error(
		&app.get("/api/tags?limit=0", &ada).await,
		StatusCode::BAD_REQUEST,
		"bad_request",
	);

	// Per-relationship listing.
	assert_eq!(
		app.get(&format!("/api/relationships/{ada_rel2}/tags"), &ada)
			.await
			.body,
		json!([
			{ "name": "Beach", "memory_count": 1 },
			{ "name": "zoo", "memory_count": 1 },
		])
	);
	assert_error(
		&app.get(&format!("/api/relationships/{eve_rel}/tags"), &ada)
			.await,
		StatusCode::NOT_FOUND,
		"not_found",
	);

	// Creating a tag returns an existing one, without revealing another user's spelling.
	let created = app
		.post("/api/tags", &ada, json!({ "name": "  Picnic " }))
		.await;
	assert_eq!(created.status, StatusCode::OK, "{}", created.body);
	assert_eq!(created.body, json!({ "name": "Picnic", "memory_count": 0 }));
	let again = app
		.post("/api/tags", &eve, json!({ "name": "PICNIC" }))
		.await;
	assert_eq!(again.body, json!({ "name": "PICNIC", "memory_count": 0 }));
	let secret = app
		.post("/api/tags", &ada, json!({ "name": "secret diagnosis" }))
		.await;
	assert_eq!(
		secret.body,
		json!({ "name": "secret diagnosis", "memory_count": 0 })
	);
	let visible = app
		.post("/api/tags", &ada, json!({ "name": "BEACH" }))
		.await;
	assert_eq!(visible.body, json!({ "name": "Beach", "memory_count": 3 }));
	let tag_rows: i64 = sqlx::query_scalar("SELECT count(*) FROM tags")
		.fetch_one(&pool)
		.await
		.unwrap();
	assert_eq!(tag_rows, 5);
	// Unused tags don't show up in listings.
	assert!(
		!app.get("/api/tags", &ada)
			.await
			.body
			.as_array()
			.unwrap()
			.iter()
			.any(|t| t["name"] == "Picnic")
	);

	for name in ["", "   ", &"x".repeat(51)] {
		assert_error(
			&app.post("/api/tags", &ada, json!({ "name": name })).await,
			StatusCode::BAD_REQUEST,
			"bad_request",
		);
	}
	assert_error(
		&app.call("GET", "/api/tags", None, None).await,
		StatusCode::UNAUTHORIZED,
		"unauthorized",
	);
}

#[sqlx::test]
async fn strangers_cannot_see_or_touch_memories(pool: PgPool) {
	let app = TestApp::new(pool);
	let owner = app.register("ada@example.com").await;
	let stranger = app.register("eve@example.com").await;
	let (_, relationship_id) = app.child(&owner, "Mira").await;
	let memory = app
		.memory(&owner, relationship_id, simple("Private", "2025-01-01"))
		.await;
	let path = format!("/api/memories/{}", memory["id"]);
	let list = format!("/api/relationships/{relationship_id}/memories");

	for response in [
		app.get(&list, &stranger).await,
		app.post(&list, &stranger, simple("Intruder", "2025-01-01"))
			.await,
		app.get(&path, &stranger).await,
		app.put(&path, &stranger, simple("Hacked", "2025-01-01"))
			.await,
		app.delete(&path, &stranger).await,
		app.get("/api/memories/999999", &owner).await,
		app.get("/api/relationships/999999/memories", &owner).await,
	] {
		assert_error(&response, StatusCode::NOT_FOUND, "not_found");
	}
	assert_error(
		&app.call("GET", &path, None, None).await,
		StatusCode::UNAUTHORIZED,
		"unauthorized",
	);
	assert_eq!(app.get(&path, &owner).await.body["title"], "Private");
}

#[sqlx::test]
async fn roles_control_who_may_edit_memories(pool: PgPool) {
	let app = TestApp::new(pool);
	let owner = app.register("owner@example.com").await;
	let parent = app.register("parent@example.com").await;
	let member = app.register("member@example.com").await;
	let other_member = app.register("other@example.com").await;
	let viewer = app.register("viewer@example.com").await;
	let (_, relationship_id) = app.child(&owner, "Mira").await;
	for (email, role) in [
		("parent@example.com", "PARENT"),
		("member@example.com", "MEMBER"),
		("other@example.com", "MEMBER"),
		("viewer@example.com", "VIEWER"),
	] {
		app.add_member(&owner, relationship_id, email, role).await;
	}
	let list = format!("/api/relationships/{relationship_id}/memories");

	// VIEWER: read only.
	let owners = app
		.memory(&owner, relationship_id, simple("Owner's", "2025-01-01"))
		.await;
	let owners_path = format!("/api/memories/{}", owners["id"]);
	assert_eq!(app.get(&owners_path, &viewer).await.status, StatusCode::OK);
	assert_eq!(app.get(&list, &viewer).await.body["total"], 1);
	assert_eq!(app.get("/api/tags", &viewer).await.status, StatusCode::OK);
	for response in [
		app.post(&list, &viewer, simple("Nope", "2025-01-01")).await,
		app.put(&owners_path, &viewer, simple("Nope", "2025-01-01"))
			.await,
		app.delete(&owners_path, &viewer).await,
	] {
		assert_error(&response, StatusCode::FORBIDDEN, "forbidden");
	}

	// MEMBER: may create and edit their own, not others'.
	let mine = app
		.memory(&member, relationship_id, simple("Member's", "2025-02-01"))
		.await;
	let mine_path = format!("/api/memories/{}", mine["id"]);
	let edited = app
		.put(&mine_path, &member, simple("Member's edited", "2025-02-01"))
		.await;
	assert_eq!(edited.status, StatusCode::OK, "{}", edited.body);
	for response in [
		app.put(&owners_path, &member, simple("Nope", "2025-01-01"))
			.await,
		app.delete(&owners_path, &member).await,
		app.put(&mine_path, &other_member, simple("Nope", "2025-01-01"))
			.await,
		app.delete(&mine_path, &other_member).await,
	] {
		assert_error(&response, StatusCode::FORBIDDEN, "forbidden");
	}

	// PARENT: may edit and delete anyone's.
	let by_parent = app
		.put(&mine_path, &parent, simple("Parent edited", "2025-02-02"))
		.await;
	assert_eq!(by_parent.status, StatusCode::OK, "{}", by_parent.body);
	// The author stays the original creator.
	assert_eq!(by_parent.body["created_by"], mine["created_by"]);
	assert_eq!(
		app.put(&owners_path, &parent, simple("Parent edited", "2025-01-01"))
			.await
			.status,
		StatusCode::OK
	);

	// A creator demoted to VIEWER loses the right to edit their memory.
	let response = app
		.put(
			&format!(
				"/api/relationships/{relationship_id}/members/{}",
				mine["created_by"]["id"]
			),
			&owner,
			json!({ "role": "VIEWER" }),
		)
		.await;
	assert_eq!(response.status, StatusCode::OK, "{}", response.body);
	assert_error(
		&app.put(&mine_path, &member, simple("Nope", "2025-01-01"))
			.await,
		StatusCode::FORBIDDEN,
		"forbidden",
	);

	assert_eq!(
		app.delete(&mine_path, &parent).await.status,
		StatusCode::NO_CONTENT
	);
	assert_eq!(
		app.delete(&owners_path, &owner).await.status,
		StatusCode::NO_CONTENT
	);
}

#[sqlx::test]
async fn attached_media_appears_in_memories(pool: PgPool) {
	let app = TestApp::new(pool.clone());
	let token = app.register("ada@example.com").await;
	let (_, relationship_id) = app.child(&token, "Mira").await;
	let with_media = app
		.memory(&token, relationship_id, simple("With photos", "2025-01-01"))
		.await;
	let without = app
		.memory(&token, relationship_id, simple("Plain", "2025-01-02"))
		.await;
	let memory_id = with_media["id"].as_i64().unwrap();
	let user_id = with_media["created_by"]["id"].as_i64().unwrap();

	let mut media_ids = Vec::new();
	for (type_id, name, mime, size) in [
		(1_i16, "beach.jpg", "image/jpeg", Some(1234_i64)),
		(2, "waves.mp4", "video/mp4", None),
	] {
		let id: i64 = sqlx::query_scalar(
			"INSERT INTO media (media_type_id, storage_path, file_name, mime_type, file_size, uploaded_by)
			VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
		)
		.bind(type_id)
		.bind(format!("test/{name}"))
		.bind(name)
		.bind(mime)
		.bind(size)
		.bind(user_id)
		.fetch_one(&pool)
		.await
		.unwrap();
		sqlx::query("INSERT INTO memory_media (memory_id, media_id) VALUES ($1, $2)")
			.bind(memory_id)
			.bind(id)
			.execute(&pool)
			.await
			.unwrap();
		media_ids.push(id);
	}

	let fetched = app.get(&format!("/api/memories/{memory_id}"), &token).await;
	let media = fetched.body["media"].as_array().unwrap();
	assert_eq!(media.len(), 2);
	assert_eq!(media[0]["id"], media_ids[0]);
	assert_eq!(media[0]["media_type"], "IMAGE");
	assert_eq!(media[0]["file_name"], "beach.jpg");
	assert_eq!(media[0]["mime_type"], "image/jpeg");
	assert_eq!(media[0]["file_size"], 1234);
	assert!(media[0]["created_at"].is_string());
	assert_eq!(
		media[0]["content_url"],
		format!("/api/media/{}/content", media_ids[0])
	);
	assert!(media[0].get("storage_path").is_none());
	assert_eq!(media[1]["media_type"], "VIDEO");
	assert_eq!(media[1]["file_size"], Value::Null);

	// The timeline batch-loads media per memory.
	let timeline = app
		.get(
			&format!("/api/relationships/{relationship_id}/memories?order=asc"),
			&token,
		)
		.await;
	assert_eq!(
		timeline.body["memories"][0]["media"]
			.as_array()
			.unwrap()
			.len(),
		2
	);
	assert_eq!(timeline.body["memories"][1]["id"], without["id"]);
	assert_eq!(timeline.body["memories"][1]["media"], json!([]));
}
