-- This file should undo anything in `up.sql`
ALTER TABLE players
	DROP CONSTRAINT  players_file_id_fkey;

ALTER TABLE players
	DROP COLUMN file_id;
