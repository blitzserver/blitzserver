-- This file should undo anything in `up.sql`
ALTER TABLE games
	DROP COLUMN thrones_t1;
ALTER TABLE games
	DROP COLUMN thrones_t2;
ALTER TABLE games
	DROP COLUMN thrones_t3;
ALTER TABLE games
	DROP COLUMN throne_points_required;
