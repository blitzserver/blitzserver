use crate::models::{Player, Mod};
use std::io::Read;
use crate::files::saves::SaveFile;
use crate::packets::{ MapImageFileResp, ModFileResp, TwoHResp, DmFileResp, MapWinterFileResp,  Submit2hResp, AstralPacketResp, MapFileResp, TrnResp, MapResp, TwoHCrcResp, BodyContents, PasswordsResp, PAResp, DisconnectResp, GameInfoResp, LoadingMessageResp, Packet };
use std::io::{BufReader};
use std::net::TcpStream;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum UploadPretenderErr {
    #[error("Password required")]
    PassswordRequired,
    #[error("CD key differs")]
    CdKeyDiffer,
    #[error("Illegal nation")]
    IllegalNation,
    #[error("Missing nation")]
    MissingNation(i32)
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
    pub db_pool: sqlx::postgres::PgPool,
    pub active_connection_ids: Vec<i32>,
}

impl Drop for Dom5Conn {
    fn drop(&mut self) {
        smol::block_on(async {
            sqlx::query!("UPDATE connections SET disconnected=NOW() WHERE id = ANY($1::int[])", &self.active_connection_ids).execute(&self.db_pool).await.unwrap();
        })
    }
}

impl Dom5Conn {
    pub fn new(stream: std::net::TcpStream, game_id: i32, db_pool: sqlx::postgres::PgPool) -> Self {
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
            let twoh =
                SaveFile::read_contents(std::io::Cursor::new(&req.pretender_contents[..])).unwrap();
            // Make sure cdkey isnt violated
            let old_cdkey = Player::get_cdkey(self.game_id, twoh.header.nationid, &self.db_pool).map_err(|_| UploadPretenderErr::CdKeyDiffer)?;
            if let Some(old_cdkey) = old_cdkey {
                if old_cdkey != twoh.header.cdkey as i64 {
                    return Err(UploadPretenderErr::CdKeyDiffer);
                }
            }
            // Make sure password isnt violated
            let password_required = sqlx::query!("SELECT require_password FROM games WHERE id=$1", self.game_id).fetch_one(&self.db_pool).await.map_err(|_| UploadPretenderErr::PassswordRequired)?;
            if password_required.require_password && twoh.header.password.len() == 0 {
                return Err(UploadPretenderErr::PassswordRequired);
            }
            // Insert
            sqlx::query!(r#"
            WITH 
                updated_game as (
                    UPDATE games SET last_activity=NOW() WHERE id=$1),
                inserted_file as (
                    INSERT INTO files (filename, filebinary) 
                    SELECT CONCAT(n.filename, '.2h'), $6 FROM nations n WHERE n.game_id=$1 AND n.nation_id=$2
                    ON CONFLICT (filename,digest(filebinary, 'sha1'::text)) 
                    DO UPDATE SET filename=EXCLUDED.filename
                    RETURNING *)
            INSERT INTO players (file_id, game_id, nationid, name, password, cdkey) 
            SELECT id, $1, $2, $3, $4, $5 FROM inserted_file
            ON CONFLICT (game_id, nationid) DO UPDATE SET file_id=EXCLUDED.file_id, name=EXCLUDED.name, password=EXCLUDED.password
            "#,
            self.game_id,
            twoh.header.nationid,
            if let crate::files::saves::SaveBody::TwoHContents(o) = twoh.body {
                o.pretender_name
            } else {
                "".to_string()
            },
            twoh.header.password,
            twoh.header.cdkey as i64,
            &req.pretender_contents).execute(&self.db_pool).await.map_err(|_|UploadPretenderErr::IllegalNation)?;
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
                                    era, 
                                    teamgame, 
                                    g.next_turn, 
                                    t.turn_number as "turn_number?"
                                FROM games g
                                LEFT JOIN LATERAL (
                                    SELECT turn_number, game_id
                                    FROM turns t
                                    WHERE t.game_id=g.id AND t.archived != TRUE
                                    ORDER BY t.turn_number DESC
                                    LIMIT 1
                                ) AS t ON t.game_id=g.id
                                WHERE g.id=$1"#,
                                self.game_id
                            ).fetch_one(&self.db_pool).await,
                            sqlx::query!(r#"SELECT d.nation_id, d.is_disciple, d.team FROM disciples d WHERE d.game_id=$1"#, self.game_id).fetch_all(&self.db_pool) .await,
                            sqlx::query!(r#"SELECT
                                p.nationid,
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
                                            unk1: 0,
                                            game_state: game_record.turn_number.map(|_| 2).unwrap_or(1) as u8,
                                            game_name: game_record.name,
                                            era: game_record.era,
                                            unk2: 0,
                                            disciples: game_record.teamgame,
                                            renaming: game_record.renaming,
                                            unk3: 0,
                                            milliseconds_to_host: game_record.next_turn.map(|nt| (nt - chrono::Utc::now().naive_utc()).num_milliseconds() as u32),
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
                                            }
                                        })
                                    },
                                    _ => {
                                        Err(Dom5ConnErr::GameInfoErr)
                                    }

                                }
        })
    }
    pub fn run(mut self) {
        std::thread::spawn(move || {
            smol::block_on(async {
                let peer_addr = self.stream.peer_addr();
                let mut buffered_conn = BufReader::new(self.stream.try_clone().unwrap());
                while let Ok(packet) = Packet::from_reader(&mut buffered_conn) {
                    log::debug!("{:?} ===> {:?}", peer_addr, packet);
                    match packet.body {
                        /*
                         * User wishes the game to be started, set the next_turn to 5 seconds
                         */
                        crate::packets::Body::StartGameReq(_) => {
                            if let Err(e) = sqlx::query!("UPDATE games SET next_turn=$1 WHERE id=$2",
                                                         chrono::Utc::now().naive_utc() + chrono::Duration::seconds(5),
                                                         self.game_id)
                                .execute(&self.db_pool).await {
                                    log::error!( "Failed to set countdown for game {}: {:?}", self.game_id, e);
                                }
                        }
                        /*
                         * Initial connect and request for game state
                         */
                        crate::packets::Body::GameInfoReq(_) => {
                            self.generate_gameinfo()
                                .unwrap()
                                .write_packet(&mut self.stream);
                        }

                        /*
                         * AstralPacket (mod CRCs)
                         */
                        crate::packets::Body::AstralPacketReq(_) => {
                            let crcs = Mod::get_crcs(self.game_id, &self.db_pool).unwrap();
                            AstralPacketResp { dmfiles: crcs }.write_packet(&mut self.stream);
                        }
                        /*
                         * Heartbeat, expects gameinfo but also accepts a message
                         */
                        crate::packets::Body::HeartbeatReq(_) => {
                            self.generate_gameinfo()
                                .unwrap()
                                .write_packet(&mut self.stream);
                        }

                        /*
                         * Player is fucking off, expects an ack
                         */
                        crate::packets::Body::DisconnectReq(_) => {
                            DisconnectResp {}.write_packet(&mut self.stream);
                            break;
                        }
                        /*
                         * Player wants to set or overwrite a newlord
                         */
                        crate::packets::Body::UploadPretenderReq(pkt) => {
                            let err_msg = match self.accept_pretender(pkt) {
                                Err(UploadPretenderErr::CdKeyDiffer) => 
                                    Some("This pretender is already claimed by another player. Your submission has been discarded".to_owned()),
                                Err(UploadPretenderErr::PassswordRequired) => 
                                    Some("This game requires passwords for all pretenders. Your submission has been discarded".to_owned()),
                                Err(UploadPretenderErr::IllegalNation) => 
                                    Some("There was an error accepting your pretender, contact the admin".to_owned()),
                                Err(UploadPretenderErr::MissingNation(n)) => 
                                    Some(format!("Unable to identify nation {}, contact the admin", n)),
                                _ => None
                            };
                            if let Some(err_msg) = err_msg {
                                for i in (1..10).rev() {
                                    LoadingMessageResp {
                                        message: format!("{} ({} seconds)", err_msg, i),
                                    }
                                    .write_packet(&mut self.stream);
                                    std::thread::sleep(std::time::Duration::from_secs(1));
                                }
                                LoadingMessageResp {
                                    message: "".to_string(),
                                }
                                .write_packet(&mut self.stream);
                            }
                        }
                        /*
                         * Player waiting for game to start
                         */
                        crate::packets::Body::PAReq(pkt) => {
                            smol::block_on(async {
                                loop {
                                    struct R { motd: Option<String>, max: Option<i32> };
                                    match sqlx::query_as!(R, r#"
                                         WITH tt as (SELECT MAX(turn_number) as max, game_id from turns where archived != true GROUP BY game_id)
                                         SELECT tt.max as "max?", g.motd as "motd?"
                                         FROM games g
                                         LEFT JOIN tt ON tt.game_id=g.id
                                         WHERE g.id=$1"#,
                                         self.game_id).fetch_one(&self.db_pool).await {
                                        Ok(R { motd: Some(motd), max: Some(t) } ) => {
                                            LoadingMessageResp { message: format!("{}\nCurrent turn: {}", motd, t) }.write_packet(&mut self.stream);
                                            PAResp {}.write_packet(&mut self.stream);
                                            break;
                                        }
                                        Ok(R { motd: None, max: Some(t) } ) => {
                                            LoadingMessageResp { message: format!("Current turn: {}", t) }.write_packet(&mut self.stream);
                                            PAResp {}.write_packet(&mut self.stream);
                                            break;
                                        }
                                        _ => {
                                            LoadingMessageResp { message: "Game being started..".to_owned() }.write_packet(&mut self.stream);
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
                            match sqlx::query!("INSERT INTO connections
                                (connected, game_id, nation_id, ip)
                                VALUES (NOW(), $1, UNNEST($2::Integer[]), $3) RETURNING *",
                                self.game_id, &pkt.nations_selected, self.stream.peer_addr().map(|p| p.to_string()).unwrap_or("Unknown IP".to_owned())).fetch_all(&self.db_pool).await {
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
                            if let Err(e) = sqlx::query!("UPDATE disciples SET team=$3 WHERE game_id=$1 AND nation_id=$2", self.game_id, pkt.nation_id as i32, pkt.team as i32).execute(&self.db_pool).await {
                                log::error!("Unable to set team for nation {} game {}", pkt.nation_id, self.game_id);
                                for t in 1..10 {
                                    LoadingMessageResp { message: format!("Could not set team! Contact the admin! ({} secs)", 10-t) }.write_packet(&mut self.stream);
                                    std::thread::sleep(std::time::Duration::from_secs(1));
                                }
                                LoadingMessageResp { message: "".to_owned() }.write_packet(&mut self.stream);
                            }
                        }
                        crate::packets::Body::SetDiscReq(pkt) => {
                            if let Err(e) = sqlx::query!("UPDATE disciples SET is_disciple=$3 WHERE game_id=$1 AND nation_id=$2", self.game_id, pkt.nation_id as i32, pkt.is_disc as i32).execute(&self.db_pool).await {
                                log::error!("Unable to set disciple status for nation {} game {}", pkt.nation_id, self.game_id);
                                for t in 1..10 {
                                    LoadingMessageResp { message: format!("Could not set disciple status! Contact the admin! ({} secs)", 10-t) }.write_packet(&mut self.stream);
                                    std::thread::sleep(std::time::Duration::from_secs(1));
                                }
                                LoadingMessageResp { message: "".to_owned() }.write_packet(&mut self.stream);
                            }
                        }
                        /*
                         * Player wants to know which nations require passwords, so
                         * that it may prompt the user to enter them
                         */
                        crate::packets::Body::PasswordsReq(_) => {
                            match sqlx::query!("SELECT nationid FROM players WHERE password IS NOT NULL AND game_id=$1", self.game_id).fetch_all(&self.db_pool).await {
                                Err(e) => {
                                    log::error!("Failed to determine passwords: {:?}", e);
                                    PasswordsResp::new(&vec![]).write_packet(&mut self.stream);
                                }
                                Ok(records) => {
                                    PasswordsResp::new(&records.into_iter().map(|r| r.nationid).collect::<Vec<i32>>()).write_packet(&mut self.stream);
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
                                        TwoHCrcResp{ crcs: std::collections::HashMap::new() }.write_packet(&mut self.stream);
                                    }
                                    Ok(records) => {
                                        TwoHCrcResp{ crcs: records.into_iter().fold(std::collections::HashMap::new(),
                                        |mut acc, t| {
                                            acc.insert(t.nation_id as u16, t.twoh_crc.unwrap_or(0) as u32);
                                            acc
                                        })}.write_packet(&mut self.stream);
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
                            self.game_id, pkt.nation_desired as i32).fetch_one(&self.db_pool).await {
                                Err(e) => {
                                    log::error!("Failed to get trn for nation {}: {:?}", pkt.nation_desired, e);
                                    TrnResp { trn_contents: vec![] }.write_packet(&mut self.stream);
                                }
                                Ok(record) => {
                                    TrnResp { trn_contents: record.filebinary }.write_packet(&mut self.stream);
                                }
                            }
                        }
                        /*
                         * Map crcs
                         */
                        crate::packets::Body::MapReq(pkt) => {
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
                                    }.write_packet(&mut self.stream);
                                }
                                Err(e) => {
                                    log::error!("Unable to get map crcs: {:?}", e);
                                }
                            }
                        }
                        /*
                         * Client wants the .map for the currently active map
                         */
                        crate::packets::Body::MapFileReq(_) => {
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
                                        MapFileResp { map_contents: map.filebinary }.write_packet(&mut self.stream);
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
                        crate::packets::Body::MapImageFileReq(_) => {
                            match sqlx::query!(r#"SELECT f.filebinary
                                FROM games g
                                    INNER JOIN maps m ON m.id=g.map_id
                                    INNER JOIN files f ON f.id=m.tgafile_id
                                WHERE g.id=$1"#, self.game_id).fetch_one(&self.db_pool).await {
                                    Ok(m) => 
                                        MapImageFileResp {
                                            image_contents: m.filebinary,
                                        }.write_packet(&mut self.stream),
                                    Err(e) => log::error!("Failed to get TGA for game {}: {:?}", self.game_id, e)
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
                                        }.write_packet(&mut self.stream),
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
                            WHERE pt.nation_id=$2 AND pt.archived != TRUE"#, self.game_id, pkt.nation_desired as i32).fetch_one(&self.db_pool).await {
                                Ok(filebinary) => {
                                    TwoHResp {
                                        nation_id: pkt.nation_desired as u16,
                                        twoh_contents: filebinary.filebinary
                                    }.write_packet(&mut self.stream);
                                }
                                Err(e) => {
                                    log::error!("Failed to get save for nation {} game {}", pkt.nation_desired, self.game_id);
                                    for t in 1..10 {
                                        LoadingMessageResp { message: format!("There was an issue downloading the orders from the server! You may have to contact the admin! ({} secs)", 10-t) }.write_packet(&mut self.stream);
                                        std::thread::sleep(std::time::Duration::from_secs(1));
                                    }
                                    DisconnectResp {}.write_packet(&mut self.stream);
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
                                    match sqlx::query!("SELECT MAX(turn_number) turn_number FROM turns WHERE game_id=$1 AND archived != TRUE", self.game_id).fetch_one(&self.db_pool).await {
                                        Ok(t) => if t.turn_number != Some(twoh.header.turnnumber) {
                                            Submit2hResp {}.write_packet(&mut self.stream);
                                            continue;
                                        }
                                        Err(e) => {
                                            log::error!("Failed to check current turn for game {}: {:?}", self.game_id, e);
                                            for t in 1..10 {
                                                LoadingMessageResp { message: format!("There was an issue submitting your turn! You may have to contact the admin! ({} secs)", 10-t) }.write_packet(&mut self.stream);
                                                std::thread::sleep(std::time::Duration::from_secs(1));
                                            }
                                            LoadingMessageResp { message: "".to_owned()}.write_packet(&mut self.stream);
                                            Submit2hResp {}.write_packet(&mut self.stream);
                                        }
                                    }
                                    let twoh_crc = crate::util::calculate_crc(&pkt.twoh_contents[..]);
                                    match sqlx::query!(r#"
                                        WITH 
                                            topturn AS (SELECT MAX(turn_number) as turn_number FROM turns WHERE game_id=$3 AND archived != TRUE),
                                            inserted as (INSERT INTO files (filename, filebinary) 
                                            SELECT concat(n.filename, '.2h'), $1
                                            FROM nations n WHERE n.nation_id=$2 AND n.game_id=$3 
                                            ON CONFLICT (filename,digest(filebinary, 'sha1'::text)) DO UPDATE 
                                                SET filename=EXCLUDED.filename RETURNING *),
                                            updated as (UPDATE games SET last_activity=NOW() WHERE id=$3)
                                        UPDATE player_turns pt SET twohfile_id=inserted.id, twoh_crc=$5, status=$6
                                        FROM topturn tt, inserted
                                        WHERE archived != TRUE AND pt.game_id=$3 AND pt.nation_id=$2 AND pt.turn_number=tt.turn_number AND pt.turn_number=$4 RETURNING pt.id"#,
                                        pkt.twoh_contents, twoh.header.nationid, self.game_id, twoh.header.turnnumber, twoh_crc as i32, if twoh.header.status == 1 { 1 } else { 2 } ).fetch_one(&self.db_pool).await {
                                        Ok(_) => {
                                            Submit2hResp {}.write_packet(&mut self.stream);
                                        }
                                        Err(e) => {
                                            log::error!("Failed to receive and save 2h for nation {} in game {}: {:?}", twoh.header.turnnumber, self.game_id, e);
                                            for t in 1..10 {
                                                LoadingMessageResp { message: format!("There was an issue submitting your turn! You may have to contact the admin! ({} secs)", 10-t) }.write_packet(&mut self.stream);
                                                std::thread::sleep(std::time::Duration::from_secs(1));
                                            }
                                            LoadingMessageResp { message: "".to_owned()}.write_packet(&mut self.stream);
                                            Submit2hResp {}.write_packet(&mut self.stream);
                                        }
                                    }
                                },
                                Err(e) => {
                                    log::error!("Corrupt orders received for game {}", self.game_id);
                                    for t in 10..1 {
                                        LoadingMessageResp { message: format!("Your orders seem to be corrupt to me, this is probably a bug with the server! Contact the admin! ({} secs)", t) }.write_packet(&mut self.stream);
                                    }
                                }
                            }
                        }
                        /*
                         * Player wants a mod definition file for one of the mods
                         * enabled for the game as reported by the astral packet.
                         */
                        crate::packets::Body::DmFileReq(pkt) => {
                            match sqlx::query!("SELECT gm.mod_id, m.file_id,m.dm_filename FROM game_mods gm LEFT JOIN mods m ON m.id=gm.mod_id WHERE game_id=$1", self.game_id).fetch_all(&self.db_pool).await {
                                Err(e) => {
                                    log::error!("Unable to find mod list for game {}", self.game_id);
                                    for t in 10..1 {
                                        LoadingMessageResp { message: format!("Somethings wrong with the mods. Contact the admin! ({} secs)", t) }.write_packet(&mut self.stream);
                                    }
                                }
                                Ok(mods) => {
                                    for m in mods {
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
                                                    LoadingMessageResp { message: format!("Somethings wrong with the mods. Contact the admin!") }.write_packet(&mut self.stream);
                                                    continue;
                                                }
                                            };
                                            let mut archive = match zip::ZipArchive::new(std::io::Cursor::new(filebinary.filebinary)) {
                                                Ok(archive) => archive,
                                                Err(e) => {
                                                    log::error!("Unable to extract mod for game {}: {:?}", self.game_id, e);
                                                    LoadingMessageResp { message: format!("Somethings wrong with the mods. Contact the admin!") }.write_packet(&mut self.stream);
                                                    continue;
                                                }
                                            };
                                            if let Err(e) = archive.extract(&mod_dir) {
                                                log::error!("Unable to extract mod for game {}: {:?}", self.game_id, e);
                                                LoadingMessageResp { message: format!("Somethings wrong with the mods. Contact the admin!") }.write_packet(&mut self.stream);
                                                continue;
                                            }
                                        }
                                        if m.dm_filename == pkt.filename {
                                            match std::fs::File::open(mod_dir.join(&pkt.filename)) {
                                                Err(e) => {
                                                    log::error!("Unable to find dm file for mod that should have it: {:?}", e);
                                                    LoadingMessageResp { message: format!("Somethings wrong with the mods. Contact the admin!") }.write_packet(&mut self.stream);
                                                    continue;
                                                }
                                                Ok(mut file) => {
                                                    let mut contents = vec![];
                                                    file.read_to_end(&mut contents);
                                                    DmFileResp {
                                                        contents: contents
                                                    }.write_packet(&mut self.stream);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        crate::packets::Body::ModFileReq(pkt) => {
                            match sqlx::query!("SELECT gm.mod_id, m.file_id,m.dm_filename FROM game_mods gm LEFT JOIN mods m ON m.id=gm.mod_id WHERE game_id=$1", self.game_id).fetch_all(&self.db_pool).await {
                                Err(e) => {
                                    log::error!("Unable to find mod list for game {}", self.game_id);
                                    for t in 10..1 {
                                        LoadingMessageResp { message: format!("Somethings wrong with the mods. Contact the admin! ({} secs)", t) }.write_packet(&mut self.stream);
                                    }
                                }
                                Ok(mods) => {
                                    match mods.into_iter().flat_map(|m| {
                                        std::fs::File::open(std::env::current_dir()
                                            .unwrap()
                                            .join("tmp")
                                            .join("mods")
                                            .join(m.mod_id.to_string())
                                            .join(&pkt.filename))
                                    }).next() {
                                        Some(mut file) => {
                                            let mut contents = vec![];
                                            file.read_to_end(&mut contents);
                                            ModFileResp::new(contents).write_packet(&mut self.stream);
                                        }
                                        None => {
                                            log::error!("Unable to find file {} for game {}", pkt.filename, self.game_id);
                                            for t in 10..1 {
                                                LoadingMessageResp { message: format!("Somethings wrong with the mods. Contact the admin! ({} secs)", t) }.write_packet(&mut self.stream);
                                            }
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
