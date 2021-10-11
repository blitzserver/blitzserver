-- Your SQL goes here
CREATE TABLE email_configs (
	id SERIAL PRIMARY KEY,
	nation_id INT NOT NULL,
	game_id INT NOT NULL,
	hours_before_host INT NOT NULL,
	email_address VARCHAR NOT NULL,
	last_turn_notified INT
);

