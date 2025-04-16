-- Add up migration script here
ALTER TABLE games ADD COLUMN borked BOOLEAN NOT NULL DEFAULT false;
