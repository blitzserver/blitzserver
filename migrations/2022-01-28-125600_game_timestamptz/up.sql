-- Your SQL goes here

UPDATE games SET last_activity=NOW() WHERE last_activity IS NULL;
ALTER TABLE games ALTER last_activity TYPE timestamptz USING last_activity AT TIME ZONE 'UTC';
