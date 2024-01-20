-- This file should undo anything in `up.sql`
ALTER TABLE games ADD FOREIGN KEY (map_id) REFERENCES maps(id);
ALTER TABLE games ALTER COLUMN map_id DROP DEFAULT;
