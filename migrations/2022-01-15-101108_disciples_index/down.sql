-- This file should undo anything in `up.sql`
ALTER TABLE disciples DROP CONSTRAINT nation_game_unique;
ALTER TABLE disciples ALTER COLUMN is_disciple DROP DEFAULT;
