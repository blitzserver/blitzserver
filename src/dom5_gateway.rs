use crate::dom5_conn::Dom5Conn;
use sqlx::sqlite::SqlitePool;
use std::net::TcpListener;
use thiserror::Error;

pub struct Gateway {
    pub pool: SqlitePool,
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
                    "UPDATE connections SET disconnected=datetime('now') WHERE disconnected IS NULL"
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
                        r#"SELECT port "port: i32" FROM games WHERE archived != TRUE AND port IS NOT NULL"#
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
                                Ok(listener) => {
                                    // Set nonblocking for cross-platform compatibility
                                    // On Windows, this might fail but we can still try to use it
                                    if let Err(e) = listener.set_nonblocking(true) {
                                        log::warn!(
                                            "Unable to set nonblocking on port {} (this is normal on some platforms): {:?}",
                                            port,
                                            e
                                        );
                                    }
                                    listen_pool.insert(port, listener);
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

                    // Check for incoming connections on all listeners
                    for listener in listen_pool.values() {
                        match listener.accept() {
                            Ok((stream, _)) => {
                                // On Windows, accepted sockets inherit nonblocking from listener
                                // We need blocking mode for packet processing
                                if let Err(e) = stream.set_nonblocking(false) {
                                    log::warn!("Failed to set socket to blocking mode: {:?}", e);
                                }

                                match stream.local_addr() {
                                    Ok(la) => {
                                        let port = la.port();
                                        match sqlx::query!(
                                            r#"SELECT id "id: i32" FROM games WHERE port = $1"#,
                                            port
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
                            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                // No connection available, this is normal for nonblocking
                                continue;
                            }
                            Err(e) => {
                                // Only log actual errors, not WouldBlock
                                if e.kind() != std::io::ErrorKind::WouldBlock {
                                    log::error!("Error accepting connection: {:?}", e);
                                }
                            }
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }
            })
        });
    }
}
