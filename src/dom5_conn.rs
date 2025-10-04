use crate::{models::{Player, Mod, self, settings::Setting}, packets::astralpacket_resp::AstralPacketResp};
use std::{io::Read};
use chrono::Utc;
use crate::packets::{ MapImageFileResp, ModFileResp, TwoHResp, DmFileResp, MapWinterFileResp,  Submit2hResp, MapFileResp, TrnResp, MapResp, TwoHCrcResp, BodyContents, PasswordsResp, PAResp, DisconnectResp, GameInfoResp, LoadingMessageResp, Packet };
use std::io::BufReader;
use std::net::TcpStream;
use thiserror::Error;
use serde_dom5::{Body, SaveFile};

#[derive(Error, Debug)]
pub enum UploadPretenderErr {
    #[error("Password required")]
    PassswordRequired,
    #[error("CD key differs")]
    CdKeyDiffer,
    #[error("Illegal nation")]
    IllegalNation,
    #[error("Missing nation")]
    MissingNation(i32),
    #[error("Version mismatch")]
    VersionMismatch((u32, u32))
}

#[derive(Error, Debug)]
pub enum Dom5ConnErr {
    #[error("Gameinfo generation error")]
    GameInfoErr,
    #[error("Pretender uplod error")]
    UploadPretenderErr(#[from] UploadPretenderErr),
    #[error("Unable to find turn")]
    TurnError(crate::models::TurnError)
}

pub struct Dom5Conn {
    pub stream: TcpStream,
    pub game_id: i32,
    pub db_pool: sqlx::sqlite::SqlitePool,
    pub active_connection_ids: Vec<i32>,
}

impl Drop for Dom5Conn {
    fn drop(&mut self) {
        smol::block_on(async {
            if !self.active_connection_ids.is_empty() {
                let placeholders = (0..self.active_connection_ids.len()).map(|i| format!("${}", i + 1)).collect::<Vec<_>>().join(",");
                let query = format!("UPDATE connections SET disconnected=datetime('now') WHERE id IN ({})", placeholders);
                let mut query = sqlx::query(&query);
                for id in &self.active_connection_ids {
                    query = query.bind(id);
                }
                query.execute(&self.db_pool).await.unwrap();
            }
        })
    }
}

impl Dom5Conn {
    pub fn new(stream: std::net::TcpStream, game_id: i32, db_pool: sqlx::sqlite::SqlitePool) -> Self {
        Self {
            stream,
            game_id,
            db_pool,
            active_connection_ids: vec![]
        }
    }
    fn accept_pretender(
        &mut self,
        req: crate::packets::UploadPretenderReq,
        ) -> Result<(), UploadPretenderErr> {
        smol::block_on(async {
            let twoh: SaveFile = bincode::deserialize(&req.pretender_contents[..]).unwrap();
            // Make sure pretender is new enough
            let required_version = Setting::get_or_env(&self.db_pool, "MIN_VERSION").await
                .or_else(|| {
                    log::warn!("No MIN_VERSION specified! Unable to validate pretenders min versions.");
                    None
                })
                .and_then(|s| {
                    s.parse::<u32>().or_else(|e| {
                        log::warn!("Failed to parse MIN_VERSION as an integer! Unable to validate pretender min versions: {:?}.", e);
                        Err(e)
                    }).ok()
                });
            if required_version.map(|r| r > twoh.header.version) == Some(true) {
                return Err(UploadPretenderErr::VersionMismatch((twoh.header.version, required_version.unwrap())));
            }
            // Make sure cdkey isnt violated
            let old_cdkey = Player::get_cdkey(self.game_id, twoh.header.nationid, &self.db_pool).map_err(|_| UploadPretenderErr::CdKeyDiffer)?;
            if let Some(old_cdkey) = old_cdkey {
                if old_cdkey != twoh.header.cdkey {
                    return Err(UploadPretenderErr::CdKeyDiffer);
                }
            }
            // Make sure password isnt violated
            let password_required = sqlx::query!("SELECT require_password FROM games WHERE id=$1", self.game_id).fetch_one(&self.db_pool).await.map_err(|_| UploadPretenderErr::PassswordRequired)?;
            if password_required.require_password && twoh.header.password.len() == 0 {
                return Err(UploadPretenderErr::PassswordRequired);
            }
            // Insert
            // Update game activity
            sqlx::query!("UPDATE games SET last_activity=datetime('now') WHERE id=$1", self.game_id).execute(&self.db_pool).await.map_err(|_|UploadPretenderErr::IllegalNation)?;

            // Insert file and get ID
            let file_record = sqlx::query!(r#"
                INSERT INTO files (filename, filebinary)
                SELECT n.filename || '.2h', $3 FROM nations n WHERE n.game_id=$1 AND n.nation_id=$2
                RETURNING id
            "#, self.game_id, twoh.header.nationid, req.pretender_contents).fetch_one(&self.db_pool).await.map_err(|_|UploadPretenderErr::IllegalNation)?;

            let name = if let Body::NewlordContents(nl) = twoh.body {
                nl.get_name().to_owned()
            } else {
                "".to_string()
            };

            // Insert/update player
            sqlx::query!(r#"
                INSERT INTO players (file_id, game_id, nationid, name, password, cdkey)
                VALUES ($1, $2, $3, $4, $5, $6)
                ON CONFLICT (game_id, nationid) DO UPDATE SET file_id=EXCLUDED.file_id, name=EXCLUDED.name, password=EXCLUDED.password
            "#,
            file_record.id,
            self.game_id,
            twoh.header.nationid,
            name,
            twoh.header.password,
            twoh.header.cdkey).execute(&self.db_pool).await.map_err(|_|UploadPretenderErr::IllegalNation)?;
            return Ok(());
        })
    }

    pub fn generate_gameinfo(&self) -> Result<GameInfoResp, Dom5ConnErr> {
        smol::block_on(async {
            match (
                sqlx::query!(r#"
                                SELECT 
                                    g.name, 
                                    renaming, 
                                    research_9_slow,
                                    formal_diplo,
                                    era "era: i32", 
                                    teamgame, 
                                    g.next_turn "next_turn: chrono::DateTime<Utc>", 
                                    t.turn_number "turn_number: u32"
                                FROM games g
                                LEFT JOIN (
                                    SELECT turn_number, game_id
                                    FROM turns t
                                    WHERE t.archived != TRUE
                                    ORDER BY t.turn_number DESC
                                    LIMIT 1
                                ) AS t ON t.game_id=g.id
                                WHERE g.id=$1"#,
                                self.game_id
                            ).fetch_one(&self.db_pool).await,
                            sqlx::query!(r#"SELECT d.nation_id "nation_id: i32", d.is_disciple, d.team FROM disciples d WHERE d.game_id=$1"#, self.game_id).fetch_all(&self.db_pool) .await,
                            sqlx::query!(r#"SELECT
                                p.nationid "nationid: i32",
                                pt.status AS "status?",
                                c.connected as "connected?"
                                FROM players p 
                                LEFT JOIN player_turns pt ON pt.archived!=TRUE AND pt.nation_id=p.nationid AND pt.game_id=$1 AND pt.turn_number=(SELECT MAX(turn_number) FROM turns WHERE game_id=$1 AND archived != TRUE)
                                LEFT JOIN connections c ON c.nation_id=p.nationid AND c.game_id=p.game_id AND c.disconnected IS NULL
                                WHERE p.game_id=$1"#,
                                self.game_id
                                ).fetch_all(&self.db_pool).await
                                ) {
                                    (Ok(game_record), Ok(disciple_records), Ok(player_status_records)) => {
                                        Ok(GameInfoResp {
                                            limited_research: game_record.research_9_slow,
                                            diplo_off: game_record.formal_diplo == 0,
                                            diplo_weak: game_record.formal_diplo == 1,
                                            unk1: 0,
                                            game_state: game_record.turn_number.map(|_| 2).unwrap_or(1) as u8,
                                            game_name: game_record.name,
                                            era: game_record.era,
                                            unk2: 0,
                                            disciples: game_record.teamgame,
                                            renaming: game_record.renaming,
                                            unk3: 0,
                                            milliseconds_to_host: game_record.next_turn.map(|nt| (nt - chrono::Utc::now()).num_milliseconds() as u32),
                                            unk4: 0,
                                            conn_statuses: player_status_records.iter().filter(|p| p.connected.is_some()).fold( std::collections::HashMap::new(), |mut acc, record| {
                                                acc.insert(record.nationid, 1);
                                                acc
                                            }),
                                            nation_statuses: player_status_records.iter().fold( std::collections::HashMap::new(), |mut acc, record| {
                                                match record.status {
                                                    Some(4) => { acc.insert(record.nationid, 255); },
                                                    Some(5) => { acc.insert(record.nationid, 254); },
                                                    Some(3) => { acc.insert(record.nationid, 2); },
                                                    _ => { acc.insert(record.nationid, 1); },
                                                }
                                                acc
                                            }),
                                            nation_teams: disciple_records.iter().fold( std::collections::HashMap::new(), |mut acc, current| { 
                                                if let Some(team) = current.team { 
                                                    acc.insert(current.nation_id, team as u8); 
                                                } 
                                                acc 
                                            }),
                                            nation_discs: disciple_records.iter().fold( std::collections::HashMap::new(), |mut acc, current| { 
                                                acc.insert( current.nation_id, current.is_disciple as u8); 
                                                acc 
                                            }),
                                            turn_statuses: player_status_records.iter().fold( std::collections::HashMap::new(), |mut acc, record| {
                                                acc.insert(record.nationid, record.status.unwrap_or(0) as u8);
                                                acc
                                            }),
                                            remaining: vec![],
                                            turn_number: game_record.turn_number.unwrap_or(0) as u32,
                                            turnkey: match game_record.turn_number {
                                                Some(t) => crate::models::Turn::get_turnkey(self.game_id, t, &self.db_pool).map_err(Dom5ConnErr::TurnError)? as u32,
                                                None => 0
                                            },
                                        })
                                    },
                                    _ => {
                                        Err(Dom5ConnErr::GameInfoErr)
                                    }

                                }
        })
    }
    fn send_message(&mut self, msg: &str) {
        for i in (1..10).rev() {
            LoadingMessageResp {
                message: format!("{} ({} seconds)", msg, i),
            }
            .write_packet(&mut self.stream, None);
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
        LoadingMessageResp {
            message: "".to_string(),
        }
        .write_packet(&mut self.stream, None);
    }
    pub fn run(mut self) {
        std::thread::spawn(move || {
            smol::block_on(async {
                let mut client_version: Option<u16> = None;
                let peer_addr = self.stream.peer_addr();
                let mut buffered_conn = BufReader::new(self.stream.try_clone().unwrap());
                while let Ok(packet) = Packet::from_reader(&mut buffered_conn, client_version, crate::packets::Protocol::GAME) {
                    log::info!("{:?} ===> {:?}", peer_addr, packet);
                    match packet.body {
                        /*
                         * User wishes the game to be started, set the next_turn to 5 seconds
                         */
                        crate::packets::Body::StartGameReq(_) => {
                            let in_5_seconds = chrono::Utc::now() + chrono::Duration::seconds(5);
                            if let Err(e) = sqlx::query!("UPDATE games SET next_turn=$1 WHERE id=$2",
                                                         in_5_seconds,
                                                         self.game_id)
                                .execute(&self.db_pool).await {
                                    log::error!( "Failed to set countdown for game {}: {:?}", self.game_id, e);
                                }
                        }
                        /*
                         * Initial connect and request for game state
                         */
                        crate::packets::Body::GameInfoReq(req) => {
                            client_version = Some(req.version);
                            self.generate_gameinfo()
                                .unwrap()
                                .write_packet(&mut self.stream, None);
                        }

                        /*
                         * AstralPacket (mod CRCs)
                         */
                        crate::packets::Body::AstralPacketReq(_) => {
                            let crcs = Mod::get_crcs(self.game_id, &self.db_pool).unwrap();
                            AstralPacketResp { dmfiles: crcs }.write_packet(&mut self.stream, client_version);
                        }
                        /*
                         * Heartbeat, expects gameinfo but also accepts a message
                         */
                        crate::packets::Body::HeartbeatReq(_) => {
                            self.generate_gameinfo()
                                .unwrap()
                                .write_packet(&mut self.stream, None);
                        }

                        /*
                         * Player is fucking off, expects an ack
                         */
                        crate::packets::Body::DisconnectReq(_) => {
                            DisconnectResp {}.write_packet(&mut self.stream, None);
                            break;
                        }
                        /*
                         * Player wants to set or overwrite a newlord
                         */
                        crate::packets::Body::UploadPretenderReq(pkt) => {
                            match self.accept_pretender(pkt) {
                                Err(UploadPretenderErr::CdKeyDiffer) => 
                                    self.send_message("This pretender is already claimed by another player. Your submission has been discarded"),
                                Err(UploadPretenderErr::PassswordRequired) => 
                                    self.send_message("This game requires passwords for all pretenders. Your submission has been discarded"),
                                Err(UploadPretenderErr::IllegalNation) => 
                                    self.send_message("There was an error accepting your pretender, contact the admin"),
                                Err(UploadPretenderErr::MissingNation(n)) => 
                                    self.send_message(&format!("Unable to identify nation {}, contact the admin", n)),
                                Err(UploadPretenderErr::VersionMismatch((n,m))) => 
                                    self.send_message(&format!("Pretender was made with Dom5 version {}, but minimum version is {}, please remake it!", (n as f32) / 100., (m as f32) / 100.)),
                                _ => {}
                            }
                        }
                        /*
                         * Player waiting for game to start
                         */
                        crate::packets::Body::PAReq(pkt) => {
                            smol::block_on(async {
                                loop {
                                    struct R { motd: Option<String>, max: Option<i32>, winner_name: Option<String> }
                                    match sqlx::query_as!(R, r#"
                                         WITH tt as (SELECT MAX(turn_number) as max, game_id from turns where archived != true GROUP BY game_id)
                                         SELECT 
                                            tt.max "max: i32", 
                                            g.motd "motd?", 
                                            n.name "winner_name?"
                                         FROM games g
                                         LEFT JOIN tt ON tt.game_id=g.id
                                         LEFT JOIN turns t ON t.turn_number=tt.max AND t.archived IS NOT TRUE and t.game_id=g.id
                                         LEFT JOIN nations n ON n.game_id=g.id AND n.nation_id=t.winner
                                         WHERE g.id=$1"#,
                                         self.game_id).fetch_one(&self.db_pool).await {
                                        Ok(R { motd: Some(motd), winner_name: Some(w), .. } ) => {
                                            LoadingMessageResp { message: format!("{}\nGame won by {}! Congratulations :)", motd, w) }.write_packet(&mut self.stream, None);
                                            PAResp {}.write_packet(&mut self.stream, None);
                                            break;
                                        }
                                        Ok(R { motd: None, winner_name: Some(w), .. } ) => {
                                            LoadingMessageResp { message: format!("Game won by {}! Congratulations :)", w) }.write_packet(&mut self.stream, None);
                                            PAResp {}.write_packet(&mut self.stream, None);
                                            break;
                                        }
                                        Ok(R { motd: Some(motd), max: Some(t), winner_name: None } ) => {
                                            LoadingMessageResp { message: format!("{}\nCurrent turn: {}", motd, t) }.write_packet(&mut self.stream, None);
                                            PAResp {}.write_packet(&mut self.stream, None);
                                            break;
                                        }
                                        Ok(R { motd: None, max: Some(t), winner_name: None } ) => {
                                            LoadingMessageResp { message: format!("Current turn: {}", t) }.write_packet(&mut self.stream, None);
                                            PAResp {}.write_packet(&mut self.stream, None);
                                            break;
                                        }
                                        _ => {
                                            LoadingMessageResp { message: "Game being started..".to_owned() }.write_packet(&mut self.stream, None);
                                        }
                                    }
                                    std::thread::sleep(std::time::Duration::from_secs(1));
                                }
                            })
                        }
                        /*
                         * Player sesssion started and nations selected
                         */
                        crate::packets::Body::NationsSelectedReq(pkt) => {
                            let nations = serde_json::to_string(&pkt.nations_selected).unwrap();
                            let ip = self.stream.peer_addr().map(|p| p.to_string()).unwrap_or("Unknown IP".to_owned());
                            match sqlx::query!(r#"INSERT INTO connections
                                (connected, game_id, nation_id, ip)
                                SELECT datetime('now'), $1, value, $3 FROM json_each($2) RETURNING id "id: i32""#,
                                self.game_id, 
                                nations,
                                ip
                                ).fetch_all(&self.db_pool).await {
                                Ok(connections) => {
                                    self.active_connection_ids = connections.into_iter().map(|c| c.id).collect(); 
                                }
                                Err(e) => log::error!("Failed to create new connections for nations {:?}: {:?}", &pkt.nations_selected, e)
                            }
                        }
                        /*
                         * Player is trying to set the team of a disciple or pretender
                         */
                        crate::packets::Body::SetTeamReq(pkt) => {
                            // Find count of pretender's on the specified team
                            // Find is_disciple status of possibly existing disciple record 
                            //
                            // Insert disciple record with is_disciple set to true if there already
                            // is a pretender, or if the nation was already set to be a disciple
                            // (otherwise default to pretender)
                            //
                            // If there already is a record, set the team and is_disciple based on
                            // aforementioned rules
                            if let Err(e) = sqlx::query!("
                                    WITH current_pretender as (SELECT COUNT(*) as p_count FROM disciples WHERE game_id=$1 AND team=$3 AND team>0 AND is_disciple=0 GROUP BY game_id),
                                    existing as (SELECT is_disciple FROM disciples WHERE game_id=$1 AND nation_id=$2),
                                    insert_vars as (SELECT $1 as game_id, $2 as nation_id, $3 as team)
                                    INSERT INTO disciples AS d (game_id,nation_id, team, is_disciple)
                                    SELECT iv.*,MAX(0, existing.is_disciple, p_count)
                                    FROM insert_vars iv
                                    LEFT JOIN current_pretender cp ON 1=1
                                    LEFT JOIN existing ON 1=1
                                    ON CONFLICT (game_id,nation_id) DO UPDATE SET (team, is_disciple) = (EXCLUDED.team, EXCLUDED.is_disciple)
                                    ", 
                                    self.game_id, 
                                    pkt.nation_id, 
                                    pkt.team).execute(&self.db_pool).await {
                                log::error!("Unable to set team for nation {} game {}: {:?}", pkt.nation_id, self.game_id, e);
                                self.send_message("Could not set team! Contact the admin!");
                            }
                        }
                        crate::packets::Body::SetDiscReq(pkt) => {
                            // Insert or update record to be disciple, set ALL OTHER records on
                            // this team in this game to disciple (there can only ever be one
                            // pretender, so we don't have to check if we're actually setting
                            // ourselves to be a pretender, since if we're not, then everything 
                            // will already be a disciple anyway).
                            // Insert/update disciple record
                            let disciple_result = sqlx::query!("
                                INSERT INTO disciples (is_disciple, game_id, nation_id) VALUES ($3, $1, $2)
                                ON CONFLICT (game_id, nation_id) DO UPDATE SET is_disciple=EXCLUDED.is_disciple
                                RETURNING team", self.game_id, 
                                pkt.nation_id, 
                                pkt.is_disc).fetch_one(&self.db_pool).await;

                            match disciple_result {
                                Ok(record) => {
                                    if let Some(team) = record.team {
                                        if team > 0 {
                                            if let Err(e) = sqlx::query!("UPDATE disciples SET is_disciple=1 WHERE team=$1 AND team>0 AND game_id=$2", team, self.game_id).execute(&self.db_pool).await {
                                                log::error!("Unable to update team disciples for nation {} game {}: {:?}", pkt.nation_id, self.game_id, e);
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    log::error!("Unable to set disciple status for nation {} game {}: {:?}", pkt.nation_id, self.game_id, e);
                                    self.send_message("Could not set disciple status! Contact the admin!");
                                }
                            }
                        }
                        /*
                         * Player wants to know which nations require passwords, so
                         * that it may prompt the user to enter them
                         */
                        crate::packets::Body::PasswordsReq(_) => {
                            match sqlx::query!(r#"SELECT nationid "nationid: i32" FROM players WHERE password IS NOT NULL AND game_id=$1"#, self.game_id).fetch_all(&self.db_pool).await {
                                Err(e) => {
                                    log::error!("Failed to determine passwords: {:?}", e);
                                    PasswordsResp::new(&vec![]).write_packet(&mut self.stream, None);
                                }
                                Ok(records) => {
                                    PasswordsResp::new(&records.into_iter().map(|r| r.nationid).collect::<Vec<i32>>()).write_packet(&mut self.stream, None);
                                }
                            }
                        }
                        /*
                         * Client wants the CRCs for all orders for the currently
                         * active turn for the purpose of uploading local turns or
                         * downloading turns from the remote server (us).
                         */
                        crate::packets::Body::TwoHCrcReq(_) => {
                            match sqlx::query!(r#"
                            WITH top_turn AS (SELECT MAX(turn_number) as max FROM turns WHERE game_id=$1 AND archived != TRUE)
                            SELECT pt.twoh_crc,pt.nation_id
                            FROM player_turns pt
                            INNER JOIN top_turn tt ON tt.max=pt.turn_number
                            WHERE pt.game_id=$1 AND pt.archived != TRUE"#, self.game_id)
                                .fetch_all(&self.db_pool)
                                .await {
                                    Err(e) => {
                                        log::error!("Failed to find twoh crc records: {:?}", e);
                                        TwoHCrcResp{ crcs: std::collections::HashMap::new() }.write_packet(&mut self.stream, None);
                                    }
                                    Ok(records) => {
                                        TwoHCrcResp{ crcs: records.into_iter().fold(std::collections::HashMap::new(),
                                        |mut acc, t| {
                                            acc.insert(t.nation_id as u16, t.twoh_crc.unwrap_or(0) as u32);
                                            acc
                                        })}.write_packet(&mut self.stream, None);
                                    }
                                }

                        }
                        /*
                         * Client wants the .trn for a specified nation for the
                         * current turn.
                         */
                        crate::packets::Body::TrnReq(pkt) => {
                            match sqlx::query!(r#"
                            WITH top_turn AS (SELECT MAX(turn_number) as max FROM turns WHERE game_id=$1 AND archived != TRUE)
                            SELECT f.filebinary
                            FROM player_turns pt 
                            INNER JOIN top_turn tt ON tt.max=pt.turn_number
                            INNER JOIN files f ON f.id=pt.trnfile_id
                            WHERE pt.nation_id = $2 AND pt.game_id=$1 AND pt.archived != TRUE"#,
                            self.game_id, 
                            pkt.nation_desired
                            ).fetch_one(&self.db_pool).await {
                                Err(e) => {
                                    log::error!("Failed to get trn for nation {}: {:?}", pkt.nation_desired, e);
                                    TrnResp { trn_contents: vec![] }.write_packet(&mut self.stream, None);
                                }
                                Ok(record) => {
                                    TrnResp { trn_contents: record.filebinary }.write_packet(&mut self.stream, None);
                                }
                            }
                        }
                        /*
                         * Map crcs
                         */
                        crate::packets::Body::MapReq(pkt) => {
                            match client_version {
                                Some(v) if v >= 600 => {
                                    models::maps::Map::get_mapresp(self.game_id, &self.db_pool).await.unwrap().write_packet(&mut self.stream, None);
                                }
                                _ => {
                                    match crate::models::Map::get_crcs(self.game_id, &self.db_pool) {
                                        Ok(((map_filename, map_len, map_crc), (tga_filename, tga_len, tga_crc), winter_stuff)) => {
                                            MapResp {
                                                map_name: map_filename,
                                                image_name: tga_filename,
                                                winter_name: winter_stuff.as_ref().map(|w| w.0.clone()),
                                                map_len: map_len as u32,
                                                image_len: tga_len as u32,
                                                winter_len: winter_stuff.as_ref().map(|w| w.1 as u32),
                                                map_crc: map_crc as u32,
                                                image_crc: tga_crc as u32,
                                                winter_crc: winter_stuff.as_ref().map(|w| w.2 as u32)
                                            }.write_packet(&mut self.stream, None);
                                        }
                                        Err(e) => {
                                            log::error!("Unable to get map crcs: {:?}", e);
                                        }
                                    }
                                }
                            }
                        }
                        /*
                         * Client wants the .map for the currently active map
                         */
                        crate::packets::Body::MapFileReq(p) => {
                            match (
                                sqlx::query!(r#"SELECT f.filebinary
                                FROM games g
                                    INNER JOIN maps m ON m.id=g.map_id
                                    INNER JOIN files f ON f.id=m.mapfile_id
                                WHERE g.id=$1"#, self.game_id).fetch_one(&self.db_pool).await,
                                sqlx::query!(r#"SELECT p.nationid, p.startprov
                                FROM players p
                                WHERE p.game_id=$1 AND p.startprov IS NOT NULL"#, self.game_id).fetch_all(&self.db_pool).await) {
                                    (Ok(mut map), Ok(startprovs)) => {
                                        for p in startprovs {
                                            map.filebinary.append( &mut format!( "\n#specstart {} {}\n", p.nationid, p.startprov.unwrap_or(0)).into_bytes());
                                        }
                                        MapFileResp { map_contents: map.filebinary }.write_packet(&mut self.stream, None);
                                    }
                                    (e1, e2) => {
                                        log::error!("Failed to fetch map info: {:?}, {:?}", e1, e2);
                                    }
                            }
                        }
                        /*
                         * Client wants the normal image file for the currently
                         * active map.
                         */
                        crate::packets::Body::MapImageFileReq(p) => {
                            match client_version {
                                Some(v) if v >= 600 => {
                                    match sqlx::query!(r#"
                                    SELECT filebinary as "filebinary!"
                                    FROM games g
                                    INNER JOIN maps_dom6 md ON md.map_id=g.maps_dom6_id
                                    INNER JOIN map_plane_images pi ON pi.map_id=md.map_id
                                    INNER JOIN files f ON f.id=pi.image_file_id AND f.filename=$2
                                    WHERE g.id=$1
                                    UNION ALL
                                    SELECT filebinary as "filebinary!"
                                    FROM games g
                                    INNER JOIN maps_dom6 md ON md.map_id=g.maps_dom6_id
                                    INNER JOIN map_planes pl ON pl.map_id=md.map_id
                                    INNER JOIN files f ON f.id=pl.map_file_id AND f.filename=$2
                                    WHERE g.id=$1
                                    "#,
                                    self.game_id,
                                    p.filename
                                    ).fetch_one(&self.db_pool).await {
                                        Ok(record) => {
                                            MapImageFileResp {
                                                image_contents: record.filebinary
                                            }.write_packet(&mut self.stream, Some(v))
                                        },
                                        Err(e) => {
                                            log::error!("Failed to fetch map image for dom6 client {:?}", e);
                                        }
                                    }
                                }
                                _ => {
                                    match sqlx::query!(r#"SELECT f.filebinary
                                        FROM games g
                                            INNER JOIN maps m ON m.id=g.map_id
                                            INNER JOIN files f ON f.id=m.tgafile_id
                                        WHERE g.id=$1"#, self.game_id).fetch_one(&self.db_pool).await {
                                        Ok(m) => 
                                            MapImageFileResp {
                                                image_contents: m.filebinary,
                                            }.write_packet(&mut self.stream, None),
                                        Err(e) => log::error!("Failed to get TGA for game {}: {:?}", self.game_id, e)
                                    }
                                }
                            }
                        }
                        /*
                         * Client wants the winter image file for the currently
                         * active map
                         */
                        crate::packets::Body::MapWinterFileReq(_) => {
                            match sqlx::query!(r#"SELECT f.filebinary as "filebinary?"
                                FROM games g
                                    INNER JOIN maps m ON m.id=g.map_id
                                    INNER JOIN files f ON f.id=m.winterfile_id
                                WHERE g.id=$1"#, self.game_id).fetch_one(&self.db_pool).await {
                                    Ok(m) => 
                                        MapWinterFileResp {
                                            winter_contents: m.filebinary.unwrap_or(vec![]),
                                        }.write_packet(&mut self.stream, None),
                                    Err(e) => log::error!("Failed to get winter for game {}: {:?}", self.game_id, e)
                                }
                        }

                        /*
                         * Player wants the current twoh file for a given turn and
                         * nation, as they have been notified that there are already
                         * orders submitted via the twoh crcs packet.
                         */
                        crate::packets::Body::TwoHReq(pkt) => {
                            match sqlx::query!(r#"
                            WITH topturn AS (SELECT MAX(turn_number) turn_number FROM turns WHERE game_id=$1 AND archived != TRUE)
                            SELECT f.filebinary
                            FROM player_turns pt
                            INNER JOIN topturn tt ON tt.turn_number=pt.turn_number
                            INNER JOIN files f ON f.id=pt.twohfile_id
                            WHERE pt.nation_id=$2 AND pt.archived != TRUE AND game_id=$1"#, 
                            self.game_id, 
                            pkt.nation_desired).fetch_one(&self.db_pool).await {
                                Ok(filebinary) => {
                                    TwoHResp {
                                        nation_id: pkt.nation_desired as u16,
                                        twoh_contents: filebinary.filebinary
                                    }.write_packet(&mut self.stream, None);
                                }
                                Err(e) => {
                                    log::error!("Failed to get save for nation {} game {}", pkt.nation_desired, self.game_id);
                                    self.send_message("There was an issue downloading the orders from the server! You may have to contact the admin!");
                                }
                            }
                        }
                        /*
                         * Player has uploaded a turn. If its for an old turn,
                         * then just throw it in the trash.
                         */
                        crate::packets::Body::Submit2hReq(pkt) => {
                            match crate::files::saves::SaveFile::read_contents( std::io::Cursor::new(&pkt.twoh_contents)) {
                                Ok(twoh) => {
                                    match sqlx::query!(r#"SELECT MAX(turn_number) "turn_number: i32" FROM turns WHERE game_id=$1 AND archived != TRUE"#, self.game_id).fetch_one(&self.db_pool).await {
                                        Ok(t) => if t.turn_number != Some(twoh.header.turnnumber) {
                                            Submit2hResp {}.write_packet(&mut self.stream, None);
                                            continue;
                                        }
                                        Err(e) => {
                                            log::error!("Failed to check current turn for game {}: {:?}", self.game_id, e);
                                            self.send_message("There was an issue submitting your turn! You may have to contact the admin!");
                                            Submit2hResp {}.write_packet(&mut self.stream, None);
                                        }
                                    }
                                    let twoh_crc = crate::util::calculate_crc(&pkt.twoh_contents[..]);
                                    // Update game activity
                                    sqlx::query!("UPDATE games SET last_activity=datetime('now') WHERE id=$1", self.game_id).execute(&self.db_pool).await.ok();

                                    // Insert file and get ID
                                    let file_result = sqlx::query!(r#"
                                        INSERT INTO files (filename, filebinary)
                                        SELECT n.filename || '.2h', $1
                                        FROM nations n WHERE n.nation_id=$2 AND n.game_id=$3
                                        RETURNING id
                                    "#, pkt.twoh_contents, twoh.header.nationid, self.game_id).fetch_one(&self.db_pool).await;

                                    match file_result {
                                        Ok(file_record) => {
                                            let status = if twoh.header.status == 1 { 1 } else { 2 };
                                            match sqlx::query!(r#"
                                                UPDATE player_turns SET twohfile_id=$1, twoh_crc=$2, status=$3
                                                WHERE archived != TRUE AND game_id=$4 AND nation_id=$5 AND turn_number=$6
                                                RETURNING id
                                            "#, 
                                            file_record.id, 
                                            twoh_crc, 
                                            status,
                                            self.game_id, 
                                            twoh.header.nationid, 
                                            twoh.header.turnnumber
                                            ).fetch_one(&self.db_pool).await {
                                        Ok(_) => {
                                            Submit2hResp {}.write_packet(&mut self.stream, None);
                                        }
                                        Err(e) => {
                                            log::error!("Failed to receive and save 2h for nation {} in game {}: {:?}", twoh.header.turnnumber, self.game_id, e);
                                            self.send_message("There was an issue submitting your turn! You may have to contact the admin!");
                                            Submit2hResp {}.write_packet(&mut self.stream, None);
                                        }
                                    }
                                }
                                Err(e) => {
                                    log::error!("Failed to insert file for 2h submission: {:?}", e);
                                    self.send_message("There was an issue submitting your turn! You may have to contact the admin!");
                                    Submit2hResp {}.write_packet(&mut self.stream, None);
                                }
                            }
                                },
                                Err(e) => {
                                    log::error!("Corrupt orders received for game {}", self.game_id);
                                    self.send_message("Your orders seem to be corrupt! This is probably a bug with the server! Contact the admin!");
                                    Submit2hResp {}.write_packet(&mut self.stream, None);
                                }
                            }
                        }
                        /*
                         * Player wants a mod definition file for one of the mods
                         * enabled for the game as reported by the astral packet.
                         */
                        crate::packets::Body::DmFileReq(pkt) => {
                            let filename = format!("%{}", pkt.filename);
                            match sqlx::query!("SELECT gm.mod_id, m.file_id,m.dm_filename FROM game_mods gm LEFT JOIN mods m ON m.id=gm.mod_id WHERE game_id=$1 AND m.dm_filename LIKE $2", self.game_id, filename).fetch_one(&self.db_pool).await {
                                Err(e) => {
                                    log::error!("Unable to find mod with dm file {} in game {}", pkt.filename, self.game_id);
                                    self.send_message(&format!("Somethings wrong with the mods finding DM file {}. Contact the admin!", pkt.filename));
                                }
                                Ok(m) => {
                                        let mod_dir= std::env::current_dir()
                                            .unwrap()
                                            .join("tmp")
                                            .join("mods")
                                            .join(m.mod_id.to_string());
                                        if !mod_dir.exists() {
                                            let filebinary = match sqlx::query!("SELECT filebinary FROM files WHERE id=$1", m.file_id).fetch_one(&self.db_pool).await {
                                                Ok(marchive) => marchive,
                                                Err(e) => {
                                                    log::error!("Unable to extract mod for game {}: {:?}", self.game_id, e);

                                                    self.send_message("Somethings wrong with the mods. Contact the admin!");
                                                    continue;
                                                }
                                            };
                                            let mut archive = match zip::ZipArchive::new(std::io::Cursor::new(filebinary.filebinary)) {
                                                Ok(archive) => archive,
                                                Err(e) => {
                                                    log::error!("Unable to extract mod for game {}: {:?}", self.game_id, e);
                                                    self.send_message("Somethings wrong with the mods. Contact the admin!");
                                                    continue;
                                                }
                                            };
                                            if let Err(e) = archive.extract(&mod_dir) {
                                                log::error!("Unable to extract mod for game {}: {:?}", self.game_id, e);
                                                self.send_message("Somethings wrong with the mods. Contact the admin!");
                                                continue;
                                            }
                                        }
                                        let Some(location) = walkdir::WalkDir::new(&mod_dir).into_iter().flatten().filter(|e| {
                                            let filename = e.file_name().to_string_lossy();
                                            return filename == pkt.filename;
                                        }).next() else {
                                                log::error!("Unable to find dm file {} for mod that should have it {}", pkt.filename, m.mod_id);
                                                self.send_message("Somethings wrong with the mods. Contact the admin!");
                                                continue;
                                        };
                                        let mut contents = vec![];
                                        let mut file = std::fs::File::open(location.path()).unwrap();
                                        file.read_to_end(&mut contents).unwrap();
                                        DmFileResp {
                                            contents
                                        }.write_packet(&mut self.stream, None);
                                    }
                                }
                            }
                        crate::packets::Body::ModFileReq(pkt) => {
                            match sqlx::query!(r#"SELECT gm.mod_id, m.file_id,m.dm_filename "dm_filename!" FROM game_mods gm LEFT JOIN mods m ON m.id=gm.mod_id WHERE game_id=$1"#, self.game_id).fetch_all(&self.db_pool).await {
                                Err(e) => {
                                    log::error!("Unable to find mod list for game {}", self.game_id);
                                    self.send_message("Somethings wrong with the mods. Contact the admin!");
                                }
                                Ok(mods) => {
                                    match mods.into_iter().flat_map(|m| {
                                        let containing_folder = std::path::PathBuf::from(m.dm_filename).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or("".to_owned());
                                        std::fs::File::open(std::env::current_dir()
                                            .unwrap()
                                            .join("tmp")
                                            .join("mods")
                                            .join(m.mod_id.to_string())
                                            .join(containing_folder)
                                            .join(&pkt.filename))
                                    }).next() {
                                        Some(mut file) => {
                                            let mut contents = vec![];
                                            file.read_to_end(&mut contents).unwrap();
                                            ModFileResp::new(contents).write_packet(&mut self.stream, None);
                                        }
                                        None => {
                                            log::error!("Unable to find file {} for game {}", pkt.filename, self.game_id);
                                            self.send_message("Somethings wrong with the mods. Contact the admin!");
                                        }
                                    }
                                }
                            }
                        }

                        _ => {}
                    }
                }
            });
        });
    }
}
