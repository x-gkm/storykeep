//! Server-side authorization, based on relationship membership.
//!
//! | Role   | Read | Write content | Manage relationship & members | Delete relationship |
//! |--------|------|---------------|-------------------------------|---------------------|
//! | OWNER  | yes  | yes           | yes                           | yes                 |
//! | PARENT | yes  | yes           | yes                           | no                  |
//! | MEMBER | yes  | yes           | no                            | no                  |
//! | VIEWER | yes  | no            | no                            | no                  |
//!
//! "Content" is memories, media, tags, development data and time capsules.
//! Non-members get 404 rather than 403, so IDs can't be probed for existence.

use sqlx::PgExecutor;

use crate::{
	auth::CurrentUser,
	error::{ApiError, ApiResult},
	reference::Role,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
	Read,
	Write,
	Manage,
	Own,
}

impl Role {
	pub fn allows(self, access: Access) -> bool {
		match access {
			Access::Read => true,
			Access::Write => self != Role::Viewer,
			Access::Manage => matches!(self, Role::Owner | Role::Parent),
			Access::Own => self == Role::Owner,
		}
	}
}

fn check(role: Option<Role>, access: Access, resource: &'static str) -> ApiResult<Role> {
	let role = role.ok_or(ApiError::NotFound(resource))?;
	if !role.allows(access) {
		return Err(ApiError::Forbidden);
	}
	Ok(role)
}

/// The user's role in a relationship, or `None` if they aren't a member (or it doesn't exist).
pub async fn relationship_role<'e>(
	db: impl PgExecutor<'e>,
	user: CurrentUser,
	relationship_id: i64,
) -> ApiResult<Option<Role>> {
	Ok(sqlx::query_scalar(
		"SELECT role_id FROM relationship_members WHERE relationship_id = $1 AND user_id = $2",
	)
	.bind(relationship_id)
	.bind(user.id)
	.fetch_optional(db)
	.await?)
}

/// Requires `access` to a relationship; returns the user's role.
pub async fn require_relationship<'e>(
	db: impl PgExecutor<'e>,
	user: CurrentUser,
	relationship_id: i64,
	access: Access,
) -> ApiResult<Role> {
	check(
		relationship_role(db, user, relationship_id).await?,
		access,
		"relationship",
	)
}

/// The user's strongest role across all relationships with a profile, or `None` if they have no access.
pub async fn profile_role<'e>(
	db: impl PgExecutor<'e>,
	user: CurrentUser,
	profile_id: i64,
) -> ApiResult<Option<Role>> {
	// Role ids are ordered from most to least privileged, so MIN picks the strongest.
	let role_id: Option<i16> = sqlx::query_scalar(
		"SELECT min(m.role_id)
		FROM relationship_members m
		JOIN relationships r ON r.id = m.relationship_id
		WHERE r.profile_id = $1 AND m.user_id = $2",
	)
	.bind(profile_id)
	.bind(user.id)
	.fetch_one(db)
	.await?;
	Ok(role_id.map(Role::from_id))
}

/// Requires `access` to a profile through any of the user's relationships with it.
pub async fn require_profile<'e>(
	db: impl PgExecutor<'e>,
	user: CurrentUser,
	profile_id: i64,
	access: Access,
) -> ApiResult<Role> {
	check(profile_role(db, user, profile_id).await?, access, "profile")
}

impl Role {
	fn from_id(id: i16) -> Self {
		match id {
			1 => Role::Owner,
			2 => Role::Parent,
			3 => Role::Member,
			4 => Role::Viewer,
			_ => unreachable!("relationship_roles only contains ids 1-4"),
		}
	}
}
