//! Part 7: child development records, measurements and chart series.

mod common;

use common::{TestApp, assert_error};
use serde_json::{Value, json};
use sqlx::PgPool;
use warp::http::StatusCode;

fn record(date: &str, observations: Value) -> Value {
	json!({ "record_date": date, "notes": "Good week", "observations": observations })
}

impl TestApp {
	async fn create_record(&self, token: &str, profile_id: i64, body: Value) -> Value {
		let response = self
			.post(
				&format!("/api/profiles/{profile_id}/development-records"),
				token,
				body,
			)
			.await;
		assert_eq!(response.status, StatusCode::CREATED, "{}", response.body);
		response.body
	}

	async fn create_measurement(
		&self,
		token: &str,
		profile_id: i64,
		kind: &str,
		value: f64,
		date: &str,
	) -> Value {
		let response = self
			.post(
				&format!("/api/profiles/{profile_id}/measurements"),
				token,
				json!({ "measurement_type": kind, "value": value, "measurement_date": date }),
			)
			.await;
		assert_eq!(response.status, StatusCode::CREATED, "{}", response.body);
		response.body
	}

	async fn pet(&self, token: &str) -> i64 {
		let response = self
			.post(
				"/api/profiles",
				token,
				json!({ "profile_type": "PET", "name": "Biscuit", "relationship_type": "OWNER_PET" }),
			)
			.await;
		assert_eq!(response.status, StatusCode::CREATED, "{}", response.body);
		response.body["profile"]["id"].as_i64().unwrap()
	}
}

fn ids(body: &Value) -> Vec<i64> {
	body.as_array()
		.unwrap()
		.iter()
		.map(|item| item["id"].as_i64().unwrap())
		.collect()
}

#[sqlx::test]
async fn development_records_crud_with_observations(pool: PgPool) {
	let app = TestApp::new(pool);
	let token = app.register("ada@example.com").await;
	let (profile_id, _) = app.child(&token, "Mira").await;

	let created = app
		.create_record(
			&token,
			profile_id,
			json!({
				"record_date": "2025-03-01",
				"notes": "  First birthday week  ",
				"observations": [
					{ "domain": "MOTOR", "observation": "Took three steps unaided" },
					{ "domain": "LANGUAGE", "observation": " Says \"mama\" " },
				],
			}),
		)
		.await;
	let id = created["id"].as_i64().unwrap();
	assert_eq!(created["profile_id"], profile_id);
	assert_eq!(created["record_date"], "2025-03-01");
	assert_eq!(created["notes"], "First birthday week");
	assert_eq!(created["created_by"]["last_name"], "ada");
	assert_eq!(created["created_by"]["first_name"], "Test");
	assert!(created["created_by"]["id"].is_i64());
	assert!(created["created_at"].is_string());
	let observations = created["observations"].as_array().unwrap();
	assert_eq!(observations.len(), 2);
	assert_eq!(observations[0]["domain"], "MOTOR");
	assert_eq!(observations[1]["domain"], "LANGUAGE");
	assert_eq!(observations[1]["observation"], "Says \"mama\"");
	assert!(observations[0]["id"].is_i64());
	assert!(observations[0].get("development_record_id").is_none());

	let path = format!("/api/development-records/{id}");
	let fetched = app.get(&path, &token).await;
	assert_eq!(fetched.status, StatusCode::OK);
	assert_eq!(fetched.body, created);

	// PUT replaces the date, notes and the whole observation list.
	let updated = app
		.put(
			&path,
			&token,
			json!({
				"record_date": "2025-03-02",
				"observations": [{ "domain": "SOCIAL_EMOTIONAL", "observation": "Waves goodbye" }],
			}),
		)
		.await;
	assert_eq!(updated.status, StatusCode::OK, "{}", updated.body);
	assert_eq!(updated.body["record_date"], "2025-03-02");
	assert_eq!(updated.body["notes"], Value::Null);
	assert_eq!(updated.body["observations"].as_array().unwrap().len(), 1);
	assert_eq!(
		updated.body["observations"][0]["domain"],
		"SOCIAL_EMOTIONAL"
	);
	let remaining: i64 = sqlx::query_scalar(
		"SELECT count(*) FROM development_observations WHERE development_record_id = $1",
	)
	.bind(id)
	.fetch_one(&app.state.db)
	.await
	.unwrap();
	assert_eq!(remaining, 1);

	let list = app
		.get(
			&format!("/api/profiles/{profile_id}/development-records"),
			&token,
		)
		.await;
	assert_eq!(list.status, StatusCode::OK);
	assert_eq!(list.body, json!([updated.body]));

	assert_eq!(
		app.delete(&path, &token).await.status,
		StatusCode::NO_CONTENT
	);
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
async fn development_records_require_a_child_profile(pool: PgPool) {
	let app = TestApp::new(pool);
	let token = app.register("ada@example.com").await;
	let pet_id = app.pet(&token).await;
	let path = format!("/api/profiles/{pet_id}/development-records");

	let response = app
		.post(
			&path,
			&token,
			record(
				"2025-03-01",
				json!([{ "domain": "MOTOR", "observation": "Fetches" }]),
			),
		)
		.await;
	assert_error(&response, StatusCode::BAD_REQUEST, "bad_request");
	assert!(
		response.body["error"]["message"]
			.as_str()
			.unwrap()
			.contains("CHILD"),
		"{}",
		response.body
	);
	assert_error(
		&app.get(&path, &token).await,
		StatusCode::BAD_REQUEST,
		"bad_request",
	);

	// Measurements work for any profile type.
	let weight = app
		.create_measurement(&token, pet_id, "WEIGHT", 12.4, "2025-03-01")
		.await;
	assert_eq!(weight["unit"], "kg");
}

#[sqlx::test]
async fn development_record_input_is_validated(pool: PgPool) {
	let app = TestApp::new(pool);
	let token = app.register("ada@example.com").await;
	let (profile_id, _) = app.child(&token, "Mira").await; // born 2024-03-01
	let path = format!("/api/profiles/{profile_id}/development-records");
	let motor = json!([{ "domain": "MOTOR", "observation": "Crawls" }]);

	for body in [
		record(
			"2025-03-01",
			json!([{ "domain": "MOTOR", "observation": "   " }]),
		),
		record(
			"2025-03-01",
			json!([{ "domain": "FLYING", "observation": "Soars" }]),
		),
		record("2025-03-01", json!([{ "observation": "No domain" }])),
		record("2999-01-01", motor.clone()),
		record("2024-02-29", motor.clone()),
		record("not-a-date", motor.clone()),
		json!({ "notes": "No date", "observations": motor }),
		json!({ "record_date": "2025-03-01", "notes": " ", "observations": [] }),
		json!({ "record_date": "2025-03-01", "notes": "x".repeat(10_001) }),
	] {
		let response = app.post(&path, &token, body.clone()).await;
		assert_error(&response, StatusCode::BAD_REQUEST, "bad_request");
	}

	// Nothing was stored, and the birth date itself is allowed.
	assert_eq!(app.get(&path, &token).await.body, json!([]));
	app.create_record(&token, profile_id, record("2024-03-01", motor.clone()))
		.await;
	// Notes alone are enough.
	app.create_record(
		&token,
		profile_id,
		json!({ "record_date": "2025-01-01", "notes": "Quiet month" }),
	)
	.await;

	// The same rules apply on update.
	let id = app.get(&path, &token).await.body[0]["id"].as_i64().unwrap();
	let response = app
		.put(
			&format!("/api/development-records/{id}"),
			&token,
			record("2999-01-01", motor),
		)
		.await;
	assert_error(&response, StatusCode::BAD_REQUEST, "bad_request");
}

#[sqlx::test]
async fn development_records_are_listed_newest_first_and_filterable(pool: PgPool) {
	let app = TestApp::new(pool);
	let token = app.register("ada@example.com").await;
	let (profile_id, _) = app.child(&token, "Mira").await;
	let path = format!("/api/profiles/{profile_id}/development-records");

	let mut created = Vec::new();
	for (date, domain) in [
		("2025-01-10", "MOTOR"),
		("2025-06-01", "LANGUAGE"),
		("2025-03-15", "MOTOR"),
	] {
		let body = app
			.create_record(
				&token,
				profile_id,
				record(
					date,
					json!([
						{ "domain": domain, "observation": "Something new" },
						{ "domain": "COGNITIVE", "observation": "Stacks blocks" },
					]),
				),
			)
			.await;
		created.push(body["id"].as_i64().unwrap());
	}
	let (jan, jun, mar) = (created[0], created[1], created[2]);

	let all = app.get(&path, &token).await;
	assert_eq!(ids(&all.body), vec![jun, mar, jan]);
	// Every record carries all of its observations.
	for item in all.body.as_array().unwrap() {
		assert_eq!(item["observations"].as_array().unwrap().len(), 2);
	}

	let filtered = |query: &'static str| {
		let path = format!("{path}?{query}");
		let app = &app;
		let token = &token;
		async move { ids(&app.get(&path, token).await.body) }
	};
	assert_eq!(filtered("from=2025-03-15").await, vec![jun, mar]);
	assert_eq!(filtered("to=2025-03-15").await, vec![mar, jan]);
	assert_eq!(filtered("from=2025-02-01&to=2025-05-01").await, vec![mar]);
	assert_eq!(filtered("domain=MOTOR").await, vec![mar, jan]);
	assert_eq!(filtered("domain=COGNITIVE").await, vec![jun, mar, jan]);
	assert_eq!(filtered("domain=PHYSICAL").await, Vec::<i64>::new());
	assert_eq!(filtered("domain=MOTOR&from=2025-02-01").await, vec![mar]);

	for query in [
		"domain=FLYING",
		"from=yesterday",
		"from=2025-05-01&to=2025-01-01",
	] {
		assert_error(
			&app.get(&format!("{path}?{query}"), &token).await,
			StatusCode::BAD_REQUEST,
			"bad_request",
		);
	}
}

#[sqlx::test]
async fn measurements_crud(pool: PgPool) {
	let app = TestApp::new(pool);
	let token = app.register("ada@example.com").await;
	let (profile_id, _) = app.child(&token, "Mira").await;

	let created = app
		.create_measurement(&token, profile_id, "HEIGHT", 74.25, "2025-03-01")
		.await;
	let id = created["id"].as_i64().unwrap();
	assert_eq!(created["profile_id"], profile_id);
	assert_eq!(created["measurement_type"], "HEIGHT");
	assert_eq!(created["unit"], "cm");
	assert_eq!(created["value"], json!(74.25));
	assert_eq!(created["measurement_date"], "2025-03-01");
	assert_eq!(created["created_by"]["last_name"], "ada");
	assert!(created["created_at"].is_string());

	let path = format!("/api/measurements/{id}");
	assert_eq!(app.get(&path, &token).await.body, created);

	let updated = app
		.put(
			&path,
			&token,
			json!({ "measurement_type": "WEIGHT", "value": 9.5, "measurement_date": "2025-03-02" }),
		)
		.await;
	assert_eq!(updated.status, StatusCode::OK, "{}", updated.body);
	assert_eq!(updated.body["measurement_type"], "WEIGHT");
	assert_eq!(updated.body["unit"], "kg");
	assert_eq!(updated.body["value"], json!(9.5));
	assert_eq!(updated.body["measurement_date"], "2025-03-02");

	let list = app
		.get(&format!("/api/profiles/{profile_id}/measurements"), &token)
		.await;
	assert_eq!(list.status, StatusCode::OK);
	assert_eq!(list.body, json!([updated.body]));

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
async fn measurement_input_is_validated(pool: PgPool) {
	let app = TestApp::new(pool);
	let token = app.register("ada@example.com").await;
	let (profile_id, _) = app.child(&token, "Mira").await; // born 2024-03-01
	let path = format!("/api/profiles/{profile_id}/measurements");
	let body = |kind: &str, value: Value, date: &str| json!({ "measurement_type": kind, "value": value, "measurement_date": date });

	for invalid in [
		body("HEIGHT", json!(0), "2025-03-01"),
		body("HEIGHT", json!(-3.5), "2025-03-01"),
		body("HEIGHT", json!(1.2345), "2025-03-01"),
		body("HEIGHT", json!(10_000_000), "2025-03-01"),
		body("HEIGHT", json!("tall"), "2025-03-01"),
		body("SHOE_SIZE", json!(20), "2025-03-01"),
		body("HEIGHT", json!(80), "2999-01-01"),
		body("HEIGHT", json!(50), "2024-02-29"),
		json!({ "measurement_type": "HEIGHT", "measurement_date": "2025-03-01" }),
	] {
		let response = app.post(&path, &token, invalid.clone()).await;
		assert_error(&response, StatusCode::BAD_REQUEST, "bad_request");
	}
	assert_eq!(app.get(&path, &token).await.body, json!([]));

	// Boundaries that fit NUMERIC(10, 3).
	let small = app
		.create_measurement(&token, profile_id, "WEIGHT", 0.001, "2024-03-01")
		.await;
	assert_eq!(small["value"], json!(0.001));
	let large = app
		.create_measurement(&token, profile_id, "WEIGHT", 9_999_999.999, "2025-03-01")
		.await;
	assert_eq!(large["value"], json!(9_999_999.999));

	let id = small["id"].as_i64().unwrap();
	let response = app
		.put(
			&format!("/api/measurements/{id}"),
			&token,
			body("WEIGHT", json!(-1), "2025-03-01"),
		)
		.await;
	assert_error(&response, StatusCode::BAD_REQUEST, "bad_request");
}

#[sqlx::test]
async fn measurements_are_listed_by_date_and_filterable(pool: PgPool) {
	let app = TestApp::new(pool);
	let token = app.register("ada@example.com").await;
	let (profile_id, _) = app.child(&token, "Mira").await;
	let path = format!("/api/profiles/{profile_id}/measurements");

	let h2 = app
		.create_measurement(&token, profile_id, "HEIGHT", 80.0, "2025-06-01")
		.await["id"]
		.as_i64()
		.unwrap();
	let w1 = app
		.create_measurement(&token, profile_id, "WEIGHT", 9.1, "2025-01-01")
		.await["id"]
		.as_i64()
		.unwrap();
	let h1 = app
		.create_measurement(&token, profile_id, "HEIGHT", 72.0, "2025-01-15")
		.await["id"]
		.as_i64()
		.unwrap();

	assert_eq!(ids(&app.get(&path, &token).await.body), vec![w1, h1, h2]);
	assert_eq!(
		ids(&app.get(&format!("{path}?type=HEIGHT"), &token).await.body),
		vec![h1, h2]
	);
	assert_eq!(
		ids(&app
			.get(&format!("{path}?from=2025-01-10&to=2025-05-01"), &token)
			.await
			.body),
		vec![h1]
	);
	assert_eq!(
		ids(&app
			.get(&format!("{path}?type=WEIGHT&from=2025-02-01"), &token)
			.await
			.body),
		Vec::<i64>::new()
	);
	for query in ["type=SHOE_SIZE", "from=2025-05-01&to=2025-01-01"] {
		assert_error(
			&app.get(&format!("{path}?{query}"), &token).await,
			StatusCode::BAD_REQUEST,
			"bad_request",
		);
	}
}

#[sqlx::test]
async fn measurement_series_for_charts(pool: PgPool) {
	let app = TestApp::new(pool);
	let token = app.register("ada@example.com").await;
	let (profile_id, _) = app.child(&token, "Mira").await;
	let path = format!("/api/profiles/{profile_id}/measurements/series");

	// Nothing recorded yet.
	assert_eq!(app.get(&path, &token).await.body, json!([]));
	assert_eq!(
		app.get(&format!("{path}?type=HEIGHT"), &token).await.body,
		json!({ "measurement_type": "HEIGHT", "unit": "cm", "points": [] })
	);

	for (kind, value, date) in [
		("HEIGHT", 80.5, "2025-06-01"),
		("WEIGHT", 9.1, "2025-01-01"),
		("HEIGHT", 72.0, "2025-01-15"),
		("HEIGHT", 76.125, "2025-03-01"),
	] {
		app.create_measurement(&token, profile_id, kind, value, date)
			.await;
	}

	let height = app.get(&format!("{path}?type=HEIGHT"), &token).await;
	assert_eq!(height.status, StatusCode::OK, "{}", height.body);
	assert_eq!(
		height.body,
		json!({
			"measurement_type": "HEIGHT",
			"unit": "cm",
			"points": [
				{ "date": "2025-01-15", "value": 72.0 },
				{ "date": "2025-03-01", "value": 76.125 },
				{ "date": "2025-06-01", "value": 80.5 },
			],
		})
	);

	let all = app.get(&path, &token).await;
	assert_eq!(
		all.body,
		json!([
			{
				"measurement_type": "HEIGHT",
				"unit": "cm",
				"points": [
					{ "date": "2025-01-15", "value": 72.0 },
					{ "date": "2025-03-01", "value": 76.125 },
					{ "date": "2025-06-01", "value": 80.5 },
				],
			},
			{
				"measurement_type": "WEIGHT",
				"unit": "kg",
				"points": [{ "date": "2025-01-01", "value": 9.1 }],
			},
		])
	);

	let ranged = app
		.get(&format!("{path}?type=HEIGHT&from=2025-02-01"), &token)
		.await;
	assert_eq!(ranged.body["points"].as_array().unwrap().len(), 2);
	assert_error(
		&app.get(&format!("{path}?type=SHOE_SIZE"), &token).await,
		StatusCode::BAD_REQUEST,
		"bad_request",
	);
}

#[sqlx::test]
async fn strangers_get_not_found_everywhere(pool: PgPool) {
	let app = TestApp::new(pool);
	let owner = app.register("ada@example.com").await;
	let stranger = app.register("eve@example.com").await;
	let (profile_id, _) = app.child(&owner, "Mira").await;
	let record_id = app
		.create_record(
			&owner,
			profile_id,
			record(
				"2025-03-01",
				json!([{ "domain": "MOTOR", "observation": "Walks" }]),
			),
		)
		.await["id"]
		.as_i64()
		.unwrap();
	let measurement_id = app
		.create_measurement(&owner, profile_id, "HEIGHT", 75.0, "2025-03-01")
		.await["id"]
		.as_i64()
		.unwrap();

	let record_body = record(
		"2025-03-01",
		json!([{ "domain": "MOTOR", "observation": "Hacked" }]),
	);
	let measurement_body =
		json!({ "measurement_type": "HEIGHT", "value": 1, "measurement_date": "2025-03-01" });
	let requests = [
		(
			"GET",
			format!("/api/profiles/{profile_id}/development-records"),
			None,
		),
		(
			"POST",
			format!("/api/profiles/{profile_id}/development-records"),
			Some(record_body.clone()),
		),
		("GET", format!("/api/development-records/{record_id}"), None),
		(
			"PUT",
			format!("/api/development-records/{record_id}"),
			Some(record_body),
		),
		(
			"DELETE",
			format!("/api/development-records/{record_id}"),
			None,
		),
		(
			"GET",
			format!("/api/profiles/{profile_id}/measurements"),
			None,
		),
		(
			"GET",
			format!("/api/profiles/{profile_id}/measurements/series"),
			None,
		),
		(
			"POST",
			format!("/api/profiles/{profile_id}/measurements"),
			Some(measurement_body.clone()),
		),
		("GET", format!("/api/measurements/{measurement_id}"), None),
		(
			"PUT",
			format!("/api/measurements/{measurement_id}"),
			Some(measurement_body),
		),
		(
			"DELETE",
			format!("/api/measurements/{measurement_id}"),
			None,
		),
		("GET", "/api/development-records/999999".to_owned(), None),
		("GET", "/api/measurements/999999".to_owned(), None),
	];
	for (method, path, body) in requests {
		let response = app.call(method, &path, Some(&stranger), body).await;
		assert_error(&response, StatusCode::NOT_FOUND, "not_found");
	}

	// Without a token it's 401, and nothing was changed.
	let response = app
		.call(
			"GET",
			&format!("/api/development-records/{record_id}"),
			None,
			None,
		)
		.await;
	assert_error(&response, StatusCode::UNAUTHORIZED, "unauthorized");
	let fetched = app
		.get(&format!("/api/development-records/{record_id}"), &owner)
		.await;
	assert_eq!(fetched.body["observations"][0]["observation"], "Walks");
	let fetched = app
		.get(&format!("/api/measurements/{measurement_id}"), &owner)
		.await;
	assert_eq!(fetched.body["value"], json!(75.0));
}

#[sqlx::test]
async fn roles_control_who_may_edit_development_data(pool: PgPool) {
	let app = TestApp::new(pool);
	let owner = app.register("owner@example.com").await;
	let parent = app.register("parent@example.com").await;
	let member = app.register("member@example.com").await;
	let viewer = app.register("viewer@example.com").await;
	let (profile_id, relationship_id) = app.child(&owner, "Mira").await;
	for (email, role) in [
		("parent@example.com", "PARENT"),
		("member@example.com", "MEMBER"),
		("viewer@example.com", "VIEWER"),
	] {
		app.add_member(&owner, relationship_id, email, role).await;
	}

	let records = format!("/api/profiles/{profile_id}/development-records");
	let measurements = format!("/api/profiles/{profile_id}/measurements");
	let body = record(
		"2025-03-01",
		json!([{ "domain": "MOTOR", "observation": "Walks" }]),
	);
	let measurement =
		json!({ "measurement_type": "HEIGHT", "value": 75.0, "measurement_date": "2025-03-01" });

	let owners_record = app.create_record(&owner, profile_id, body.clone()).await["id"]
		.as_i64()
		.unwrap();
	let members_record = app.create_record(&member, profile_id, body.clone()).await["id"]
		.as_i64()
		.unwrap();
	let owners_measurement = app
		.create_measurement(&owner, profile_id, "HEIGHT", 75.0, "2025-03-01")
		.await["id"]
		.as_i64()
		.unwrap();
	let members_measurement = app
		.create_measurement(&member, profile_id, "WEIGHT", 9.0, "2025-03-01")
		.await["id"]
		.as_i64()
		.unwrap();

	// VIEWER: reads everything, writes nothing.
	assert_eq!(app.get(&records, &viewer).await.status, StatusCode::OK);
	assert_eq!(app.get(&measurements, &viewer).await.status, StatusCode::OK);
	assert_eq!(
		app.get(&format!("{measurements}/series"), &viewer)
			.await
			.status,
		StatusCode::OK
	);
	assert_eq!(
		app.get(
			&format!("/api/development-records/{members_record}"),
			&viewer
		)
		.await
		.status,
		StatusCode::OK
	);
	for response in [
		app.post(&records, &viewer, body.clone()).await,
		app.post(&measurements, &viewer, measurement.clone()).await,
		app.put(
			&format!("/api/development-records/{owners_record}"),
			&viewer,
			body.clone(),
		)
		.await,
		app.delete(
			&format!("/api/development-records/{owners_record}"),
			&viewer,
		)
		.await,
		app.put(
			&format!("/api/measurements/{owners_measurement}"),
			&viewer,
			measurement.clone(),
		)
		.await,
		app.delete(&format!("/api/measurements/{owners_measurement}"), &viewer)
			.await,
	] {
		assert_error(&response, StatusCode::FORBIDDEN, "forbidden");
	}

	// MEMBER: edits their own items, not other people's.
	let response = app
		.put(
			&format!("/api/development-records/{members_record}"),
			&member,
			record(
				"2025-03-02",
				json!([{ "domain": "LANGUAGE", "observation": "Says hi" }]),
			),
		)
		.await;
	assert_eq!(response.status, StatusCode::OK, "{}", response.body);
	let response = app
		.put(
			&format!("/api/measurements/{members_measurement}"),
			&member,
			json!({ "measurement_type": "WEIGHT", "value": 9.2, "measurement_date": "2025-03-01" }),
		)
		.await;
	assert_eq!(response.status, StatusCode::OK, "{}", response.body);
	for response in [
		app.put(
			&format!("/api/development-records/{owners_record}"),
			&member,
			body.clone(),
		)
		.await,
		app.delete(
			&format!("/api/development-records/{owners_record}"),
			&member,
		)
		.await,
		app.put(
			&format!("/api/measurements/{owners_measurement}"),
			&member,
			measurement.clone(),
		)
		.await,
		app.delete(&format!("/api/measurements/{owners_measurement}"), &member)
			.await,
	] {
		assert_error(&response, StatusCode::FORBIDDEN, "forbidden");
	}

	// PARENT: may edit and delete anyone's items.
	let response = app
		.put(
			&format!("/api/development-records/{owners_record}"),
			&parent,
			body.clone(),
		)
		.await;
	assert_eq!(response.status, StatusCode::OK, "{}", response.body);
	let response = app
		.put(
			&format!("/api/measurements/{members_measurement}"),
			&parent,
			measurement.clone(),
		)
		.await;
	assert_eq!(response.status, StatusCode::OK, "{}", response.body);
	assert_eq!(
		app.delete(
			&format!("/api/development-records/{members_record}"),
			&parent
		)
		.await
		.status,
		StatusCode::NO_CONTENT
	);
	assert_eq!(
		app.delete(&format!("/api/measurements/{owners_measurement}"), &parent)
			.await
			.status,
		StatusCode::NO_CONTENT
	);

	// A creator demoted to VIEWER can no longer edit their own items.
	let mine = app.create_record(&member, profile_id, body.clone()).await["id"]
		.as_i64()
		.unwrap();
	let member_id: i64 =
		sqlx::query_scalar("SELECT id FROM users WHERE email = 'member@example.com'")
			.fetch_one(&app.state.db)
			.await
			.unwrap();
	let response = app
		.put(
			&format!("/api/relationships/{relationship_id}/members/{member_id}"),
			&owner,
			json!({ "role": "VIEWER" }),
		)
		.await;
	assert_eq!(response.status, StatusCode::OK, "{}", response.body);
	assert_error(
		&app.delete(&format!("/api/development-records/{mine}"), &member)
			.await,
		StatusCode::FORBIDDEN,
		"forbidden",
	);
}

#[sqlx::test]
async fn access_through_a_second_relationship_counts(pool: PgPool) {
	let app = TestApp::new(pool);
	let owner = app.register("owner@example.com").await;
	let grandma = app.register("grandma@example.com").await;
	let (profile_id, parents) = app.child(&owner, "Mira").await;
	app.add_member(&owner, parents, "grandma@example.com", "VIEWER")
		.await;

	// As a VIEWER only, grandma can't write.
	let body = record(
		"2025-03-01",
		json!([{ "domain": "SOCIAL_EMOTIONAL", "observation": "Laughs at peekaboo" }]),
	);
	let records = format!("/api/profiles/{profile_id}/development-records");
	assert_error(
		&app.post(&records, &grandma, body.clone()).await,
		StatusCode::FORBIDDEN,
		"forbidden",
	);

	// A MEMBER role in another relationship with the same profile grants write access.
	let family = app
		.post(
			"/api/relationships",
			&owner,
			json!({ "profile_id": profile_id, "relationship_type": "FAMILY" }),
		)
		.await;
	assert_eq!(family.status, StatusCode::CREATED, "{}", family.body);
	let family_id = family.body["id"].as_i64().unwrap();
	app.add_member(&owner, family_id, "grandma@example.com", "MEMBER")
		.await;

	let created = app.create_record(&grandma, profile_id, body).await;
	assert_eq!(created["created_by"]["last_name"], "grandma");
	let measurement = app
		.create_measurement(
			&grandma,
			profile_id,
			"HEAD_CIRCUMFERENCE",
			45.5,
			"2025-03-01",
		)
		.await;
	assert_eq!(measurement["unit"], "cm");
	assert_eq!(
		app.get(&records, &owner)
			.await
			.body
			.as_array()
			.unwrap()
			.len(),
		1
	);
}
