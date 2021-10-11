-- This file should undo anything in `up.sql`
ALTER TABLE games ADD COLUMN newai BOOLEAN NOT NULL DEFAULT true;
