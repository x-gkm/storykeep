//! Part 4: profiles, relationships, membership and role-based authorization.

mod common;

use common::{TestApp, assert_error};
use serde_json::json;
use sqlx::PgPool;
use warp::http::StatusCode;

#[sqlx::test]
async fn creating_a_profile_makes_the_creator_its_owner(pool: PgPool) {
	let app = TestApp::new(pool);
	let token = app.register("ada@example.com").await;

	let response = app
		.post(
			"/api/profiles",
			&token,
			json!({
				"profile_type": "PET",
				"name": "Biscuit",
				"relationship_type": "OWNER_PET",
				"started_at": "2025-06-01",
			}),
		)
		.await;
	assert_eq!(response.status, StatusCode::CREATED, "{}", response.body);
	let profile = &response.body["profile"];
	assert_eq!(profile["profile_type"], "PET");
	assert_eq!(profile["role"], "OWNER");
	let relationship = &response.body["relationship"];
	assert_eq!(relationship["profile_id"], profile["id"]);
	assert_eq!(relationship["relationship_type"], "OWNER_PET");
	assert_eq!(relationship["started_at"], "2025-06-01");
	assert_eq!(relationship["role"], "OWNER");

	let profiles = app.get("/api/profiles", &token).await;
	assert_eq!(profiles.body.as_array().unwrap().len(), 1);
	let relationships = app.get("/api/relationships", &token).await;
	assert_eq!(relationships.body.as_array().unwrap().len(), 1);
	assert_eq!(relationships.body[0]["profile_name"], "Biscuit");
}

#[sqlx::test]
async fn profile_input_is_validated(pool: PgPool) {
	let app = TestApp::new(pool);
	let token = app.register("ada@example.com").await;
	for body in [
		json!({ "profile_type": "DRAGON", "name": "Smaug", "relationship_type": "OTHER" }),
		json!({ "profile_type": "CHILD", "name": "  ", "relationship_type": "PARENT_CHILD" }),
		json!({ "profile_type": "CHILD", "name": "Mira" }),
	] {
		let response = app.post("/api/profiles", &token, body).await;
		assert_error(&response, StatusCode::BAD_REQUEST, "bad_request");
	}
}

#[sqlx::test]
async fn non_members_cannot_see_profiles_or_relationships(pool: PgPool) {
	let app = TestApp::new(pool);
	let owner = app.register("ada@example.com").await;
	let stranger = app.register("eve@example.com").await;
	let (profile_id, relationship_id) = app.child(&owner, "Mira").await;

	assert_eq!(app.get("/api/profiles", &stranger).await.body, json!([]));
	assert_eq!(
		app.get("/api/relationships", &stranger).await.body,
		json!([])
	);
	for path in [
		format!("/api/profiles/{profile_id}"),
		format!("/api/relationships/{relationship_id}"),
		format!("/api/relationships/{relationship_id}/members"),
	] {
		assert_error(
			&app.get(&path, &stranger).await,
			StatusCode::NOT_FOUND,
			"not_found",
		);
	}
	let update = app
		.put(
			&format!("/api/profiles/{profile_id}"),
			&stranger,
			json!({ "profile_type": "CHILD", "name": "Hacked" }),
		)
		.await;
	assert_error(&update, StatusCode::NOT_FOUND, "not_found");
	assert_error(
		&app.delete(&format!("/api/relationships/{relationship_id}"), &stranger)
			.await,
		StatusCode::NOT_FOUND,
		"not_found",
	);
	// Unknown ids look the same as ones the user can't access.
	assert_error(
		&app.get("/api/profiles/999999", &stranger).await,
		StatusCode::NOT_FOUND,
		"not_found",
	);
}

#[sqlx::test]
async fn roles_control_who_may_edit_a_profile(pool: PgPool) {
	let app = TestApp::new(pool);
	let owner = app.register("owner@example.com").await;
	let parent = app.register("parent@example.com").await;
	let member = app.register("member@example.com").await;
	let viewer = app.register("viewer@example.com").await;
	let (profile_id, relationship_id) = app.child(&owner, "Mira").await;
	app.add_member(&owner, relationship_id, "parent@example.com", "PARENT")
		.await;
	app.add_member(&owner, relationship_id, "member@example.com", "MEMBER")
		.await;
	app.add_member(&owner, relationship_id, "viewer@example.com", "VIEWER")
		.await;

	let path = format!("/api/profiles/{profile_id}");
	let rename = |name: &str| json!({ "profile_type": "CHILD", "name": name, "date_of_birth": "2024-03-01" });

	let viewed = app.get(&path, &viewer).await;
	assert_eq!(viewed.status, StatusCode::OK);
	assert_eq!(viewed.body["role"], "VIEWER");

	assert_error(
		&app.put(&path, &viewer, rename("V")).await,
		StatusCode::FORBIDDEN,
		"forbidden",
	);
	assert_error(
		&app.put(&path, &member, rename("M")).await,
		StatusCode::FORBIDDEN,
		"forbidden",
	);
	let updated = app.put(&path, &parent, rename("Mira Rose")).await;
	assert_eq!(updated.status, StatusCode::OK, "{}", updated.body);
	assert_eq!(updated.body["name"], "Mira Rose");

	assert_error(
		&app.delete(&path, &parent).await,
		StatusCode::FORBIDDEN,
		"forbidden",
	);
	assert_eq!(
		app.delete(&path, &owner).await.status,
		StatusCode::NO_CONTENT
	);
	assert_error(
		&app.get(&path, &owner).await,
		StatusCode::NOT_FOUND,
		"not_found",
	);
	assert_eq!(app.get("/api/relationships", &viewer).await.body, json!([]));
}

#[sqlx::test]
async fn deleting_a_profile_requires_owning_every_relationship(pool: PgPool) {
	let app = TestApp::new(pool);
	let ada = app.register("ada@example.com").await;
	let grace = app.register("grace@example.com").await;
	let (profile_id, relationship_id) = app.child(&ada, "Mira").await;
	app.add_member(&ada, relationship_id, "grace@example.com", "PARENT")
		.await;

	// Grace starts a second relationship with the same profile, which she owns.
	let family = app
		.post(
			"/api/relationships",
			&grace,
			json!({ "profile_id": profile_id, "relationship_type": "FAMILY" }),
		)
		.await;
	assert_eq!(family.status, StatusCode::CREATED, "{}", family.body);
	assert_eq!(family.body["role"], "OWNER");

	assert_error(
		&app.delete(&format!("/api/profiles/{profile_id}"), &ada)
			.await,
		StatusCode::FORBIDDEN,
		"forbidden",
	);
}

#[sqlx::test]
async fn adding_relationships_to_a_profile_requires_managing_it(pool: PgPool) {
	let app = TestApp::new(pool);
	let owner = app.register("owner@example.com").await;
	let viewer = app.register("viewer@example.com").await;
	let stranger = app.register("eve@example.com").await;
	let (profile_id, relationship_id) = app.child(&owner, "Mira").await;
	app.add_member(&owner, relationship_id, "viewer@example.com", "VIEWER")
		.await;

	let body = json!({ "profile_id": profile_id, "relationship_type": "FAMILY" });
	assert_error(
		&app.post("/api/relationships", &viewer, body.clone()).await,
		StatusCode::FORBIDDEN,
		"forbidden",
	);
	assert_error(
		&app.post("/api/relationships", &stranger, body).await,
		StatusCode::NOT_FOUND,
		"not_found",
	);
}

#[sqlx::test]
async fn relationships_can_be_updated_and_deleted_by_the_right_roles(pool: PgPool) {
	let app = TestApp::new(pool);
	let owner = app.register("owner@example.com").await;
	let parent = app.register("parent@example.com").await;
	let member = app.register("member@example.com").await;
	let (_, relationship_id) = app.child(&owner, "Mira").await;
	app.add_member(&owner, relationship_id, "parent@example.com", "PARENT")
		.await;
	app.add_member(&owner, relationship_id, "member@example.com", "MEMBER")
		.await;
	let path = format!("/api/relationships/{relationship_id}");

	let body = json!({ "relationship_type": "FAMILY", "started_at": "2024-03-01" });
	assert_error(
		&app.put(&path, &member, body.clone()).await,
		StatusCode::FORBIDDEN,
		"forbidden",
	);
	let updated = app.put(&path, &parent, body).await;
	assert_eq!(updated.status, StatusCode::OK, "{}", updated.body);
	assert_eq!(updated.body["relationship_type"], "FAMILY");
	assert_eq!(updated.body["role"], "PARENT");

	let backwards = app
		.put(
			&path,
			&owner,
			json!({ "relationship_type": "FAMILY", "started_at": "2025-01-01", "ended_at": "2024-01-01" }),
		)
		.await;
	assert_error(&backwards, StatusCode::BAD_REQUEST, "bad_request");

	assert_error(
		&app.delete(&path, &parent).await,
		StatusCode::FORBIDDEN,
		"forbidden",
	);
	assert_eq!(
		app.delete(&path, &owner).await.status,
		StatusCode::NO_CONTENT
	);
	assert_error(
		&app.get(&path, &member).await,
		StatusCode::NOT_FOUND,
		"not_found",
	);
}

#[sqlx::test]
async fn members_can_be_listed_and_added(pool: PgPool) {
	let app = TestApp::new(pool);
	let owner = app.register("owner@example.com").await;
	let parent = app.register("parent@example.com").await;
	let member = app.register("member@example.com").await;
	app.register("viewer@example.com").await;
	app.register("other@example.com").await;
	let (_, relationship_id) = app.child(&owner, "Mira").await;
	let path = format!("/api/relationships/{relationship_id}/members");

	app.add_member(&owner, relationship_id, "PARENT@example.com", "PARENT")
		.await;
	app.add_member(&parent, relationship_id, "member@example.com", "MEMBER")
		.await;

	let duplicate = app
		.post(
			&path,
			&owner,
			json!({ "email": "member@example.com", "role": "VIEWER" }),
		)
		.await;
	assert_error(&duplicate, StatusCode::CONFLICT, "conflict");
	let unknown = app
		.post(
			&path,
			&owner,
			json!({ "email": "nobody@example.com", "role": "VIEWER" }),
		)
		.await;
	assert_error(&unknown, StatusCode::NOT_FOUND, "not_found");
	let by_member = app
		.post(
			&path,
			&member,
			json!({ "email": "viewer@example.com", "role": "VIEWER" }),
		)
		.await;
	assert_error(&by_member, StatusCode::FORBIDDEN, "forbidden");
	let owner_by_parent = app
		.post(
			&path,
			&parent,
			json!({ "email": "other@example.com", "role": "OWNER" }),
		)
		.await;
	assert_error(&owner_by_parent, StatusCode::FORBIDDEN, "forbidden");

	let members = app.get(&path, &member).await;
	assert_eq!(members.status, StatusCode::OK);
	let roles: Vec<_> = members
		.body
		.as_array()
		.unwrap()
		.iter()
		.map(|m| m["role"].clone())
		.collect();
	assert_eq!(roles, [json!("OWNER"), json!("PARENT"), json!("MEMBER")]);
	assert!(members.body[0].get("password_hash").is_none());
}

#[sqlx::test]
async fn member_roles_can_change_but_an_owner_must_remain(pool: PgPool) {
	let app = TestApp::new(pool);
	let owner = app.register("owner@example.com").await;
	let parent = app.register("parent@example.com").await;
	app.register("second@example.com").await;
	let (_, relationship_id) = app.child(&owner, "Mira").await;
	app.add_member(&owner, relationship_id, "parent@example.com", "PARENT")
		.await;
	app.add_member(&owner, relationship_id, "second@example.com", "MEMBER")
		.await;
	let ids: Vec<(i64, String)> = sqlx::query_as("SELECT id, email FROM users ORDER BY id")
		.fetch_all(&app.state.db)
		.await
		.unwrap();
	let id_of = |email: &str| ids.iter().find(|(_, e)| e == email).unwrap().0;
	let member_path = |email: &str| {
		format!(
			"/api/relationships/{relationship_id}/members/{}",
			id_of(email)
		)
	};

	// The only owner can't demote themselves.
	let demote_self = app
		.put(
			&member_path("owner@example.com"),
			&owner,
			json!({ "role": "MEMBER" }),
		)
		.await;
	assert_error(&demote_self, StatusCode::CONFLICT, "conflict");

	// Parents can change non-owner roles but can't touch owners.
	let promoted = app
		.put(
			&member_path("second@example.com"),
			&parent,
			json!({ "role": "VIEWER" }),
		)
		.await;
	assert_eq!(promoted.status, StatusCode::OK, "{}", promoted.body);
	let demote_owner = app
		.put(
			&member_path("owner@example.com"),
			&parent,
			json!({ "role": "MEMBER" }),
		)
		.await;
	assert_error(&demote_owner, StatusCode::FORBIDDEN, "forbidden");

	// With a second owner, the first can step down.
	let co_owner = app
		.put(
			&member_path("parent@example.com"),
			&owner,
			json!({ "role": "OWNER" }),
		)
		.await;
	assert_eq!(co_owner.status, StatusCode::OK, "{}", co_owner.body);
	let step_down = app
		.put(
			&member_path("owner@example.com"),
			&owner,
			json!({ "role": "VIEWER" }),
		)
		.await;
	assert_eq!(step_down.status, StatusCode::OK, "{}", step_down.body);
	let relationship = app
		.get(&format!("/api/relationships/{relationship_id}"), &owner)
		.await;
	assert_eq!(relationship.body["role"], "VIEWER");
}

#[sqlx::test]
async fn members_can_leave_or_be_removed(pool: PgPool) {
	let app = TestApp::new(pool);
	let owner = app.register("owner@example.com").await;
	let parent = app.register("parent@example.com").await;
	let viewer = app.register("viewer@example.com").await;
	let (_, relationship_id) = app.child(&owner, "Mira").await;
	app.add_member(&owner, relationship_id, "parent@example.com", "PARENT")
		.await;
	app.add_member(&owner, relationship_id, "viewer@example.com", "VIEWER")
		.await;
	let ids: Vec<i64> = sqlx::query_scalar("SELECT id FROM users ORDER BY id")
		.fetch_all(&app.state.db)
		.await
		.unwrap();
	let (owner_id, parent_id, viewer_id) = (ids[0], ids[1], ids[2]);
	let member_path = |id: i64| format!("/api/relationships/{relationship_id}/members/{id}");

	assert_error(
		&app.delete(&member_path(parent_id), &viewer).await,
		StatusCode::FORBIDDEN,
		"forbidden",
	);
	assert_error(
		&app.delete(&member_path(owner_id), &parent).await,
		StatusCode::FORBIDDEN,
		"forbidden",
	);
	assert_error(
		&app.delete(&member_path(owner_id), &owner).await,
		StatusCode::CONFLICT,
		"conflict",
	);

	// A viewer may leave on their own, and then loses access.
	assert_eq!(
		app.delete(&member_path(viewer_id), &viewer).await.status,
		StatusCode::NO_CONTENT
	);
	assert_error(
		&app.get(&format!("/api/relationships/{relationship_id}"), &viewer)
			.await,
		StatusCode::NOT_FOUND,
		"not_found",
	);

	assert_eq!(
		app.delete(&member_path(parent_id), &owner).await.status,
		StatusCode::NO_CONTENT
	);
	assert_error(
		&app.delete(&member_path(parent_id), &owner).await,
		StatusCode::NOT_FOUND,
		"not_found",
	);
}
