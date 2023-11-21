-- Your SQL goes here
CREATE TABLE binaries (
  binary_id INT REFERENCES files(id) PRIMARY KEY,
  executable_path TEXT NOT NULL,
  version TEXT NOT NULL,
  version_number INT NOT NULL GENERATED ALWAYS AS (regexp_replace(version, '[^0-9]+', '', 'g')::numeric) STORED,
  foldered_maps BOOL NOT NULL GENERATED ALWAYS AS (regexp_replace(version, '[^0-9]+', '', 'g')::numeric > 561) STORED
);

ALTER TABLE games ADD COLUMN binary_id INT REFERENCES binaries(binary_id);

ALTER TABLE maps ADD COLUMN min_version INT NOT NULL DEFAULT 0;
