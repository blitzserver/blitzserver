-- Your SQL goes here
CREATE TABLE turns (
	id SERIAL PRIMARY KEY,
	game_id INT NOT NULL,
	turn_number INT NOT NULL,
	file_id INT NOT NULL,

	CONSTRAINT fk_game_game_id
		FOREIGN KEY(game_id)
			REFERENCES games(id),

	CONSTRAINT fk_file_file_id
		FOREIGN KEY(file_id)
			REFERENCES files(id),

	CONSTRAINT turn_number_game_id UNIQUE (turn_number, game_id)
)
