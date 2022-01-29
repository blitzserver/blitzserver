-- This file should undo anything in `up.sql`
ALTER TABLE subjobs DROP CONSTRAINT job_id;
TRUNCATE TABLE subjobs;
TRUNCATE TABLE jobs;
DROP TABLE subjobs;
DROP TABLE jobs;
ALTER TABLE games ALTER next_turn TYPE timestamp;
