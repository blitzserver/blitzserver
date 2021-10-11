-- Your SQL goes here
CREATE TABLE mods (
	id SERIAL PRIMARY KEY,
	dm_filename VARCHAR NOT NULL,
	name VARCHAR NOT NULL,
	file_id INT NOT NULL,

	CONSTRAINT file_id UNIQUE (file_id)
)
