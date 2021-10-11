-- This file should undo anything in `up.sql`
DELETE FROM player_turns WHERE trnfile_id IS NULL;
ALTER TABLE player_turns ALTER COLUMN trnfile_id SET NOT NULL;
