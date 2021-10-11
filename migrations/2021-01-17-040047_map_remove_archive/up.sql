-- Your SQL goes here
ALTER TABLE maps DROP COLUMN archive_id;
ALTER TABLE maps ADD UNIQUE (mapfile_id, tgafile_id, winterfile_id);
