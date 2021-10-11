-- Your SQL goes here
ALTER TABLE player_turns
	ADD COLUMN status INT NOT NULL DEFAULT 0;
