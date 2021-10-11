-- Your SQL goes here
CREATE TABLE game_logs (
	id SERIAL PRIMARY KEY,
	game_id INT NOT NULL,
	datetime TIMESTAMP NOT NULL,
	turn_number INT NOT NULL,
	output VARCHAR NOT NULL,
	error VARCHAR NOT NULL
)

