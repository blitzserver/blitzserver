-- Insert the file first
INSERT INTO files (filename, filebinary) VALUES($1, $2);

-- Then insert the binary record using the last inserted file id
INSERT INTO binaries (binary_id, executable_path, version)
SELECT id, $3, $4
FROM files
WHERE filename = $1 AND filebinary = $2
ORDER BY id DESC
LIMIT 1
ON CONFLICT(binary_id) DO UPDATE SET version=EXCLUDED.version
RETURNING *;