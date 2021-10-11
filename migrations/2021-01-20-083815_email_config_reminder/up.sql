-- Your SQL goes here
ALTER TABLE email_configs ADD COLUMN is_reminder BOOLEAN NOT NULL DEFAULT false;
