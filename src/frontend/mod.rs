use std::env;
use crate::msgbus::MsgBusTx;
use crate::email_manager::EmailManager;

pub mod mods;
pub mod maps;
pub mod games;
pub mod utils;
pub mod help;
pub mod binaries;
pub mod fragments;
pub mod api;

#[derive(Clone)]
pub struct AppData {
    pub sqlx_pool: sqlx::postgres::PgPool,
    pub msgbus_sender: MsgBusTx,
    pub email_manager: EmailManager,
}

impl AppData {
    pub fn get_antibot_question() -> Option<String> {
        env::var("ANTIBOT_QUESTION").ok()
    }
    pub fn get_antibot_answer() -> Option<String> {
        env::var("ANTIBOT_ANSWER").ok()
    }
}

