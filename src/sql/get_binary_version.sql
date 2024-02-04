
SELECT 
  b.executable_path, 
  b.version,
  f.filename
FROM 
  files f, 
  binaries b,
  games g
WHERE g.id=$1 AND f.id=g.binary_id AND b.binary_id=g.binary_id;


