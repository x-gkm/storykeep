CREATE TABLE memory_categories (
	id SMALLINT PRIMARY KEY,
	name VARCHAR(50) NOT NULL UNIQUE
);

CREATE TABLE memories (
	id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
	relationship_id BIGINT NOT NULL REFERENCES relationships (id) ON DELETE CASCADE,
	category_id SMALLINT NOT NULL REFERENCES memory_categories (id),
	title VARCHAR(200) NOT NULL CHECK (btrim(title) <> ''),
	description TEXT,
	memory_date DATE NOT NULL,
	created_by BIGINT NOT NULL REFERENCES users (id),
	created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
	updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Timeline: a relationship's memories ordered by event date.
CREATE INDEX memories_relationship_id_memory_date_idx ON memories (relationship_id, memory_date);
CREATE INDEX memories_category_id_idx ON memories (category_id);
CREATE INDEX memories_created_by_idx ON memories (created_by);

CREATE TRIGGER memories_set_updated_at BEFORE UPDATE ON memories
	FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TABLE media_types (
	id SMALLINT PRIMARY KEY,
	name VARCHAR(50) NOT NULL UNIQUE
);

CREATE TABLE media (
	id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
	media_type_id SMALLINT NOT NULL REFERENCES media_types (id),
	storage_path TEXT NOT NULL UNIQUE CHECK (btrim(storage_path) <> ''),
	file_name VARCHAR(255) NOT NULL CHECK (btrim(file_name) <> ''),
	mime_type VARCHAR(255) NOT NULL CHECK (mime_type LIKE '_%/_%'),
	file_size BIGINT CHECK (file_size >= 0),
	uploaded_by BIGINT NOT NULL REFERENCES users (id),
	created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX media_media_type_id_idx ON media (media_type_id);
CREATE INDEX media_uploaded_by_idx ON media (uploaded_by);

CREATE TABLE memory_media (
	memory_id BIGINT NOT NULL REFERENCES memories (id) ON DELETE CASCADE,
	media_id BIGINT NOT NULL REFERENCES media (id) ON DELETE CASCADE,
	PRIMARY KEY (memory_id, media_id)
);

CREATE INDEX memory_media_media_id_idx ON memory_media (media_id);

CREATE TABLE tags (
	id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
	name VARCHAR(50) NOT NULL CHECK (btrim(name) <> '')
);

-- Tag names are unique regardless of case.
CREATE UNIQUE INDEX tags_name_key ON tags (lower(name));

CREATE TABLE memory_tags (
	memory_id BIGINT NOT NULL REFERENCES memories (id) ON DELETE CASCADE,
	tag_id BIGINT NOT NULL REFERENCES tags (id) ON DELETE CASCADE,
	PRIMARY KEY (memory_id, tag_id)
);

CREATE INDEX memory_tags_tag_id_idx ON memory_tags (tag_id);
