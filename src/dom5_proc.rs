use crate::files::saves::SaveFile;
use nonblock::NonBlockingReader;
use std::io::Read;
use std::io::Seek;
use std::ops::Add;

use super::diesel::prelude::*;
use super::models::{
    File, Game, GameMod, Map, Mod, Nation, NewFile, NewGameLog, NewNation, NewPlayerTurn, Player,
    PlayerTurn, Turn,
};
use crate::files::saves::kingdom::KingdomType;

use std::io::Write;

pub enum GameCmd {
    Shutdown,
    SetTimerCmd,
}

pub enum ProcEvent {
    NewTurn,
}

pub struct Dom5ProcHandle {
    pub sender: crossbeam_channel::Sender<GameCmd>,
    pub port: i32,
    is_dead: std::sync::atomic::AtomicBool,
}
impl Dom5ProcHandle {
    pub fn new(sender: crossbeam_channel::Sender<GameCmd>, port: i32) -> Self {
        Self {
            sender,
            port,
            is_dead: std::sync::atomic::AtomicBool::new(false),
        }
    }
    pub fn shutdown(&self) {
        if !self.is_dead.load(std::sync::atomic::Ordering::SeqCst) {
            self.sender.send(GameCmd::Shutdown).unwrap();
        }
        self.is_dead
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }
}
impl Drop for Dom5ProcHandle {
    fn drop(&mut self) {
        self.shutdown();
    }
}

pub struct Dom5Proc {
    pub name: String,
    pub era: i32,
    datadir: std::path::PathBuf,
    pub savedir: std::path::PathBuf,
    pub game_id: i32,
}

impl Dom5Proc {
    pub fn new(game: &Game) -> Self {
        Self {
            savedir: std::env::current_dir()
                .unwrap()
                .join("tmp")
                .join("games")
                .join(game.id.to_string())
                .join("savedgames")
                .join(&game.name),
            datadir: std::env::current_dir()
                .unwrap()
                .join("tmp")
                .join("games")
                .join(game.id.to_string()),
            name: game.name.clone(),
            era: game.era,
            game_id: game.id,
        }
    }

    fn find_unused_port() -> Option<i32> {
        static PORT_COUNTER: std::sync::atomic::AtomicU16 = std::sync::atomic::AtomicU16::new(0);
        for _ in 0..5 {
            let grabbed =
                1024 + (PORT_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst) % 1024);
            match std::net::TcpListener::bind(("127.0.0.1", grabbed)) {
                Ok(_l) => {
                    return Some(grabbed.into());
                }
                _ => {}
            }
        }
        None
    }

    pub fn update_nations<D>(&self, db: &D)
    where
        D: diesel::Connection<Backend = diesel::pg::Pg>,
    {
        let mut arguments = {
            let mods: Vec<(GameMod, Mod)> = crate::schema::game_mods::dsl::game_mods
                .filter(crate::schema::game_mods::dsl::game_id.eq(self.game_id))
                .inner_join(
                    crate::schema::mods::dsl::mods
                        .on(crate::schema::mods::dsl::id.eq(crate::schema::game_mods::dsl::mod_id)),
                )
                .get_results::<(GameMod, Mod)>(db)
                .unwrap();
            mods.iter()
                .flat_map(|(_, m)| vec![String::from("-M"), m.dm_filename.clone()])
                .collect::<Vec<String>>()
        };
        let game = Game::get(self.game_id, db).unwrap();
        arguments.append(&mut vec![
            "--statusdump".to_string(),
            "--port".to_string(),
            format!("{}", Self::find_unused_port().unwrap()),
            "--era".to_string(),
            game.era.to_string(),
            "-T".to_string(),
            "--tcpserver".to_string(),
            "-g".to_string(),
            format!("{}", self.name),
        ]);
        match std::fs::remove_file(self.savedir.join("statusdump.txt")) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => panic!("{}", e),
            Ok(_) => {}
        }
        let mut proc = std::process::Command::new(std::path::PathBuf::from(
            std::env::var("DOM5_BIN")
                .expect("DOM5_BIN not specified")
                .to_string(),
        ))
        .stdout(std::process::Stdio::piped())
        .env("DOM5_CONF", &self.datadir)
        .args(arguments)
        .spawn()
        .expect(&format!(
            "Failed to launch dom5 binary for game {} while fetching nations",
            self.name
        ));
        let statusdump;
        let mut wait_counter = 10;
        loop {
            match std::fs::File::open(self.savedir.join("statusdump.txt")) {
                Ok(f) => {
                    statusdump = Some(f);
                    break;
                }
                Err(_) => {
                    if wait_counter == 0 {
                        let mut s = String::new();
                        match proc.stdout.unwrap().read_to_string(&mut s) {
                            Ok(_) => panic!("Unable to find statusdump with nation info for game {}\nStdout:\n{}", game.id, &s),
                            Err(e) => panic!("Unable to find statusdump with nation info for game {} and unable to fetch logs ({:?})", game.id, e)
                        }
                    } else {
                        wait_counter -= 1
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
        let nation_info_regex = regex::Regex::new(
            r#"Nation\t([0-9]+)\t[0-9]+\t[0-9]+\t[0-9]+\t[0-9]+\t([a-zA-Z_0-9]+)\t([^\t]+)\t([^$]+)"#,
        )
        .unwrap();
        if let Some(mut statusdump) = statusdump {
            let mut contents = vec![];
            statusdump.read_to_end(&mut contents).unwrap();
            let statusdump_str = String::from_utf8_lossy(&contents);
            let _nations: Vec<Nation> = statusdump_str
                .lines()
                .filter(|l| nation_info_regex.is_match(l))
                .map(|l| {
                    let result = nation_info_regex.captures(l).unwrap();
                    NewNation {
                        game_id: self.game_id,
                        nation_id: result.get(1).unwrap().as_str().parse().unwrap(),
                        filename: result.get(2).unwrap().as_str().to_owned(),
                        name: result.get(3).unwrap().as_str().to_owned(),
                        epithet: result.get(4).unwrap().as_str().to_owned(),
                    }
                })
                .map(|n| n.insert(db).unwrap())
                .collect();
        }
        proc.kill().unwrap();
        proc.wait().unwrap();
    }
    fn handle_new_turn<D>(&self, db: &D)
    where
        D: diesel::Connection<Backend = diesel::pg::Pg>,
    {
        let mut file = if let Ok(f) = std::fs::File::open(&self.savedir.join("ftherlnd")) {
            f
        } else {
            return;
        };
        let mut contents = Vec::new();
        file.read_to_end(&mut contents).unwrap();
        file.seek(std::io::SeekFrom::Start(0)).unwrap();
        let ftherlnd = SaveFile::read_contents(file).unwrap();
        let new_file: File = NewFile::new("ftherlnd", &contents).insert(db);
        let _inserted = crate::models::NewTurn {
            game_id: self.game_id,
            turn_number: ftherlnd.header.turnnumber,
            file_id: new_file.id,
        }
        .insert(db)
        .unwrap();
        let nations = crate::models::Nation::get_all(self.game_id, db).unwrap();
        let turn_number = ftherlnd.header.turnnumber;
        let kingdoms = match ftherlnd.body {
            crate::files::saves::SaveBody::TrnContents(trn) => trn.kingdoms,
            _ => panic!("Ftherlnd contains orders."),
        };
        let player_turns = kingdoms
            .into_iter()
            .map(|kingdom| match kingdom.player_type {
                KingdomType::Computer => Some(
                    (NewPlayerTurn {
                        trnfile_id: None,
                        status: 3,
                        nation_id: kingdom.nation_id.into(),
                        game_id: self.game_id,
                        turn_number,
                    })
                    .insert(db)
                    .unwrap(),
                ),
                KingdomType::Human => {
                    let nation = nations
                        .iter()
                        .find(|n| n.nation_id == kingdom.nation_id as i32)
                        .unwrap();
                    let mut twoh = std::fs::File::open(
                        &self.savedir.join(format!("{}.trn", &nation.filename)),
                    )
                    .unwrap();

                    let mut contents = Vec::new();
                    twoh.read_to_end(&mut contents).unwrap();
                    let file: File = NewFile::new(&nation.filename, &contents).insert(db);

                    Some(
                        (NewPlayerTurn {
                            trnfile_id: Some(file.id),
                            nation_id: kingdom.nation_id.into(),
                            game_id: self.game_id,
                            turn_number,
                            status: 0,
                        })
                        .insert(db)
                        .unwrap(),
                    )
                }
                KingdomType::Special => None,
                KingdomType::DefeatedWithTurn => None,
                KingdomType::Defeated => None,
            })
            .collect::<Vec<Option<PlayerTurn>>>();
    }
    fn populate_savegame<D>(&self, db: &D)
    where
        D: diesel::Connection<Backend = diesel::pg::Pg>,
    {
        if std::path::PathBuf::from(&self.savedir).exists() {
            std::fs::remove_dir_all(&self.savedir).unwrap();
            std::fs::create_dir_all(&self.savedir).unwrap();
        } else {
            std::fs::create_dir_all(&self.savedir).unwrap();
        }
        let latest_turn: Vec<(Turn, File)> = {
            use super::schema::turns::dsl::*;
            turns
                .filter(game_id.eq(self.game_id).and(archived.eq(false)))
                .order(turn_number.desc())
                .inner_join(
                    super::schema::files::dsl::files.on(super::schema::files::dsl::id.eq(file_id)),
                )
                .limit(1)
                .get_results(db)
                .unwrap()
        };
        if latest_turn.len() == 0 {
            let pf: Vec<(Player, File)> = {
                use super::schema::players::dsl::*;
                players
                    .filter(game_id.eq(self.game_id))
                    .inner_join(
                        super::schema::files::dsl::files
                            .on(super::schema::files::dsl::id.eq(file_id)),
                    )
                    .get_results(db)
                    .unwrap()
            };

            for (_player, file) in pf.iter() {
                let mut os_file =
                    std::fs::File::create(&self.savedir.join(&file.filename)).unwrap();
                os_file.write_all(&file.filebinary).unwrap();
            }
        } else {
            for (turn, file) in latest_turn.iter() {
                let mut os_file =
                    std::fs::File::create(&self.savedir.join(&file.filename)).unwrap();
                os_file.write_all(&file.filebinary).unwrap();
                use super::schema::player_turns::dsl::*;
                for (player_turn, trnfile) in player_turns
                    .filter(
                        game_id
                            .eq(self.game_id)
                            .and(turn_number.eq(turn.turn_number))
                            .and(archived.eq(false)),
                    )
                    .inner_join(
                        super::schema::files::dsl::files
                            .on(super::schema::files::dsl::id.nullable().eq(trnfile_id)),
                    )
                    .get_results::<(PlayerTurn, File)>(db)
                    .unwrap()
                    .iter()
                {
                    let mut os_file =
                        std::fs::File::create(&self.savedir.join(&trnfile.filename)).unwrap();
                    os_file.write_all(&trnfile.filebinary).unwrap();
                    if let Some(twohid) = player_turn.twohfile_id {
                        let twohfile: File = super::schema::files::dsl::files
                            .filter(super::schema::files::dsl::id.eq(twohid))
                            .get_result(db)
                            .unwrap();
                        let mut os_file =
                            std::fs::File::create(&self.savedir.join(&twohfile.filename)).unwrap();
                        os_file.write_all(&twohfile.filebinary).unwrap();
                    }
                }
            }
        }
    }
    pub fn populate_mods<D>(&mut self, db: &D)
    where
        D: diesel::Connection<Backend = diesel::pg::Pg>,
    {
        let mods: Vec<(GameMod, Mod, File)> = crate::schema::game_mods::dsl::game_mods
            .filter(crate::schema::game_mods::dsl::game_id.eq(self.game_id))
            .inner_join(
                crate::schema::mods::dsl::mods
                    .on(crate::schema::mods::dsl::id.eq(crate::schema::game_mods::dsl::mod_id)),
            )
            .inner_join(
                crate::schema::files::dsl::files
                    .on(crate::schema::mods::dsl::file_id.eq(crate::schema::files::dsl::id)),
            )
            .get_results::<(GameMod, Mod, File)>(db)
            .unwrap();
        let mod_dir = std::path::PathBuf::from(&self.datadir).join("mods");
        if !mod_dir.exists() {
            std::fs::create_dir_all(&mod_dir).unwrap();
        }
        for (_, _, cmodfile) in mods.iter() {
            let mut archive =
                zip::ZipArchive::new(std::io::Cursor::new(&cmodfile.filebinary)).unwrap();
            for i in 0..archive.len() {
                let mut f = archive.by_index(i).unwrap();
                if f.is_dir() {
                    if !mod_dir.join(f.name()).exists() {
                        std::fs::create_dir_all(&mod_dir.join(f.name())).unwrap();
                    }
                } else {
                    let mut folder_name = std::path::PathBuf::from(f.name());
                    folder_name.pop();
                    if !mod_dir.join(&folder_name).exists() {
                        std::fs::create_dir_all(&mod_dir.join(folder_name)).unwrap();
                    }
                    let mut os_f = std::fs::File::create(&mod_dir.join(f.name())).unwrap();
                    std::io::copy(&mut f, &mut os_f).unwrap();
                }
            }
        }
    }

    pub fn host_turn<D>(mut self, db: &D)
    where
        D: diesel::Connection<Backend = diesel::pg::Pg>,
    {
        self.populate_savegame(db);
        self.populate_mods(db);
        self.populate_maps(db);
        self.update_nations(db);
        let mut arguments = {
            let mods: Vec<(GameMod, Mod)> = crate::schema::game_mods::dsl::game_mods
                .filter(crate::schema::game_mods::dsl::game_id.eq(self.game_id))
                .inner_join(
                    crate::schema::mods::dsl::mods
                        .on(crate::schema::mods::dsl::id.eq(crate::schema::game_mods::dsl::mod_id)),
                )
                .get_results::<(GameMod, Mod)>(db)
                .unwrap();
            mods.iter()
                .flat_map(|(_, m)| vec![String::from("-M"), m.dm_filename.clone()])
                .collect::<Vec<String>>()
        };
        use crate::schema::files::dsl as files_dsl;
        use crate::schema::games::dsl as games_dsl;
        use crate::schema::maps::dsl as maps_dsl;
        let (game, _map, file) = games_dsl::games
            .filter(games_dsl::id.eq(self.game_id))
            .inner_join(maps_dsl::maps.on(maps_dsl::id.eq(games_dsl::map_id)))
            .inner_join(files_dsl::files.on(files_dsl::id.eq(maps_dsl::mapfile_id)))
            .get_result::<(Game, Map, File)>(db)
            .unwrap();
        let disciples = crate::models::Disciple::get_all(self.game_id, db).unwrap();
        arguments.append(disciples.into_iter().fold(&mut Vec::new(), |acc, d| {
            acc.append(&mut vec![
                "--team".to_string(),
                d.nation_id.to_string(),
                match d.team {
                    Some(d) => d.to_string(),
                    None => "0".to_string(),
                },
                (1 + d.is_disciple).to_string(),
            ]);
            acc
        }));
        arguments.append(&mut vec![
            "-d".to_string(),
            "--noclientstart".to_string(),
            if game.renaming { "--renaming" } else { "" }.to_string(),
            "--thrones".to_string(),
            game.thrones_t1.to_string(),
            game.thrones_t2.to_string(),
            game.thrones_t3.to_string(),
            "--requiredap".to_string(),
            game.throne_points_required.to_string(),
            "--research".to_string(),
            game.research_diff.to_string(),
            if game.research_rand {
                ""
            } else {
                "--norandres"
            }
            .to_string(),
            "--hofsize".to_string(),
            game.hof_size.to_string(),
            "--globals".to_string(),
            game.global_size.to_string(),
            "--indepstr".to_string(),
            game.indepstr.to_string(),
            "--magicsites".to_string(),
            game.magicsites.to_string(),
            "--eventrarity".to_string(),
            game.eventrarity.to_string(),
            "--richness".to_string(),
            game.richness.to_string(),
            "--resources".to_string(),
            game.resources.to_string(),
            "--supplies".to_string(),
            game.supplies.to_string(),
            "--startprov".to_string(),
            game.startprov.to_string(),
            if game.scoregraphs {
                "--scoregraphs"
            } else {
                ""
            }
            .to_string(),
            if game.nationinfo {
                ""
            } else {
                "--nonationinfo"
            }
            .to_string(),
            "--era".to_string(),
            game.era.to_string(),
            if game.artrest { "" } else { "--noartrest" }.to_string(),
            if game.teamgame { "--teamgame" } else { "" }.to_string(),
            if game.clustered { "--clustered" } else { "" }.to_string(),
            match game.storyevents {
                0 => "--nostoryevents",
                1 => "--storyevents",
                2 => "--allstoryevents",
                _ => "",
            }
            .to_string(),
        ]);
        match game.newailvl {
            0 => arguments.append(&mut vec!["--nonewai".to_string(), "".to_string()]),
            l => arguments.append(&mut vec!["--newailvl".to_string(), l.to_string()]),
        }
        if let Some(masterpass) = game.masterpass.as_ref() {
            arguments.append(&mut vec!["--masterpass".to_string(), masterpass.clone()]);
        }
        use crate::schema::turns::dsl as turns_dsl;
        let turns: Vec<Turn> = turns_dsl::turns
            .filter(
                turns_dsl::game_id
                    .eq(game.id)
                    .and(turns_dsl::archived.eq(false)),
            )
            .get_results(db)
            .unwrap();
        if turns.len() == 0 {
            arguments.append(&mut vec!["--newgame".to_string()]);
        }
        arguments.append(&mut vec![
            String::from("-T"),
            String::from("--mapfile"),
            file.filename,
            String::from("-g"),
            format!("{}", self.name),
        ]);
        let mut child = std::process::Command::new(std::path::PathBuf::from(
            std::env::var("DOM5_BIN")
                .expect("DOM5_BIN not specified")
                .to_string(),
        ))
        .env("DOM5_CONF", &self.datadir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .args(&arguments)
        .spawn()
        .expect(&format!(
            "Failed to launch dom5 binary for game {}",
            self.name
        ));

        let turn_number = turns.last().map(|t| t.turn_number).unwrap_or(0);
        let game_log = NewGameLog {
            game_id: game.id,
            datetime: std::time::SystemTime::now(),
            turn_number,
            output: "",
            error: "",
            log_command: &arguments.join(" "),
        }
        .insert(db)
        .unwrap();
        let stdout = child.stdout.take().unwrap();
        let mut noblock_stdout = NonBlockingReader::from_fd(stdout).unwrap();
        let stderr = child.stderr.take().unwrap();
        let mut noblock_stderr = NonBlockingReader::from_fd(stderr).unwrap();
        let mut waits = 0;
        while !noblock_stdout.is_eof() {
            std::thread::sleep(std::time::Duration::from_secs(1));
            let mut outbuf = String::new();
            noblock_stdout
                .read_available_to_string(&mut outbuf)
                .unwrap();
            let mut outerr = String::new();
            noblock_stderr
                .read_available_to_string(&mut outerr)
                .unwrap();
            if outbuf.len() == 0 && outerr.len() == 0 {
                waits = waits + 1;
                if waits == 240 {
                    outbuf.push_str(&format!(
                        "WARN: Printed nothing for {} seconds, forcefully killing\n",
                        waits
                    ));
                    game_log.update_logs(&outbuf, &outerr, db).unwrap();
                    log::error!(
                        "Turn {} for game {} hung, forcefully killing",
                        turn_number,
                        game.id
                    );
                    child.kill().unwrap();
                    child.wait().unwrap();
                    return;
                } else {
                    outbuf.push_str(&format!(
                        "WARN: Printed nothing for {} seconds, possibly hung\n",
                        waits
                    ));
                }
            } else {
                waits = 0;
            }
            game_log.update_logs(&outbuf, &outerr, db).unwrap();
            if noblock_stdout.is_eof() {
                break;
            }
        }
        child.kill().unwrap();
        child.wait().unwrap();
        self.handle_new_turn(db);
        match game.timer {
            Some(timer) => {
                game.schedule_turn(
                    std::time::SystemTime::now()
                        .add(std::time::Duration::from_secs((60 * timer) as u64)),
                    db,
                )
                .unwrap();
            }
            None => {
                game.remove_timer(db).unwrap();
            }
        }
    }

    pub fn populate_maps<D>(&mut self, db: &D)
    where
        D: diesel::Connection<Backend = diesel::pg::Pg>,
    {
        std::fs::create_dir_all(&std::path::PathBuf::from(&self.datadir).join("maps")).unwrap();
        let (_, map): (Game, Map) = {
            use super::schema::maps::dsl::*;
            super::schema::games::dsl::games
                .filter(super::schema::games::dsl::id.eq(self.game_id))
                .inner_join(maps.on(id.eq(super::schema::games::dsl::map_id)))
                .get_result(db)
                .unwrap()
        };
        let (mut mapfile, tgafile, winterfile) = {
            use super::schema::files::dsl::*;
            (
                files
                    .filter(id.eq(map.mapfile_id))
                    .get_result::<File>(db)
                    .unwrap(),
                files
                    .filter(id.eq(map.tgafile_id))
                    .get_result::<File>(db)
                    .unwrap(),
                map.winterfile_id.map_or(None, |wfid| {
                    Some(files.filter(id.eq(wfid)).get_result::<File>(db).unwrap())
                }),
            )
        };
        let players = Player::get_players(self.game_id, db).unwrap();
        let game = Game::get(self.game_id, db).unwrap();
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
        std::fs::write(
            std::path::PathBuf::from(&self.datadir)
                .join("maps")
                .join(&mapfile.filename),
            mapfile.filebinary,
        )
        .unwrap();
        std::fs::write(
            std::path::PathBuf::from(&self.datadir)
                .join("maps")
                .join(&tgafile.filename),
            tgafile.filebinary,
        )
        .unwrap();
        winterfile.map(|wf| {
            std::fs::write(
                std::path::PathBuf::from(&self.datadir)
                    .join("maps")
                    .join(&wf.filename),
                wf.filebinary,
            )
            .unwrap();
        });
    }
}
