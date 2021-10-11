-- Your SQL goes here
CREATE TABLE map_provinces (
  id serial PRIMARY KEY,
  map_id INTEGER NOT NULL,
  province_id INTEGER NOT NULL,
  x INTEGER NOT NULL,
  y INTEGER NOT NULL,
  unit_x FLOAT NOT NULL,
  unit_y FLOAT NOT NULL
)
