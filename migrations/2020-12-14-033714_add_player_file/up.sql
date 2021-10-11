-- Your SQL goes here
ALTER TABLE players
	ADD COLUMN file_id INTEGER NOT NULL;

ALTER TABLE players
	ADD FOREIGN KEY(file_id) REFERENCES files(id);

