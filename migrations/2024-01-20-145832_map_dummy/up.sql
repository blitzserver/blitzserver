-- Your SQL goes here
ALTER TABLE games DROP CONSTRAINT games_map_id_fkey;
ALTER TABLE games ALTER COLUMN map_id SET DEFAULT 0;
