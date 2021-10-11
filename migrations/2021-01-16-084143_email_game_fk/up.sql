-- Your SQL goes here
ALTER TABLE email_configs ADD CONSTRAINT FK_GAME_EMAIL FOREIGN KEY(game_id) REFERENCES games(id);
