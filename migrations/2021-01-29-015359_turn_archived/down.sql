-- This file should undo anything in `up.sql`
DROP INDEX turn_number_game_id;
ALTER TABLE turns DROP COLUMN archived;
ALTER TABLE turns ADD CONSTRAINT turn_number_game_id UNIQUE (turn_number, game_id);

DROP INDEX nation_turn;
ALTER TABLE player_turns DROP COLUMN archived;
ALTER TABLE player_turns ADD CONSTRAINT nation_turn UNIQUE (game_id, nation_id, turn_number);
