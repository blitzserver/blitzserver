pub mod player;
pub mod binary;
pub mod maps;

use std::env;
use chrono::NaiveDateTime;
pub use player::{NewPlayer, Player};
use sqlx::Postgres;
use thiserror::Error;

use crate::packets::astralpacket_resp::DmFileDef;

use self::maps::MapSummary;

pub struct Era;
impl Era {
    pub const EARLY: i32 = 1;
    pub const MIDDLE: i32 = 2;
    pub const LATE: i32 = 3;
}

#[derive(Clone, derive_more::Into, derive_more::From, Debug)]
pub struct Game {
    pub id: i32,
    pub name: String,
    pub era: i32,
    pub map_id: i32,
    pub maps_dom6_id: Option<i32>,
    pub port: Option<i32>,
    pub timer: Option<i32>,
    pub thrones_t1: i32,
    pub thrones_t2: i32,
    pub thrones_t3: i32,
    pub throne_points_required: i32,
    pub research_diff: i32,
    pub research_rand: bool,
    pub hof_size: i32,
    pub global_size: i32,
    pub indepstr: i32,
    pub magicsites: i32,
    pub eventrarity: i32,
    pub richness: i32,
    pub resources: i32,
    pub recruitment: i32,
    pub supplies: i32,
    pub startprov: i32,
    pub renaming: bool,
    pub scoregraphs: bool,
    pub nationinfo: bool,
    pub artrest: bool,
    pub teamgame: bool,
    pub clustered: bool,
    pub storyevents: i32,
    pub newailvl: i32,
    pub next_turn: Option<chrono::DateTime::<chrono::Utc>>,
    pub password: String,
    pub archived: bool,
    pub masterpass: Option<String>,
    pub private: bool,
    pub last_activity: chrono::DateTime::<chrono::Utc>,
    pub require_password: bool,
    pub select_start: i32,
    pub modified_mapcrc: Option<i32>,
    pub cataclysm: Option<i32>,
    pub max_stales: Option<i32>,
    pub motd: Option<String>,
    pub conq_all: bool,
    pub binary_id: Option<i32>,
    pub formal_diplo: i32,
    pub research_9_slow: bool,
    pub cheat_detection: bool,
    pub mercslots: i32,
    
}

#[derive(derive_more::From, derive_more::Into)]
pub struct SqlxGame {
    pub id: i32,
    pub name: String,
    pub era: i32,
    pub map_id: i32,
    pub port: Option<i32>,
    pub timer: Option<i32>,
    pub thrones_t1: i32,
    pub thrones_t2: i32,
    pub thrones_t3: i32,
    pub throne_points_required: i32,
    pub research_diff: i32,
    pub research_rand: bool,
    pub hof_size: i32,
    pub global_size: i32,
    pub indepstr: i32,
    pub magicsites: i32,
    pub eventrarity: i32,
    pub richness: i32,
    pub resources: i32,
    pub recruitment: i32,
    pub supplies: i32,
    pub startprov: i32,
    pub renaming: bool,
    pub scoregraphs: bool,
    pub nationinfo: bool,
    pub artrest: bool,
    pub teamgame: bool,
    pub clustered: bool,
    pub storyevents: i32,
    pub newailvl: i32,
    pub next_turn: Option<chrono::Utc>,
    pub password: String,
    pub archived: bool,
    pub masterpass: Option<String>,
    pub private: bool,
    pub last_activity: chrono::NaiveDateTime,
    pub require_password: bool,
    pub select_start: i32,
    pub formal_diplo: i32,
    pub research_9_slow: bool,
    pub cheat_detection: bool,
    pub mercslots: i32,
}

struct GameNationCount {
    pub game_id: i32,
    pub count: i64,
}

impl Game {
    pub async fn update<'q, D>(&self, mod_ids: &[i32], db: D) -> Result<Game, sqlx::error::Error> 
        where D: sqlx::Executor<'q, Database = Postgres> {
        sqlx::query_as!(Game, "
        WITH 
            removed_mods AS (DELETE FROM game_mods WHERE game_id=$38),
            updated_mods AS (INSERT INTO game_mods (game_id, mod_id) VALUES($38, UNNEST($39::integer[])) ON CONFLICT (game_id, mod_id) DO NOTHING)
        UPDATE games SET 
            era                    = $1,
            thrones_t1             = $2,
            thrones_t2             = $3,
            thrones_t3             = $4,
            throne_points_required = $5,
            research_diff          = $6,
            research_rand          = $7,
            hof_size               = $8,
            global_size            = $9,
            indepstr               = $10,
            magicsites             = $11,
            eventrarity            = $12,
            richness               = $13,
            resources              = $14,
            recruitment            = $15,
            supplies               = $16,
            startprov              = $17,
            renaming               = $18,
            scoregraphs            = $19,
            nationinfo             = $20,
            artrest                = $21,
            teamgame               = $22,
            clustered              = $23,
            storyevents            = $24,
            newailvl               = $25,
            maps_dom6_id           = $26,
            masterpass             = $27,
            private                = $28,
            require_password       = $29,
            select_start           = $30,
            cataclysm              = $31,
            max_stales             = $32,
            conq_all               = $33,
            formal_diplo           = $34,
            research_9_slow        = $35,
            cheat_detection        = $36,
            mercslots              = $37
        WHERE
            id                     = $38
        RETURNING *
        " , 
                self.era,
                self.thrones_t1,
                self.thrones_t2,
                self.thrones_t3,
                self.throne_points_required,
                self.research_diff,
                self.research_rand,
                self.hof_size,
                self.global_size,
                self.indepstr,
                self.magicsites,
                self.eventrarity,
                self.richness,
                self.resources,
                self.recruitment,
                self.supplies,
                self.startprov,
                self.renaming,
                self.scoregraphs,
                self.nationinfo,
                self.artrest,
                self.teamgame,
                self.clustered,
                self.storyevents,
                self.newailvl,
                self.maps_dom6_id,
                self.masterpass,
                self.private,
                self.require_password,
                self.select_start,
                self.cataclysm,
                self.max_stales,
                self.conq_all,
                self.formal_diplo,
                self.research_9_slow,
                self.cheat_detection,
                self.mercslots,
                self.id,
                mod_ids).fetch_one(db).await
    }
    pub async fn get_mods<'c, D: sqlx::Executor<'c, Database = Postgres>>(&self, db: D) -> Result<Vec<Mod>, sqlx::error::Error>
    {
        sqlx::query_as!(Mod, "SELECT m.* FROM mods m INNER JOIN game_mods gm ON gm.mod_id=m.id AND gm.game_id=$1", self.id).fetch_all(db).await
    }
    pub async fn get<'c, D: sqlx::Executor<'c , Database = Postgres>>(game_id: i32, db: D) -> Result<Game, sqlx::error::Error>
    {
        return sqlx::query_as!(Game, "SELECT * FROM games WHERE id=$1", game_id).fetch_one(db).await
    }

    pub async fn schedule_turn<'c, D>( &self, next_turn: chrono::DateTime::<chrono::Utc>, db: D) -> Result<Game, sqlx::error::Error>
        where D: sqlx::Executor<'c, Database = Postgres>
    {
        sqlx::query_as!(Game, "UPDATE games SET next_turn=$2 WHERE id=$1 RETURNING *", self.id, next_turn).fetch_one(db).await
    }
    pub fn next_turn_string(&self) -> String {
        if self.archived {
            return "Never".to_string();
        }
        match self.next_turn {
            None => "When these shazbots submit their turns".to_string(),
            Some(next_turn) => {
                let duration = chrono::Utc::now() - next_turn;
                if duration.num_seconds() > 0 {
                    format!(
                        "{}{}{}",
                        if duration.num_seconds() > (60 * 60) {
                            // More than one hour
                            format!("{} hours, ", (duration.num_seconds() as f32 / 60.0 / 60.0).floor())
                        } else {
                            "".to_string()
                        },
                        if duration.num_seconds() % (60 * 60) > 0 {
                            // More than one minute within the hour
                            format!(
                                "{} minutes",
                                (duration.num_seconds() as f32 % (60.0 * 60.0) / 60.0).floor(),
                            )
                        } else {
                            "".to_string()
                        },
                        if duration.num_seconds() < 60 {
                            format!("{} seconds", duration.num_seconds())
                        } else {
                            "".to_string()
                        }
                    )
                } else {
                    "The past".to_string()
                }
            }
        }
    }
    pub fn timer_string(&self) -> String {
        match self.timer {
            None => "".to_owned(),
            Some(t) => t.to_string(),
        }
    }
    pub fn era_name(&self) -> String {
        match self.era {
            Era::EARLY => "Early".to_string(),
            Era::MIDDLE => "Middle".to_string(),
            Era::LATE => "Late".to_string(),
            _ => "Unknown".to_string(),
        }
    }

    pub async fn unstart<'a, D>(game_id: i32, db: D) -> Result<(), sqlx::error::Error>
    where
        D: sqlx::Executor<'a, Database = Postgres>
    {
        sqlx::query!("
        WITH 
            turns AS (
                UPDATE turns SET archived=TRUE where game_id=$1
                RETURNING *
            ), 
            plyer_turns AS (
                UPDATE player_turns SET archived=TRUE WHERE game_id=$1
                RETURNING *
            )
        INSERT INTO admin_logs (game_id, datetime, action) VALUES ($1, NOW(), 'Returned game top lobby') RETURNING *", game_id).fetch_one(db).await;
        Ok(())
    }

    pub async fn rollback<'a, D>(game_id: i32, db: D) -> Result<(), sqlx::error::Error>
    where
        D: sqlx::Executor<'a, Database = Postgres>
    {
        sqlx::query!("
            WITH archived_turn AS (
                    UPDATE turns t
                    SET archived=TRUE 
                    WHERE t.game_id=$1 
                        AND turn_number > 1 
                        AND turn_number=(SELECT MAX(turn_number) FROM turns t1 WHERE t1.game_id=$1 AND t1.archived != TRUE)
                    RETURNING *
                ),
                archived_player_turns AS (
                    UPDATE player_turns pt
                    SET archived=TRUE
                    WHERE pt.game_id=$1
                        AND pt.turn_number=(SELECT MAX(turn_number) FROM turns t1 WHERE t1.game_id=$1 AND t1.archived != TRUE)
                    RETURNING * 
                ),
                unfinished_player_turns AS (
                    UPDATE player_turns pt
                    SET status = 1
                    WHERE pt.game_id=$1
                        AND pt.turn_number=(SELECT MAX(turn_number) FROM turns t1 WHERE t1.game_id=$1 AND t1.archived != TRUE) - 1
                        AND pt.status = 2
                    RETURNING * 
                )
                INSERT INTO admin_logs (game_id, datetime, action) SELECT $1, NOW(), 'Rolled back turn' FROM archived_turn RETURNING *
             ", game_id).fetch_one(db).await.unwrap();
        Ok(())
    }

    pub async fn get_player_count<'a, D>(game_ids: &[i32], db: D) -> Result<std::collections::HashMap<i32, i32>, sqlx::error::Error>
    where
        D: sqlx::Executor<'a, Database = Postgres>
    {
        Ok(sqlx::query!(
            "SELECT game_id, COUNT(*) count FROM players WHERE game_id = ANY($1) GROUP BY game_id", game_ids
        )
        .fetch_all(db)
        .await?
        .iter()
        .fold(std::collections::HashMap::new(), |mut hm, row| {
            hm.insert(row.game_id, row.count.unwrap_or(0) as i32);
            hm
        }))
    }

    pub async fn remove_timer<'a, D>(game_id: i32, db: D) -> Result<Self, sqlx::error::Error>
    where
        D: sqlx::Executor<'a, Database = Postgres>
    {
        sqlx::query_as!(Game, "UPDATE games SET next_turn = NULL, timer = NULL WHERE id=$1 RETURNING *", game_id).fetch_one(db).await
    }

    pub async fn archive<'a, D>(game_id: i32, db: D) -> Result<(), sqlx::error::Error>
    where
        D: sqlx::Executor<'a, Database = Postgres>
    {
        sqlx::query!("WITH archive AS (
                UPDATE games SET port=NULL, archived=TRUE,last_activity=NOW() WHERE id=$1 RETURNING *)
            INSERT INTO admin_logs (game_id, datetime, action) VALUES ($1, NOW(), 'Archived game') RETURNING *", game_id).fetch_one(db).await.unwrap();
        Ok(())
    }
    pub async fn unarchive<'a, D>(game_id: i32, db: D) -> Result<(), sqlx::error::Error>
    where
        D: sqlx::Executor<'a, Database = Postgres>
    {
        let port_var = env::var("PORT_RANGE").expect("PORT_RANGE must be set (ie. '10000,10999')");
        let range: Vec<&str> = port_var.split(",").collect();
        sqlx::query!("WITH unarchive AS (
                     UPDATE games 
                     SET port=(SELECT MIN(num) 
                             FROM generate_series($2::integer, $3::integer) num 
                             LEFT OUTER JOIN games g ON g.port=num), 
                         archived=FALSE,
                         last_activity=NOW() 
                    WHERE id=$1 RETURNING *)
            INSERT INTO admin_logs (game_id, datetime, action) VALUES ($1, NOW(), 'Reinstated game') RETURNING *", game_id, range[0].parse::<i32>().unwrap(), range[1].parse::<i32>().unwrap()).fetch_one(db).await.unwrap();
        Ok(())
    }
}

pub struct NewGame {
    pub name: String,
    pub era: i32,
    pub map_id: i32,
    pub port: i32,
    pub thrones_t1: i32,
    pub thrones_t2: i32,
    pub thrones_t3: i32,
    pub throne_points_required: i32,
    pub research_diff: i32,
    pub research_rand: bool,
    pub hof_size: i32,
    pub global_size: i32,
    pub indepstr: i32,
    pub magicsites: i32,
    pub eventrarity: i32,
    pub richness: i32,
    pub resources: i32,
    pub recruitment: i32,
    pub supplies: i32,
    pub startprov: i32,
    pub renaming: bool,
    pub scoregraphs: bool,
    pub nationinfo: bool,
    pub artrest: bool,
    pub teamgame: bool,
    pub clustered: bool,
    pub storyevents: i32,
    pub newailvl: i32,
    pub password: String,
    pub masterpass: Option<String>,
}
impl Default for NewGame {
    fn default() -> Self {
        Self {
            name: "".to_string(),
            port: 0,
            era: 1,
            map_id: 1,
            thrones_t1: 5,
            thrones_t2: 0,
            thrones_t3: 0,
            throne_points_required: 5,
            research_diff: 2,
            research_rand: true,
            hof_size: 10,
            global_size: 5,
            indepstr: 5,
            magicsites: 60,
            eventrarity: 1,
            richness: 100,
            resources: 100,
            recruitment: 100,
            supplies: 100,
            startprov: 1,
            renaming: true,
            scoregraphs: false,
            nationinfo: true,
            artrest: true,
            teamgame: false,
            clustered: false,
            storyevents: 1,
            newailvl: 2,
            password: "password".to_string(),
            masterpass: None,
        }
    }
}

impl NewGame {
    pub async fn insert<'q, D>(&self, db: D) -> Result<Game, sqlx::error::Error> 
        where D: sqlx::Executor<'q, Database = Postgres> 
    {
        sqlx::query_as!(Game, "
        INSERT INTO games ( name, era, maps_dom6_id, port, thrones_t1, thrones_t2, thrones_t3, throne_points_required, research_diff, research_rand, hof_size, global_size, indepstr, magicsites, eventrarity, richness, resources, recruitment, supplies, startprov, renaming, scoregraphs, nationinfo, artrest, teamgame, clustered, storyevents, newailvl, password, masterpass, binary_id)
        SELECT              $1,   $2,  $3,     $4,   $5,         $6,         $7,         $8,                     $9,            $10,           $11,      $12,         $13,      $14,        $15,         $16,      $17,       $18,         $19,      $20,       $21,      $22,         $23,        $24,     $25,      $26,       $27,         $28,      $29,      $30,        b.binary_id
        FROM (SELECT binary_id FROM binaries  ORDER BY binary_id DESC LIMIT 1) b
        RETURNING games.*
        " , 
        self.name,
        self.era,
        self.map_id,
        self.port,
        self.thrones_t1,
        self.thrones_t2,
        self.thrones_t3,
        self.throne_points_required,
        self.research_diff,
        self.research_rand,
        self.hof_size,
        self.global_size,
        self.indepstr,
        self.magicsites,
        self.eventrarity,
        self.richness,
        self.resources,
        self.recruitment,
        self.supplies,
        self.startprov,
        self.renaming,
        self.scoregraphs,
        self.nationinfo,
        self.artrest,
        self.teamgame,
        self.clustered,
        self.storyevents,
        self.newailvl,
        self.password,
        self.masterpass).fetch_one(db).await
    }
}

#[derive(Clone, Debug)]
pub struct File {
    pub id: i32,
    pub filename: String,
    pub filebinary: Vec<u8>,
}

pub struct NewFile<'a> {
    pub filename: &'a str,
    pub filebinary: &'a [u8],
}

impl<'a> NewFile<'a> {
    pub fn new(filename: &'a str, filebinary: &'a [u8]) -> NewFile<'a> {
        NewFile {
            filename,
            filebinary,
        }
    }
    pub async fn insert<'q, D>(self, db: D) -> Result<File, sqlx::error::Error>
    where
        D: sqlx::Executor<'q, Database = Postgres>
    {
        sqlx::query_as!(File, "
        INSERT INTO files (filename, filebinary)
        VALUES ($1, $2)
        ON CONFLICT (filename,digest(filebinary, 'sha1'::text))
        DO UPDATE SET filename=EXCLUDED.filename
        RETURNING *", self.filename, &self.filebinary
        )
        .fetch_one(db)
        .await
    }
}

// YUCK! Newtype hack to allow multiple relations from map to filew without colliding
pub struct MapFile(pub File);
pub struct TgaFile(pub File);
pub struct WinterFile(pub File);

#[derive(Debug, Clone)]
pub struct Map {
    pub id: i32,
    pub name: String,
    pub mapfile_id: i32,
    pub tgafile_id: i32,
    pub winterfile_id: Option<i32>,
    pub province_count: i32,
    pub uw_count: i32,
    pub archived: bool,
    pub map_crc: Option<i32>,
    pub tga_crc: Option<i32>,
    pub winter_crc: Option<i32>,
    pub min_version: i32,
}

#[derive(Debug, Error)]
pub enum MapCrcError {
    #[error("DB error")]
    DbError(#[from] sqlx::Error)
}

impl Map {
    pub fn get_crcs(game_id: i32, db: &sqlx::postgres::PgPool) -> Result<((String, i32, i32), (String, i32, i32), Option<(String, i32, i32)>), MapCrcError> {
        smol::block_on(async {
            let mut record =sqlx::query!(r#"
                SELECT 
                    m.id, 
                    modified_mapcrc, 
                    COUNT(p.*) AS custom_starts, 
                    m.map_crc, mapf.filename as mapfilename, length(mapf.filebinary) as "mapflen", 
                    m.tga_crc, tgaf.filename as tgafilename, length(tgaf.filebinary) as "tgaflen", 
                    m.winter_crc as "winter_crc?", winf.filename as "winterfilename?", length(winf.filebinary) as "winflen?"
                FROM games g
                    LEFT JOIN maps m ON m.id=g.map_id
                    INNER JOIN files mapf ON mapf.id=m.mapfile_id
                    INNER JOIN files tgaf ON tgaf.id=m.tgafile_id
                    LEFT JOIN files winf ON winf.id=m.winterfile_id
                    LEFT JOIN players p ON p.game_id=g.id AND p.startprov IS NOT NULL
                WHERE g.id=$1
                GROUP BY g.id, m.id, mapf.id, tgaf.id, winf.id"#, game_id).fetch_one(db).await?;
            // Map crcs need to be generated
            if let (None, None) = (record.map_crc, record.tga_crc) {
                let binary_record = sqlx::query!(r#"
                    SELECT mapf.filebinary as mapbinary, tgaf.filebinary as tgabinary, winf.filebinary as "winbinary?"
                    FROM maps m
                    INNER JOIN files mapf ON mapf.id=m.mapfile_id
                    INNER JOIN files tgaf ON tgaf.id=m.tgafile_id
                    LEFT JOIN files winf ON winf.id=m.winterfile_id
                    WHERE m.id=$1"#, record.id).fetch_one(db).await?;
                record.map_crc = Some(crate::util::calculate_crc(&binary_record.mapbinary[..]) as i32);
                record.tga_crc = Some(crate::util::calculate_crc(&binary_record.tgabinary[..]) as i32);
                if let Some(winbinary) = binary_record.winbinary {
                    record.winter_crc = Some(crate::util::calculate_crc(&winbinary[..]) as i32);
                }
                sqlx::query!("UPDATE maps m SET map_crc=$2, tga_crc=$3, winter_crc=$4 WHERE id=$1", record.id, record.map_crc, record.tga_crc, record.winter_crc).execute(db).await?;
            }
            // Extended map crc needs to be generated
            if let (Some(_), None) = (record.custom_starts, record.modified_mapcrc) {
                let mut mapbinary = sqlx::query!("SELECT mapf.filebinary FROM maps m INNER JOIN files mapf ON mapf.id=m.mapfile_id WHERE m.id=$1", record.id).fetch_one(db).await?;
                let custom_starts = sqlx::query!("SELECT nationid, startprov FROM players WHERE game_id=$1", game_id).fetch_all(db).await?;
                for (player, prov) in custom_starts.iter().flat_map(|p| match p.startprov {
                    Some(startprov) => Some((p.nationid, startprov)),
                    None => None,
                }) {
                   mapbinary 
                        .filebinary
                        .append(&mut format!("\n#specstart {} {}\n", player, prov).into_bytes());
                }
                record.modified_mapcrc = Some(crate::util::calculate_crc(&mapbinary.filebinary[..]) as i32);
                sqlx::query!("UPDATE games SET modified_mapcrc=$2 WHERE id=$1", game_id, record.modified_mapcrc).execute(db).await?;
            }
            Ok((
                (record.mapfilename,
                 match record.custom_starts {
                    Some(_) => {
                        record.mapflen.unwrap_or(0) + sqlx::query!(r#"SELECT nationid, startprov as "startprov" FROM players WHERE game_id=$1 AND startprov IS NOT NULL"#, game_id)
                            .fetch_all(db).await?
                            .into_iter()
                            .filter_map(|record| match record.startprov {
                                Some(sp) => Some((record.nationid, sp)),
                                None => None
                            })
                            .fold(0i32, |acc, (nationid, startprov)| acc + format!("\n#specstart {} {}\n", nationid, startprov).into_bytes().len() as i32)
                    },
                    None => record.mapflen.unwrap_or(0)
                }, match record.custom_starts {
                    Some(_) => record.modified_mapcrc.unwrap_or(0),
                    None => record.map_crc.unwrap_or(0)
                }),
                (record.tgafilename, record.tgaflen.unwrap_or(0), record.tga_crc.unwrap_or(0)),
                match (record.winterfilename, record.winflen, record.winter_crc) {
                    (Some(a), Some(b), Some(c)) => Some((a,b,c)),
                    _ => None 
                }
            ))
        })
    }
    pub async fn get_available_maps<'q, D>(
        game_id: i32,
        is_admin: bool,
        db: D,
    ) -> Result<Vec<Map>, sqlx::error::Error>
    where
        D: sqlx::Executor<'q, Database = Postgres>
    {
        sqlx::query_as!(Map,
            "
        SELECT M.* 
        FROM maps M
        LEFT JOIN games G 
            ON M.id=G.map_id AND G.id = $1
        WHERE (G.id IS NOT NULL OR M.archived IS FALSE)
            AND (G.id IS NOT NULL OR $2 = TRUE)", game_id, is_admin
        )
        .fetch_all(db).await
    }

    pub async fn insert_provinces<'q, D>(
        &self,
        provs: Vec<crate::files::maps::ProvinceLocation>,
        db: D,
    ) -> Result<Vec<MapProvince>, sqlx::error::Error>
    where
        D: sqlx::Executor<'q, Database = Postgres>
    {
        sqlx::query_as!(MapProvince, "
        INSERT INTO map_provinces (map_id, province_id, x, y, unit_x, unit_y) VALUES ($1, UNNEST($2::integer[]), UNNEST($3::integer[]), UNNEST($4::integer[]), UNNEST($5::float[]), UNNEST($6::float[])) RETURNING *",
        self.id,
        &provs.iter().enumerate().map(|(id, _)| id as i32).collect::<Vec<i32>>(),
        &provs.iter().map(|p| p.x as i32).collect::<Vec<i32>>(),
        &provs.iter().map(|p| p.y as i32).collect::<Vec<i32>>(),
        &provs.iter().map(|p| p.unit_x).collect::<Vec<f64>>(),
        &provs.iter().map(|p| p.unit_y).collect::<Vec<f64>>(),
        ).fetch_all(db).await
    }

    pub async fn get_provinces<'q, D>(&self, db: D) -> Result<Vec<MapProvince>, sqlx::error::Error>
    where
        D: sqlx::Executor<'q, Database = Postgres>
    {
        sqlx::query_as!(MapProvince, "SELECT * FROM map_provinces WHERE map_id=$1", self.id).fetch_all(db).await
    }

    pub async fn get_files<'q, D>(map_id: i32, db: D) -> Result<(File, File, Option<File>), sqlx::error::Error>
    where
        D: sqlx::Executor<'q, Database = Postgres>
    {
        let file_contents = sqlx::query!(r#"
        SELECT ma.id map_id, ma.filename map_filename, ma.filebinary map_binary, i.id image_id, i.filename image_filename, i.filebinary image_binary, wi.id "winter_id?", wi.filename "wi_filename?", wi.filebinary "winter_binary?"
        FROM maps m
        INNER JOIN files i ON i.id=m.tgafile_id
        INNER JOIN files ma ON ma.id=m.mapfile_id
        LEFT JOIN files wi ON wi.id=m.winterfile_id
        WHERE m.id=$1"#, map_id).fetch_one(db).await?;

        return Ok((
            File {
                id: file_contents.map_id,
                filename: file_contents.map_filename,
                filebinary: file_contents.map_binary
            },
            File {
                id: file_contents.image_id,
                filename: file_contents.image_filename,
                filebinary: file_contents.image_binary,
            },
            match (file_contents.winter_id, file_contents.wi_filename, file_contents.winter_binary) {
                (Some(id), Some(filename), Some(filebinary)) => Some(File { id, filename, filebinary}),
                _ => None
            }
        ));
    }
}

#[derive(Debug)]
pub struct NewMap {
    pub name: String,
    pub mapfile_id: i32,
    pub tgafile_id: i32,
    pub winterfile_id: Option<i32>,
    pub province_count: i32,
    pub uw_count: i32,
    pub min_version: i32,
}

impl NewMap {
    pub async fn insert<'q, D>(&self, db: D) -> Result<Map, sqlx::error::Error> 
        where D: sqlx::Executor<'q, Database = Postgres>
    {
        sqlx::query_as!(Map, "INSERT INTO maps (name, mapfile_id, tgafile_id, winterfile_id, province_count, uw_count, min_version) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING *",
        self.name,
        self.mapfile_id,
        self.tgafile_id,
        self.winterfile_id,
        self.province_count,
        self.uw_count,
        self.min_version
        ).fetch_one(db).await
    }
}

#[derive(Debug)]
pub struct Nation {
    pub id: i32,
    pub game_id: i32,
    pub nation_id: i32,
    pub name: String,
    pub epithet: String,
    pub filename: String,
}

impl Nation {
    pub async fn get<'q, D>(game_id: i32, nation_id: i32, db: D) -> Result<Nation, sqlx::error::Error>
    where
        D: sqlx::Executor<'q, Database = Postgres>
    {
        sqlx::query_as!(Nation, "SELECT * FROM nations WHERE nation_id=$1 AND game_id=$2", nation_id, game_id).fetch_one(db).await
    }
    pub async fn get_all<'q, D>(game_id: i32, db: D) -> Result<Vec<Nation>, sqlx::error::Error>
    where
        D: sqlx::Executor<'q, Database = Postgres>
    {
        sqlx::query_as!(Nation, "SELECT * FROM nations WHERE game_id=$1", game_id).fetch_all(db).await
    }
}

#[derive(Debug)]
pub struct Turn {
    pub id: i32,
    pub game_id: i32,
    pub turn_number: i32,
    pub file_id: i32,
    pub created_at: NaiveDateTime,
    pub archived: bool,
    pub turnkey: Option<i32>,
    pub winner: Option<i32>
}

#[derive(Clone)]
pub struct TurnSummary {
    pub game_id: i32,
    pub next_turn: Option<chrono::DateTime<chrono::Utc>>,
    pub turn_number: i32,
    pub submitted: Option<i64>,
    pub total: Option<i64>,
    pub winner_name: Option<String>
}

#[derive(Debug, Error)]
pub enum TurnError {
    #[error("Database error")]
    DbError(sqlx::Error),
    #[error("Corrupt ftherlnd")]
    CorruptFtherlnd(crate::files::saves::DomSaveReadError),
}

impl Turn {
    pub fn get_turnkey(
        game_id: i32,
        turn_number: i32,
        db_pool: &sqlx::postgres::PgPool,
    ) -> Result<i32, TurnError> {
        smol::block_on(async {
            let record = sqlx::query!("SELECT id,turnkey,file_id FROM turns WHERE game_id=$1 AND turn_number=$2 AND archived != TRUE", game_id, turn_number).fetch_one(db_pool).await.map_err(TurnError::DbError)?;
            Ok(match record.turnkey {
                Some(r) => r,
                None => {
                    let file =
                        sqlx::query!("SELECT filebinary FROM files WHERE id=$1", record.file_id)
                            .fetch_one(db_pool)
                            .await
                            .map_err(TurnError::DbError)?;
                    let ftherlnd = crate::files::saves::SaveFile::read_contents(
                        std::io::Cursor::new(&file.filebinary[..]),
                    )
                    .map_err(TurnError::CorruptFtherlnd)?;
                    sqlx::query!(
                        "UPDATE turns SET turnkey=$1 WHERE id=$2",
                        ftherlnd.header.turnkey as i32,
                        record.id
                    )
                    .execute(db_pool)
                    .await
                    .map_err(TurnError::DbError)?;
                    ftherlnd.header.turnkey as i32
                }
            })
        })
    }
    pub async fn get_all<'q,D>(game_id: i32, db: D) -> Result<Vec<Turn>, sqlx::error::Error>
    where
        D: sqlx::Executor<'q, Database = Postgres>
    {
        sqlx::query_as!(Turn, "SELECT * FROM turns WHERE game_id=$1 AND archived=FALSE ORDER BY turn_number ASC", game_id).fetch_all(db).await
    }
    pub async fn current_turn<'q, D>(game_ids: &[i32], db: D) -> Result<Vec<Turn>, sqlx::error::Error>
    where
        D: sqlx::Executor<'q, Database = Postgres>
    {
        sqlx::query_as!(Turn, r#"
        SELECT DISTINCT ON(game_id) *
        FROM turns t
        WHERE archived=false AND game_id=ANY($1)
        ORDER BY game_id, turn_number DESC
        "#, game_ids)
        .fetch_all(db).await
    }
        
    pub fn turn_summary(game_ids: &[i32], db_pool: &sqlx::postgres::PgPool) -> Vec<TurnSummary> {
        smol::block_on(async {
            if game_ids.len() == 0 {
                vec![]
            } else {
                sqlx::query_as!(TurnSummary, r#"
WITH tt as (SELECT game_id,MAX(turn_number) as "turn_number" FROM turns WHERE ARCHIVED != true GROUP BY game_id)
SELECT t.game_id, t.turn_number, n.name as "winner_name?", COUNT(submitted) as "submitted", COUNT(total) as "total", next_turn
FROM turns t
INNER JOIN tt ON tt.game_id=t.game_id AND tt.turn_number=t.turn_number AND t.archived != TRUE
LEFT JOIN nations n ON t.winner=n.nation_id AND t.game_id=n.game_id
LEFT JOIN player_turns total ON total.turn_number=t.turn_number AND total.archived != TRUE and total.game_id=t.game_id AND total.status IN (0,1,2)
LEFT JOIN player_turns submitted ON submitted.turn_number=t.turn_number AND submitted.archived != TRUE and submitted.game_id=t.game_id AND submitted.status=2 AND submitted.nation_id=total.nation_id
LEFT JOIN games g ON g.id=t.game_id
WHERE t.game_id = ANY($1)
GROUP BY t.game_id,t.turn_number, n.name, g.next_turn
"#, game_ids).fetch_all(db_pool).await.unwrap()
            }
        })
    }
}

#[derive(Debug)]
pub struct PlayerTurn {
    id: i32,
    pub turn_number: i32,
    pub nation_id: i32,
    pub game_id: i32,
    pub trnfile_id: Option<i32>,
    pub twohfile_id: Option<i32>,
    pub archived: bool,
    pub status: i32,
    pub twoh_crc: Option<i32>,
}

impl PlayerTurn {
    pub async fn get_player_turns<'q, D>(
        game_id: i32,
        db: D,
    ) -> Result<std::collections::HashMap<i32, Vec<PlayerTurn>>, sqlx::error::Error>
    where
        D: sqlx::Executor<'q, Database = Postgres>
    {
        let player_turns = sqlx::query_as!(PlayerTurn, "
        SELECT * FROM player_turns WHERE game_id=$1 AND archived=false ORDER BY turn_number", game_id).fetch_all(db).await?;
        Ok(player_turns.into_iter().fold(
            std::collections::HashMap::new(),
            |mut hm: std::collections::HashMap<i32, Vec<PlayerTurn>>, pt: PlayerTurn| {
                if !hm.contains_key(&pt.nation_id) {
                    hm.insert(pt.nation_id, vec![]);
                }
                hm.get_mut(&pt.nation_id).unwrap().push(pt);
                hm
            },
        ))
    }
}

pub struct NewMod<'a> {
    pub dm_filename: &'a str,
    pub name: &'a str,
    pub file_id: i32,
    pub icon_file_id: Option<i32>,
}

#[derive(Clone)]
pub struct Mod {
    pub id: i32,
    pub dm_filename: String,
    pub name: String,
    pub file_id: i32,
    pub icon_file_id: Option<i32>,
    pub crc: Option<i32>,
}

#[derive(Debug, Error)]
pub enum ModError {
    #[error("CRC generation error")]
    CrcError,
}

impl Mod {
    pub fn get_crcs(
        game_id: i32,
        db_pool: &sqlx::postgres::PgPool,
    ) -> Result<Vec<DmFileDef>, ModError> {
        smol::block_on(async {
            match sqlx::query!(
                r#"
                            SELECT m.id,crc,file_id,dm_filename FROM mods m
                            INNER JOIN game_mods gm ON gm.game_id=$1 AND gm.mod_id=m.id"#,
                game_id
            )
            .fetch_all(db_pool)
            .await
            {
                Ok(mod_records) => Ok(mod_records
                    .into_iter()
                    .map(|mod_record| {

                        let path = std::path::PathBuf::from(&mod_record.dm_filename);
                        let filename: String = path.file_name().unwrap().to_string_lossy().to_string();
                        let folder: String = path.parent().unwrap().to_string_lossy().to_string();
                        return match mod_record.crc {
                        Some(crc) => {
                            return DmFileDef {
                                filename,
                                folder,
                                hash: crc as u32
                            }
                        },
                        None => smol::block_on(async {
                            if let Ok(file_record) = sqlx::query!(
                                r#"SELECT filebinary FROM files WHERE id=$1"#,
                                mod_record.file_id
                            )
                            .fetch_one(db_pool)
                            .await
                            {
                                let mut archive = zip::ZipArchive::new(std::io::Cursor::new(
                                    file_record.filebinary,
                                ))
                                .unwrap();
                                let hash = crate::util::calculate_crc(
                                    archive.by_name(&mod_record.dm_filename).unwrap(),
                                );
                                if let Err(e) = sqlx::query!(
                                    r#"UPDATE mods SET crc=$1 WHERE id=$2"#,
                                    hash as i32,
                                    mod_record.id
                                )
                                .execute(db_pool)
                                .await
                                {
                                    log::error!(
                                        "Failed to update mod {} crc: {:?}",
                                        mod_record.id,
                                        e
                                    );
                                }
                                return DmFileDef {
                                    filename,
                                    folder,
                                    hash: mod_record.crc.unwrap() as u32
                                }
                            } else {
                                return DmFileDef {
                                    filename,
                                    folder,
                                    hash: 0
                                }
                            }
                        }),
                    }})
                    .collect::<Vec<DmFileDef>>()),
                Err(e) => Err(ModError::CrcError),
            }
        })
    }
}

#[derive(Clone)]
pub struct ModDefinition {
    pub dm_filename: String,
    pub icon_filename: String,
    pub name: String,
}

pub struct NewGameMod {
    pub game_id: i32,
    pub mod_id: i32,
}

pub struct GameMod {
    id: i32,
    pub game_id: i32,
    pub mod_id: i32,
}

#[derive(Debug)]
pub struct Wakeup {
    pub timestamp: std::time::SystemTime,
}

#[derive(Debug)]
pub struct EmailConfig {
    pub id: i32,
    pub nation_id: i32,
    pub game_id: i32,
    pub hours_before_host: i32,
    pub email_address: String,
    pub last_turn_notified: Option<i32>,
    pub subject: String,
    pub body: String,
    pub is_reminder: bool,
}

impl EmailConfig {
    pub async fn delete<'q, D>(email_id: i32, email_address: &str, db: D) -> Result<EmailConfig, sqlx::error::Error> 
    where
        D: sqlx::Executor<'q, Database = Postgres> 
    {
        sqlx::query_as!(EmailConfig, "DELETE FROM email_configs WHERE id=$1 AND email_address=$2 RETURNING *", email_id, email_address).fetch_one(db).await
    }
    pub async fn mark_sent<'q, D>(&self, turn: i32, db: D) -> Result<EmailConfig, sqlx::error::Error>
    where
        D: sqlx::Executor<'q, Database = Postgres>
    {
        sqlx::query_as!(EmailConfig, "UPDATE email_configs SET last_turn_notified=$1 WHERE id=$2 RETURNING *", turn, self.id).fetch_one(db).await
    }
    pub async fn get_due_reminders<'q, D>(db: D) -> Result<Vec<EmailConfig>, sqlx::error::Error>
    where
        D: sqlx::Executor<'q, Database = Postgres>
    {
        // Fetch all email_configs which are reminder types attached to games which have at least
        // one turn, whose attached nation has not submitted orders for the current turn and the
        // timer has come to pass
        sqlx::query_as!(EmailConfig, r#"
SELECT ec.id "id!", ec.nation_id "nation_id!", ec.game_id "game_id!", ec.hours_before_host "hours_before_host!", ec.email_address "email_address!", ec.last_turn_notified, ec.subject "subject!", ec.body "body!", ec.is_reminder "is_reminder!"
FROM email_configs ec
LEFT JOIN (SELECT game_id,MAX(turn_number) as turn_number FROM turns WHERE archived = false GROUP BY game_id) t
    ON t.game_id=ec.game_id
INNER JOIN games g ON g.id=t.game_id AND g.archived = FALSE
LEFT OUTER JOIN (SELECT nation_id, game_id, MAX(turn_number) as turn_number FROM player_turns pt WHERE archived = false AND twohfile_id IS NOT NULL GROUP BY game_id,nation_id) pt
    ON pt.game_id=ec.game_id AND pt.turn_number = t.turn_number AND pt.nation_id = ec.nation_id
WHERE
    (ec.last_turn_notified IS NULL OR t.turn_number != ec.last_turn_notified)
    AND g.next_turn IS NOT NULL
    AND g.next_turn - interval '1' hour * ec.hours_before_host < NOW()
    AND ec.is_reminder IS TRUE
    AND t.turn_number IS NOT NULL
    "#).fetch_all(db).await
    }
    pub async fn get_due_notifications<'q, D>(db: D) -> Result<Vec<EmailConfig>, sqlx::error::Error>
    where
        D: sqlx::Executor<'q, Database = Postgres>
    {
        sqlx::query_as!(EmailConfig, r#"
SELECT ec.id "id!", ec.nation_id "nation_id!", ec.game_id "game_id!", ec.hours_before_host "hours_before_host!", ec.email_address "email_address!", ec.last_turn_notified, ec.subject "subject!", ec.body "body!", ec.is_reminder "is_reminder!"
FROM email_configs ec
LEFT JOIN (SELECT game_id,MAX(turn_number) as turn_number FROM turns WHERE archived = false GROUP BY game_id) t
    ON t.game_id=ec.game_id
INNER JOIN games g ON g.id=t.game_id AND g.archived = FALSE
LEFT OUTER JOIN (SELECT nation_id, game_id, MAX(turn_number) as turn_number FROM player_turns pt WHERE archived = false AND twohfile_id IS NOT NULL GROUP BY game_id,nation_id) pt
    ON pt.game_id=ec.game_id AND pt.turn_number = t.turn_number AND pt.nation_id = ec.nation_id
WHERE
    (ec.last_turn_notified IS NULL OR t.turn_number != ec.last_turn_notified)
    AND t.turn_number IS NOT NULL 
    AND ec.is_reminder = FALSE
    AND (ec.last_turn_notified IS NULL OR ec.last_turn_notified != t.turn_number)
    "#).fetch_all(db).await
    }
}

pub struct NewEmailConfig {
    pub nation_id: i32,
    pub game_id: i32,
    pub hours_before_host: i32,
    pub email_address: String,
    pub subject: String,
    pub body: String,
    pub is_reminder: bool,
}

impl NewEmailConfig {
    pub async fn insert<'q, D>(self, db: D) -> Result<EmailConfig, sqlx::error::Error>
    where
        D: sqlx::Executor<'q, Database = Postgres>
    {
        sqlx::query_as!(EmailConfig, "INSERT INTO email_configs (nation_id, game_id, hours_before_host, email_address, subject, body, is_reminder) VALUES($1, $2, $3, $4, $5, $6, $7) RETURNING *", self.nation_id, self.game_id, self.hours_before_host, self.email_address, self.subject, self.body, self.is_reminder).fetch_one(db).await
    }
}

#[derive(Debug)]
pub struct Disciple {
    pub id: i32,
    pub game_id: i32,
    pub nation_id: i32,
    pub is_disciple: i32,
    pub team: Option<i32>,
}

impl Disciple {
    pub async fn get<'q, D>(game_id: i32, nation_id: i32, db: D) -> Result<Disciple, sqlx::error::Error>
    where
        D: sqlx::Executor<'q, Database = Postgres>
    {
        sqlx::query_as!(Disciple, "
        INSERT INTO disciples (game_id, nation_id, is_disciple) VALUES($1, $2, 0) ON CONFLICT (game_id, nation_id) DO UPDATE SET nation_id=EXCLUDED.nation_id RETURNING *
        ", game_id, nation_id).fetch_one(db).await
    }
    pub async fn remove<'q, D>(&self, db: D) -> Result<Disciple, sqlx::error::Error>
    where
        D: sqlx::Executor<'q, Database = Postgres>
    {
        sqlx::query_as!(Disciple, "DELETE FROM disciples WHERE game_id=$1 AND nation_id=$2 RETURNING *", self.game_id, self.nation_id).fetch_one(db).await
    }

    pub async fn set_team<'q, D>(&self, team: Option<i32>, db: D) -> Result<Disciple, sqlx::error::Error>
    where
        D: sqlx::Executor<'q, Database = Postgres>
    {
        sqlx::query_as!(Disciple, "
        INSERT INTO disciples (game_id, nation_id, is_disciple, team)
        SELECT $1, $2, $4, COALESCE($3, (SELECT MAX(team) + 1 FROM disciples WHERE game_id=$1))
        ON CONFLICT (game_id, nation_id) DO UPDATE SET team=EXCLUDED.team, is_disciple=EXCLUDED.is_disciple
        RETURNING *
        ", self.game_id, self.nation_id, team, self.is_disciple).fetch_one(db).await
    }
    pub async fn unset_team<'q, D>(&self, db: D) -> Result<Disciple, sqlx::error::Error>
    where
        D: sqlx::Executor<'q, Database = Postgres>
    {
        sqlx::query_as!(Disciple, "UPDATE disciples SET team=NULL WHERE game_id=$1 AND nation_id=$2 RETURNING *", self.game_id, self.nation_id).fetch_one(db).await
    }
    pub async fn set_disc<'q, D>(&self, is_disc: i32, db: D) -> Result<Disciple, sqlx::error::Error>
    where
        D: sqlx::Executor<'q, Database = Postgres>
    {
        sqlx::query_as!(Disciple, "UPDATE disciples SET is_disciple=$3 WHERE game_id=$1 AND nation_id=$2 RETURNING *", self.game_id, self.nation_id, is_disc).fetch_one(db).await
    }
}

pub struct NewDisciple {
    game_id: i32,
    nation_id: i32,
    is_disciple: i32,
    team: Option<i32>,
}

impl NewDisciple {
    pub async fn insert<'q, D>(&self, db: D) -> Result<Disciple, sqlx::error::Error>
    where
        D: sqlx::Executor<'q, Database = Postgres>
    {
        sqlx::query_as!(Disciple, "INSERT INTO disciples (game_id, nation_id, is_disciple, team) VALUES ($1, $2, $3, $4) RETURNING *", self.game_id, self.nation_id, self.is_disciple, self.team).fetch_one(db).await
    }
}

#[derive(Debug)]
pub struct AdminLog {
    id: i32,
    pub game_id: i32,
    pub datetime: NaiveDateTime,
    pub action: String,
}

impl AdminLog {
    pub async fn get_all<'q, D>(game_id: i32, db: D) -> Result<Vec<AdminLog>, sqlx::error::Error>
    where
        D: sqlx::Executor<'q, Database = Postgres>
    {
        sqlx::query_as!(AdminLog, "SELECT * FROM admin_logs WHERE game_id=$1 ORDER BY datetime DESC", game_id).fetch_all(db).await
    }
    pub fn datetime_string(&self) -> String {
        self.datetime.to_string()
    }
}

pub struct NewAdminLog {
    pub game_id: i32,
    pub datetime: NaiveDateTime,
    pub action: String,
}
impl NewAdminLog {
    pub async fn insert<'q, D>(&self, db: D) -> Result<AdminLog, sqlx::error::Error>
    where
        D: sqlx::Executor<'q, Database = Postgres>
    {
        sqlx::query_as!(AdminLog, "INSERT INTO admin_logs (game_id, datetime, action) VALUES ($1, $2, $3) RETURNING *", self.game_id, self.datetime, self.action).fetch_one(db).await
    }
}

#[derive(Debug)]
pub struct DiscordConfig {
    pub id: i32,
    pub game_id: i32,
    pub last_turn_notified: Option<i32>,
    pub discord_guildid: String,
    pub discord_channelid: String,
    pub message: String,
    pub hours_remaining: Option<i32>,
}

impl DiscordConfig {
    pub async fn get_notifications<'q, D>(
        game_id: i32,
        db: D,
    ) -> Result<Vec<DiscordConfig>, sqlx::error::Error>
    where
        D: sqlx::Executor<'q, Database = Postgres>
    {
        sqlx::query_as!(DiscordConfig, "SELECT * FROM discord_configs WHERE game_id=$1", game_id).fetch_all(db).await
    }
}

#[derive( Debug)]
pub struct MapProvince {
    pub id: i32,
    pub map_id: i32,
    pub province_id: i32,
    pub x: i32,
    pub y: i32,
    pub unit_x: f64,
    pub unit_y: f64,
}

pub struct NewMapProvince {
    pub map_id: i32,
    pub province_id: i32,
    pub x: i32,
    pub y: i32,
    pub unit_x: f64,
    pub unit_y: f64,
}
