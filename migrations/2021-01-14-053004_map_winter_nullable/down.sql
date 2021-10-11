-- This file should undo anything in `up.sql`
ALTER TABLE maps
	ALTER COLUMN winterfile_id SET DEFAULT 1,
	ALTER COLUMN winterfile_id SET NOT NULL;
