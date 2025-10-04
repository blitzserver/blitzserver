use std::path::{Path, PathBuf};

use sqlx::{Executor, Sqlite};
use zip::ZipArchive;

pub struct Binary {
    pub binary_id: i32,
    pub executable_path: String,
    pub version: String,
    pub version_number: i32,
    pub foldered_maps: bool,
}

impl Binary {
    pub async fn new(
        version: &str,
        filename: &str,
        contents: &[u8],
        executable_path: &str,
        db: &sqlx::sqlite::SqlitePool,
    ) -> Result<Self, sqlx::error::Error> {
        let mut tx = db.begin().await?;

        // Insert file first
        let file = sqlx::query!(
            "INSERT INTO files (filename, filebinary) VALUES($1, $2) RETURNING *",
            filename,
            contents
        )
        .fetch_one(&mut *tx)
        .await?;

        println!("Version {}", &version);
        let version_regex = regex::Regex::new("([.0-9]+)").unwrap();
        let version_number: String = version_regex
            .captures(&version)
            .unwrap()
            .get(1)
            .unwrap()
            .as_str()
            .to_owned()
            .replace(".", "");
        println!("Version number {} {}", &version, &version_number);
        let version_number: i32 = version_number.parse().unwrap();
        let foldered_maps = version_number > 561;

        // Insert binary
        let binary = sqlx::query_as!(
            Binary,
            r#"INSERT INTO binaries (binary_id, executable_path, version, version_number, foldered_maps)
            VALUES($1, $2, $3, $4, $5)
            ON CONFLICT(binary_id) DO UPDATE SET version=EXCLUDED.version
            RETURNING
                binary_id "binary_id: i32",
                executable_path,
                version,
                version_number "version_number: i32",
                foldered_maps"#,
            file.id,
            executable_path,
            version,
            version_number,
            foldered_maps
        )
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(binary)
    }
    pub async fn get_all<'q, D>(db: D) -> Result<Vec<Self>, sqlx::error::Error>
    where
        D: Executor<'q, Database = Sqlite>,
    {
        sqlx::query_file_as!(Binary, "src/sql/get_binaries.sql")
            .fetch_all(db)
            .await
    }

    pub async fn extract_to_tmp<'q, D>(game_id: i32, db: &D) -> Result<PathBuf, sqlx::error::Error>
    where
        for<'a> &'a D: Executor<'q, Database = Sqlite>,
    {
        let binary = sqlx::query_file!("src/sql/get_binary_version.sql", game_id)
            .fetch_one(db)
            .await
            .unwrap();
        let binary_path = std::env::current_dir()
            .unwrap()
            .join("tmp")
            .join("binaries")
            .join(binary.version);
        if !binary_path.exists() {
            let binary = sqlx::query_file!("src/sql/get_binary_contents.sql", game_id)
                .fetch_one(db)
                .await
                .unwrap();
            let mut zip = ZipArchive::new(std::io::Cursor::new(&binary.filebinary)).unwrap();
            zip.extract(Path::new(&binary_path)).unwrap();

            // Set executable permission on dom6_amd64 for Windows-uploaded binaries
            let executable_path = binary_path.join(&binary.executable_path);
            if executable_path.exists() {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mut perms = std::fs::metadata(&executable_path).unwrap().permissions();
                    perms.set_mode(perms.mode() | 0o111); // Add execute permission for owner, group, and others
                    std::fs::set_permissions(&executable_path, perms).unwrap();
                }
            }
        }

        return Ok(PathBuf::from(&binary_path).join(std::path::Path::new(&binary.executable_path)));
    }
}
