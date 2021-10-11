-- Your SQL goes here
ALTER TABLE turns ADD COLUMN archived BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE turns DROP CONSTRAINT turn_number_game_id;
CREATE UNIQUE INDEX turn_number_game_id ON turns (game_id, turn_number) WHERE (archived IS false);

ALTER TABLE player_turns ADD COLUMN archived BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE player_turns DROP CONSTRAINT nation_turn;
CREATE UNIQUE INDEX nation_turn ON player_turns (game_id, nation_id, turn_number) WHERE (archived IS false);
