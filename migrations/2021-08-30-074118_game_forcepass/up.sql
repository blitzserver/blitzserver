-- Your SQL goes here
ALTER TABLE games ADD COLUMN require_password BOOLEAN NOT NULL DEFAULT false;
