use std::path::{Path, PathBuf};

use futures::executor;
use sqlx::{Executor, Postgres};
use zip::ZipArchive;

pub struct Binary {
    pub binary_id: i32,
    pub executable_path: String,
    pub name: String,
}

impl Binary {
    pub async fn new<'q, D>(
        name: &str,
        filename: &str,
        contents: &[u8],
        executable_path: &str,
        db: D,
    ) -> Result<Self, sqlx::error::Error>
    where
        D: Executor<'q, Database = Postgres>,
    {
        sqlx::query_file_as!(
            Binary,
            "src/sql/insert_binary.sql",
            filename,
            contents,
            executable_path,
            name
        )
        .fetch_one(db)
        .await
    }
    pub async fn get_all<'q, D>(db: D) -> Result<Vec<Self>, sqlx::error::Error>
    where
        D: Executor<'q, Database = Postgres>,
    {
        sqlx::query_file_as!(Binary, "src/sql/get_binaries.sql")
            .fetch_all(db)
            .await
    }

    pub async fn extract_to_tmp<'q, D>(game_id: i32, db: D) -> Result<PathBuf, sqlx::error::Error>
    where
        D: Executor<'q, Database = Postgres>,
    {
        let binary = sqlx::query_file!("src/sql/get_binary_contents.sql", game_id)
            .fetch_one(db)
            .await
            .unwrap();
        let binary_path = format!("/tmp/{}", binary.name);
        if !Path::new(&binary_path).exists() {
            let mut zip = ZipArchive::new(std::io::Cursor::new(&binary.filebinary)).unwrap();
            zip.extract(Path::new(&binary_path)).unwrap();
        }

        return Ok(PathBuf::from(&binary_path).join(std::path::Path::new(&binary.executable_path)));
    }
}
