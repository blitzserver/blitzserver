-- This file should undo anything in `up.sql`
ALTER TABLE games DROP COLUMN binary_id;
DROP TABLE binaries;

