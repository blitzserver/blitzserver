use std::env;

pub mod mods;
pub mod maps;
pub mod games;
pub mod utils;
pub mod help;
pub mod settings;
pub mod binaries;
pub mod fragments;
pub mod api;

#[derive(Clone)]
pub struct AppData {
    pub sqlx_pool: sqlx::sqlite::SqlitePool,
}

impl AppData {
    pub fn get_antibot_question() -> Option<String> {
        return env::var("ANTIBOT_QUESTION").ok()
    }
    pub fn get_antibot_answer() -> Option<String> {
        return env::var("ANTIBOT_ANSWER").ok()
    }
}

