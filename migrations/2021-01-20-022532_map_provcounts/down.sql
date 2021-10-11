-- This file should undo anything in `up.sql`
ALTER TABLE maps
	DROP COLUMN province_count,
	DROP COLUMN uw_count;
