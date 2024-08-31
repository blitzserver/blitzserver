-- Add up migration script here
ALTER TABLE games ADD COLUMN log_level INTEGER NOT NULL DEFAULT 0;
