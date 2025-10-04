use sqlx::{FromRow, SqlitePool};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Setting {
    pub key: String,
    pub value: Option<String>,
}

impl Setting {
    pub async fn get_all(pool: &SqlitePool) -> Result<Vec<Setting>, sqlx::Error> {
        sqlx::query_as!(
            Setting,
            r#"SELECT key "key!", value "value: String" FROM settings ORDER BY key"#
        )
        .fetch_all(pool)
        .await
    }

    pub async fn update(pool: &SqlitePool, key: &str, value: Option<&str>) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE settings SET value = ? WHERE key = ?",
            value,
            key
        )
        .execute(pool)
        .await?;
        Ok(())
    }
}
