-- This file should undo anything in `up.sql`
ALTER TABLE games DROP COLUMN maps_dom6_id;
DROP TABLE map_plane_images;
DROP TABLE map_planes;
DROP TABLE maps_dom6;

