-- Your SQL goes here
ALTER TABLE games ADD COLUMN provs_per_player INTEGER NOT NULL DEFAULT 15;
ALTER TABLE maps_dom6 ADD COLUMN is_randommap BOOLEAN NOT NULL DEFAULT false;
