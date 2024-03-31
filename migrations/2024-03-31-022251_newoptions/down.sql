-- This file should undo anything in `up.sql`
ALTER TABLE games ALTER COLUMN indepstr SET DEFAULT 5;
ALTER TABLE games DROP COLUMN mercslots;
ALTER TABLE games DROP COLUMN cheat_detection;

