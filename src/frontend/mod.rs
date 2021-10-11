use diesel::r2d2::ConnectionManager;
use std::env;
use diesel::PgConnection;
use crate::msgbus::MsgBusTx;
use crate::discord::DiscordManager;
use crate::email_manager::EmailManager;

pub mod mods;
pub mod maps;
pub mod games;
pub mod utils;
pub mod help;

#[derive(Clone)]
pub struct AppData {
    pub pool: r2d2::Pool<ConnectionManager<PgConnection>>,
    pub msgbus_sender: MsgBusTx,
    pub email_manager: EmailManager,
    pub discord_manager: Option<DiscordManager>,
}

impl AppData {
    pub fn get_antibot_question() -> Option<String> {
        return env::var("ANTIBOT_QUESTION").ok()
    }
    pub fn get_antibot_answer() -> Option<String> {
        return env::var("ANTIBOT_ANSWER").ok()
    }
}

