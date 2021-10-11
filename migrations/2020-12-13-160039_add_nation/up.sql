-- Your SQL goes here
CREATE TABLE nations (
	id SERIAL PRIMARY KEY,
	game_id INT NOT NULL,
	nation_id INT NOT NULL,
	name VARCHAR NOT NULL,
	epithet VARCHAR NOT NULL,

	CONSTRAINT FK_GAME
		FOREIGN KEY(game_id)
			REFERENCES games(id),

	CONSTRAINT gamenation_constraint UNIQUE (game_id, nation_id)
)
