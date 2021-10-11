-- Your SQL goes here
ALTER TABLE games
	ADD COLUMN password VARCHAR NOT NULL DEFAULT 'password';
