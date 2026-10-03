-- Keeps updated_at current on every UPDATE; attached to each table that has the column.
CREATE FUNCTION set_updated_at() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
	NEW.updated_at = now();
	RETURN NEW;
END;
$$;

CREATE TABLE users (
	id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
	email VARCHAR(254) NOT NULL CHECK (email LIKE '_%@_%'),
	password_hash VARCHAR(255) NOT NULL,
	first_name VARCHAR(100) NOT NULL CHECK (btrim(first_name) <> ''),
	last_name VARCHAR(100) NOT NULL CHECK (btrim(last_name) <> ''),
	date_of_birth DATE CHECK (date_of_birth <= CURRENT_DATE),
	created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
	updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Emails are unique regardless of case.
CREATE UNIQUE INDEX users_email_key ON users (lower(email));

CREATE TRIGGER users_set_updated_at BEFORE UPDATE ON users
	FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TABLE profile_types (
	id SMALLINT PRIMARY KEY,
	name VARCHAR(50) NOT NULL UNIQUE
);

CREATE TABLE profiles (
	id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
	profile_type_id SMALLINT NOT NULL REFERENCES profile_types (id),
	name VARCHAR(100) NOT NULL CHECK (btrim(name) <> ''),
	date_of_birth DATE,
	created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
	updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX profiles_profile_type_id_idx ON profiles (profile_type_id);

CREATE TRIGGER profiles_set_updated_at BEFORE UPDATE ON profiles
	FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TABLE relationship_types (
	id SMALLINT PRIMARY KEY,
	name VARCHAR(50) NOT NULL UNIQUE
);

CREATE TABLE relationships (
	id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
	profile_id BIGINT NOT NULL REFERENCES profiles (id) ON DELETE CASCADE,
	relationship_type_id SMALLINT NOT NULL REFERENCES relationship_types (id),
	started_at DATE,
	ended_at DATE,
	created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
	CHECK (ended_at >= started_at)
);

CREATE INDEX relationships_profile_id_idx ON relationships (profile_id);
CREATE INDEX relationships_relationship_type_id_idx ON relationships (relationship_type_id);

CREATE TABLE relationship_roles (
	id SMALLINT PRIMARY KEY,
	name VARCHAR(50) NOT NULL UNIQUE
);

CREATE TABLE relationship_members (
	id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
	relationship_id BIGINT NOT NULL REFERENCES relationships (id) ON DELETE CASCADE,
	user_id BIGINT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
	role_id SMALLINT NOT NULL REFERENCES relationship_roles (id),
	joined_at TIMESTAMPTZ NOT NULL DEFAULT now(),
	-- A user holds exactly one role per relationship.
	UNIQUE (relationship_id, user_id)
);

CREATE INDEX relationship_members_user_id_idx ON relationship_members (user_id);
CREATE INDEX relationship_members_role_id_idx ON relationship_members (role_id);
