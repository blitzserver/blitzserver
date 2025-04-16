use itertools::Itertools;
use sqlx::PgPool;
use std::{
    net::{SocketAddr, TcpStream},
};

use crate::{
    packets::{
        lobby::{
            joingame_resp::{JoinGameResp, RespCode::Success},
            lobby_gameinfo::{self, GameInfoPlayer, LobbyGameInfo},
            lobbyinfo_resp::{LobbyInfoGame, LobbyInfoResp},
            serverstatus_resp::ServerStatusResp,
        },
        BodyContents, Packet,
    },
};

pub struct Dom5LobbyConn {
    stream: TcpStream,
    client_addr: SocketAddr,
    pool: PgPool,
}

impl Dom5LobbyConn {
    pub fn new((stream, client_addr): (TcpStream, SocketAddr), pool: PgPool) -> Self {
        Self {
            stream,
            client_addr,
            pool,
        }
    }
    pub fn run(mut self) {
        std::thread::spawn(move || {
            smol::block_on(async {
                let mut buffered_conn = std::io::BufReader::new(self.stream.try_clone().unwrap());
                while let Ok(packet) =
                    Packet::from_reader(&mut buffered_conn, None, crate::packets::Protocol::LOBBY)
                {
                    log::info!("{:?} ===> {:?}", self.client_addr, packet);
                    match packet.body {
                        crate::packets::Body::JoinGameReq(req) => {
                            let resp = JoinGameResp { respcode: Success };
                            resp.write_packet(&mut self.stream, None);
                            let mut resp = ServerStatusResp::default();
                            resp.unkbyte = 60;
                            resp.unkbyte1 = 60;
                            resp.write_packet(&mut self.stream, None);
                            let game =
                                sqlx::query!("SELECT name FROM games WHERE id=$1", req.game_id)
                                    .fetch_one(&self.pool)
                                    .await
                                    .unwrap();
                            let players = sqlx::query!(
                                "SELECT name, nationid FROM players WHERE game_id=$1",
                                req.game_id
                            )
                            .fetch_all(&self.pool)
                            .await
                            .unwrap();
                            let resp = LobbyGameInfo {
                                game_name: game.name,
                                players: players
                                    .iter()
                                    .map(|p| GameInfoPlayer {
                                        nation_id: p.nationid as u8,
                                        status: lobby_gameinfo::PlayerStatus::Player,
                                        ..Default::default()
                                    })
                                    .collect_vec(),
                                    game_id: req.game_id,
                                ..Default::default()
                            };
                            resp.write_packet(&mut self.stream, None);
                            return;
                        }
                        crate::packets::Body::LobbyInfoReq(req) => {
                            let games = sqlx::query!(
                                r#"
                    SELECT g.id, g.name, g.motd, COALESCE(COUNT(gm.*), 0) "mod_count!"
                    FROM games g
                    LEFT JOIN game_mods gm ON gm.game_id=g.id
                    GROUP BY g.id
                    "#
                            )
                            .fetch_all(&self.pool)
                            .await
                            .unwrap();
                            let resp = LobbyInfoResp {
                                unk: 2,
                                motd: "".to_owned(),
                                games: games
                                    .into_iter()
                                    .map(|g| {
                                        LobbyInfoGame {
                                status: crate::packets::lobby::lobbyinfo_resp::Status::Preparing,
                                has_pass: match g.mod_count {
                                    0 => crate::packets::lobby::lobbyinfo_resp::GameInfo::Normal,
                                    _ => crate::packets::lobby::lobbyinfo_resp::GameInfo::Mods,
                                },
                                game_id: g.id,
                                unk: [0u8; 12],
                                name: g.name,
                                motd: g.motd.unwrap_or("".to_owned()),
                            }
                                    })
                                    .collect(),
                            };
                            resp.write_packet(&mut self.stream, None)
                        }
                        _ => {}
                    }
                }
            })
        });
    }
}
