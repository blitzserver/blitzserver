use std::env;

pub mod mods;
pub mod maps;
pub mod games;
pub mod utils;
pub mod help;
pub mod settings;
pub mod fragments;
pub mod api;

use crate::models::settings::Setting;

#[derive(Clone)]
pub struct AppData {
    pub sqlx_pool: sqlx::sqlite::SqlitePool,
}

impl AppData {
    pub async fn get_antibot_question(&self) -> Option<String> {
        Setting::get_or_env(&self.sqlx_pool, "ANTIBOT_QUESTION").await
    }
    pub async fn get_antibot_answer(&self) -> Option<String> {
        Setting::get_or_env(&self.sqlx_pool, "ANTIBOT_ANSWER").await
    }
}

