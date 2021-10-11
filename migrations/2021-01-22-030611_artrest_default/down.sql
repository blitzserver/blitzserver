-- This file should undo anything in `up.sql`
ALTER TABLE games
	ALTER COLUMN artrest SET DEFAULT FALSE;
