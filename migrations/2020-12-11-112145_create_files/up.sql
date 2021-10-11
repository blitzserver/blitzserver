-- Your SQL goes here
CREATE TABLE files (
	id SERIAL PRIMARY KEY,
	filename VARCHAR NOT NULL,
	filebinary BYTEA NOT NULL,
	hash BIGINT NOT NULL,

	CONSTRAINT hash UNIQUE (hash)
)
