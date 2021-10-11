-- This file should undo anything in `up.sql`
ALTER TABLE turns DROP COLUMN created_at;
ALTER TABLE games DROP COLUMN next_turn;

DROP TRIGGER next_turn ON turns;
DROP FUNCTION update_next_turn;

