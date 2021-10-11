-- This file should undo anything in `up.sql`
ALTER TABLE maps DROP COLUMN map_crc;
ALTER TABLE maps DROP COLUMN tga_crc;
ALTER TABLE maps DROP COLUMN winter_crc;
ALTER TABLE games DROP COLUMN modified_mapcrc 
