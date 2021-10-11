-- This file should undo anything in `up.sql`
-- Your SQL goes here
ALTER TABLE games
	DROP COLUMN research_diff,
	DROP COLUMN research_rand,
	DROP COLUMN hof_size,
	DROP COLUMN global_size,
	DROP COLUMN indepstr,
	DROP COLUMN magicsites,
	DROP COLUMN eventrarity,
	DROP COLUMN richness,
	DROP COLUMN resources,
	DROP COLUMN recruitment,
	DROP COLUMN supplies,
	DROP COLUMN startprov,
	DROP COLUMN renaming,
	DROP COLUMN scoregraphs,
	DROP COLUMN nationinfo,
	DROP COLUMN artrest,
	DROP COLUMN teamgame,
	DROP COLUMN clustered,
	DROP COLUMN storyevents,
	DROP COLUMN newailvl,
	DROP COLUMN newai;

