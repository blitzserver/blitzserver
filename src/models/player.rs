use super::{File, Game, Nation, NewAdminLog};
use crate::diesel::RunQueryDsl;
use crate::diesel::{ExpressionMethods, QueryDsl};
use crate::files::saves::{DomSaveReadError, SaveFile};
use crate::schema::players;
use thiserror::Error;
#[derive(Identifiable, Queryable, Associations, PartialEq, Debug)]
#[belongs_to(Game)]
#[table_name = "players"]
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
    pub fn set_start<D>(&self, province: i32, db: &D) -> Result<Player, diesel::result::Error>
    where
        D: diesel::Connection<Backend = diesel::pg::Pg>,
    {
        use crate::schema::players::dsl as pdsl;
        diesel::update(pdsl::players.filter(pdsl::id.eq(self.id)))
            .set(pdsl::startprov.eq(province))
            .get_result(db)
    }
    pub fn get_players<D>(game_id: i32, db: &D) -> Result<Vec<Player>, diesel::result::Error>
    where
        D: diesel::Connection<Backend = diesel::pg::Pg>,
    {
        use crate::schema::players::dsl as players_dsl;
        players_dsl::players
            .filter(players_dsl::game_id.eq(game_id))
            .get_results(db)
    }

    pub fn get_newlord<D>(&self, db: &D) -> Result<File, diesel::result::Error>
    where
        D: diesel::Connection<Backend = diesel::pg::Pg>,
    {
        use crate::schema::files::dsl as files_dsl;
        files_dsl::files
            .filter(files_dsl::id.eq(self.file_id))
            .get_result(db)
    }

    pub fn remove<D>(&self, db: &D) -> Result<(), diesel::result::Error>
    where
        D: diesel::Connection<Backend = diesel::pg::Pg>,
    {
        use crate::schema::players::dsl as p_dsl;
        diesel::delete(p_dsl::players)
            .filter(p_dsl::id.eq(self.id))
            .execute(db)?;
        Ok(())
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

#[derive(Insertable)]
#[table_name = "players"]
pub struct NewPlayer {
    pub nationid: i32,
    pub game_id: i32,
    pub file_id: i32,
    pub name: String,
    pub password: Option<String>,
}

impl NewPlayer {
    pub fn insert<D>(&self, db: &D) -> Result<Player, crate::diesel::result::Error>
    where
        D: diesel::Connection<Backend = diesel::pg::Pg>,
    {
        use crate::schema::players::dsl as pdsl;
        diesel::insert_into(pdsl::players)
            .values(self)
            .on_conflict((pdsl::game_id, pdsl::nationid))
            .do_update()
            .set((
                pdsl::file_id.eq(self.file_id),
                pdsl::name.eq(&self.name),
                pdsl::password.eq(&self.password),
            ))
            .get_result(db)
    }
}
