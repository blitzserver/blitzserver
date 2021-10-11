-- Your SQL goes here
CREATE TABLE game_mods (
	id SERIAL PRIMARY KEY,
	game_id INT NOT NULL,
	mod_id INT NOT NULL,

	CONSTRAINT fk_game_game_id
		FOREIGN KEY(game_id)
			REFERENCES games(id),

	CONSTRAINT fk_mod_mod_id
		FOREIGN KEY(mod_id)
			REFERENCES mods(id),

	CONSTRAINT game_mod UNIQUE(game_id, mod_id)

)
