-- Your SQL goes here
ALTER TABLE email_configs
	ADD COLUMN subject VARCHAR NOT NULL DEFAULT 'Default email notifcation subject, you probably have a new turn',
	ADD COLUMN body VARCHAR NOT NULL DEFAULT 'Default email notification body, you probably have a new turn'
