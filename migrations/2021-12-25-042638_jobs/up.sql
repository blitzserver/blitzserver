
/**
 * Turn hosts being carried out
 */
CREATE TABLE jobs (
  job_id SERIAL PRIMARY KEY,
  game_id INT NOT NULL,
  turn_number INT NOT NULL,
  started_at timestamptz NOT NULL DEFAULT NOW(),
  updated_at timestamptz NOT NULL DEFAULT NOW(),
  reaped BOOLEAN NOT NULL DEFAULT false,
  finished_at timestamptz
);

/**
 * Steps in a turn host, ie. nation load, mod load, save load, process exec, ftherlnd interpretation, player_turn interpretation
 */
CREATE TABLE subjobs (
  subjob_id SERIAL PRIMARY KEY,
  job_id INT NOT NULL,
  label TEXT NOT NULL,
  logs TEXT,
  started_at timestamptz NOT NULL DEFAULT NOW(),
  finished_at timestamptz
);

ALTER TABLE subjobs ADD CONSTRAINT job_id FOREIGN KEY (job_id) REFERENCES jobs(job_id);

WITH topturn as (SELECT MAX(turns.turn_number) as "max", turns.game_id FROM turns GROUP BY turns.game_id)
INSERT INTO jobs ("game_id", "turn_number", "started_at", "updated_at", "finished_at")
  SELECT turns.game_id as "game_id", turns.turn_number as "turn_number", turns.created_at as "started_at", turns.created_at as "updated_at", turns.created_at as "finished_at"
  FROM turns
  LEFT JOIN topturn tt ON tt.game_id=turns.game_id
  WHERE turns.turn_number != tt."max";

INSERT INTO subjobs ("job_id", "label", "started_at", "finished_at")
  SELECT jobs.job_id as "job_id", 'dom5 processing turn' as "label", jobs.started_at as "started_at", jobs.finished_at as "finished_at"
  FROM jobs;

ALTER TABLE games ALTER next_turn TYPE timestamptz USING next_turn AT TIME ZONE 'UTC';

