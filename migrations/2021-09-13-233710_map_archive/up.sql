-- Your SQL goes here
ALTER TABLE maps
ADD COLUMN archived BOOLEAN NOT NULL DEFAULT false;
