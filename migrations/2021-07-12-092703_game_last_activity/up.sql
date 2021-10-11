-- Your SQL goes here
ALTER TABLE games ADD COLUMN last_activity TIMESTAMP NOT NULL DEFAULT NOW();
