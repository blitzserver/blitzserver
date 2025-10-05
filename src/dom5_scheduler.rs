pub struct Scheduler {
    pub db_pool: sqlx::sqlite::SqlitePool,
}

impl Scheduler {
    pub fn run(self) {
        std::thread::spawn(move || {
            smol::block_on(async {
                loop {
                    if let Err(e) = sqlx::query!(r#"
                                                 UPDATE games
                                                 SET archived=true, port=NULL
                                                 WHERE last_activity < datetime('now', '-14 days') AND archived!=true"#
                                                ).execute(&self.db_pool).await {
                        log::error!("Failed to archive stale games {:?}", e);
                    }
                    // First, reap old jobs
                    if let Err(e) = sqlx::query!("UPDATE jobs SET reaped=TRUE, finished_at=datetime('now') WHERE datetime('now') > datetime(updated_at, '+30 seconds') AND finished_at IS NULL").execute(&self.db_pool).await {
                        log::error!("Failed to reap old jobs: {:?}", e);
                    }

                    // Then find games that need hosting
                    match sqlx::query!(r#"
                        WITH top_turn as (SELECT game_id,MAX(turn_number) as turn_number FROM turns WHERE archived!=TRUE GROUP BY game_id)
                        INSERT INTO jobs ("game_id", "turn_number", "started_at", "updated_at")
                        SELECT g.id, COALESCE(t.turn_number, 0), datetime('now'), datetime('now')
                        FROM games g
                        LEFT JOIN top_turn t
                            ON t.game_id=g.id
                        LEFT JOIN turns tt
                            ON tt.game_id=g.id AND tt.turn_number=t.turn_number AND tt.archived != TRUE
                        LEFT JOIN jobs j
                            ON j.game_id=g.id AND j.turn_number=COALESCE(t.turn_number, 0) AND j.reaped!=TRUE AND finished_at IS NULL
                        LEFT JOIN player_turns pt
                            ON pt.archived!=TRUE
                            AND pt.game_id=g.id
                            AND pt.turn_number=t.turn_number
                            AND pt.status IN (0,1)
                            AND COALESCE((
                                SELECT COUNT(*)
                                FROM player_turns pt2
                                WHERE pt2.turn_number < t.turn_number
                                  AND pt2.turn_number >= (t.turn_number - g.max_stales)
                                  AND pt2.archived!=TRUE
                                  AND pt2.status=0
                                  AND pt2.game_id=g.id
                                  AND pt2.nation_id=pt.nation_id
                                ) < g.max_stales, TRUE)
                        WHERE g.archived != TRUE AND tt.winner IS NULL AND j.job_id IS NULL
                        GROUP BY g.id,t.turn_number
                        HAVING datetime(g.next_turn) <= datetime('now') OR (COUNT(pt.id) = 0 AND t.turn_number > 0)
                        RETURNING game_id "game_id: i32", job_id "job_id: i32", turn_number "turn_number: i32"
                      "#)
                        .fetch_all(&self.db_pool)
                        .await {
                            Ok(game_records) => {
                                for game in game_records {
                                    let game_id = game.game_id;
                                        let db_pool = self.db_pool.clone();
                                        std::thread::spawn(move || {
                                            match game.turn_number {
                                                0 => log::info!("Starting game {}", game_id),
                                                t => log::info!("Hosting turn {} for game {}", t, game_id),
                                            }

                                            let dom5_turn = crate::dom5_turn::Dom5Turn {
                                                db_pool,
                                                game_id: game.game_id
                                            };
                                            smol::block_on(dom5_turn.run(game.job_id)).unwrap();
                                            log::debug!("Completed host for game {}", game_id);
                                        });
                                    }
                                }
                            Err(e) => { log::error!("Failed to fetch games due to host: {:?}", e); }
                        }
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }
            })
        });
    }
}
