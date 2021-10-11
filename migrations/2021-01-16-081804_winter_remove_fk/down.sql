-- This file should undo anything in `up.sql`
ALTER TABLE maps ADD CONSTRAINT FK_WINTERFILE FOREIGN KEY(winterfile_id) REFERENCES files(id);
ALTER TABLE maps
	ALTER COLUMN winterfile_id SET DEFAULT 1;
