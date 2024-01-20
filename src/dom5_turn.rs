use crate::files::saves::DomSaveReadError;
use crate::models::binary::Binary;
use encoding::{all::ISO_8859_1, DecoderTrap, Encoding};
use nonblock::NonBlockingReader;
use serde::Deserialize;
use serde_dom5::ftherlnd_contents::FtherlndContents;
use serde_dom5::game_settings::GameSettings;
use serde_dom5::{Body, SaveFile};
use std::io::{Read, Seek, Write};
use thiserror::Error;
use serde_dom5::kingdom::KingdomType;

pub struct AutokilledProc(std::process::Child);

impl Drop for AutokilledProc {
    fn drop(&mut self) {
        match self.0.kill() {
            Err(ref e) if e.kind() == std::io::ErrorKind::InvalidInput => {}
            Err(ref e) => {
                log::warn!("Failed to kill process on drop: {:?}", e);
            }
            _ => {}
        }
        match self.0.wait() {
            Err(ref e) => {
                log::warn!("Failed to wait process on drop: {:?}", e);
            }
            _ => {}
        }
    }
}
impl std::ops::Deref for AutokilledProc {
    type Target = std::process::Child;
    fn deref(&self) -> &std::process::Child {
        &self.0
    }
}
impl std::ops::DerefMut for AutokilledProc {
    fn deref_mut(&mut self) -> &mut std::process::Child {
        &mut self.0
    }
}

pub struct Dom5Turn {
    pub db_pool: sqlx::postgres::PgPool,
    pub game_id: i32,
}

#[derive(Debug, Error)]
pub enum Dom5TurnErr {
    #[error("Unclean mod directory")]
    UncleanModDir(std::io::Error),
    #[error("Unclean save directory")]
    UncleanSaveDir(std::io::Error),
    #[error("Unclean map directory")]
    UncleanMapDir(std::io::Error),
    #[error("DB error")]
    DbError(#[from] sqlx::Error),
    #[error("No turns or pretenders")]
    NoTurns,
    #[error("Unexpected Dom5 output")]
    UnexpectedOutput(regex::Error),
    #[error("Error spawning dom5 process")]
    ProcessError(std::io::Error),
    #[error("Hung while processing")]
    TurnHang,
    #[error("Corrupt ftherlnd")]
    CorruptFthrlnd(Box<bincode::ErrorKind>),
    #[error("Missing ftherlnd")]
    MissingFthrlnd,
    #[error("Ftherlnd mixup")]
    FtherlndMixup,
    #[error("Missing trn file")]
    MissingTrn(std::io::Error),
}

impl Dom5Turn {
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
    fn populate_mods(&mut self) -> Result<(), Dom5TurnErr> {
        smol::block_on(async {
            let mod_dir = std::env::current_dir()
                .map_err(Dom5TurnErr::UncleanModDir)?
                .join("tmp")
                .join("games")
                .join(self.game_id.to_string())
                .join("mods");
            for mod_record in sqlx::query!(
                "SELECT f.filebinary
            FROM games g
            INNER JOIN game_mods gm ON gm.game_id=g.id
            INNER JOIN mods m ON m.id=gm.mod_id
            INNER JOIN files f ON f.id=m.file_id
            WHERE g.id=$1",
                self.game_id
            )
            .fetch_all(&self.db_pool)
            .await?
            .into_iter()
            {
                let mut archive =
                    zip::ZipArchive::new(std::io::Cursor::new(mod_record.filebinary)).unwrap();
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
            Ok(())
        })
    }
    fn populate_maps(&mut self) -> Result<(), Dom5TurnErr> {
        smol::block_on(async {
            let mapstuff = sqlx::query!(r#"
                SELECT m.name as "name!", mf.filename as "filename!", mf.filebinary as "filebinary!"
                FROM games g
                INNER JOIN maps_dom6 m ON m.map_id=g.maps_dom6_id
                INNER JOIN map_planes mp ON mp.map_id=m.map_id
                INNER JOIN files mf ON mf.id=mp.map_file_id
                WHERE g.id=$1
                UNION ALL
                SELECT m.name as "name!", mf.filename as "filename!", mf.filebinary as "filebinary!"
                FROM games g
                INNER JOIN maps_dom6 m ON m.map_id=g.maps_dom6_id
                INNER JOIN map_planes mp ON mp.map_id=m.map_id
                INNER JOIN map_plane_images mi ON mi.map_file_id=mp.map_file_id
                INNER JOIN files mf ON mf.id=mi.image_file_id
                WHERE g.id=$1
                "#,
                self.game_id
            ).fetch_all(&self.db_pool).await?;
            let map_dir = std::env::current_dir()
                .map_err(Dom5TurnErr::UncleanModDir)?
                .join("tmp")
                .join("games")
                .join(self.game_id.to_string())
                .join("maps")
                .join(format!("{}.map", &mapstuff.first().unwrap().name));
            if !map_dir.exists() {
                std::fs::create_dir_all(&map_dir).map_err(Dom5TurnErr::UncleanMapDir)?;
            }
            for f in mapstuff {
            std::fs::File::create(&map_dir.join(&f.filename))
                .map_err(Dom5TurnErr::UncleanMapDir)?
                .write_all(&f.filebinary)
                .map_err(Dom5TurnErr::UncleanMapDir)?;
            }
            Ok(())
        })
    }
    fn populate_pretenders(&mut self) -> Result<(), Dom5TurnErr> {
        smol::block_on(async {
            let save_files = sqlx::query!(
                r#"SELECT g.name, filename,filebinary
                FROM games g
                INNER JOIN players p ON p.game_id=g.id
                INNER JOIN files f ON f.id=p.file_id
                WHERE g.id=$1"#,
                self.game_id
            )
            .fetch_all(&self.db_pool)
            .await?;
            if save_files.len() == 0 {
                return Err(Dom5TurnErr::NoTurns);
            }
            let save_dir = std::env::current_dir()
                .map_err(Dom5TurnErr::UncleanModDir)?
                .join("tmp")
                .join("games")
                .join(self.game_id.to_string())
                .join("savedgames")
                .join(&save_files[0].name);
            if save_dir.exists() {
                std::fs::remove_dir_all(&save_dir).map_err(Dom5TurnErr::UncleanSaveDir)?;
            }
            std::fs::create_dir_all(&save_dir).map_err(Dom5TurnErr::UncleanSaveDir)?;
            for pretender in save_files.iter() {
                std::fs::File::create(save_dir.join(&pretender.filename))
                    .map_err(Dom5TurnErr::UncleanSaveDir)?
                    .write_all(&pretender.filebinary)
                    .map_err(Dom5TurnErr::UncleanSaveDir)?;
            }

            Ok(())
        })
    }
    fn populate_saves(&mut self, turn_number: i32) -> Result<(), Dom5TurnErr> {
        smol::block_on(async {
            let ftherlnd = sqlx::query!(
                r#"
                SELECT g.name, f.filename, f.filebinary 
                FROM games g
                INNER JOIN turns t
                ON t.turn_number=$2 AND t.game_id=$1 AND t.archived != TRUE
                INNER JOIN files f ON f.id=t.file_id
                WHERE g.id=$1"#,
                self.game_id,
                turn_number
            )
            .fetch_one(&self.db_pool)
            .await?;
            let save_files = sqlx::query!(
                r#"
                SELECT trn.filename as trnfn, trn.filebinary as trnfb, twoh.filename "twohfn?", twoh.filebinary "twohfb?"
                FROM games g
                INNER JOIN player_turns pt ON pt.turn_number=$2 AND pt.game_id=g.id AND pt.archived!=TRUE
                INNER JOIN files trn ON trn.id=pt.trnfile_id
                LEFT JOIN files twoh ON twoh.id=pt.twohfile_id
                WHERE g.id=$1"#, self.game_id, turn_number).fetch_all(&self.db_pool).await?;
            let save_dir = std::env::current_dir()
                .map_err(Dom5TurnErr::UncleanModDir)?
                .join("tmp")
                .join("games")
                .join(self.game_id.to_string())
                .join("savedgames")
                .join(&ftherlnd.name);
            if save_dir.exists() {
                std::fs::remove_dir_all(&save_dir).map_err(Dom5TurnErr::UncleanSaveDir)?;
            }
            std::fs::create_dir_all(&save_dir).map_err(Dom5TurnErr::UncleanSaveDir)?;
            std::fs::File::create(save_dir.join(&ftherlnd.filename))
                .map_err(Dom5TurnErr::UncleanSaveDir)?
                .write_all(&ftherlnd.filebinary)
                .map_err(Dom5TurnErr::UncleanSaveDir)?;
            for saves in save_files.iter() {
                std::fs::File::create(save_dir.join(&saves.trnfn))
                    .map_err(Dom5TurnErr::UncleanSaveDir)?
                    .write_all(&saves.trnfb)
                    .map_err(Dom5TurnErr::UncleanSaveDir)?;
                if let (Some(twohfn), Some(twohfb)) = (saves.twohfn.as_ref(), saves.twohfb.as_ref())
                {
                    std::fs::File::create(save_dir.join(&twohfn))
                        .map_err(Dom5TurnErr::UncleanSaveDir)?
                        .write_all(&twohfb)
                        .map_err(Dom5TurnErr::UncleanSaveDir)?;
                }
            }
            Ok(())
        })
    }
    fn _update_nations(&mut self, subjob_id: Option<i32>) -> Result<(), Dom5TurnErr> {
        smol::block_on(async {
            let game = sqlx::query!(
                "SELECT g.era, g.name FROM games g WHERE g.id=$1",
                self.game_id
            )
            .fetch_one(&self.db_pool)
            .await?;

            let data_dir = std::env::current_dir()
                .unwrap()
                .join("tmp")
                .join("games")
                .join(self.game_id.to_string());
            let save_dir = std::env::current_dir()
                .map_err(Dom5TurnErr::UncleanModDir)?
                .join("tmp")
                .join("games")
                .join(self.game_id.to_string())
                .join("savedgames")
                .join(&game.name);
            let modnames = sqlx::query!(
                "
            SELECT m.dm_filename
            FROM games g
            INNER JOIN game_mods gm ON gm.game_id=g.id
            INNER JOIN mods m ON m.id=gm.mod_id
            WHERE g.id=$1",
                self.game_id
            )
            .fetch_all(&self.db_pool)
            .await?;
            let mut arguments = vec![
                "--statusdump".to_string(),
                "--port".to_string(),
                format!("{}", Self::find_unused_port().unwrap()),
                "--era".to_string(),
                game.era.to_string(),
                "-T".to_string(),
                "--tcpserver".to_string(),
                "-g".to_string(),
                game.name.clone(),
            ];
            for modname in modnames {
                arguments.push("-M".to_string());
                arguments.push(modname.dm_filename);
            }
            match std::fs::remove_file(save_dir.join("statusdump.txt")) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => panic!("{}", e),
                Ok(_) => {}
            }
            println!("Extracting");
            let path = Binary::extract_to_tmp(self.game_id, &self.db_pool)
                .await
                .unwrap();
            println!("Extracted");
            let mut proc = AutokilledProc(
                std::process::Command::new(path)
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .env("DOM5_CONF", &data_dir)
                    .env("DOM6_CONF", &data_dir)
                    .args(arguments)
                    .spawn()
                    .map_err(Dom5TurnErr::ProcessError)?,
            );
            let statusdump;
            let mut wait_counter: i32 = 10;

            let stdout = proc.stdout.take().unwrap();
            let mut noblock_stdout = NonBlockingReader::from_fd(stdout).unwrap();
            let stderr = proc.stderr.take().unwrap();
            let mut noblock_stderr = NonBlockingReader::from_fd(stderr).unwrap();

            loop {
                if let Some(subjob_id) = subjob_id {
                    let mut outbuf: Vec<u8> = vec![];
                    let mut errbuf: Vec<u8> = vec![];
                    noblock_stdout.read_available(&mut outbuf).unwrap();
                    noblock_stderr.read_available(&mut errbuf).unwrap();
                    let outbuf = match ISO_8859_1.decode(&outbuf, DecoderTrap::Ignore) {
                        Ok(s) => s,
                        Err(e) => format!("\nERROR: Failed to read stdout from Dom5: {:?}", e),
                    };
                    let errbuf = match ISO_8859_1.decode(&errbuf, DecoderTrap::Ignore) {
                        Ok(s) => s,
                        Err(e) => format!("\nERROR: Failed to read stderr from Dom5: {:?}", e),
                    };
                    sqlx::query!("UPDATE subjobs SET logs=concat(logs, $2::text, $3::text) WHERE subjob_id=$1",
                        subjob_id,
                        outbuf,
                        errbuf
                    )
                    .execute(&self.db_pool)
                    .await?;
                }
                match std::fs::File::open(save_dir.join("statusdump.txt")) {
                    Ok(f) => {
                        statusdump = Some(f);
                        break;
                    }
                    Err(_) => {
                        if wait_counter == 0 {
                            panic!("Ran out of waits");
                        } else {
                            wait_counter -= 1
                        }
                    }
                }
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
            let nation_info_regex = regex::Regex::new( r#"Nation\t([0-9]+)\t[0-9]+\t[0-9]+\t[0-9]+\t[0-9]+\t([a-zA-Z_0-9]+)\t([^\t]+)\t([^$]+)"#).unwrap();
            let mut nation_ids = vec![];
            let mut filenames = vec![];
            let mut names = vec![];
            let mut epithets = vec![];
            if let Some(mut statusdump) = statusdump {
                let mut contents = vec![];
                statusdump.read_to_end(&mut contents).unwrap();
                let statusdump_str = String::from_utf8_lossy(&contents);
                if let Some(subjob_id) = subjob_id {
                    sqlx::query!(
                        "UPDATE subjobs SET logs=concat(logs, $2::text) WHERE subjob_id=$1",
                        subjob_id,
                        &*statusdump_str
                    )
                    .execute(&self.db_pool)
                    .await?;
                }
                for l in statusdump_str
                    .lines()
                    .filter(|l| nation_info_regex.is_match(l))
                {
                    if let Some(result) = nation_info_regex.captures(l) {
                        nation_ids.push(
                            result
                                .get(1)
                                .expect("Regex for nation succeeded, but somehow we're missing the nation_id group")
                                .as_str()
                                .parse()
                                .expect("Regex for nation succeeded, but somehow we're missing the nation_id group")
                        );
                        filenames.push(
                            result
                                .get(2)
                                .expect("Regex for nation succeeded, but somehow we're missing the filename group")
                                .as_str()
                                .to_owned(),
                        );
                        names.push(
                            result
                                .get(3)
                                .expect("Regex for nation succeeded, but somehow we're missing the name group")
                                .as_str()
                                .to_owned(),
                        );
                        epithets.push(
                            result
                                .get(4)
                                .expect("Regex for nation succeeded, but somehow we're missing the epithet group")
                                .as_str()
                                .to_owned(),
                        );
                    }
                }
            }
            sqlx::query!(r#"
                        INSERT 
                        INTO nations 
                            (game_id, nation_id, name, epithet, filename) 
                        SELECT $1, UNNEST($2::integer[]),  UNNEST($3::text[]), UNNEST($4::text[]), UNNEST($5::text[])
                        ON CONFLICT (game_id, nation_id)
                        DO UPDATE SET name=EXCLUDED.name, epithet=EXCLUDED.epithet, filename=EXCLUDED.filename
                        "#,
            self.game_id, &nation_ids, &names, &epithets, &filenames).execute(&self.db_pool).await?;
            proc.kill().map_err(Dom5TurnErr::ProcessError)?;
            proc.wait().map_err(Dom5TurnErr::ProcessError)?;
            Ok(())
        })
    }

    pub fn update_nations(mut self) -> Result<(), Dom5TurnErr> {
        self.populate_mods()?;
        self._update_nations(None)?;
        Ok(())
    }
    pub fn run(mut self, job_id: i32) -> Result<(), Dom5TurnErr> {
        smol::block_on(async {
            let job = sqlx::query!("SELECT * FROM jobs WHERE job_id=$1", job_id)
                .fetch_one(&self.db_pool)
                .await?;

            let subjob = sqlx::query!(
                "INSERT INTO subjobs (job_id, label) VALUES ($1, 'populate saves/pretenders') RETURNING *",
                job_id
            )
            .fetch_one(&self.db_pool)
            .await?;
            match job.turn_number {
                0 => self.populate_pretenders()?,
                tt => self.populate_saves(tt)?,
            }
            let subjob = sqlx::query!(
                "
                WITH _0 as (UPDATE subjobs SET finished_at=NOW() WHERE subjob_id=$2),
                     _1 as (UPDATE jobs SET updated_at=NOW() WHERE job_id=$1)
                INSERT INTO subjobs (job_id, label) VALUES ($1, 'populate maps') RETURNING *",
                job_id,
                subjob.subjob_id
            )
            .fetch_one(&self.db_pool)
            .await?;
            self.populate_maps()?;
            let subjob = sqlx::query!(
                "
                WITH _0 as (UPDATE subjobs SET finished_at=NOW() WHERE subjob_id=$2),
                     _1 as (UPDATE jobs SET updated_at=NOW() WHERE job_id=$1)
                INSERT INTO subjobs (job_id, label) VALUES ($1, 'populate mods') RETURNING *",
                job_id,
                subjob.subjob_id
            )
            .fetch_one(&self.db_pool)
            .await?;
            self.populate_mods()?;
            let subjob = sqlx::query!(
                "
                WITH _0 as (UPDATE subjobs SET finished_at=NOW() WHERE subjob_id=$2),
                     _1 as (UPDATE jobs SET updated_at=NOW() WHERE job_id=$1)
                INSERT INTO subjobs (job_id, label) VALUES ($1, 'update nations') RETURNING *",
                job_id,
                subjob.subjob_id
            )
            .fetch_one(&self.db_pool)
            .await?;
            self._update_nations(Some(subjob.subjob_id))?;
            let subjob = sqlx::query!("
                WITH _0 as (UPDATE subjobs SET finished_at=NOW() WHERE subjob_id=$2),
                     _1 as (UPDATE jobs SET updated_at=NOW() WHERE job_id=$1)
                INSERT INTO subjobs (job_id, label) VALUES ($1, 'dom5 processing turn') RETURNING *", job_id, subjob.subjob_id).fetch_one(&self.db_pool).await?;
            let game = sqlx::query!(
                r#"SELECT g.*,MAX(t.turn_number) as "turn?",m.name as mapfilename FROM games g
                                    LEFT JOIN turns t ON t.game_id=g.id AND t.archived != TRUE
                                    LEFT JOIN maps_dom6 m ON g.maps_dom6_id=m.map_id
                                    WHERE g.id=$1 GROUP BY g.id, m.name"#,
                self.game_id
            )
            .fetch_one(&self.db_pool)
            .await?;
            let disciples = sqlx::query!(
                "SELECT
                is_disciple, team, nation_id
                FROM disciples
                WHERE game_id=$1",
                self.game_id
            )
            .fetch_all(&self.db_pool)
            .await?;
            let modnames = sqlx::query!(
                "
                SELECT m.dm_filename
                FROM games g
                INNER JOIN game_mods gm ON gm.game_id=g.id
                INNER JOIN mods m ON m.id=gm.mod_id
                WHERE g.id=$1",
                self.game_id
            )
            .fetch_all(&self.db_pool)
            .await?;
            let mut arguments = vec![];
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
                if game.conq_all { "--conqall" } else { "" }.to_string(),
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
                "--recruitment".to_string(),
                game.recruitment.to_string(),
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
                "-T".to_owned(),
                "--mapfile".to_owned(),
                format!("{}.map", game.mapfilename),
                "--noundercaves".to_owned(),
                "-g".to_owned(),
                game.name.clone(),
            ]);
            match game.newailvl {
                0 => arguments.append(&mut vec!["--nonewai".to_string()]),
                l => arguments.append(&mut vec!["--newailvl".to_string(), l.to_string()]),
            }
            if let Some(c) = game.cataclysm {
                arguments.append(&mut vec!["--cataclysm".to_string(), format!("{}", c)]);
            }
            if let Some(masterpass) = game.masterpass.as_ref() {
                arguments.append(&mut vec!["--masterpass".to_string(), masterpass.clone()]);
            }

            for modname in modnames {
                arguments.push("-M".to_string());
                arguments.push(modname.dm_filename);
            }
            if let None = game.turn {
                arguments.append(&mut vec!["--newgame".to_string()]);
            }
            println!("Extracting");
            let path = Binary::extract_to_tmp(self.game_id, &self.db_pool)
                .await
                .unwrap();
            println!("Extracted");
            let mut child = AutokilledProc(
                std::process::Command::new(path)
                    .env(
                        "DOM5_CONF",
                        std::env::current_dir()
                            .unwrap()
                            .join("tmp")
                            .join("games")
                            .join(self.game_id.to_string()),
                    )
                    .env(
                        "DOM6_CONF",
                        std::env::current_dir()
                            .unwrap()
                            .join("tmp")
                            .join("games")
                            .join(self.game_id.to_string()),
                    )
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .args(&arguments)
                    .spawn()
                    .map_err(Dom5TurnErr::ProcessError)?,
            );

            let stdout = child.stdout.take().unwrap();
            let mut noblock_stdout = NonBlockingReader::from_fd(stdout).unwrap();
            let stderr = child.stderr.take().unwrap();
            let mut noblock_stderr = NonBlockingReader::from_fd(stderr).unwrap();
            let mut waits: i32 = 0;
            while !noblock_stdout.is_eof() {
                std::thread::sleep(std::time::Duration::from_secs(1));
                let mut outbuf: Vec<u8> = vec![];
                let mut errbuf: Vec<u8> = vec![];
                noblock_stdout.read_available(&mut outbuf).unwrap();
                noblock_stderr.read_available(&mut errbuf).unwrap();
                let mut outbuf = match ISO_8859_1.decode(&outbuf, DecoderTrap::Ignore) {
                    Ok(s) => s,
                    Err(e) => format!("\nERROR: Failed to read output from Dom5: {:?}", e),
                };
                let errbuf = match ISO_8859_1.decode(&errbuf, DecoderTrap::Ignore) {
                    Ok(s) => s,
                    Err(e) => format!("\nERROR: Failed to read output from Dom5: {:?}", e),
                };
                if outbuf.len() == 0 && errbuf.len() == 0 {
                    waits = waits + 1;
                    if waits == 240 {
                        outbuf.push_str(&format!(
                            "WARN: Printed nothing for {} seconds, forcefully killing\n",
                            waits
                        ));
                        log::error!(
                            "Turn {} for game {} hung, forcefully killing",
                            job.turn_number,
                            game.id
                        );
                        sqlx::query!(
                            "WITH _ as (UPDATE jobs SET updated_at=NOW() WHERE job_id=$1)
                            UPDATE subjobs SET logs=concat(logs, $3::text, $4::text) WHERE subjob_id=$2
                            ",
                            job_id,
                            subjob.subjob_id,
                            outbuf,
                            errbuf
                        )
                        .execute(&self.db_pool)
                        .await?;
                        child.kill().unwrap();
                        child.wait().unwrap();
                        return Err(Dom5TurnErr::TurnHang);
                    } else {
                        outbuf.push_str(&format!(
                            "WARN: Printed nothing for {} seconds, possibly hung\n",
                            waits
                        ));
                    }
                } else {
                    waits = 0;
                }
                sqlx::query!("WITH _ as (UPDATE jobs SET updated_at=NOW() WHERE job_id=$1) UPDATE subjobs SET logs=concat(logs, $3::text, $4::text) WHERE subjob_id=$2",
                    job_id,
                    subjob.subjob_id,
                    outbuf,
                    errbuf
                )
                .execute(&self.db_pool)
                .await?;

                if noblock_stdout.is_eof() {
                    break;
                }
            }
            let subjob = sqlx::query!(
                "
                WITH _0 as (UPDATE subjobs SET finished_at=NOW() WHERE subjob_id=$2),
                     _1 as (UPDATE jobs SET updated_at=NOW() WHERE job_id=$1)
                INSERT INTO subjobs (job_id, label) VALUES ($1, 'commit new turn') RETURNING *",
                job_id,
                subjob.subjob_id
            )
            .fetch_one(&self.db_pool)
            .await?;
            let mut transaction = self.db_pool.begin().await?;
            let save_dir = std::env::current_dir()
                .unwrap()
                .join("tmp")
                .join("games")
                .join(self.game_id.to_string())
                .join("savedgames")
                .join(&game.name);
            let mut contents = Vec::new();
            let mut file = if let Ok(f) = std::fs::File::open(save_dir.join("ftherlnd")) {
                f
            } else {
                return Err(Dom5TurnErr::MissingFthrlnd);
            };
            file.read_to_end(&mut contents).unwrap();
            file.seek(std::io::SeekFrom::Start(0)).unwrap();
            let ftherlnd: SaveFile =
                bincode::deserialize_from(&file).map_err(Dom5TurnErr::CorruptFthrlnd)?;
            let (winner, kingdoms) = match ftherlnd.body {
                Body::FtherlndContents(ref fth) => (
                    match fth.get_settings().winner() {
                        -1 => None,
                        0 => None,
                        w => Some(w as i32),
                    },
                    fth.get_kingdoms(),
                ),
                _ => return Err(Dom5TurnErr::FtherlndMixup),
            };
            let turn_number = ftherlnd.header.turnnumber;
            sqlx::query!(
                r#"
            WITH inserted AS (INSERT INTO files (filename, filebinary) 
                VALUES('ftherlnd', $1) 
            ON CONFLICT (filename,digest(filebinary, 'sha1'::text)) DO UPDATE 
                SET filename=EXCLUDED.filename RETURNING *)
            INSERT INTO turns (game_id, turn_number, file_id, turnkey, winner)
            SELECT $2, $3, id, $4, $5 FROM inserted"#,
                &contents,
                self.game_id,
                turn_number,
                ftherlnd.header.turnkey as i32,
                winner
            )
            .execute(&mut *transaction)
            .await?;

            let nations = sqlx::query!("SELECT * FROM nations WHERE game_id=$1", self.game_id)
                .fetch_all(&mut *transaction)
                .await?;
            let mut computer_nationids = vec![];
            let mut defeated_nationids = vec![];
            for (nation_id, kingdom) in kingdoms {
                match kingdom.kingdom_type {
                    // Player_turns which should be inserted with files
                    KingdomType::DefeatedWithTurn | KingdomType::Human => {
                        let status = match kingdom.kingdom_type {
                            KingdomType::DefeatedWithTurn => 5,
                            KingdomType::Human => 0,
                            _ => panic!("Impossible to reach defeated & human branch without being defeated or human")
                        };
                        let nation = nations
                            .iter()
                            .find(|n| n.nation_id == *nation_id as i32)
                            .unwrap();
                        let mut player_turn_file =
                            std::fs::File::open(save_dir.join(format!("{}.trn", nation.filename)))
                                .map_err(Dom5TurnErr::MissingTrn)?;
                        let mut contents = vec![];
                        player_turn_file
                            .read_to_end(&mut contents)
                            .map_err(Dom5TurnErr::MissingTrn)?;
                        sqlx::query!(
                            "
                        WITH inserted AS (INSERT INTO files (filename, filebinary) 
                            VALUES($1, $2) 
                        ON CONFLICT (filename,digest(filebinary, 'sha1'::text)) DO UPDATE 
                            SET filebinary=EXCLUDED.filebinary RETURNING *)
                        INSERT INTO player_turns
                        (trnfile_id, nation_id, game_id, turn_number, status)
                        SELECT id, $3, $4, $5, $6 FROM inserted",
                            &nation.filename,
                            &contents,
                            *nation_id as i32,
                            self.game_id,
                            turn_number,
                            status
                        )
                        .execute(&mut *transaction)
                        .await?;
                    }
                    // Player_turns which should be inserted without files (in bulk, later on)
                    KingdomType::Computer => computer_nationids.push(*nation_id as i32),
                    KingdomType::Defeated => defeated_nationids.push(*nation_id as i32),
                    // Kingdom types we shall ignore
                    KingdomType::Special => {}
                    KingdomType::Unknown => {}
                }
            }
            // Bulk insert player_turns without files
            sqlx::query!(
                "
            WITH inserted AS (INSERT INTO player_turns
            (status, game_id, nation_id, turn_number)
            SELECT 3, $1, UNNEST($3::integer[]), $2)
            INSERT INTO player_turns
            (status, game_id, nation_id, turn_number)
            SELECT 4, $1, UNNEST($4::integer[]), $2;
            ",
                self.game_id,
                turn_number,
                &computer_nationids,
                &defeated_nationids
            )
            .execute(&mut *transaction)
            .await?;

            if winner.is_none() {
                sqlx::query!("UPDATE games SET next_turn=NOW()+ timer * interval '1 minute' WHERE id=$1 AND timer IS NOT NULL", self.game_id).execute(&mut *transaction).await?;
            } else {
                sqlx::query!(
                    "UPDATE games SET next_turn=NULL WHERE id=$1 AND timer IS NOT NULL",
                    self.game_id
                )
                .execute(&mut *transaction)
                .await?;
            }
            transaction.commit().await?;

            sqlx::query!(
                "
                WITH _0 as (UPDATE subjobs SET finished_at=NOW() WHERE subjob_id=$2)
                UPDATE jobs SET finished_at=NOW(), updated_at=NOW() WHERE job_id=$1",
                job_id,
                subjob.subjob_id
            )
            .execute(&self.db_pool)
            .await?;

            Ok(())
        })
    }
}
