CREATE TABLE development_domains (
	id SMALLINT PRIMARY KEY,
	name VARCHAR(50) NOT NULL UNIQUE
);

CREATE TABLE development_records (
	id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
	profile_id BIGINT NOT NULL REFERENCES profiles (id) ON DELETE CASCADE,
	record_date DATE NOT NULL,
	created_by BIGINT NOT NULL REFERENCES users (id),
	notes TEXT,
	created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX development_records_profile_id_record_date_idx ON development_records (profile_id, record_date);
CREATE INDEX development_records_created_by_idx ON development_records (created_by);

-- Development records only make sense for CHILD profiles. A CHECK can't look at
-- another table, so enforce it from both sides with triggers.
CREATE FUNCTION development_records_require_child() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
	IF NOT EXISTS (
		SELECT 1
		FROM profiles p
		JOIN profile_types pt ON pt.id = p.profile_type_id
		WHERE p.id = NEW.profile_id AND pt.name = 'CHILD'
	) THEN
		RAISE EXCEPTION 'development records require a CHILD profile (profile %)', NEW.profile_id
			USING ERRCODE = 'check_violation';
	END IF;
	RETURN NEW;
END;
$$;

CREATE TRIGGER development_records_require_child BEFORE INSERT OR UPDATE OF profile_id ON development_records
	FOR EACH ROW EXECUTE FUNCTION development_records_require_child();

CREATE FUNCTION profiles_keep_child_with_development() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
	IF EXISTS (SELECT 1 FROM development_records WHERE profile_id = NEW.id)
		AND NOT EXISTS (SELECT 1 FROM profile_types WHERE id = NEW.profile_type_id AND name = 'CHILD')
	THEN
		RAISE EXCEPTION 'profile % has development records and must stay a CHILD profile', NEW.id
			USING ERRCODE = 'check_violation';
	END IF;
	RETURN NEW;
END;
$$;

CREATE TRIGGER profiles_keep_child_with_development BEFORE UPDATE OF profile_type_id ON profiles
	FOR EACH ROW EXECUTE FUNCTION profiles_keep_child_with_development();

CREATE TABLE development_observations (
	id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
	development_record_id BIGINT NOT NULL REFERENCES development_records (id) ON DELETE CASCADE,
	domain_id SMALLINT NOT NULL REFERENCES development_domains (id),
	observation TEXT NOT NULL CHECK (btrim(observation) <> ''),
	created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX development_observations_development_record_id_idx ON development_observations (development_record_id);
CREATE INDEX development_observations_domain_id_idx ON development_observations (domain_id);

CREATE TABLE measurement_types (
	id SMALLINT PRIMARY KEY,
	name VARCHAR(50) NOT NULL UNIQUE,
	unit VARCHAR(20) NOT NULL CHECK (btrim(unit) <> '')
);

CREATE TABLE measurements (
	id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
	profile_id BIGINT NOT NULL REFERENCES profiles (id) ON DELETE CASCADE,
	measurement_type_id SMALLINT NOT NULL REFERENCES measurement_types (id),
	value NUMERIC(10, 3) NOT NULL CHECK (value > 0),
	measurement_date DATE NOT NULL,
	created_by BIGINT NOT NULL REFERENCES users (id),
	created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Charts: one profile's series of a given measurement over time.
CREATE INDEX measurements_profile_id_type_date_idx ON measurements (profile_id, measurement_type_id, measurement_date);
CREATE INDEX measurements_measurement_type_id_idx ON measurements (measurement_type_id);
CREATE INDEX measurements_created_by_idx ON measurements (created_by);
