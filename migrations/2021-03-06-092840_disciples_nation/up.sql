-- Your SQL goes here
CREATE TABLE disciples (
	id SERIAL PRIMARY KEY,
	game_id INTEGER NOT NULL,
	nation_id INTEGER NOT NULL,
	is_disciple INTEGER NOT NULL,
	team INTEGER
) 
