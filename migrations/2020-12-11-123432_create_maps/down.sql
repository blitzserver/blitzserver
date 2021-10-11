-- This file should undo anything in `up.sql`
ALTER TABLE games DROP CONSTRAINT games_map_id_fkey;;
ALTER TABLE games	DROP COLUMN map_id;
DROP TABLE maps;
