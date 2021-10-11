-- This file should undo anything in `up.sql`
ALTER TABLE player_turns
	DROP COLUMN status;
