use super::diesel::prelude::*;
use super::models::Game;
use crate::msgbus::{
    CreateGameMsg, Msg, MsgBusTx, NewTurnMsg,
    TurnHostStartMsg,
};
use crossbeam_channel::Sender;
use diesel::r2d2::ConnectionManager;
use diesel::PgConnection;

pub struct GameManager {
    db_pool: r2d2::Pool<ConnectionManager<PgConnection>>,
    tmp_dir: std::path::PathBuf,
    sender: Sender<ManagerMsg>,
    pub bus_rx: crate::msgbus::MsgBusRx,
    pub bus_tx: crate::msgbus::MsgBusTx,
}

pub enum ManagerMsg {
    Start(i32),
    Archive(i32),
}

pub struct GameManagerConfig<'a> {
    pub bus_rx: crate::msgbus::MsgBusRx,
    pub bus_tx: crate::msgbus::MsgBusTx,
    pub db_pool: &'a r2d2::Pool<ConnectionManager<PgConnection>>,
    pub tmp_dir: &'a std::path::Path,
}

impl GameManager {
    pub fn new(cfg: GameManagerConfig) -> GameManager {
        let (s, _r) = crossbeam_channel::unbounded();
        GameManager {
            bus_tx: cfg.bus_tx,
            bus_rx: cfg.bus_rx,
            db_pool: cfg.db_pool.clone(),
            tmp_dir: cfg.tmp_dir.to_owned(),
            sender: s,
        }
    }

    pub fn get_sender(&self) -> Sender<ManagerMsg> {
        self.sender.clone()
    }

    pub fn start(&mut self) {
        if !std::path::Path::exists(&self.tmp_dir.join("games")) {
            std::fs::create_dir_all(&self.tmp_dir.join("games")).unwrap();
        }
        self.launch_games()
    }
    pub fn monitor(&mut self) {
        loop {
            match self.bus_rx.recv() {
                Ok(Msg::CreateGame(CreateGameMsg { game_id })) => {
                    self.launch_game(game_id);
                }
                _ => {}
            }
        }
    }

    fn launch_game(&mut self, launch_id: i32) {
        let emu = crate::dom5_emu::Dom5Emu::new(
            launch_id,
            self.bus_tx.clone(),
            self.bus_rx.new_recv(),
            self.db_pool.clone(),
        );
        emu.launch();
    }

    fn launch_games(&mut self) {
        let current_games: Vec<Game> = {
            let db = self.db_pool.get().expect("Unable to connect to database");
            use super::schema::games::dsl as games_dsl;
            games_dsl::games
                .filter(games_dsl::archived.eq(false))
                .get_results(&db)
                .expect("Error loading games")
        };

        for game in current_games {
            self.launch_game(game.id);
        }
        self.monitor();
    }
}
