-- Your SQL goes here
CREATE TABLE maps_dom6 (
  map_id INTEGER NOT NULL GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  name TEXT NOT NULL,
  archive_file_id INTEGER NOT NULL REFERENCES files(id)
);

CREATE TABLE map_planes (
  map_id INTEGER REFERENCES maps_dom6(map_id),
  name TEXT NOT NULL,
  map_file_id INTEGER NOT NULL REFERENCES files(id),

  PRIMARY KEY (map_id, name)
);

CREATE TABLE map_plane_images (
  map_id INTEGER REFERENCES maps_dom6(map_id),
  plane_name TEXT NOT NULL,
  image_file_id INTEGER NOT NULL REFERENCES files(id),

  PRIMARY KEY (map_id, plane_name, image_file_id)
);
