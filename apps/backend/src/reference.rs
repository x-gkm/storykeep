//! Lookup values. IDs match the rows seeded in `migrations/0005_reference_data.up.sql`;
//! the API exchanges them by name (e.g. `"CHILD"`), the database by SMALLINT id.

use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::error::ApiResult;

macro_rules! reference_enum {
	($(#[$meta:meta])* $name:ident { $($variant:ident = $id:literal),+ $(,)? }) => {
		$(#[$meta])*
		#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
		#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
		#[repr(i16)]
		pub enum $name {
			$($variant = $id),+
		}
	};
}

reference_enum!(ProfileType {
	Child = 1,
	Pet = 2,
	Person = 3,
	Other = 4,
});

reference_enum!(RelationshipType {
	ParentChild = 1,
	OwnerPet = 2,
	Friend = 3,
	Family = 4,
	Partner = 5,
	Other = 6,
});

reference_enum!(
	/// A member's role within a relationship; see [`crate::authz`] for what each may do.
	Role {
		Owner = 1,
		Parent = 2,
		Member = 3,
		Viewer = 4,
	}
);

reference_enum!(MemoryCategory {
	General = 1,
	Milestone = 2,
	Birthday = 3,
	Holiday = 4,
	Travel = 5,
	FirstTime = 6,
	Everyday = 7,
});

reference_enum!(MediaType {
	Image = 1,
	Video = 2,
	Audio = 3,
	Document = 4,
});

reference_enum!(DevelopmentDomain {
	Physical = 1,
	Motor = 2,
	Language = 3,
	Cognitive = 4,
	SocialEmotional = 5,
});

reference_enum!(MeasurementType {
	Height = 1,
	Weight = 2,
	HeadCircumference = 3,
});

reference_enum!(CapsuleStatus {
	Locked = 1,
	Available = 2,
	Opened = 3,
	Cancelled = 4,
});

#[derive(Serialize)]
pub struct ReferenceData {
	profile_types: Vec<String>,
	relationship_types: Vec<String>,
	relationship_roles: Vec<String>,
	memory_categories: Vec<String>,
	media_types: Vec<String>,
	development_domains: Vec<String>,
	measurement_types: Vec<MeasurementTypeInfo>,
	time_capsule_statuses: Vec<String>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct MeasurementTypeInfo {
	name: String,
	unit: String,
}

/// All lookup values, read from the database so clients see exactly what's seeded.
pub async fn load(db: &PgPool) -> ApiResult<ReferenceData> {
	async fn names(db: &PgPool, sql: &'static str) -> ApiResult<Vec<String>> {
		Ok(sqlx::query_scalar(sql).fetch_all(db).await?)
	}

	Ok(ReferenceData {
		profile_types: names(db, "SELECT name FROM profile_types ORDER BY id").await?,
		relationship_types: names(db, "SELECT name FROM relationship_types ORDER BY id").await?,
		relationship_roles: names(db, "SELECT name FROM relationship_roles ORDER BY id").await?,
		memory_categories: names(db, "SELECT name FROM memory_categories ORDER BY id").await?,
		media_types: names(db, "SELECT name FROM media_types ORDER BY id").await?,
		development_domains: names(db, "SELECT name FROM development_domains ORDER BY id").await?,
		measurement_types: sqlx::query_as("SELECT name, unit FROM measurement_types ORDER BY id")
			.fetch_all(db)
			.await?,
		time_capsule_statuses: names(db, "SELECT name FROM time_capsule_statuses ORDER BY id")
			.await?,
	})
}
