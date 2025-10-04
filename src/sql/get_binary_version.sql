
SELECT
  b.executable_path,
  b.version,
  f.filename
FROM
  files f,
  binaries b,
  games g
WHERE g.id=$1 AND f.id=COALESCE(g.binary_id, (SELECT binary_id FROM binaries ORDER BY binary_id DESC LIMIT 1)) AND b.binary_id=COALESCE(g.binary_id, (SELECT binary_id FROM binaries ORDER BY binary_id DESC LIMIT 1));


