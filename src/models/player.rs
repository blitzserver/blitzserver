use super::{File};
use crate::files::saves::{DomSaveReadError, SaveFile};
use sqlx::Postgres;
use thiserror::Error;
#[derive(PartialEq, Debug)]
pub struct Player {
    pub id: i32,
    pub nationid: i32,
    pub game_id: i32,
    pub file_id: i32,
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
    pub async fn set_start<'q,D>(&self, province: i32, db: D) -> Result<Player, sqlx::error::Error>
    where
        D: sqlx::Executor<'q, Database = Postgres>
    {
        sqlx::query_as!(Player, "UPDATE players SET startprov=$2 WHERE id=$1 RETURNING *", self.id, province).fetch_one(db).await
    }
    pub async fn get_players<'q, D>(game_id: i32, db: D) -> Result<Vec<Player>, sqlx::error::Error>
    where
        D: sqlx::Executor<'q, Database = Postgres>
    {
        sqlx::query_as!(Player, "SELECT * FROM players WHERE game_id=$1", game_id).fetch_all(db).await
    }

    pub async fn get_newlord<'q, D>(&self, db: D) -> Result<File, sqlx::error::Error>
    where
        D: sqlx::Executor<'q, Database = Postgres>
    {
        sqlx::query_as!(File, "SELECT * FROM files WHERE id=$1", self.file_id).fetch_one(db).await
    }

    pub async fn remove<'q, D>(&self, db: D) -> Result<Player, sqlx::error::Error>
    where
        D: sqlx::Executor<'q, Database = Postgres>
    {
        sqlx::query_as!(Player, "DELETE FROM players WHERE id=$1 RETURNING *", self.id).fetch_one(db).await
    }

    pub fn get_cdkey(
        game_id: i32,
        nation_id: i32,
        db_pool: &sqlx::postgres::PgPool,
    ) -> Result<Option<i64>, PlayerErr> {
        smol::block_on(async {
            match sqlx::query!(
                "SELECT id, cdkey, file_id FROM players WHERE game_id=$1 AND nationid=$2",
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
                            save_file.header.cdkey as i64,
                            record.id
                        )
                        .execute(db_pool)
                        .await?;
                        Ok(Some(save_file.header.cdkey as i64))
                    }
                },
                Err(_) => Ok(None),
            }
        })
    }
}

pub struct NewPlayer {
    pub nationid: i32,
    pub game_id: i32,
    pub file_id: i32,
    pub name: String,
    pub password: Option<String>,
}

impl NewPlayer {
    pub async fn insert<'q, D>(&self, db: D) -> Result<Player, ::sqlx::error::Error>
    where
        D: sqlx::Executor<'q, Database = Postgres>
    {
        sqlx::query_as!(Player, "INSERT INTO players (nationid, game_id, file_id, name, password) VALUES ($1, $2, $3, $4, $5) RETURNING *", self.nationid, self.game_id, self.file_id, self.name, self.password).fetch_one(db).await
    }
}
