-- This file should undo anything in `up.sql`
ALTER TABLE map_plane_images DROP CONSTRAINT map_plane_images_map_plane_fkey ;
ALTER TABLE map_planes DROP CONSTRAINT map_planes_pkey;
ALTER TABLE map_planes ADD PRIMARY KEY (map_file_id);
ALTER TABLE map_plane_images ADD CONSTRAINT map_plane_images_map_file_id_fkey FOREIGN KEY (map_file_id) REFERENCES map_planes (map_file_id);
