-- Your SQL goes here
ALTER TABLE nations
	ADD COLUMN filename VARCHAR NOT NULL DEFAULT 'unknown';
