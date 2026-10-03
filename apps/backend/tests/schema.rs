//! Schema constraint tests. Each `#[sqlx::test]` runs against a fresh database
//! with all migrations applied (requires `DATABASE_URL`, e.g. via `nix develop`).

use sqlx::{AssertSqlSafe, PgPool, SqlSafeStr, error::ErrorKind};

const CHILD: i16 = 1;
const PET: i16 = 2;
const PARENT_CHILD: i16 = 1;
const OWNER: i16 = 1;
const VIEWER: i16 = 4;
const GENERAL: i16 = 1;
const IMAGE: i16 = 1;
const HEIGHT: i16 = 1;
const LOCKED: i16 = 1;

async fn user(pool: &PgPool, email: &str) -> i64 {
	sqlx::query_scalar(
		"INSERT INTO users (email, password_hash, first_name, last_name)
		VALUES ($1, 'hash', 'Ada', 'Lovelace') RETURNING id",
	)
	.bind(email)
	.fetch_one(pool)
	.await
	.unwrap()
}

async fn profile(pool: &PgPool, profile_type_id: i16) -> i64 {
	sqlx::query_scalar(
		"INSERT INTO profiles (profile_type_id, name) VALUES ($1, 'Mira') RETURNING id",
	)
	.bind(profile_type_id)
	.fetch_one(pool)
	.await
	.unwrap()
}

async fn relationship(pool: &PgPool, profile_id: i64) -> i64 {
	sqlx::query_scalar(
		"INSERT INTO relationships (profile_id, relationship_type_id) VALUES ($1, $2) RETURNING id",
	)
	.bind(profile_id)
	.bind(PARENT_CHILD)
	.fetch_one(pool)
	.await
	.unwrap()
}

async fn memory(pool: &PgPool, relationship_id: i64, created_by: i64) -> i64 {
	sqlx::query_scalar(
		"INSERT INTO memories (relationship_id, category_id, title, memory_date, created_by)
		VALUES ($1, $2, 'First steps', '2026-05-01', $3) RETURNING id",
	)
	.bind(relationship_id)
	.bind(GENERAL)
	.bind(created_by)
	.fetch_one(pool)
	.await
	.unwrap()
}

async fn count(pool: &PgPool, sql: impl SqlSafeStr) -> i64 {
	sqlx::query_scalar(sql).fetch_one(pool).await.unwrap()
}

/// Asserts that `result` failed with the given kind of constraint violation.
#[track_caller]
fn assert_violation<T: std::fmt::Debug>(result: Result<T, sqlx::Error>, kind: ErrorKind) {
	match result {
		Err(sqlx::Error::Database(err)) => assert_eq!(err.kind(), kind, "{err}"),
		other => panic!("expected {kind:?} violation, got {other:?}"),
	}
}

#[sqlx::test]
async fn reference_data_is_seeded(pool: PgPool) {
	for (table, expected) in [
		("profile_types", 4),
		("relationship_types", 6),
		("relationship_roles", 4),
		("memory_categories", 7),
		("media_types", 4),
		("development_domains", 5),
		("measurement_types", 3),
		("time_capsule_statuses", 4),
	] {
		assert_eq!(
			// Table names come from the fixed list above.
			count(
				&pool,
				AssertSqlSafe(format!("SELECT count(*) FROM {table}"))
			)
			.await,
			expected,
			"{table}"
		);
	}
}

#[sqlx::test]
async fn emails_are_unique_ignoring_case(pool: PgPool) {
	user(&pool, "ada@example.com").await;
	let result = sqlx::query(
		"INSERT INTO users (email, password_hash, first_name, last_name)
		VALUES ('ADA@example.com', 'hash', 'Ada', 'Lovelace')",
	)
	.execute(&pool)
	.await;
	assert_violation(result, ErrorKind::UniqueViolation);
}

#[sqlx::test]
async fn users_reject_invalid_values(pool: PgPool) {
	for (email, first_name) in [("not-an-email", "Ada"), ("ada@example.com", "   ")] {
		let result = sqlx::query(
			"INSERT INTO users (email, password_hash, first_name, last_name)
			VALUES ($1, 'hash', $2, 'Lovelace')",
		)
		.bind(email)
		.bind(first_name)
		.execute(&pool)
		.await;
		assert_violation(result, ErrorKind::CheckViolation);
	}
}

#[sqlx::test]
async fn updated_at_changes_on_update(pool: PgPool) {
	let id = user(&pool, "ada@example.com").await;
	// now() is fixed per transaction, so backdate instead of waiting.
	sqlx::query("UPDATE users SET created_at = created_at - interval '1 day', updated_at = updated_at - interval '1 day' WHERE id = $1")
		.bind(id)
		.execute(&pool)
		.await
		.unwrap();
	sqlx::query("UPDATE users SET first_name = 'Augusta' WHERE id = $1")
		.bind(id)
		.execute(&pool)
		.await
		.unwrap();
	let refreshed: bool =
		sqlx::query_scalar("SELECT updated_at > created_at FROM users WHERE id = $1")
			.bind(id)
			.fetch_one(&pool)
			.await
			.unwrap();
	assert!(refreshed);
}

#[sqlx::test]
async fn relationships_cannot_end_before_they_start(pool: PgPool) {
	let profile_id = profile(&pool, CHILD).await;
	let result = sqlx::query(
		"INSERT INTO relationships (profile_id, relationship_type_id, started_at, ended_at)
		VALUES ($1, $2, '2026-01-02', '2026-01-01')",
	)
	.bind(profile_id)
	.bind(PARENT_CHILD)
	.execute(&pool)
	.await;
	assert_violation(result, ErrorKind::CheckViolation);
}

#[sqlx::test]
async fn a_user_has_one_role_per_relationship(pool: PgPool) {
	let user_id = user(&pool, "ada@example.com").await;
	let relationship_id = relationship(&pool, profile(&pool, CHILD).await).await;
	let add_member = |role_id: i16| {
		sqlx::query(
			"INSERT INTO relationship_members (relationship_id, user_id, role_id) VALUES ($1, $2, $3)",
		)
		.bind(relationship_id)
		.bind(user_id)
		.bind(role_id)
		.execute(&pool)
	};
	add_member(OWNER).await.unwrap();
	assert_violation(add_member(VIEWER).await, ErrorKind::UniqueViolation);
}

#[sqlx::test]
async fn memories_require_an_existing_category(pool: PgPool) {
	let user_id = user(&pool, "ada@example.com").await;
	let relationship_id = relationship(&pool, profile(&pool, CHILD).await).await;
	let result = sqlx::query(
		"INSERT INTO memories (relationship_id, category_id, title, memory_date, created_by)
		VALUES ($1, 999, 'First steps', '2026-05-01', $2)",
	)
	.bind(relationship_id)
	.bind(user_id)
	.execute(&pool)
	.await;
	assert_violation(result, ErrorKind::ForeignKeyViolation);
}

#[sqlx::test]
async fn tags_are_unique_and_attach_once(pool: PgPool) {
	let user_id = user(&pool, "ada@example.com").await;
	let relationship_id = relationship(&pool, profile(&pool, CHILD).await).await;
	let memory_id = memory(&pool, relationship_id, user_id).await;

	let tag_id: i64 = sqlx::query_scalar("INSERT INTO tags (name) VALUES ('Beach') RETURNING id")
		.fetch_one(&pool)
		.await
		.unwrap();
	assert_violation(
		sqlx::query("INSERT INTO tags (name) VALUES ('beach')")
			.execute(&pool)
			.await,
		ErrorKind::UniqueViolation,
	);

	let tag = || {
		sqlx::query("INSERT INTO memory_tags (memory_id, tag_id) VALUES ($1, $2)")
			.bind(memory_id)
			.bind(tag_id)
			.execute(&pool)
	};
	tag().await.unwrap();
	assert_violation(tag().await, ErrorKind::UniqueViolation);
}

#[sqlx::test]
async fn deleting_a_relationship_removes_its_data_but_keeps_media(pool: PgPool) {
	let user_id = user(&pool, "ada@example.com").await;
	let relationship_id = relationship(&pool, profile(&pool, CHILD).await).await;
	let memory_id = memory(&pool, relationship_id, user_id).await;
	sqlx::query(
		"INSERT INTO relationship_members (relationship_id, user_id, role_id) VALUES ($1, $2, $3)",
	)
	.bind(relationship_id)
	.bind(user_id)
	.bind(OWNER)
	.execute(&pool)
	.await
	.unwrap();
	let media_id: i64 = sqlx::query_scalar(
		"INSERT INTO media (media_type_id, storage_path, file_name, mime_type, uploaded_by)
		VALUES ($1, 'media/1.jpg', '1.jpg', 'image/jpeg', $2) RETURNING id",
	)
	.bind(IMAGE)
	.bind(user_id)
	.fetch_one(&pool)
	.await
	.unwrap();
	sqlx::query("INSERT INTO memory_media (memory_id, media_id) VALUES ($1, $2)")
		.bind(memory_id)
		.bind(media_id)
		.execute(&pool)
		.await
		.unwrap();

	sqlx::query("DELETE FROM relationships WHERE id = $1")
		.bind(relationship_id)
		.execute(&pool)
		.await
		.unwrap();

	assert_eq!(count(&pool, "SELECT count(*) FROM memories").await, 0);
	assert_eq!(count(&pool, "SELECT count(*) FROM memory_media").await, 0);
	assert_eq!(
		count(&pool, "SELECT count(*) FROM relationship_members").await,
		0
	);
	assert_eq!(count(&pool, "SELECT count(*) FROM media").await, 1);
}

#[sqlx::test]
async fn development_records_require_a_child_profile(pool: PgPool) {
	let user_id = user(&pool, "ada@example.com").await;
	let add_record = |profile_id: i64| {
		sqlx::query(
			"INSERT INTO development_records (profile_id, record_date, created_by)
			VALUES ($1, '2026-05-01', $2)",
		)
		.bind(profile_id)
		.bind(user_id)
		.execute(&pool)
	};

	let pet_id = profile(&pool, PET).await;
	assert_violation(add_record(pet_id).await, ErrorKind::CheckViolation);

	let child_id = profile(&pool, CHILD).await;
	add_record(child_id).await.unwrap();
	let result = sqlx::query("UPDATE profiles SET profile_type_id = $1 WHERE id = $2")
		.bind(PET)
		.bind(child_id)
		.execute(&pool)
		.await;
	assert_violation(result, ErrorKind::CheckViolation);
}

#[sqlx::test]
async fn measurements_must_be_positive(pool: PgPool) {
	let user_id = user(&pool, "ada@example.com").await;
	let profile_id = profile(&pool, CHILD).await;
	let result = sqlx::query(
		"INSERT INTO measurements (profile_id, measurement_type_id, value, measurement_date, created_by)
		VALUES ($1, $2, 0, '2026-05-01', $3)",
	)
	.bind(profile_id)
	.bind(HEIGHT)
	.bind(user_id)
	.execute(&pool)
	.await;
	assert_violation(result, ErrorKind::CheckViolation);
}

#[sqlx::test]
async fn time_capsules_must_unlock_in_the_future(pool: PgPool) {
	let user_id = user(&pool, "ada@example.com").await;
	let relationship_id = relationship(&pool, profile(&pool, CHILD).await).await;
	let add_capsule = |unlock_at: &'static str| {
		sqlx::query(
			"INSERT INTO time_capsules (relationship_id, created_by, title, unlock_at, status_id)
			VALUES ($1, $2, 'For your 18th', now() + $3::interval, $4)",
		)
		.bind(relationship_id)
		.bind(user_id)
		.bind(unlock_at)
		.bind(LOCKED)
		.execute(&pool)
	};
	assert_violation(add_capsule("-1 day").await, ErrorKind::CheckViolation);
	add_capsule("18 years").await.unwrap();
}
