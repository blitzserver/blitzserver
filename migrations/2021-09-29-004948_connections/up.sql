-- Your SQL goes here
CREATE TABLE connections (
  id SERIAL PRIMARY KEY,
  connected TIMESTAMP NOT NULL,
  disconnected TIMESTAMP,
  game_id INT,
  nation_id INT,
  ip TEXT,

  CONSTRAINT fk_game_id
    FOREIGN KEY(game_id) 
	  REFERENCES games(id),
  CONSTRAINT fk_nation
    FOREIGN KEY(game_id, nation_id) 
	  REFERENCES nations(game_id, nation_id)
)



