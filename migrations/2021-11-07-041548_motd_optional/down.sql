-- This file should undo anything in `up.sql`
UPDATE games SET motd='' WHERE motd IS NULL;
ALTER TABLE games ALTER COLUMN motd SET DEFAULT '';
ALTER TABLE games ALTER COLUMN motd SET NOT NULL;
