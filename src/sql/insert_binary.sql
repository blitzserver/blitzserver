WITH new_file AS (
  INSERT INTO files (filename, filebinary) VALUES($1, $2) 
  ON CONFLICT (filename,digest(filebinary, 'sha1'::text)) DO UPDATE 
  SET filename=EXCLUDED.filename
  RETURNING *
)
INSERT INTO binaries (binary_id, executable_path, version) 
SELECT id, $3, $4 
FROM new_file 
ON CONFLICT(binary_id) DO UPDATE SET version=EXCLUDED.version
RETURNING *;
