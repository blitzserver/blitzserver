-- Your SQL goes here
CREATE TABLE binaries (
  binary_id INT REFERENCES files(id) PRIMARY KEY,
  executable_path TEXT NOT NULL,
  name TEXT NOT NULL
);

ALTER TABLE games ADD COLUMN binary_id INT REFERENCES binaries(binary_id);
