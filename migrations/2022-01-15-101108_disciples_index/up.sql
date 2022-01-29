-- Your SQL goes here
ALTER TABLE disciples ADD CONSTRAINT nation_game_unique UNIQUE(nation_id, game_id);
