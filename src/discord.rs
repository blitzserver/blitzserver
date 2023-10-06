use crate::models::DiscordConfig;
use crate::msgbus::{MsgBusRx, MsgBusTx};
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Clone)]
pub struct DiscordManager {
    bot_token: String,
    client: Option<reqwest::blocking::Client>,
    pub client_id: String,
    session_id: Option<String>,
    seq: Option<i32>,
    bus_tx: MsgBusTx,
    channel_cache: Arc<Mutex<HashMap<String, String>>>,
    server_cache: Arc<Mutex<HashMap<String, String>>>,
}

#[derive(Debug, Deserialize)]
struct DiscordChannel {
    id: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct DiscordGuild {
    id: String,
    name: String,
}

#[derive(Error, Debug)]
pub enum DiscordManagerError {
    #[error("No DISCORD_CLIENTID")]
    NoClientId,
    #[error("No DISCORD_CLIENTSECRET")]
    NoClientSecret,
    #[error("No DISCORD_BOTTOKEN")]
    NoBotToken,
    #[error("Unable to authenticate")]
    UnableToAuth(#[from] reqwest::Error),
    #[error("Unable to connect to Discord gateway")]
    GatewayUrlParseError(#[from] url::ParseError),
    #[error("Unable to connect to Discord gateway")]
    GatewayConnectionError(#[from] tungstenite::Error),
}

#[derive(Debug, Deserialize)]
struct DiscordUser {
    id: String,
    username: String,
}

#[derive(Debug, Deserialize)]
struct IdentifyResponsePayload {
    user: DiscordUser,
    session_id: String,
}
#[derive(Debug, Deserialize)]
struct MessagePayload {
    content: String,
    channel_id: String,
    author: DiscordUser,
    guild_id: String,
}

#[derive(Debug, Deserialize)]
struct HelloPayload {
    heartbeat_interval: i32,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ProtocolMsg {
    IdentifyResponse {
        s: Option<i32>,
        op: i32,
        d: IdentifyResponsePayload,
    },
    Message {
        s: Option<i32>,
        op: i32,
        d: MessagePayload,
    },
    Hello {
        s: Option<i32>,
        op: i32,
        d: HelloPayload,
    },
    UnkResponse {
        op: i32,
    },
}

static HELP_MESSAGE: &'static str = r#"
Get my attention by @ing me, the following requests are available:

Setup a new reminder:
**`@me reminder <gamename> <hours to host> <message>`**
Examples:
- `@me reminder Blitz 6 There are six hours remaining to submit turns`
- `@me reminder Blitz 12 @Player1, @Player2, Player3 there are 12 hours remaining to submit turns`

Setup a new notification:
**`@me notification <gamename> <message>`**
Examples:
- `@me notification Blitz @Player1, @Player2 there is a new turn for Blitz`
"#;

impl DiscordManager {
    pub fn new(
        bus_tx: MsgBusTx,
    ) -> Result<Self, DiscordManagerError> {
        let bot_token = env::var_os("DISCORD_BOTTOKEN")
            .ok_or(DiscordManagerError::NoBotToken)?
            .to_str()
            .ok_or(DiscordManagerError::NoBotToken)?
            .to_owned();
        let client_id = env::var_os("DISCORD_CLIENTID")
            .ok_or(DiscordManagerError::NoClientId)?
            .to_str()
            .ok_or(DiscordManagerError::NoClientId)?
            .to_owned();
        Ok(Self {
            client_id,
            client: None,
            bot_token,
            session_id: None,
            seq: None,
            bus_tx,
            channel_cache: Arc::new(Mutex::new(HashMap::new())),
            server_cache: Arc::new(Mutex::new(HashMap::new())),
        })
    }
    pub fn monitor_bus(mut self, bus_rx: MsgBusRx) -> Result<(), DiscordManagerError> {
        unimplemented!("Fuck off");
    }

    pub fn get_channel_name(&self, server_id: &str, channel_id: &str) -> String {
        unimplemented!("Fuck off");
    }

    pub fn get_server_name(&self, server_id: &str) -> String {
        unimplemented!("Fuck off");
    }

    pub fn monitor_discord(mut self) -> Result<(), DiscordManagerError> {
        unimplemented!("Fuck off");
    }
}
