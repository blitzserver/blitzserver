-- Your SQL goes here
CREATE TABLE maps (
	id SERIAL PRIMARY KEY,
	name VARCHAR NOT NULL,
	mapfile_id INT NOT NULL,
	tgafile_id INT NOT NULL,
	winterfile_id INT NOT NULL,
	archive_id INT NOT NULL,

	CONSTRAINT FK_MAPFILE
		FOREIGN KEY(mapfile_id)
			REFERENCES files(id),

	CONSTRAINT FK_TGAFILE
		FOREIGN KEY(tgafile_id)
			REFERENCES files(id),

	CONSTRAINT FK_WINTERFILE
		FOREIGN KEY(winterfile_id)
			REFERENCES files(id),

	CONSTRAINT FK_ARCHIVE
		FOREIGN KEY(archive_id)
			REFERENCES files(id)
);

ALTER TABLE games 
	ADD map_id INT NOT NULL;

ALTER TABLE games
	ADD FOREIGN KEY(map_id)
		REFERENCES maps(id);
