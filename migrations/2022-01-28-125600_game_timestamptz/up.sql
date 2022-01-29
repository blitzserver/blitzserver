-- Your SQL goes here

ALTER TABLE games ALTER last_activity TYPE timestamptz USING next_turn AT TIME ZONE 'UTC';
