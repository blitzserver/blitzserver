-- Your SQL goes here
CREATE EXTENSION "pgcrypto";
CREATE UNIQUE INDEX file_filebinary
ON files (filename, digest(filebinary, 'sha1'));
ALTER TABLE files DROP column IF EXISTS "hash";

