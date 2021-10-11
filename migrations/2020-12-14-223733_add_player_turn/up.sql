-- Your SQL goes here
CREATE TABLE player_turns (
	id SERIAL PRIMARY KEY,
	turn_number INT NOT NULL,
	nation_id INT NOT NULL,
	game_id INT NOT NULL,
	trnfile_id INT NOT NULL,
	twohfile_id INT,

	CONSTRAINT fk_game_game_id
		FOREIGN KEY(game_id)
			REFERENCES games(id),
	
	CONSTRAINT fk_nation_nation_id_game_id 
		FOREIGN KEY(nation_id, game_id)
			REFERENCES nations(nation_id, game_id),


	CONSTRAINT fk_file_trnfile_id
		FOREIGN KEY(trnfile_id)
			REFERENCES files(id),

	CONSTRAINT fk_file_twohfile_id
		FOREIGN KEY(twohfile_id)
			REFERENCES files(id),

	CONSTRAINT nation_turn UNIQUE (game_id, nation_id, turn_number)
)
