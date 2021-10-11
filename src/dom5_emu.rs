use crate::diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use crate::dom5_proc::Dom5Proc;
use crate::models::{Disciple, Game, Map, Nation, Player, PlayerTurn, Turn};
use crate::msgbus::{
    ClientDiscMsg, EraChangedMsg, GameArchivedMsg, GameScheduleMsg, MapChangedMsg, ModsChangedMsg,
    Msg, NewTurnMsg, OrdersSubmittedMsg, PktMsg, TurnHostStartMsg,
};
use crate::msgbus::{MsgBusRx, MsgBusTx};
use crate::packets::BodyContents;
use crate::packets::{
    AstralPacketResp, DisconnectResp, DmFileResp, GameInfoResp, LoadingMessageResp, MapFileResp,
    MapImageFileResp, MapResp, MapWinterFileResp, ModFileResp, PAResp, Packet, PasswordsResp,
    Submit2hResp, TrnResp, TwoHCrcResp, TwoHResp,
};
use crossbeam_channel::RecvTimeoutError;
use diesel::r2d2::ConnectionManager;
use diesel::PgConnection;
use std::env;
use std::io::Read;
use std::ops::Add;
pub struct Dom5Emu {
    game_id: i32,
    bus_rx: MsgBusRx,
    bus_tx: MsgBusTx,
    pub db_pool: r2d2::Pool<ConnectionManager<PgConnection>>,
}

impl Dom5Emu {
    pub fn new(
        game_id: i32,
        bus_tx: MsgBusTx,
        bus_rx: MsgBusRx,
        db_pool: r2d2::Pool<ConnectionManager<PgConnection>>,
    ) -> Self {
        Self {
            game_id,
            bus_rx,
            bus_tx,
            db_pool,
        }
    }

    fn generate_gameinfo<D>(
        game_id: i32,
        connlist: &std::collections::HashMap<i32, std::sync::Weak<()>>,
        db: D,
    ) -> GameInfoResp
    where
        D: diesel::Connection<Backend = diesel::pg::Pg>,
    {
        use crate::schema::games::dsl as games_dsl;
        let game: Game = games_dsl::games
            .filter(games_dsl::id.eq(game_id))
            .get_result(&db)
            .unwrap();
        let players = crate::models::Player::get_players(game_id, &db).unwrap();
        let turn = Turn::get(game_id, &db);
        let turn_statuses = if let Ok(t) = &turn {
            let player_turns = t.get_player_turns(&db).unwrap();
            player_turns
                .iter()
                .fold(std::collections::HashMap::new(), |mut acc, t| {
                    acc.insert(t.nation_id, t.status as u8);
                    acc
                })
        } else {
            std::collections::HashMap::new()
        };
        let next_turn = match game.next_turn {
            Some(t) => match t.duration_since(std::time::SystemTime::now()) {
                Ok(t) => Some(t.as_millis() as u32),
                Err(_) => None,
            },
            None => None,
        };
        let discs = Disciple::get_all(game.id, &db).unwrap();
        let resp = GameInfoResp {
            unk1: 0,
            game_state: if turn.is_ok() { 2 } else { 1 },
            game_name: game.name,
            era: game.era,
            unk2: 0,
            disciples: game.teamgame,
            renaming: game.renaming,
            unk3: 0,
            milliseconds_to_host: next_turn,
            unk4: 0,
            conn_statuses: connlist.iter().fold(
                std::collections::HashMap::new(),
                |mut acc, (key, cur)| {
                    if let Some(_) = cur.upgrade() {
                        acc.insert(*key, 1);
                    }
                    acc
                },
            ),
            nation_statuses: if let Ok(_) = &turn {
                turn_statuses
                    .iter()
                    .fold(std::collections::HashMap::new(), |mut acc, (k, t)| {
                        if *t == 3 {
                            acc.insert(*k, 2);
                        } else {
                            acc.insert(*k, 1);
                        }
                        acc
                    })
            } else {
                players.iter().fold(
                    std::collections::HashMap::new(),
                    |mut accumulator, current| {
                        accumulator.insert(current.nationid, 1);
                        accumulator
                    },
                )
            },
            nation_teams: discs.iter().fold(
                std::collections::HashMap::new(),
                |mut acc, current| {
                    if let Some(team) = current.team {
                        acc.insert(current.nation_id, team as u8);
                    }
                    acc
                },
            ),
            nation_discs: discs.iter().fold(
                std::collections::HashMap::new(),
                |mut acc, current| {
                    acc.insert(current.nation_id, current.is_disciple as u8);
                    acc
                },
            ),
            turn_statuses: turn_statuses,
            remaining: vec![],
            turn_number: match &turn {
                Err(_) => 0,
                Ok(t) => t.turn_number as u32,
            },
            turnkey: match &turn {
                Ok(t) => {
                    let fthlnd = t.get_ftherlnd(&db).unwrap();
                    let fthrlnd_read = crate::files::saves::SaveFile::read_contents(
                        std::io::Cursor::new(&fthlnd.filebinary),
                    )
                    .unwrap();
                    fthrlnd_read.header.turnkey
                }
                Err(_) => 0,
            },
        };
        return resp;
    }

    fn fetch_port(&self) -> i32 {
        use crate::schema::games::dsl as games_dsl;
        let db = self.db_pool.get().unwrap();
        let game = games_dsl::games
            .filter(games_dsl::id.eq(self.game_id))
            .get_result::<Game>(&db)
            .unwrap();
        match game.port {
            Some(port) => port,
            None => {
                let port_var =
                    env::var("PORT_RANGE").expect("PORT_RANGE must be set (ie. '10000,10999')");
                let range: Vec<&str> = port_var.split(",").collect();
                let new_port: i32 = diesel::dsl::sql::<diesel::sql_types::Int4>(&format!(
                    "SELECT num FROM generate_series({}, {}) num LEFT OUTER JOIN games g ON g.port=num WHERE g.id IS NULL ORDER BY num ASC LIMIT 1",
                    range[0], range[1]
                ))
                .get_result(&db)
                .unwrap();
                diesel::update(games_dsl::games.filter(games_dsl::id.eq(game.id)))
                    .set(games_dsl::port.eq(new_port))
                    .execute(&db)
                    .unwrap();
                new_port
            }
        }
    }

    fn accept_pretender<D>(
        game_id: i32,
        req: crate::packets::UploadPretenderReq,
        db: &D,
    ) -> Option<String>
    where
        D: diesel::Connection<Backend = diesel::pg::Pg>,
    {
        use crate::files::saves::SaveFile;
        use crate::models::{File, NewFile, NewPlayer};
        let game = Game::get(game_id, db).unwrap();
        game.update_last_activity(db).unwrap();
        let twoh =
            SaveFile::read_contents(std::io::Cursor::new(&req.pretender_contents[..])).unwrap();
        let existing = Player::get_players(game_id, db)
            .unwrap()
            .into_iter()
            .find(|p| p.nationid == twoh.header.nationid);
        if let Some(existing) = existing {
            let file = existing.get_newlord(db).unwrap();
            let existing_twoh =
                SaveFile::read_contents(std::io::Cursor::new(file.filebinary)).unwrap();
            if existing_twoh.header.cdkey != twoh.header.cdkey {
                return Some("This nation is already claimed by another player. Your submission has been discarded.".to_owned());
            }
        }
        let password = twoh.header.password;
        if password == "" && game.require_password {
            return Some("This game requires all pretenders to have passwords. Your submission has been discarded.".to_owned());
        }
        let nation: Nation = Nation::get(game_id, twoh.header.nationid, db).unwrap();
        let file: File =
            NewFile::new(&format!("{}.2h", nation.filename), &req.pretender_contents).insert(db);

        let name = if let crate::files::saves::SaveBody::TwoHContents(o) = twoh.body {
            o.pretender_name
        } else {
            "".to_string()
        };
        NewPlayer {
            file_id: file.id,
            nationid: req.nation_id as i32,
            game_id: game_id,
            name: name,
            password: if password == "" { None } else { Some(password) },
        }
        .insert(db)
        .unwrap();
        return None;
    }

    pub fn scheduled_host(
        launch_id: i32,
        bus_tx: crate::msgbus::MsgBusTx,
        db_pool: r2d2::Pool<ConnectionManager<PgConnection>>,
    ) {
        log::info!("Hosting turn for game {}", launch_id);
        let db = db_pool.get().unwrap();
        let game = Game::get(launch_id, &db).unwrap();
        let turn_n = match Turn::get(game.id, &db) {
            Ok(turn) => turn.turn_number,
            Err(_) => 0,
        };
        bus_tx
            .send(Msg::TurnHostStart(TurnHostStartMsg {
                game_id: game.id,
                turn_number: turn_n + 1,
            }))
            .unwrap();
        let dom5_proc = Dom5Proc::new(&game);
        dom5_proc.host_turn(&db);
        log::debug!("Turn for game {} hosted", launch_id);
        let game = Game::get(launch_id, &db).unwrap();
        let turn = Turn::get(game.id, &db).unwrap();
        bus_tx
            .send(Msg::NewTurn(NewTurnMsg {
                game_id: game.id,
                turn_number: turn.turn_number,
            }))
            .unwrap();
        if let Some(timer) = game.next_turn {
            bus_tx
                .send(Msg::GameSchedule(GameScheduleMsg {
                    game_id: launch_id,
                    schedule: timer,
                }))
                .unwrap();
        }
        game.update_last_activity(&db).unwrap();
    }

    pub fn launch(self) {
        let connlist: std::sync::Arc<
            std::sync::Mutex<std::collections::HashMap<i32, std::sync::Weak<()>>>,
        > = std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));
        let rx_clone = self.bus_rx.new_recv();
        let db_pool = self.db_pool.clone();
        let launch_id = self.game_id;
        std::thread::spawn(move || {
            {
                let db = db_pool.get().unwrap();
                let game = Game::get(launch_id, &db).unwrap();
                let mut dom5_proc = Dom5Proc::new(&game);
                dom5_proc.populate_mods(&db);
                dom5_proc.populate_maps(&db);
                dom5_proc.update_nations(&db);
            }
            let modcrc_cache = ModCrcCache::new(launch_id, db_pool.clone());
            let mapcrc_cache = MapCrcCache::new(launch_id, db_pool.clone());
            mapcrc_cache.populate();
            modcrc_cache.populate();
            let mapcrc_cache_clone = mapcrc_cache.clone();
            let modcrc_cache_clone = modcrc_cache.clone();
            let port = self.fetch_port();
            let recv_addr = format!("0.0.0.0:{}", port);
            let shutdown = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let shutdown_tx = shutdown.clone();
            let recv_addr_tx = format!("0.0.0.0:{}", port);
            let pool_clone = db_pool.clone();
            let tx_clone = self.bus_tx.clone();
            std::thread::spawn(move || {
                let mapcrc_cache_clone = mapcrc_cache.clone();
                let modcrc_cache_clone = modcrc_cache.clone();
                loop {
                    let (turn_timer, abandon_timer) = {
                        let db = pool_clone.get().unwrap();
                        let game = Game::get(launch_id, &db).unwrap();
                        (
                            if let Some(t) = game.next_turn {
                                if t <= std::time::SystemTime::now() {
                                    // Timer passed, host turn
                                    Self::scheduled_host(
                                        game.id,
                                        tx_clone.clone(),
                                        pool_clone.clone(),
                                    );
                                    std::time::Duration::from_secs(1)
                                } else {
                                    t.duration_since(std::time::SystemTime::now()).unwrap()
                                }
                            } else {
                                std::time::Duration::from_secs(9_999_999)
                            },
                            if game
                                .last_activity
                                .add(std::time::Duration::from_secs(60 * 60 * 24 * 14))
                                <= std::time::SystemTime::now()
                            {
                                game.archive(true, &db).unwrap();
                                tx_clone
                                    .send(Msg::GameArchived(GameArchivedMsg { game_id: game.id }))
                                    .unwrap();
                                log::warn!("Archiving game {}:{}", game.id, game.name);
                                std::time::Duration::from_secs(9_999_999)
                            } else {
                                game.last_activity
                                    .add(std::time::Duration::from_secs(60 * 60 * 24 * 14))
                                    .duration_since(std::time::SystemTime::now())
                                    .unwrap()
                            },
                        )
                    };
                    match rx_clone.recv_timeout(std::cmp::min(turn_timer, abandon_timer)) {
                        /*
                         * Era or mod changed- regenerate nation list
                         */
                        Ok(Msg::EraChanged(EraChangedMsg {
                            game_id: inc_game_id,
                            ..
                        })) if inc_game_id == launch_id => {
                            let db = pool_clone.get().unwrap();
                            let game = Game::get(inc_game_id, &db).unwrap();
                            let mut dom5_proc = Dom5Proc::new(&game);
                            dom5_proc.populate_mods(&db);
                            dom5_proc.populate_maps(&db);
                            dom5_proc.update_nations(&db);
                        }
                        /*
                         * Orders received, check if all present then host turn
                         */
                        Ok(Msg::OrdersSubmitted(OrdersSubmittedMsg {
                            game_id: inc_game_id,
                            ..
                        })) if launch_id == inc_game_id => {
                            let db = pool_clone.get().unwrap();
                            let turn = Turn::get(inc_game_id, &db).unwrap();
                            let player_turns = turn.get_player_turns(&db).unwrap();
                            if player_turns
                                .iter()
                                .all(|pt| pt.status == 2 || pt.status == 3)
                            {
                                Self::scheduled_host(
                                    inc_game_id,
                                    tx_clone.clone(),
                                    pool_clone.clone(),
                                );
                            }
                        }
                        /*
                         * Kill the listen thread to free the port once a game is archived
                         */
                        Ok(Msg::GameArchived(GameArchivedMsg { game_id }))
                            if game_id == launch_id =>
                        {
                            shutdown_tx.swap(true, std::sync::atomic::Ordering::AcqRel);
                            std::net::TcpStream::connect(recv_addr_tx).unwrap();
                            break;
                        }
                        /*
                         * At a game server wide level (instead of connection level below) we receive
                         * map changes and recalculate the map CRC cache
                         */
                        Ok(Msg::MapChanged(MapChangedMsg { game_id, .. }))
                            if game_id == launch_id =>
                        {
                            mapcrc_cache_clone.populate();
                        }
                        /*
                         * Recalculate the mod crc cache on mod change
                         */
                        Ok(Msg::ModsChanged(ModsChangedMsg { game_id, .. }))
                            if game_id == launch_id =>
                        {
                            let db = pool_clone.get().unwrap();
                            let game = Game::get(launch_id, &db).unwrap();
                            let mut dom5_proc = Dom5Proc::new(&game);
                            dom5_proc.populate_mods(&db);
                            dom5_proc.update_nations(&db);
                            modcrc_cache_clone.populate();
                        }
                        Err(RecvTimeoutError::Disconnected) => {
                            log::error!("Msgbus failure for game {}", launch_id);
                        }
                        _ => {}
                    }
                }
            });
            let recv_sock = std::net::TcpListener::bind(recv_addr).unwrap();
            let mods = std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::<
                i32,
                zip::ZipArchive<std::io::Cursor<Vec<u8>>>,
            >::new()));
            loop {
                let mods_clone = mods.clone();
                let shutdown_rx = shutdown.clone();
                if let Ok((conn, client_addr)) = recv_sock.accept() {
                    if shutdown_rx.fetch_or(false, std::sync::atomic::Ordering::AcqRel) == true {
                        break;
                    }
                    let mut socket_send_clone = conn.try_clone().unwrap();
                    let mut buffered_conn = std::io::BufReader::new(conn);
                    let tx_clone = self.bus_tx.clone();
                    let rx_clone = self.bus_rx.new_recv();
                    std::thread::spawn(move || {
                        while let Ok(packet) = Packet::from_reader(&mut buffered_conn) {
                            if let Err(_) = tx_clone.send(Msg::Pkt(PktMsg {
                                addr: client_addr,
                                pkt: packet.body,
                            })) {
                                break;
                            }
                        }
                        tx_clone
                            .send(Msg::ClientDisc(ClientDiscMsg { addr: client_addr }))
                            .unwrap();
                        buffered_conn.into_inner()
                            .shutdown(std::net::Shutdown::Read)
                            .expect(&format!("Attempted to shutdown read socket with addr {} but its already dead", client_addr));
                    });
                    let pool_clone = self.db_pool.clone();
                    let game_id = self.game_id.clone();
                    let tx_clone = self.bus_tx.clone();
                    let mut waiting_for_pa_resp = false;
                    let mapcrc_cache_clone = mapcrc_cache_clone.clone();
                    let modcrc_cache_clone = modcrc_cache_clone.clone();
                    let connlist_clone = connlist.clone();
                    std::thread::spawn(move || {
                        let mut nation_holders: Vec<std::sync::Arc<()>> = vec![];
                        while let Ok(msg) = rx_clone.recv() {
                            match msg {
                                /*
                                 * Game has been archived, Force disconnect the client
                                 */
                                Msg::GameArchived(GameArchivedMsg {
                                    game_id: inc_game_id,
                                }) if game_id == inc_game_id => {
                                    DisconnectResp {}.write_packet(&mut socket_send_clone);
                                    break;
                                }
                                /*
                                 * Client has gone away, we can simply exit
                                 */
                                Msg::ClientDisc(ClientDiscMsg { addr }) if client_addr == addr => {
                                    log::debug!("Client {:?} disconnect", addr);
                                    socket_send_clone.shutdown(std::net::Shutdown::Both).expect("Failure while shutting down socket after unclean disconnection");
                                    break;
                                }
                                /*
                                 * When a turn starts hosting, we should send the client a message
                                 * notifying them the process has begun.
                                 * This should work at any time, however its unsafe to send it
                                 * outside a PA loading phase, as it tends to collide with
                                 * unrelated client request/response pairs
                                 */
                                Msg::TurnHostStart(TurnHostStartMsg {
                                    game_id: new_turn_game_id,
                                    turn_number,
                                }) if new_turn_game_id == game_id => {
                                    if waiting_for_pa_resp {
                                        LoadingMessageResp {
                                            message: if turn_number == 1 {
                                                "Game starting".to_owned()
                                            } else {
                                                format!("Hosting turn {}", turn_number)
                                            },
                                        }
                                        .write_packet(&mut socket_send_clone);
                                    }
                                }
                                /*
                                 * When a new turn finishes resolving, the client may be waiting
                                 * for a PA resp, if so, they must be notified of the turn now
                                 * being available
                                 */
                                Msg::NewTurn(NewTurnMsg {
                                    game_id: new_turn_game_id,
                                    turn_number,
                                }) if new_turn_game_id == game_id => {
                                    if waiting_for_pa_resp && turn_number == 1 {
                                        LoadingMessageResp {
                                            message: "".to_owned(),
                                        }
                                        .write_packet(&mut socket_send_clone);
                                        PAResp {}.write_packet(&mut socket_send_clone);
                                        waiting_for_pa_resp = true;
                                    }
                                }
                                Msg::Pkt(PktMsg { addr, pkt }) if client_addr == addr => {
                                    log::debug!("Pkt:{}=>{:x?}", client_addr, pkt);
                                    match pkt {
                                        /*
                                         * Player has hit the "Start Game" button. Expects the
                                         * countdown timer to be initiated with something
                                         * reasonably soon.
                                         */
                                        crate::packets::Body::StartGameReq(_) => {
                                            let db = pool_clone.get().unwrap();
                                            let game = Game::get(game_id, &db).unwrap();
                                            let schedule = std::time::SystemTime::now()
                                                .add(std::time::Duration::from_secs(5));
                                            game.schedule_turn(schedule, &db).unwrap();
                                            tx_clone
                                                .send(Msg::GameSchedule(
                                                    crate::msgbus::GameScheduleMsg {
                                                        game_id,
                                                        schedule,
                                                    },
                                                ))
                                                .unwrap();
                                        }
                                        /*
                                         * Player is fucking off, expects an ack
                                         */
                                        crate::packets::Body::DisconnectReq(_) => {
                                            DisconnectResp {}.write_packet(&mut socket_send_clone);
                                            socket_send_clone.shutdown(std::net::Shutdown::Both).expect("Failure while shutting down socket after client disconnect negotiation");
                                        }
                                        /*
                                         * Player wants to set or overwrite a newlord
                                         */
                                        crate::packets::Body::UploadPretenderReq(pkt) => {
                                            let db = pool_clone.get().unwrap();
                                            if let Some(error) =
                                                Self::accept_pretender(game_id, pkt, &db)
                                            {
                                                for i in (1..10).rev() {
                                                    LoadingMessageResp {
                                                        message: format!(
                                                            "{} ({} seconds)",
                                                            error, i
                                                        ),
                                                    }
                                                    .write_packet(&mut socket_send_clone);
                                                    std::thread::sleep(
                                                        std::time::Duration::from_secs(1),
                                                    );
                                                }
                                                LoadingMessageResp {
                                                    message: "".to_string(),
                                                }
                                                .write_packet(&mut socket_send_clone);
                                            }
                                        }
                                        /*
                                         * Seems to be some kind of keepalive
                                         */
                                        crate::packets::Body::HeartbeatReq(_) => {
                                            let db = pool_clone.get().unwrap();
                                            let connlist_clone_lock =
                                                connlist_clone.lock().unwrap();
                                            Self::generate_gameinfo(
                                                game_id,
                                                &*connlist_clone_lock,
                                                db,
                                            )
                                            .write_packet(&mut socket_send_clone);
                                        }
                                        /*
                                         * Client wants to know what mods are currently enabled
                                         */
                                        crate::packets::Body::AstralPacketReq(_) => {
                                            modcrc_cache_clone
                                                .fetch(4)
                                                .unwrap()
                                                .write_packet(&mut socket_send_clone);
                                        }
                                        /*
                                         * Regularly sent request for current game state. The
                                         * client does not like it if a game goes from active to
                                         * unstarted (turn >0 to turn =0).
                                         */
                                        crate::packets::Body::GameInfoReq(_) => {
                                            let db = pool_clone.get().unwrap();
                                            Self::generate_gameinfo(
                                                game_id,
                                                &*connlist_clone.lock().unwrap(),
                                                db,
                                            )
                                            .write_packet(&mut socket_send_clone);
                                        }
                                        crate::packets::Body::PAReq(_pkt) => {
                                            let db = pool_clone.get().unwrap();
                                            if Turn::get(game_id, &db).is_ok() {
                                                PAResp {}.write_packet(&mut socket_send_clone);
                                            } else {
                                                waiting_for_pa_resp = true;
                                            }
                                        }
                                        crate::packets::Body::NationsSelectedReq(pkt) => {
                                            let mut connlist_clone_lock =
                                                connlist_clone.lock().unwrap();
                                            for n in pkt.nations_selected {
                                                match connlist_clone_lock.entry(n) {
                                                    std::collections::hash_map::Entry::Vacant(
                                                        v,
                                                    ) => {
                                                        let l = std::sync::Arc::new(());
                                                        v.insert(std::sync::Arc::downgrade(&l));
                                                        nation_holders.push(l);
                                                    }
                                                    std::collections::hash_map::Entry::Occupied(
                                                        mut o,
                                                    ) => {
                                                        if let Some(l) = o.get().upgrade() {
                                                            nation_holders.push(l);
                                                        } else {
                                                            let l = std::sync::Arc::new(());
                                                            o.insert(std::sync::Arc::downgrade(&l));
                                                            nation_holders.push(l);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        /*
                                         * Player wants to know which nations require passwords, so
                                         * that it may prompt the user to enter them
                                         */
                                        crate::packets::Body::PasswordsReq(_) => {
                                            let db = pool_clone.get().unwrap();
                                            let trn = Turn::get(launch_id, &db).unwrap();
                                            let pts = trn.get_player_turns(&db).unwrap();
                                            let passworded_nations: Vec<i32> = pts
                                                .iter()
                                                .filter(|pt| pt.status != 3)
                                                .filter_map(|pt| {
                                                    let turn = pt.get_trn(&db).unwrap();
                                                    let trn = crate::files::saves::SaveFile::read_contents(
                                                        std::io::Cursor::new(turn.filebinary),
                                                    )
                                                    .unwrap();
                                                    if trn.header.password != "".to_string() {
                                                        return Some(trn.header.nationid);
                                                    } else {
                                                        return None;
                                                    }
                                                })
                                                .collect();
                                            PasswordsResp::new(&passworded_nations[..])
                                                .write_packet(&mut socket_send_clone);
                                        }
                                        /*
                                         * Client wants the CRCs for all orders for the currently
                                         * active turn for the purpose of uploading local turns or
                                         * downloading turns from the remote server (us).
                                         */
                                        crate::packets::Body::TwoHCrcReq(_) => {
                                            let db = pool_clone.get().unwrap();
                                            let crcs = match Turn::get(game_id, &db) {
                                                Ok(turn) => {
                                                    let player_turns =
                                                        turn.get_player_turns(&db).unwrap();
                                                    player_turns.iter().fold(
                                                        std::collections::HashMap::new(),
                                                        |mut acc, t| {
                                                            if let Ok(twoh) = t.get_2h(&db) {
                                                                acc.insert(
                                                                    t.nation_id as u16,
                                                                    crate::util::calculate_crc(
                                                                        &twoh.filebinary[..],
                                                                    ),
                                                                );
                                                            }
                                                            acc
                                                        },
                                                    )
                                                }
                                                Err(_) => std::collections::HashMap::new(),
                                            };
                                            TwoHCrcResp { crcs }
                                                .write_packet(&mut socket_send_clone);
                                        }
                                        /*
                                         * Client wants the .trn for a specified nation for the
                                         * current turn.
                                         */
                                        crate::packets::Body::TrnReq(pkt) => {
                                            let db = pool_clone.get().unwrap();
                                            match PlayerTurn::get(
                                                game_id,
                                                pkt.nation_desired as i32,
                                                &db,
                                            ) {
                                                Ok((_turn, file)) => TrnResp {
                                                    trn_contents: match file {
                                                        Some(f) => f.filebinary,
                                                        None => vec![],
                                                    },
                                                }
                                                .write_packet(&mut socket_send_clone),
                                                Err(_) => TrnResp {
                                                    trn_contents: vec![],
                                                }
                                                .write_packet(&mut socket_send_clone),
                                            };
                                        }
                                        /*
                                         * Client wants to know what the map configuration for the
                                         * game currently is. We preactively generate this packet
                                         * and store it in a cache. If the map cache has not
                                         * generated by the time we get here, it will retry five
                                         * times, before giving up and killing the connection.
                                         */
                                        crate::packets::Body::MapReq(_) => match mapcrc_cache_clone
                                            .fetch(5)
                                        {
                                            Some(pkt) => pkt.write_packet(&mut socket_send_clone),
                                            None => DisconnectResp {}
                                                .write_packet(&mut socket_send_clone),
                                        },
                                        /*
                                         * Client wants the .map for the currently active map
                                         */
                                        crate::packets::Body::MapFileReq(_) => {
                                            let db = pool_clone.get().unwrap();
                                            let game = Game::get(game_id, &db).unwrap();
                                            let map = Map::get(game.map_id, &db).unwrap();
                                            let (mut mapfile, _, _) = map.get_files(&db).unwrap();
                                            if game.select_start > 0 {
                                                let players =
                                                    Player::get_players(game.id, &db).unwrap();
                                                for (player, prov) in
                                                    players.iter().flat_map(|p| match p.startprov {
                                                        Some(startprov) => {
                                                            Some((p.nationid, startprov))
                                                        }
                                                        None => None,
                                                    })
                                                {
                                                    mapfile.filebinary.append(
                                                        &mut format!(
                                                            "\n#specstart {} {}\n",
                                                            player, prov
                                                        )
                                                        .into_bytes(),
                                                    );
                                                }
                                            }
                                            MapFileResp {
                                                map_contents: mapfile.filebinary,
                                            }
                                            .write_packet(&mut socket_send_clone);
                                        }
                                        /*
                                         * Client wants the normal image file for the currently
                                         * active map.
                                         */
                                        crate::packets::Body::MapImageFileReq(_) => {
                                            let db = pool_clone.get().unwrap();
                                            let game = Game::get(game_id, &db).unwrap();
                                            let map = Map::get(game.map_id, &db).unwrap();
                                            let (_, mapimagefile, _) = map.get_files(&db).unwrap();
                                            MapImageFileResp {
                                                image_contents: mapimagefile.filebinary,
                                            }
                                            .write_packet(&mut socket_send_clone);
                                        }
                                        /*
                                         * Client wants the winter image file for the currently
                                         * active map
                                         */
                                        crate::packets::Body::MapWinterFileReq(_) => {
                                            let db = pool_clone.get().unwrap();
                                            let game = Game::get(game_id, &db).unwrap();
                                            let map = Map::get(game.map_id, &db).unwrap();
                                            if let Ok((_, _, Some(winterimagefile))) =
                                                map.get_files(&db)
                                            {
                                                MapWinterFileResp {
                                                    winter_contents: winterimagefile.filebinary,
                                                }
                                                .write_packet(&mut socket_send_clone);
                                            } else {
                                                MapWinterFileResp {
                                                    winter_contents: vec![],
                                                }
                                                .write_packet(&mut socket_send_clone);
                                            }
                                        }
                                        /*
                                         * Player has uploaded a turn. It should be for the current
                                         * turn but it might not. No response data is required, so
                                         * for any case success or otherwise, an empty packet
                                         * should be sent back. The turn should be checked to ensure
                                         * that it is for the currently active turn and ditched if
                                         * it is not (time travelling pretenders are not allowed).
                                         */
                                        crate::packets::Body::Submit2hReq(pkt) => {
                                            let twoh =
                                                crate::files::saves::SaveFile::read_contents(
                                                    std::io::Cursor::new(&pkt.twoh_contents),
                                                )
                                                .unwrap();
                                            let db = pool_clone.get().unwrap();
                                            if let Ok((trn, _)) =
                                                PlayerTurn::get(game_id, twoh.header.nationid, &db)
                                            {
                                                if trn.turn_number == twoh.header.turnnumber {
                                                    let nation = crate::models::Nation::get(
                                                        game_id,
                                                        twoh.header.nationid,
                                                        &db,
                                                    )
                                                    .unwrap();
                                                    let fname = &format!("{}.2h", nation.filename);
                                                    let twohfile = crate::models::NewFile::new(
                                                        fname,
                                                        &pkt.twoh_contents,
                                                    );
                                                    trn.save_2h(
                                                        twohfile,
                                                        if twoh.header.status == 1 { 1 } else { 2 },
                                                        &db,
                                                    )
                                                    .unwrap();
                                                    tx_clone
                                                        .send(Msg::OrdersSubmitted(
                                                            OrdersSubmittedMsg {
                                                                game_id,
                                                                turn_id: trn.turn_number,
                                                                nation_id: trn.nation_id,
                                                            },
                                                        ))
                                                        .unwrap();
                                                }
                                            }
                                            Game::get(game_id, &db)
                                                .unwrap()
                                                .update_last_activity(&db)
                                                .unwrap();
                                            Submit2hResp {}.write_packet(&mut socket_send_clone);
                                        }
                                        /*
                                         * Player wants the current twoh file for a given turn and
                                         * nation, as they have been notified that there are already
                                         * orders submitted via the twoh crcs packet.
                                         */
                                        crate::packets::Body::TwoHReq(pkt) => {
                                            let db = pool_clone.get().unwrap();
                                            let (turn, _file) = PlayerTurn::get(
                                                game_id,
                                                pkt.nation_desired as i32,
                                                &db,
                                            )
                                            .unwrap();
                                            let twoh = turn.get_2h(&db).unwrap();
                                            TwoHResp {
                                                nation_id: turn.nation_id as u16,
                                                twoh_contents: twoh.filebinary,
                                            }
                                            .write_packet(&mut socket_send_clone);
                                        }
                                        /*
                                         * Player wants a mod image file, it may be a file from any mod
                                         * enabled as reported by the astral packet.
                                         */
                                        crate::packets::Body::ModFileReq(pkt) => {
                                            let db = pool_clone.get().unwrap();
                                            let game = Game::get(game_id, &db).unwrap();
                                            let mut mods = mods_clone.lock().unwrap();
                                            let mut archive = None;
                                            let mut buf = vec![];
                                            let normalized_filename =
                                                pkt.filename.replace("./", "");
                                            for a in mods.values_mut() {
                                                if a.by_name(&normalized_filename).is_ok() {
                                                    archive = Some(a);
                                                    break;
                                                }
                                            }
                                            match archive {
                                                None => {
                                                    let game_mods = game.get_mods(&db).unwrap();
                                                    for gm in game_mods {
                                                        let mut ziparchive = zip::ZipArchive::new(
                                                            std::io::Cursor::new(
                                                                gm.get_archive(&db)
                                                                    .unwrap()
                                                                    .filebinary,
                                                            ),
                                                        )
                                                        .unwrap();
                                                        if ziparchive
                                                            .by_name(&normalized_filename)
                                                            .is_ok()
                                                        {
                                                            let mut f = ziparchive
                                                                .by_name(&normalized_filename)
                                                                .unwrap();
                                                            f.read_to_end(&mut buf).unwrap();
                                                            drop(f);
                                                            mods.insert(gm.id, ziparchive);
                                                            break;
                                                        }
                                                    }
                                                }
                                                Some(a) => {
                                                    let mut f =
                                                        a.by_name(&normalized_filename).unwrap();
                                                    f.read_to_end(&mut buf).unwrap();
                                                }
                                            }
                                            if buf.len() == 0 {
                                                panic!(
                                                    "Failed to find file {} in any enabled mods",
                                                    pkt.filename
                                                );
                                            }
                                            ModFileResp::new(buf)
                                                .write_packet(&mut socket_send_clone);
                                        }
                                        /*
                                         * Player is trying to set the nation of a disciple (pr
                                         * disciple pretender)
                                         */
                                        crate::packets::Body::SetTeamReq(pkt) => {
                                            let db = pool_clone.get().unwrap();
                                            let disc =
                                                Disciple::get(launch_id, pkt.nation_id as i32, &db)
                                                    .unwrap();
                                            if pkt.team == 0 {
                                                disc.unset_team(&db).unwrap();
                                            } else {
                                                disc.set_team(pkt.team as i32, &db).unwrap();
                                            }
                                        }
                                        crate::packets::Body::SetDiscReq(pkt) => {
                                            let db = pool_clone.get().unwrap();
                                            let disc =
                                                Disciple::get(launch_id, pkt.nation_id as i32, &db)
                                                    .unwrap();
                                            disc.set_disc(pkt.is_disc as i32, &db).unwrap();
                                        }
                                        /*
                                         * Player wants a mod definition file for one of the mods
                                         * enabled for the game as reported by the astral packet.
                                         */
                                        crate::packets::Body::DmFileReq(pkt) => {
                                            let db = pool_clone.get().unwrap();
                                            let game = Game::get(game_id, &db).unwrap();
                                            let ourmod = game
                                                .get_mods(&db)
                                                .unwrap()
                                                .into_iter()
                                                .find(|m| m.dm_filename == pkt.filename)
                                                .unwrap();
                                            let mut mods = mods_clone.lock().unwrap();
                                            match mods.get_mut(&ourmod.id) {
                                                Some(archive) => DmFileResp {
                                                    contents: {
                                                        let mut buf: Vec<u8> = vec![];
                                                        archive
                                                            .by_name(&ourmod.dm_filename)
                                                            .unwrap()
                                                            .read_to_end(&mut buf)
                                                            .unwrap();
                                                        buf
                                                    },
                                                }
                                                .write_packet(&mut socket_send_clone),
                                                None => {
                                                    let modfile = ourmod.get_archive(&db).unwrap();
                                                    let mut archive = zip::ZipArchive::new(
                                                        std::io::Cursor::new(modfile.filebinary),
                                                    )
                                                    .unwrap();
                                                    DmFileResp {
                                                        contents: {
                                                            let mut buf = vec![];
                                                            archive
                                                                .by_name(&ourmod.dm_filename)
                                                                .unwrap()
                                                                .read_to_end(&mut buf)
                                                                .unwrap();
                                                            buf
                                                        },
                                                    }
                                                    .write_packet(&mut socket_send_clone);
                                                    mods.insert(ourmod.id, archive);
                                                }
                                            };
                                        }
                                        _ => {}
                                    }
                                }
                                _ => {}
                            }
                        }
                    });
                }
            }
        });
    }
}

#[derive(Clone)]
struct MapCrcCache {
    cache: std::sync::Arc<std::sync::Mutex<Option<MapResp>>>,
    db_pool: r2d2::Pool<ConnectionManager<PgConnection>>,
    game_id: i32,
}

impl MapCrcCache {
    pub fn new(game_id: i32, db_pool: r2d2::Pool<ConnectionManager<PgConnection>>) -> Self {
        Self {
            game_id,
            db_pool,
            cache: std::sync::Arc::new(std::sync::Mutex::new(None)),
        }
    }

    /**
     * Asynchronously populate the map cache within a child thread
     */
    pub fn populate(&self) {
        let db = self.db_pool.get().unwrap();
        let cache = self.cache.clone();
        let game_id = self.game_id;
        std::thread::spawn(move || {
            let mut cache = cache.lock().unwrap();
            let game = Game::get(game_id, &db).unwrap();
            let map = Map::get(game.map_id, &db).unwrap();
            let (mut mapfile, imagefile, winterfile) = map.get_files(&db).unwrap();
            let players = Player::get_players(game.id, &db).unwrap();
            if game.select_start > 0 {
                for (player, prov) in players.iter().flat_map(|p| match p.startprov {
                    Some(startprov) => Some((p.nationid, startprov)),
                    None => None,
                }) {
                    mapfile
                        .filebinary
                        .append(&mut format!("\n#specstart {} {}\n", player, prov).into_bytes());
                }
            }
            *cache = match winterfile {
                Some(wf) => Some(MapResp::new(
                    mapfile.filename,
                    mapfile.filebinary,
                    imagefile.filename,
                    imagefile.filebinary,
                    Some(wf.filename),
                    Some(wf.filebinary),
                )),
                None => Some(MapResp::new(
                    mapfile.filename,
                    mapfile.filebinary,
                    imagefile.filename,
                    imagefile.filebinary,
                    None,
                    None,
                )),
            }
        });
    }
    /**
     * Fetch the packet out of the map cache, specify the number of attempts between each try
     */
    pub fn fetch(&self, mut tries: i32) -> Option<MapResp> {
        while tries > 0 {
            let pkt = self.cache.lock().unwrap().clone();
            match pkt {
                None => {
                    tries -= 1;
                    std::thread::sleep(std::time::Duration::from_secs(4));
                }
                Some(pkt) => {
                    return Some(pkt);
                }
            }
        }
        return None;
    }
}

#[derive(Clone)]
struct ModCrcCache {
    cache: std::sync::Arc<std::sync::Mutex<Option<AstralPacketResp>>>,
    db_pool: r2d2::Pool<ConnectionManager<PgConnection>>,
    game_id: i32,
}

impl ModCrcCache {
    pub fn new(game_id: i32, db_pool: r2d2::Pool<ConnectionManager<PgConnection>>) -> Self {
        Self {
            game_id,
            db_pool,
            cache: std::sync::Arc::new(std::sync::Mutex::new(None)),
        }
    }

    /**
     * Asynchronously populate the astralpacket cache within a child thread
     */
    pub fn populate(&self) {
        let db = self.db_pool.get().unwrap();
        let game = Game::get(self.game_id, &db).unwrap();
        let cache = self.cache.clone();
        std::thread::spawn(move || {
            let mods = game.get_mods(&db).unwrap();
            let mut modcache = cache.lock().unwrap();
            *modcache = Some(AstralPacketResp {
                dmfiles: mods
                    .into_iter()
                    .map(|m| {
                        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(
                            m.get_archive(&db).unwrap().filebinary,
                        ))
                        .unwrap();
                        let hash =
                            crate::util::calculate_crc(archive.by_name(&m.dm_filename).unwrap());
                        (m.dm_filename, hash)
                    })
                    .collect(),
            });
        });
    }
    /**
     * Fetch the packet out of the astralpacket cache, specify the number of attempts between each try
     */
    pub fn fetch(&self, mut tries: i32) -> Option<AstralPacketResp> {
        while tries > 0 {
            let pkt = self.cache.lock().unwrap().clone();
            match pkt {
                None => {
                    tries -= 1;
                    std::thread::sleep(std::time::Duration::from_secs(4));
                }
                Some(pkt) => {
                    return Some(pkt);
                }
            }
        }
        return None;
    }
}
