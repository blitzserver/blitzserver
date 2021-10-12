pub struct Scheduler {
    pub db_pool: sqlx::postgres::PgPool,
}

impl Scheduler {
    pub fn run(self) {
        std::thread::spawn(move || {
            smol::block_on(async {
                let mut joins = std::collections::HashMap::<
                    i32,
                    (std::thread::JoinHandle<_>, std::sync::Weak<()>),
                >::new();
                loop {
                    for (game_id, (handle, _)) in
                        joins.drain_filter(|_, (_, weak): &mut (_, _)| weak.upgrade().is_none())
                    {
                        match handle.join() {
                            Ok(_) => log::info!("Hosted turn for game {}", game_id),
                            Err(e) => {
                                log::error!("Failed to host turn for game {}: {:?}", game_id, e)
                            }
                        }
                    }
                    if let Err(e) = sqlx::query!(r#"
                                                 UPDATE games 
                                                 SET archived=true, port=NULL 
                                                 WHERE last_activity < NOW() - interval '2 weeks' AND archived!=true"#
                                                ).execute(&self.db_pool).await {
                        log::error!("Failed to archive stale games {:?}", e);
                    }
                    match sqlx::query!(r#"
                        WITH top_turn as (SELECT game_id,MAX(turn_number) as turn_number FROM turns WHERE archived!=TRUE GROUP BY game_id)
                        SELECT g.id, t.turn_number as "tt?" FROM games g
                        LEFT JOIN top_turn t 
                            ON t.game_id=g.id 
                        LEFT JOIN player_turns pt 
                            ON pt.archived!=TRUE 
                            AND pt.game_id=g.id 
                            AND pt.turn_number=t.turn_number
                            AND pt.status IN (0,1)
                        WHERE g.archived != TRUE
                        GROUP BY g.id,t.turn_number
                        HAVING g.next_turn < NOW() OR (COUNT(pt.id) = 0 AND t.turn_number > 0)"#)
                        .fetch_all(&self.db_pool)
                        .await {
                            Ok(game_records) => {
                                for game in game_records {
                                    let game_id = game.id;
                                    if !joins.contains_key(&game_id) {
                                        let db_pool = self.db_pool.clone();
                                        let lock = std::sync::Arc::new(());
                                        let lock_weak = std::sync::Arc::downgrade(&lock);
                                        joins.insert(game_id, (std::thread::spawn(move || {
                                            match game.tt {
                                                Some(t) => log::info!("Hosting turn {} for game {}", t, game_id),
                                                None => log::info!("Starting game {}", game_id)
                                            }

                                            let dom5_turn = crate::dom5_turn::Dom5Turn {
                                                db_pool: db_pool,
                                                game_id: game.id
                                            };
                                            dom5_turn.run(game.tt).unwrap();
                                            log::debug!("Completed host for game {}", game_id);
                                            drop(lock);
                                        }), lock_weak));
                                    }
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
