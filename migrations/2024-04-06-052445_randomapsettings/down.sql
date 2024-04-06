-- This file should undo anything in `up.sql`
ALTER TABLE games DROP COLUMN provs_per_player;
ALTER TABLE maps_dom6 DROP COLUMN is_randommap;
