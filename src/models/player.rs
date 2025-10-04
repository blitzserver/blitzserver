use super::{File, Game};
use crate::files::saves::{DomSaveReadError, SaveFile};
use sqlx::Sqlite;
use thiserror::Error;
#[derive(PartialEq, Debug)]
pub struct Player {
    pub id: i64,
    pub nationid: i32,
    pub game_id: i64,
    pub file_id: i64,
    pub name: String,
    pub password: Option<String>,
    pub startprov: Option<i32>,
    pub cdkey: Option<i64>,
}

#[derive(Debug, Error)]
pub enum PlayerErr {
    #[error("Corrupt pretender file")]
    CorruptPretenderError(DomSaveReadError),
    #[error("Database error")]
    DbError(#[from] sqlx::Error),
}

impl Player {
    pub async fn set_start<'q, D>(&self, province: i32, db: D) -> Result<Player, sqlx::error::Error>
    where
        D: sqlx::Executor<'q, Database = Sqlite>,
    {
        let result = sqlx::query!(
            r#"UPDATE players SET startprov=$2 WHERE id=$1 RETURNING
                id "id!",
                nationid "nationid: i32",
                game_id "game_id!",
                file_id "file_id!",
                name,
                password,
                startprov "startprov: i32",
                cdkey"#,
            self.id,
            province
        )
        .fetch_one(db)
        .await?;
        Ok(Player {
            id: result.id,
            nationid: result.nationid,
            game_id: result.game_id,
            file_id: result.file_id,
            name: result.name,
            password: result.password,
            startprov: result.startprov,
            cdkey: result.cdkey,
        })
    }
    pub async fn get_players<'q, D>(game_id: i32, db: D) -> Result<Vec<Player>, sqlx::error::Error>
    where
        D: sqlx::Executor<'q, Database = Sqlite>,
    {
        let results = sqlx::query!(r#"SELECT
            id,
            nationid "nationid: i32",
            game_id,
            file_id,
            name,
            password,
            startprov "startprov: i32",
            cdkey
            FROM players WHERE game_id=$1"#, game_id)
            .fetch_all(db)
            .await?;
        Ok(results
            .into_iter()
            .map(|result| Player {
                id: result.id,
                nationid: result.nationid,
                game_id: result.game_id,
                file_id: result.file_id,
                name: result.name,
                password: result.password,
                startprov: result.startprov,
                cdkey: result.cdkey,
            })
            .collect())
    }

    pub async fn get_newlord<'q, D>(&self, db: D) -> Result<File, sqlx::error::Error>
    where
        D: sqlx::Executor<'q, Database = Sqlite>,
    {
        sqlx::query_as!(File, r#"SELECT
            id "id: i32",
            filename,
            filebinary
        FROM files WHERE id=$1"#, self.file_id)
            .fetch_one(db)
            .await
    }

    pub async fn remove<'q, D>(&self, db: D) -> Result<Player, sqlx::error::Error>
    where
        D: sqlx::Executor<'q, Database = Sqlite>,
    {
        let result = sqlx::query!(r#"DELETE FROM players WHERE id=$1 RETURNING
            id "id!",
            nationid "nationid: i32",
            game_id "game_id!",
            file_id "file_id!",
            name,
            password,
            startprov "startprov: i32",
            cdkey"#, self.id)
            .fetch_one(db)
            .await?;
        Ok(Player {
            id: result.id,
            nationid: result.nationid as i32,
            game_id: result.game_id,
            file_id: result.file_id,
            name: result.name,
            password: result.password,
            startprov: result.startprov.map(|x| x as i32),
            cdkey: result.cdkey,
        })
    }

    pub fn get_cdkey(
        game_id: i32,
        nation_id: i32,
        db_pool: &sqlx::sqlite::SqlitePool,
    ) -> Result<Option<u32>, PlayerErr> {
        smol::block_on(async {
            match sqlx::query!(
                r#"SELECT id, cdkey "cdkey: u32", file_id FROM players WHERE game_id=$1 AND nationid=$2"#,
                game_id,
                nation_id
            )
            .fetch_one(db_pool)
            .await
            {
                Ok(record) => match record.cdkey {
                    Some(cdkey) => Ok(Some(cdkey)),
                    None => {
                        let file_record = sqlx::query!(
                            "SELECT filebinary FROM files WHERE id=$1",
                            record.file_id
                        )
                        .fetch_one(db_pool)
                        .await?;
                        let save_file =
                            SaveFile::read_contents(std::io::Cursor::new(file_record.filebinary))
                                .map_err(PlayerErr::CorruptPretenderError)?;
                        sqlx::query!(
                            "UPDATE players SET cdkey=$1 WHERE id=$2",
                            save_file.header.cdkey,
                            record.id
                        )
                        .execute(db_pool)
                        .await?;
                        Ok(Some(save_file.header.cdkey))
                    }
                },
                Err(_) => Ok(None),
            }
        })
    }
}

pub struct NewPlayer {
    pub nationid: i32,
    pub game_id: i64,
    pub file_id: i64,
    pub name: String,
    pub password: Option<String>,
}

impl NewPlayer {
    pub async fn insert<'q, D>(&self, db: D) -> Result<Player, crate::sqlx::error::Error>
    where
        D: sqlx::Executor<'q, Database = Sqlite>,
    {
        let result = sqlx::query!(r#"INSERT INTO players (nationid, game_id, file_id, name, password) VALUES ($1, $2, $3, $4, $5) RETURNING
            id "id!",
            nationid "nationid: i32",
            game_id "game_id!",
            file_id "file_id!",
            name,
            password,
            startprov "startprov: i32",
            cdkey"#, self.nationid, self.game_id, self.file_id, self.name, self.password).fetch_one(db).await?;
        Ok(Player {
            id: result.id,
            nationid: result.nationid as i32,
            game_id: result.game_id,
            file_id: result.file_id,
            name: result.name,
            password: result.password,
            startprov: result.startprov.map(|x| x as i32),
            cdkey: result.cdkey,
        })
    }
}
