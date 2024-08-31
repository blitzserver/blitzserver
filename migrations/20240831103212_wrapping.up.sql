-- Add up migration script here
ALTER TABLE games ADD COLUMN randommap_wrapping INTEGER NOT NULL DEFAULT 0;
