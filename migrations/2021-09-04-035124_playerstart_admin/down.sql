-- This file should undo anything in `up.sql`
ALTER TABLE games ALTER COLUMN select_start DROP NOT NULL;
ALTER TABLE games ALTER COLUMN select_start DROP DEFAULT;
ALTER TABLE games ALTER COLUMN select_start TYPE BOOL USING select_start::BOOLEAN;
ALTER TABLE games ALTER COLUMN select_start SET DEFAULT false;
ALTER TABLE games ALTER COLUMN select_start SET NOT NULL;

