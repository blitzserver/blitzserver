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

    pub async fn get(pool: &SqlitePool, key: &str) -> Result<Option<String>, sqlx::Error> {
        let result = sqlx::query!(
            r#"SELECT value as "value: String" FROM settings WHERE key = ?"#,
            key
        )
        .fetch_optional(pool)
        .await?;

        Ok(result.and_then(|r| r.value))
    }

    pub async fn get_or_env(pool: &SqlitePool, key: &str) -> Option<String> {
        // First try database
        if let Ok(Some(value)) = Self::get(pool, key).await {
            return Some(value);
        }
        // Fall back to environment variable
        std::env::var(key).ok()
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
