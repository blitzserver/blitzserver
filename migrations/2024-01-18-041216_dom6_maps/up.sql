-- Your SQL goes here
CREATE TABLE maps_dom6 (
  map_id INTEGER NOT NULL GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  name TEXT NOT NULL
);

CREATE TABLE map_planes (
  map_id INTEGER REFERENCES maps_dom6(map_id),
  map_file_id INTEGER NOT NULL REFERENCES files(id),
  hash INTEGER NOT NULL,

  PRIMARY KEY (map_file_id)
);

CREATE TABLE map_plane_images (
  map_id INTEGER NOT NULL REFERENCES maps_dom6(map_id),
  map_file_id INTEGER NOT NULL REFERENCES map_planes(map_file_id),
  image_file_id INTEGER NOT NULL REFERENCES files(id),
  hash INTEGER NOT NULL,

  PRIMARY KEY (map_id, map_file_id, image_file_id)
);

ALTER TABLE games ADD COLUMN maps_dom6_id INTEGER REFERENCES maps_dom6(map_id);
