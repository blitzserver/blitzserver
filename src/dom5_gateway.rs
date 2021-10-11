use crate::dom5_conn::Dom5Conn;
use futures::future::select_all;
use sqlx::postgres::PgPool;
use std::net::TcpListener;
use thiserror::Error;

pub struct Gateway {
    pub pool: PgPool,
}

#[derive(Error, Debug)]
enum GatewayError {
    #[error("Unable to fetch port list")]
    PortListError(sqlx::Error),
}

impl Gateway {
    pub fn run(self) {
        let pool = self.pool.clone();
        std::thread::spawn(move || -> Result<!, GatewayError> {
            smol::block_on(async {
                let mut listen_pool = std::collections::HashMap::<i32, TcpListener>::new();
                if let Err(e) = sqlx::query!(
                    "UPDATE connections SET disconnected=NOW() WHERE disconnected IS NULL"
                )
                .execute(&pool)
                .await
                {
                    log::error!(
                        "Failed to end stale connections when starting gateway: {:?}",
                        e
                    );
                }
                loop {
                    for port in sqlx::query!(
                        "SELECT port FROM games WHERE archived != TRUE AND port IS NOT NULL"
                    )
                    .fetch_all(&pool)
                    .await
                    .map_err(GatewayError::PortListError)?
                    .iter()
                    .flat_map(|p| p.port)
                    {
                        if !listen_pool.contains_key(&port) {
                            log::debug!("Adding listener for port {:?}", port);
                            match TcpListener::bind(format!("0.0.0.0:{}", port)) {
                                Ok(listener) => match listener.set_nonblocking(true) {
                                    Ok(_) => {
                                        listen_pool.insert(port, listener);
                                    }
                                    Err(e) => {
                                        log::error!(
                                            "Unable to set nonblocking on port {}: {:?}",
                                            port,
                                            e
                                        );
                                    }
                                },
                                Err(e) => {
                                    log::error!(
                                        "Failed to bind new listener for port {}: {:?}",
                                        port,
                                        e
                                    );
                                }
                            }
                        }
                    }

                    for (stream, _) in listen_pool.values().flat_map(|l| l.accept()) {
                        match stream.local_addr() {
                            Ok(la) => {
                                match sqlx::query!(
                                    "SELECT id FROM games WHERE port = $1",
                                    la.port() as i32
                                )
                                .fetch_one(&pool)
                                .await
                                {
                                    Ok(id) => {
                                        log::debug!("Client connected to game {:?}", id.id);
                                        let new_conn = Dom5Conn::new(stream, id.id, pool.clone());
                                        new_conn.run();
                                    }
                                    Err(_) => {
                                        log::error!(
                                            "Unable to fetch game id with port {}",
                                            la.port()
                                        );
                                    }
                                }
                            }
                            Err(_) => {
                                log::error!("Unable to identify client");
                            }
                        };
                    }
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }
            })
        });
    }
}
