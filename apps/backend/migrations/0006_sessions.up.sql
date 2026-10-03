-- Login sessions. Clients hold a random bearer token; only its SHA-256 hash is
-- stored, so a database leak doesn't expose usable tokens.
CREATE TABLE sessions (
	id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
	user_id BIGINT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
	token_hash BYTEA NOT NULL UNIQUE CHECK (length(token_hash) = 32),
	created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
	expires_at TIMESTAMPTZ NOT NULL,
	CHECK (expires_at > created_at)
);

CREATE INDEX sessions_user_id_idx ON sessions (user_id);
