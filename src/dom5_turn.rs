use crate::files::MapFile;
use crate::files::saves::DomSaveReadError;
use crate::models::binary::Binary;
use crate::models::maps::{Plane, PlaneImage};
use crate::models::{Map, MapWrapping};
use crate::util::calculate_crc;
use encoding::{DecoderTrap, Encoding, all::ISO_8859_1};
use itertools::Itertools;
use serde::Deserialize;
use serde_dom5::ftherlnd_contents::FtherlndContents;
use serde_dom5::game_settings::GameSettings;
use serde_dom5::kingdom::KingdomType;
use serde_dom5::{Body, SaveFile};
use std::convert::TryFrom;
use std::io::{Read, Seek, Write, ErrorKind};
use std::process::{ChildStdout, ChildStderr};
use thiserror::Error;

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

// Cross-platform non-blocking reader for process output
struct NonBlockingReader<T: Read> {
    stream: T,
    eof: bool,
}

impl NonBlockingReader<ChildStdout> {
    fn from_stdout(stream: ChildStdout) -> Self {
        // Set non-blocking mode
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            unsafe {
                let fd = stream.as_raw_fd();
                let flags = libc::fcntl(fd, libc::F_GETFL, 0);
                libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
            }
        }

        #[cfg(windows)]
        {
            // On Windows, we need to use a different approach for non-blocking reads
            // We'll handle this in the read_available method by using timeouts
        }

        NonBlockingReader { stream, eof: false }
    }
}

impl NonBlockingReader<ChildStderr> {
    fn from_stderr(stream: ChildStderr) -> Self {
        // Set non-blocking mode
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            unsafe {
                let fd = stream.as_raw_fd();
                let flags = libc::fcntl(fd, libc::F_GETFL, 0);
                libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
            }
        }

        #[cfg(windows)]
        {
            // On Windows, we need to use a different approach for non-blocking reads
            // We'll handle this in the read_available method by using timeouts
        }

        NonBlockingReader { stream, eof: false }
    }
}

impl<T: Read> NonBlockingReader<T> {
    fn read_available(&mut self, out: &mut Vec<u8>) -> Result<(), std::io::Error> {
        let mut buf = [0u8; 8192];
        loop {
            match self.stream.read(&mut buf) {
                Ok(0) => {
                    self.eof = true;
                    break;
                }
                Ok(n) => {
                    out.extend_from_slice(&buf[..n]);
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::Interrupted => {
                    // No more data available right now
                    break;
                }
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    fn is_eof(&self) -> bool {
        self.eof
    }
}

pub struct Dom5Turn {
    pub db_pool: sqlx::sqlite::SqlitePool,
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
    #[error("Turn already reaped")]
    AlreadyReaped,
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
    async fn populate_mods(&mut self) -> Result<(), Dom5TurnErr> {
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
    }
    async fn populate_maps(&mut self) -> Result<(), Dom5TurnErr> {
            let mapstuff = sqlx::query!(
                r#"
                SELECT g.name as "game_name!", m.is_randommap "is_randommap!", m.name as "name!", mf.filename as "filename!", mf.filebinary as "filebinary!"
                FROM games g
                INNER JOIN maps_dom6 m ON m.map_id=g.maps_dom6_id
                INNER JOIN map_planes mp ON mp.map_id=m.map_id
                INNER JOIN files mf ON mf.id=mp.map_file_id
                WHERE g.id=$1
                UNION ALL
                SELECT g.name as "game_name!", m.is_randommap "is_randommap!", m.name as "name!", mf.filename as "filename!", mf.filebinary as "filebinary!"
                FROM games g
                INNER JOIN maps_dom6 m ON m.map_id=g.maps_dom6_id
                INNER JOIN map_planes mp ON mp.map_id=m.map_id
                INNER JOIN map_plane_images mi ON mi.map_file_id=mp.map_file_id
                INNER JOIN files mf ON mf.id=mi.image_file_id
                WHERE g.id=$1
                "#,
                self.game_id
            )
            .fetch_all(&self.db_pool)
            .await?;
            let mut map_dir = std::env::current_dir()
                .map_err(Dom5TurnErr::UncleanModDir)?
                .join("tmp")
                .join("games");
            if let Some(firstmapfile) = mapstuff.first() {
                if firstmapfile.is_randommap {
                    map_dir = map_dir
                        .join(self.game_id.to_string())
                        .join("savedgames")
                        .join(&firstmapfile.game_name);
                } else {
                    map_dir = map_dir
                        .join(self.game_id.to_string())
                        .join("maps")
                        .join(&firstmapfile.filename);
                }
                if !map_dir.exists() {
                    std::fs::create_dir_all(&map_dir).map_err(Dom5TurnErr::UncleanMapDir)?;
                }
            } else {
                return Ok(());
            }
            for f in mapstuff {
                std::fs::File::create(&map_dir.join(&f.filename))
                    .map_err(Dom5TurnErr::UncleanMapDir)?
                    .write_all(&f.filebinary)
                    .map_err(Dom5TurnErr::UncleanMapDir)?;
            }
            Ok(())
    }
    async fn populate_pretenders(&mut self) -> Result<(), Dom5TurnErr> {
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
    }
    async fn populate_saves(&mut self, turn_number: i32) -> Result<(), Dom5TurnErr> {
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
    }
    async fn _update_nations(&mut self, subjob_id: Option<i32>) -> Result<(), Dom5TurnErr> {
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
            let path = Binary::extract_to_tmp(self.game_id, &self.db_pool)
                .await
                .unwrap();
            if let Some(subjob_id) = subjob_id {
                let args = arguments.join(" ");
                sqlx::query!(
                    "UPDATE subjobs SET meta=$1 WHERE subjob_id=$2",
                    args,
                    subjob_id
                )
                .execute(&self.db_pool)
                .await
                .unwrap();
            }
            println!("Starting dom6 to scrape nations");
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

            let mut stdout = proc.stdout.take().unwrap();
            let mut stderr = proc.stderr.take().unwrap();

            loop {
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
            let mut nation_ids: Vec<i32> = vec![];
            let mut filenames = vec![];
            let mut names = vec![];
            let mut epithets = vec![];
            if let Some(mut statusdump) = statusdump {
                let mut contents = vec![];
                statusdump.read_to_end(&mut contents).unwrap();
                let statusdump_str = String::from_utf8_lossy(&contents);
                if let Some(subjob_id) = subjob_id {
                    let mut tx = self.db_pool.begin().await?;
                    sqlx::query!(
                        "UPDATE subjobs SET logs=concat(logs, $2::text) WHERE subjob_id=$1",
                        subjob_id,
                        statusdump_str
                    )
                    .execute(&mut *tx)
                    .await?;
                    tx.commit().await?;
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
            // Insert/update nations individually since SQLite doesn't support array operations
            for i in 0..nation_ids.len() {
                sqlx::query!(r#"
                    INSERT INTO nations (game_id, nation_id, name, epithet, filename)
                    VALUES ($1, $2, $3, $4, $5)
                    ON CONFLICT (game_id, nation_id)
                    DO UPDATE SET name=EXCLUDED.name, epithet=EXCLUDED.epithet, filename=EXCLUDED.filename
                "#,
                self.game_id, nation_ids[i], names[i], epithets[i], filenames[i]).execute(&self.db_pool).await?;
            }
            println!("Killing process");
            proc.kill().map_err(Dom5TurnErr::ProcessError)?;
            println!("Reaping process");
            proc.wait().map_err(Dom5TurnErr::ProcessError)?;
            println!("Reaped process");

            // Now collect any remaining logs from the process (after it's been killed)
            if let Some(subjob_id) = subjob_id {
                let mut outbuf = Vec::new();
                let mut errbuf = Vec::new();
                println!("Reading from stdout and stderr after process killed");
                // Read whatever is available in the pipes after process termination
                let _ = stdout.read_to_end(&mut outbuf);
                let _ = stderr.read_to_end(&mut errbuf);
                println!("Read from stdout and stderr after process killed");
                let outbuf = match ISO_8859_1.decode(&outbuf, DecoderTrap::Ignore) {
                    Ok(s) => s,
                    Err(e) => format!("\nERROR: Failed to read stdout from Dom5: {:?}", e),
                };
                let errbuf = match ISO_8859_1.decode(&errbuf, DecoderTrap::Ignore) {
                    Ok(s) => s,
                    Err(e) => format!("\nERROR: Failed to read stderr from Dom5: {:?}", e),
                };
                let mut tx = self.db_pool.begin().await?;
                sqlx::query!("UPDATE subjobs SET logs=concat(logs, $2::text, $3::text) WHERE subjob_id=$1",
                    subjob_id,
                    outbuf,
                    errbuf
                )
                .execute(&mut *tx)
                .await?;
                tx.commit().await?;
            }

            Ok(())
    }

    pub async fn update_nations(mut self) -> Result<(), Dom5TurnErr> {
        self.populate_mods().await?;
        self._update_nations(None).await?;
        Ok(())
    }
    pub async fn run(mut self, job_id: i32) -> Result<(), Dom5TurnErr> {
            let job = sqlx::query!(
                r#"SELECT turn_number "turn_number: i32" FROM jobs WHERE job_id=$1"#,
                job_id
            )
            .fetch_one(&self.db_pool)
            .await?;

            let subjob = sqlx::query!(
                r#"INSERT INTO subjobs (job_id, label) VALUES ($1, 'populate saves/pretenders') RETURNING subjob_id "subjob_id: i32""#,
                job_id
            )
            .fetch_one(&self.db_pool)
            .await?;
            match job.turn_number {
                0 => self.populate_pretenders().await?,
                tt => self.populate_saves(tt).await?,
            }
            // Update previous subjob and job with explicit transaction
            let mut tx = self.db_pool.begin().await?;
            sqlx::query!(
                "UPDATE subjobs SET finished_at=datetime('now') WHERE subjob_id=$1",
                subjob.subjob_id
            )
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            sqlx::query!(
                "UPDATE jobs SET updated_at=datetime('now') WHERE job_id=$1",
                job_id
            )
            .execute(&self.db_pool)
            .await?;

            // Insert new subjob
            let subjob = sqlx::query!(
                "INSERT INTO subjobs (job_id, label) VALUES ($1, 'populate maps') RETURNING *",
                job_id
            )
            .fetch_one(&self.db_pool)
            .await?;
            self.populate_maps().await?;
            // Update previous subjob and job with explicit transaction
            let mut tx = self.db_pool.begin().await?;
            sqlx::query!(
                "UPDATE subjobs SET finished_at=datetime('now') WHERE subjob_id=$1",
                subjob.subjob_id
            )
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            sqlx::query!(
                "UPDATE jobs SET updated_at=datetime('now') WHERE job_id=$1",
                job_id
            )
            .execute(&self.db_pool)
            .await?;

            // Insert new subjob
            let subjob = sqlx::query!(
                "INSERT INTO subjobs (job_id, label) VALUES ($1, 'populate mods') RETURNING *",
                job_id
            )
            .fetch_one(&self.db_pool)
            .await?;
            self.populate_mods().await?;
            // Update previous subjob and job with explicit transaction
            let mut tx = self.db_pool.begin().await?;
            sqlx::query!(
                "UPDATE subjobs SET finished_at=datetime('now') WHERE subjob_id=$1",
                subjob.subjob_id
            )
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            sqlx::query!(
                "UPDATE jobs SET updated_at=datetime('now') WHERE job_id=$1",
                job_id
            )
            .execute(&self.db_pool)
            .await?;

            // Insert new subjob
            let subjob = sqlx::query!(
                r#"INSERT INTO subjobs (job_id, label) VALUES ($1, 'update nations') RETURNING subjob_id "subjob_id: i32""#,
                job_id
            )
            .fetch_one(&self.db_pool)
            .await?;
            self._update_nations(Some(subjob.subjob_id)).await?;
            // Update previous subjob and job with explicit transaction
            let mut tx = self.db_pool.begin().await?;
            sqlx::query!(
                "UPDATE subjobs SET finished_at=datetime('now') WHERE subjob_id=$1",
                subjob.subjob_id
            )
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            log::debug!("Nation update subjob committed for game {}", self.game_id);
            // Force WAL checkpoint to ensure changes are visible on Windows
            sqlx::raw_sql("PRAGMA wal_checkpoint(TRUNCATE)")
                .execute(&self.db_pool)
                .await?;
            log::debug!("WAL checkpoint completed for game {}", self.game_id);
            sqlx::query!(
                "UPDATE jobs SET updated_at=datetime('now') WHERE job_id=$1",
                job_id
            )
            .execute(&self.db_pool)
            .await?;

            // Insert new subjob
            let subjob = sqlx::query!("INSERT INTO subjobs (job_id, label) VALUES ($1, 'dom5 processing turn') RETURNING *", job_id).fetch_one(&self.db_pool).await?;
            let game = sqlx::query!(
                r#"SELECT
                    g.log_level "log_level!",
                    g.renaming "renaming!",
                    g.thrones_t1 "thrones_t1!: i32",
                    g.thrones_t2 "thrones_t2!: i32",
                    g.thrones_t3 "thrones_t3!: i32",
                    g.conq_all "conq_all!",
                    g.throne_points_required "throne_points_required!: i32",
                    g.research_diff "research_diff!: i32",
                    g.research_rand "research_rand!",
                    g.hof_size "hof_size!: i32",
                    g.global_size "global_size!: i32",
                    g.mercslots "mercslots!: i32",
                    g.cheat_detection "cheat_detection!",
                    g.indepstr "indepstr!: i32",
                    g.magicsites "magicsites!: i32",
                    g.eventrarity "eventrarity!: i32",
                    g.richness "richness!: i32",
                    g.resources "resources!: i32",
                    g.recruitment "recruitment!: i32",
                    g.supplies "supplies!: i32",
                    g.startprov "startprov!: i32",
                    g.research_9_slow "research_9_slow!",
                    g.formal_diplo "formal_diplo!: i32",
                    g.scoregraphs "scoregraphs!",
                    g.nationinfo "nationinfo!",
                    g.era "era!: i32",
                    g.artrest "artrest!",
                    g.teamgame "teamgame!",
                    g.clustered "clustered!",
                    g.storyevents "storyevents!: i32",
                    g.name "name!",
                    g.provs_per_player "provs_per_player!: i32",
                    g.randommap_wrapping "randommap_wrapping!: i32",
                    g.newailvl "newailvl!: i32",
                    g.cataclysm "cataclysm?: i32",
                    g.masterpass "masterpass",
                    g.borked "borked!",
                    g.id "id!: i32",
                    MAX(t.turn_number) as "turn?",
                    f.filename as "mapfilename?"
                FROM games g
                    LEFT JOIN turns t ON t.game_id=g.id AND t.archived != TRUE
                    LEFT JOIN maps_dom6 m ON g.maps_dom6_id=m.map_id
                    LEFT JOIN map_planes mp ON mp.map_id=m.map_id
                    LEFT JOIN files f ON f.id=mp.map_file_id
                WHERE g.id=$1
                GROUP BY g.id, f.filename LIMIT 1"#,
                self.game_id
            )
            .fetch_one(&self.db_pool)
            .await
            .unwrap();
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
                match game.log_level {
                    0 => "".to_string(),
                    1 => "-d".to_string(),
                    2 => "-dd".to_string(),
                    3 => "-ddd".to_string(),
                    4 => "-dddd".to_string(),
                    _ => "".to_string(),
                },
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
                "--mercsize".to_string(),
                game.mercslots.to_string(),
                if !game.cheat_detection {
                    "--nocheatdet".to_string()
                } else {
                    "".to_string()
                },
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
                if !game.research_9_slow {
                    println!("Setting no rest");
                    "--nolvl9rest"
                } else {
                    println!("Not setting no rest");
                    ""
                }
                .to_owned(),
                match game.formal_diplo {
                    0 => "--nodiplo",
                    1 => "--weakdiplo",
                    _ => "",
                }
                .to_owned(),
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
                "-g".to_owned(),
                game.name.clone(),
            ]);
            match game.mapfilename.as_ref() {
                Some(mapfilename) => arguments.append(&mut vec![
                    "--mapfile".to_owned(),
                    mapfilename.to_string(),
                    "--noundercaves".to_owned(),
                ]),
                None => {
                    arguments.append(&mut vec![
                        "--randmap".to_string(),
                        format!("{}", game.provs_per_player),
                    ]);
                    arguments.append(&mut match Into::<_>::into(game.randommap_wrapping) {
                        MapWrapping::None => vec!["--nohwrap".to_owned()],
                        MapWrapping::Both => vec!["--vwrap".to_owned()],
                        MapWrapping::Vertical => vec!["--vwrap".to_owned(), "--nohwrap".to_owned()],
                        MapWrapping::Horizontal => vec![],
                    });
                }
            }
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
            if game.borked {
                arguments.append(&mut vec!["--repair".to_string()]);
            }
            if let None = game.turn {
                arguments.append(&mut vec!["--newgame".to_string()]);
            }
            let path = Binary::extract_to_tmp(self.game_id, &self.db_pool)
                .await
                .unwrap();
            let args = arguments.iter().join(" ");
            sqlx::query!(
                "UPDATE subjobs SET meta=$1 WHERE subjob_id=$2",
                args,
                subjob.subjob_id
            )
            .execute(&self.db_pool)
            .await
            .unwrap();
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
            let mut noblock_stdout = NonBlockingReader::from_stdout(stdout);
            let stderr = child.stderr.take().unwrap();
            let mut noblock_stderr = NonBlockingReader::from_stderr(stderr);
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
                            "UPDATE jobs SET updated_at=datetime('now') WHERE job_id=$1",
                            job_id
                        )
                        .execute(&self.db_pool)
                        .await
                        .map_err(|e| Dom5TurnErr::DbError(e))?;
                        sqlx::query!(
                            "UPDATE subjobs SET logs=logs || $3 || $4 WHERE subjob_id=$2",
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
                sqlx::query!(
                    "UPDATE jobs SET updated_at=datetime('now') WHERE job_id=$1",
                    job_id
                )
                .execute(&self.db_pool)
                .await?;
                sqlx::query!(
                    "UPDATE subjobs SET logs=logs || $3 || $4 WHERE subjob_id=$2",
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
            // Update previous subjob and job
            sqlx::query!(
                "UPDATE subjobs SET finished_at=datetime('now') WHERE subjob_id=$1",
                subjob.subjob_id
            )
            .execute(&self.db_pool)
            .await?;
            sqlx::query!(
                "UPDATE jobs SET updated_at=datetime('now') WHERE job_id=$1",
                job_id
            )
            .execute(&self.db_pool)
            .await?;

            // Insert new subjob
            let subjob = sqlx::query!(
                "INSERT INTO subjobs (job_id, label) VALUES ($1, 'commit new turn') RETURNING *",
                job_id
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
            if game.mapfilename.is_none() {
                let abovemapfilenanme = format!("__randommap_{}.map", game.name);
                let aboved6mfilenanme = format!("__randommap_{}.d6m", game.name);
                let undermapfilenanme = format!("__under_{}.map", game.name);
                let underd6mfilenanme = format!("__under_{}.d6m", game.name);
                if let (
                    Ok(mut abovegroundmap),
                    Ok(mut undergroundmap),
                    Ok(mut abovegroundd6m),
                    Ok(mut undergroundd6m),
                ) = (
                    std::fs::File::open(save_dir.join(&abovemapfilenanme)),
                    std::fs::File::open(save_dir.join(&undermapfilenanme)),
                    std::fs::File::open(save_dir.join(&aboved6mfilenanme)),
                    std::fs::File::open(save_dir.join(&underd6mfilenanme)),
                ) {
                    let mut mapbuf = vec![];
                    abovegroundmap.read_to_end(&mut mapbuf).unwrap();
                    let abovemap = MapFile::try_from(&mapbuf[..]).unwrap();
                    let mut underbuf = vec![];
                    undergroundmap.read_to_end(&mut underbuf).unwrap();
                    let mut abovebytes = vec![];
                    abovegroundd6m.read_to_end(&mut abovebytes).unwrap();
                    let mut underbytes = vec![];
                    undergroundd6m.read_to_end(&mut underbytes).unwrap();

                    let map = crate::models::maps::Map {
                        is_randommap: true,
                        name: abovemap.map_name.clone(),
                        planes: vec![
                            Plane {
                                name: abovemapfilenanme,
                                hash: calculate_crc(&mapbuf[..]),
                                bytes: mapbuf,
                                images: vec![PlaneImage {
                                    filename: aboved6mfilenanme,
                                    hash: calculate_crc(&abovebytes[..]),
                                    bytes: abovebytes,
                                }],
                            },
                            Plane {
                                name: undermapfilenanme,
                                hash: calculate_crc(&underbuf[..]),
                                bytes: underbuf,
                                images: vec![PlaneImage {
                                    filename: underd6mfilenanme,
                                    hash: calculate_crc(&underbytes[..]),
                                    bytes: underbytes,
                                }],
                            },
                        ],
                    };
                    let map_id = map.insert(&mut transaction).await.unwrap();
                    sqlx::query!(
                        "UPDATE games SET maps_dom6_id=$1 WHERE id=$2 RETURNING * ",
                        map_id,
                        self.game_id
                    )
                    .fetch_all(&mut *transaction)
                    .await
                    .unwrap();
                }
            }
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
                        255 => None,
                        0 => None,
                        w => Some(w as i32),
                    },
                    fth.get_kingdoms(),
                ),
                _ => return Err(Dom5TurnErr::FtherlndMixup),
            };
            let turn_number = ftherlnd.header.turnnumber;
            // Insert file and get ID
            let file_record = sqlx::query!(
                "INSERT INTO files (filename, filebinary) VALUES('ftherlnd', $1) RETURNING id",
                contents
            )
            .fetch_one(&mut *transaction)
            .await?;

            // Insert turn
            sqlx::query!(
                "INSERT INTO turns (game_id, turn_number, file_id, turnkey, winner) VALUES ($1, $2, $3, $4, $5)",
                self.game_id,
                turn_number,
                file_record.id,
                ftherlnd.header.turnkey,
                winner
            )
            .execute(&mut *transaction)
            .await?;

            let nations = sqlx::query!(
                r#"SELECT nation_id "nation_id: i32", filename FROM nations WHERE game_id=$1"#,
                self.game_id
            )
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
                            _ => panic!(
                                "Impossible to reach defeated & human branch without being defeated or human"
                            ),
                        };
                        let nation = nations.iter().find(|n| n.nation_id == *nation_id).unwrap();
                        let mut player_turn_file =
                            std::fs::File::open(save_dir.join(format!("{}.trn", nation.filename)))
                                .map_err(Dom5TurnErr::MissingTrn)?;
                        let mut contents = vec![];
                        player_turn_file
                            .read_to_end(&mut contents)
                            .map_err(Dom5TurnErr::MissingTrn)?;
                        // Insert file and get ID
                        let file_record = sqlx::query!(
                            "INSERT INTO files (filename, filebinary) VALUES($1, $2) RETURNING id",
                            nation.filename,
                            contents
                        )
                        .fetch_one(&mut *transaction)
                        .await?;

                        // Insert player turn
                        sqlx::query!(
                            "INSERT INTO player_turns (trnfile_id, nation_id, game_id, turn_number, status) VALUES ($1, $2, $3, $4, $5)",
                            file_record.id,
                            nation_id,
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
                    KingdomType::Unknown(d) => {
                        log::warn!("Unknown nation status for {}, {}", *nation_id as i32, d);
                    }
                }
            }
            // Insert computer player turns
            for nation_id in computer_nationids {
                sqlx::query!(
                    "INSERT INTO player_turns (status, game_id, nation_id, turn_number) VALUES (3, $1, $2, $3)",
                    self.game_id,
                    nation_id,
                    turn_number
                )
                .execute(&mut *transaction)
                .await?;
            }

            // Insert defeated player turns
            for nation_id in defeated_nationids {
                sqlx::query!(
                    "INSERT INTO player_turns (status, game_id, nation_id, turn_number) VALUES (4, $1, $2, $3)",
                    self.game_id,
                    nation_id,
                    turn_number
                )
                .execute(&mut *transaction)
                .await?;
            }

            if winner.is_none() {
                sqlx::query!("UPDATE games SET next_turn=datetime('now', '+' || timer || ' minutes') WHERE id=$1 AND timer IS NOT NULL", self.game_id).execute(&mut *transaction).await?;
            } else {
                sqlx::query!(
                    "UPDATE games SET next_turn=NULL WHERE id=$1 AND timer IS NOT NULL",
                    self.game_id
                )
                .execute(&mut *transaction)
                .await?;
            }
            let job = sqlx::query!(
                "
            SELECT reaped FROM jobs WHERE job_id=$1",
                job_id
            )
            .fetch_one(&self.db_pool)
            .await?;
            if job.reaped {
                transaction.rollback().await?;
                return Err(Dom5TurnErr::AlreadyReaped);
            } else {
                transaction.commit().await?;
            }

            // Update subjob and job to mark completion
            sqlx::query!(
                "UPDATE subjobs SET finished_at=datetime('now') WHERE subjob_id=$1",
                subjob.subjob_id
            )
            .execute(&self.db_pool)
            .await?;
            sqlx::query!("UPDATE jobs SET finished_at=datetime('now'), updated_at=datetime('now') WHERE job_id=$1", job_id).execute(&self.db_pool).await?;

            Ok(())
    }
}
