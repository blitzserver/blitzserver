-- This file should undo anything in `up.sql`
ALTER TABLE nations
	DROP COLUMN filename;
