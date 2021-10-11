-- Your SQL goes here
CREATE TABLE discord_configs (
	id SERIAL PRIMARY KEY,
	game_id INT NOT NULL,
	last_turn_notified INT,
	discord_guildid VARCHAR NOT NULL,
	discord_channelid VARCHAR NOT NULL,
	message VARCHAR NOT NULL,
	hours_remaining INT
)
