CREATE TABLE time_capsule_statuses (
	id SMALLINT PRIMARY KEY,
	name VARCHAR(50) NOT NULL UNIQUE
);

CREATE TABLE time_capsules (
	id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
	relationship_id BIGINT NOT NULL REFERENCES relationships (id) ON DELETE CASCADE,
	created_by BIGINT NOT NULL REFERENCES users (id),
	title VARCHAR(200) NOT NULL CHECK (btrim(title) <> ''),
	message TEXT,
	unlock_at TIMESTAMPTZ NOT NULL,
	status_id SMALLINT NOT NULL REFERENCES time_capsule_statuses (id),
	created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
	updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
	-- A capsule must unlock in the future relative to when it was sealed.
	CHECK (unlock_at > created_at)
);

CREATE INDEX time_capsules_relationship_id_unlock_at_idx ON time_capsules (relationship_id, unlock_at);
CREATE INDEX time_capsules_created_by_idx ON time_capsules (created_by);
CREATE INDEX time_capsules_status_id_idx ON time_capsules (status_id);

CREATE TRIGGER time_capsules_set_updated_at BEFORE UPDATE ON time_capsules
	FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TABLE capsule_media (
	capsule_id BIGINT NOT NULL REFERENCES time_capsules (id) ON DELETE CASCADE,
	media_id BIGINT NOT NULL REFERENCES media (id) ON DELETE CASCADE,
	PRIMARY KEY (capsule_id, media_id)
);

CREATE INDEX capsule_media_media_id_idx ON capsule_media (media_id);
