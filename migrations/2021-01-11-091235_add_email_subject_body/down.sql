-- This file should undo anything in `up.sql`
ALTER TABLE email_configs
	DROP COLUMN subject,
	DROP COLUMN body
