-- Your SQL goes here
CREATE TABLE admin_logs (
	id SERIAL PRIMARY KEY,
	game_id INT NOT NULL,
	datetime TIMESTAMP NOT NULL,
	action VARCHAR NOT NULL
)
