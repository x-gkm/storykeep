//! Relationships between users and a profile, and their membership.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgExecutor};
use warp::{Filter, reply::Response};

use crate::{
	AppState, Routes,
	auth::{CurrentUser, authenticated},
	authz::{self, Access},
	created,
	error::{ApiError, ApiResult},
	json_body, no_content, ok,
	reference::{ProfileType, RelationshipType, Role},
	respond, validate, with_state,
};

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Relationship {
	pub id: i64,
	pub profile_id: i64,
	pub profile_name: String,
	pub profile_type: ProfileType,
	pub relationship_type: RelationshipType,
	pub started_at: Option<NaiveDate>,
	pub ended_at: Option<NaiveDate>,
	pub created_at: DateTime<Utc>,
	/// The caller's role in this relationship.
	pub role: Role,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Member {
	pub user_id: i64,
	pub email: String,
	pub first_name: String,
	pub last_name: String,
	pub role: Role,
	pub joined_at: DateTime<Utc>,
}

/// Relationships of user `$1`; `$tail` adds filters and ordering.
macro_rules! select_relationships {
	($tail:literal) => {
		concat!(
			"SELECT r.id, r.profile_id, p.name AS profile_name, p.profile_type_id AS profile_type,
				r.relationship_type_id AS relationship_type, r.started_at, r.ended_at, r.created_at,
				m.role_id AS role
			FROM relationships r
			JOIN profiles p ON p.id = r.profile_id
			JOIN relationship_members m ON m.relationship_id = r.id
			WHERE m.user_id = $1",
			$tail
		)
	};
}

pub async fn fetch<'e>(
	db: impl PgExecutor<'e>,
	user: CurrentUser,
	id: i64,
) -> ApiResult<Relationship> {
	sqlx::query_as(select_relationships!(" AND r.id = $2"))
		.bind(user.id)
		.bind(id)
		.fetch_optional(db)
		.await?
		.ok_or(ApiError::NotFound("relationship"))
}

/// Creates a relationship with `user` as its OWNER; returns its id.
pub async fn insert(
	conn: &mut PgConnection,
	user: CurrentUser,
	profile_id: i64,
	relationship_type: RelationshipType,
	started_at: Option<NaiveDate>,
	ended_at: Option<NaiveDate>,
) -> ApiResult<i64> {
	validate::date_order("started_at", started_at, "ended_at", ended_at)?;

	let id: i64 = sqlx::query_scalar(
		"INSERT INTO relationships (profile_id, relationship_type_id, started_at, ended_at)
		VALUES ($1, $2, $3, $4) RETURNING id",
	)
	.bind(profile_id)
	.bind(relationship_type)
	.bind(started_at)
	.bind(ended_at)
	.fetch_one(&mut *conn)
	.await?;
	sqlx::query(
		"INSERT INTO relationship_members (relationship_id, user_id, role_id) VALUES ($1, $2, $3)",
	)
	.bind(id)
	.bind(user.id)
	.bind(Role::Owner)
	.execute(&mut *conn)
	.await?;
	Ok(id)
}

pub fn routes(state: &AppState) -> Routes {
	let list = warp::path!("relationships")
		.and(warp::get())
		.and(with_state(state))
		.and(authenticated(state))
		.then(|state, user| async move { respond(list(state, user).await) });

	let create = warp::path!("relationships")
		.and(warp::post())
		.and(with_state(state))
		.and(authenticated(state))
		.and(json_body())
		.then(|state, user, body| async move { respond(create(state, user, body).await) });

	let get = warp::path!("relationships" / i64)
		.and(warp::get())
		.and(with_state(state))
		.and(authenticated(state))
		.then(|id, state: AppState, user| async move {
			respond(
				fetch(&state.db, user, id)
					.await
					.and_then(|relationship| ok(&relationship)),
			)
		});

	let update = warp::path!("relationships" / i64)
		.and(warp::put())
		.and(with_state(state))
		.and(authenticated(state))
		.and(json_body())
		.then(|id, state, user, body| async move { respond(update(state, user, id, body).await) });

	let delete = warp::path!("relationships" / i64)
		.and(warp::delete())
		.and(with_state(state))
		.and(authenticated(state))
		.then(|id, state, user| async move { respond(delete(state, user, id).await) });

	let list_members = warp::path!("relationships" / i64 / "members")
		.and(warp::get())
		.and(with_state(state))
		.and(authenticated(state))
		.then(|id, state, user| async move { respond(list_members(state, user, id).await) });

	let add_member = warp::path!("relationships" / i64 / "members")
		.and(warp::post())
		.and(with_state(state))
		.and(authenticated(state))
		.and(json_body())
		.then(
			|id, state, user, body| async move { respond(add_member(state, user, id, body).await) },
		);

	let update_member = warp::path!("relationships" / i64 / "members" / i64)
		.and(warp::put())
		.and(with_state(state))
		.and(authenticated(state))
		.and(json_body())
		.then(|id, member_id, state, user, body| async move {
			respond(update_member(state, user, id, member_id, body).await)
		});

	let remove_member = warp::path!("relationships" / i64 / "members" / i64)
		.and(warp::delete())
		.and(with_state(state))
		.and(authenticated(state))
		.then(|id, member_id, state, user| async move {
			respond(remove_member(state, user, id, member_id).await)
		});

	list.or(create)
		.unify()
		.or(get)
		.unify()
		.or(update)
		.unify()
		.or(delete)
		.unify()
		.or(list_members)
		.unify()
		.or(add_member)
		.unify()
		.or(update_member)
		.unify()
		.or(remove_member)
		.unify()
		.boxed()
}

async fn list(state: AppState, user: CurrentUser) -> ApiResult<Response> {
	let relationships: Vec<Relationship> =
		sqlx::query_as(select_relationships!(" ORDER BY p.name, r.id"))
			.bind(user.id)
			.fetch_all(&state.db)
			.await?;
	ok(&relationships)
}

#[derive(Deserialize)]
struct CreateRelationshipRequest {
	profile_id: i64,
	relationship_type: RelationshipType,
	started_at: Option<NaiveDate>,
	ended_at: Option<NaiveDate>,
}

/// Adds another relationship to an existing profile (e.g. a FAMILY circle next to
/// the PARENT_CHILD one). Requires managing the profile through some relationship.
async fn create(
	state: AppState,
	user: CurrentUser,
	body: CreateRelationshipRequest,
) -> ApiResult<Response> {
	authz::require_profile(&state.db, user, body.profile_id, Access::Manage).await?;

	let mut tx = state.db.begin().await?;
	let id = insert(
		&mut tx,
		user,
		body.profile_id,
		body.relationship_type,
		body.started_at,
		body.ended_at,
	)
	.await?;
	let relationship = fetch(&mut *tx, user, id).await?;
	tx.commit().await?;
	created(&relationship)
}

#[derive(Deserialize)]
struct UpdateRelationshipRequest {
	relationship_type: RelationshipType,
	started_at: Option<NaiveDate>,
	ended_at: Option<NaiveDate>,
}

async fn update(
	state: AppState,
	user: CurrentUser,
	id: i64,
	body: UpdateRelationshipRequest,
) -> ApiResult<Response> {
	authz::require_relationship(&state.db, user, id, Access::Manage).await?;
	validate::date_order("started_at", body.started_at, "ended_at", body.ended_at)?;

	sqlx::query("UPDATE relationships SET relationship_type_id = $1, started_at = $2, ended_at = $3 WHERE id = $4")
		.bind(body.relationship_type)
		.bind(body.started_at)
		.bind(body.ended_at)
		.bind(id)
		.execute(&state.db)
		.await?;
	ok(&fetch(&state.db, user, id).await?)
}

async fn delete(state: AppState, user: CurrentUser, id: i64) -> ApiResult<Response> {
	authz::require_relationship(&state.db, user, id, Access::Own).await?;
	sqlx::query("DELETE FROM relationships WHERE id = $1")
		.bind(id)
		.execute(&state.db)
		.await?;
	no_content()
}

async fn fetch_members<'e>(
	db: impl PgExecutor<'e>,
	relationship_id: i64,
) -> ApiResult<Vec<Member>> {
	Ok(sqlx::query_as(
		"SELECT u.id AS user_id, u.email, u.first_name, u.last_name, m.role_id AS role, m.joined_at
		FROM relationship_members m
		JOIN users u ON u.id = m.user_id
		WHERE m.relationship_id = $1
		ORDER BY m.role_id, u.last_name, u.first_name, u.id",
	)
	.bind(relationship_id)
	.fetch_all(db)
	.await?)
}

async fn list_members(state: AppState, user: CurrentUser, id: i64) -> ApiResult<Response> {
	authz::require_relationship(&state.db, user, id, Access::Read).await?;
	ok(&fetch_members(&state.db, id).await?)
}

#[derive(Deserialize)]
struct AddMemberRequest {
	email: String,
	role: Role,
}

async fn add_member(
	state: AppState,
	user: CurrentUser,
	id: i64,
	body: AddMemberRequest,
) -> ApiResult<Response> {
	let my_role = authz::require_relationship(&state.db, user, id, Access::Manage).await?;
	require_can_assign(my_role, body.role)?;

	let member_id: i64 = sqlx::query_scalar("SELECT id FROM users WHERE lower(email) = lower($1)")
		.bind(body.email.trim())
		.fetch_optional(&state.db)
		.await?
		.ok_or(ApiError::NotFound("user"))?;

	let inserted = sqlx::query(
		"INSERT INTO relationship_members (relationship_id, user_id, role_id)
		VALUES ($1, $2, $3) ON CONFLICT (relationship_id, user_id) DO NOTHING",
	)
	.bind(id)
	.bind(member_id)
	.bind(body.role)
	.execute(&state.db)
	.await?;
	if inserted.rows_affected() == 0 {
		return Err(ApiError::conflict("this user is already a member"));
	}
	created(&fetch_members(&state.db, id).await?)
}

#[derive(Deserialize)]
struct UpdateMemberRequest {
	role: Role,
}

async fn update_member(
	state: AppState,
	user: CurrentUser,
	id: i64,
	member_id: i64,
	body: UpdateMemberRequest,
) -> ApiResult<Response> {
	let my_role = authz::require_relationship(&state.db, user, id, Access::Manage).await?;

	let mut tx = state.db.begin().await?;
	let current = member_role(&mut tx, id, member_id).await?;
	require_can_assign(my_role, current)?;
	require_can_assign(my_role, body.role)?;
	if current == Role::Owner && body.role != Role::Owner {
		require_another_owner(&mut tx, id, member_id).await?;
	}

	sqlx::query(
		"UPDATE relationship_members SET role_id = $1 WHERE relationship_id = $2 AND user_id = $3",
	)
	.bind(body.role)
	.bind(id)
	.bind(member_id)
	.execute(&mut *tx)
	.await?;
	tx.commit().await?;
	ok(&fetch_members(&state.db, id).await?)
}

/// Removes a member. Anyone may leave; removing others requires managing the relationship.
async fn remove_member(
	state: AppState,
	user: CurrentUser,
	id: i64,
	member_id: i64,
) -> ApiResult<Response> {
	let my_role = authz::require_relationship(&state.db, user, id, Access::Read).await?;

	let mut tx = state.db.begin().await?;
	let current = member_role(&mut tx, id, member_id).await?;
	if member_id != user.id {
		if !my_role.allows(Access::Manage) {
			return Err(ApiError::Forbidden);
		}
		require_can_assign(my_role, current)?;
	}
	if current == Role::Owner {
		require_another_owner(&mut tx, id, member_id).await?;
	}

	sqlx::query("DELETE FROM relationship_members WHERE relationship_id = $1 AND user_id = $2")
		.bind(id)
		.bind(member_id)
		.execute(&mut *tx)
		.await?;
	tx.commit().await?;
	no_content()
}

/// Only owners may grant, change or revoke the OWNER role.
fn require_can_assign(my_role: Role, role: Role) -> ApiResult<()> {
	if role == Role::Owner && my_role != Role::Owner {
		return Err(ApiError::Forbidden);
	}
	Ok(())
}

/// Locks the relationship for the rest of the transaction and returns a member's role.
async fn member_role(
	conn: &mut PgConnection,
	relationship_id: i64,
	user_id: i64,
) -> ApiResult<Role> {
	// Serializes membership changes per relationship, so two owners demoting each
	// other at the same time can't both pass the last-owner check.
	sqlx::query("SELECT 1 FROM relationships WHERE id = $1 FOR UPDATE")
		.bind(relationship_id)
		.execute(&mut *conn)
		.await?;
	sqlx::query_scalar(
		"SELECT role_id FROM relationship_members WHERE relationship_id = $1 AND user_id = $2",
	)
	.bind(relationship_id)
	.bind(user_id)
	.fetch_optional(conn)
	.await?
	.ok_or(ApiError::NotFound("member"))
}

/// A relationship must always keep at least one OWNER.
async fn require_another_owner(
	conn: &mut PgConnection,
	relationship_id: i64,
	except_user_id: i64,
) -> ApiResult<()> {
	let others: i64 = sqlx::query_scalar(
		"SELECT count(*) FROM relationship_members WHERE relationship_id = $1 AND role_id = $2 AND user_id <> $3",
	)
	.bind(relationship_id)
	.bind(Role::Owner)
	.bind(except_user_id)
	.fetch_one(conn)
	.await?;
	if others == 0 {
		return Err(ApiError::conflict(
			"a relationship must keep at least one OWNER",
		));
	}
	Ok(())
}
