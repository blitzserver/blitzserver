-- Your SQL goes here
CREATE TABLE players (
	id  SERIAL PRIMARY KEY,
	nationid INT NOT NULL,
	game_id INT NOT NULL,

	CONSTRAINT FK_GAME
		FOREIGN KEY(game_id)
			REFERENCES games(id),

	CONSTRAINT nationid_game_id_unique UNIQUE (nationid, game_id)
)
