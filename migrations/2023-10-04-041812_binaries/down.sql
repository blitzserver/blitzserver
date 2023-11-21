-- This file should undo anything in `up.sql`
ALTER TABLE games DROP COLUMN binary_id;
DROP TABLE binaries;
ALTER TABLE maps DROP COLUMN min_version;
