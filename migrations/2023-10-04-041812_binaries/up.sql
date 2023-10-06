-- Your SQL goes here
CREATE TABLE binaries (
  binary_id INT REFERENCES files(id) PRIMARY KEY,
  name TEXT NOT NULL
);

ALTER TABLE games ADD COLUMN binary_id INT REFERENCES binaries(binary_id);
