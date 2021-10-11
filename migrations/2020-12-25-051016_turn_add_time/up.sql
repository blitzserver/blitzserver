-- Your SQL goes here
ALTER TABLE turns ADD COLUMN created_at TIMESTAMP NOT NULL DEFAULT now();
ALTER TABLE games ADD COLUMN next_turn TIMESTAMP;

CREATE FUNCTION update_next_turn() RETURNS TRIGGER AS
$BODY$
BEGIN
	UPDATE games g 
	SET next_turn = NEW.created_at + interval '1 minute' * timer
	WHERE id=NEW.game_id AND timer IS NOT NULL;
	UPDATE games g 
	SET next_turn = NULL
	WHERE id=NEW.game_id AND timer IS NULL;
	RETURN NEW;
END
$BODY$
language plpgsql;

CREATE TRIGGER next_turn AFTER INSERT ON turns
FOR EACH ROW
EXECUTE PROCEDURE update_next_turn();
