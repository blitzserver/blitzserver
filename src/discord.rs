use crate::models::DiscordConfig;
use crate::models::{Game, NewDiscordNotification, NewDiscordReminder};
use crate::msgbus::{DiscordConfigCreatedMsg, Msg, MsgBusRx, MsgBusTx};
use diesel::r2d2::ConnectionManager;
use diesel::PgConnection;
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::env;
use std::sync::{Arc, Mutex};
use thiserror::Error;
use tungstenite::{connect, Message};
use url::Url;

#[derive(Clone)]
pub struct DiscordManager {
    bot_token: String,
    client: Option<reqwest::blocking::Client>,
    pub client_id: String,
    db_pool: r2d2::Pool<ConnectionManager<PgConnection>>,
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
    fn send_message(&mut self, channel: &str, content: &str) -> Result<(), DiscordManagerError> {
        reqwest::blocking::Client::new()
            .post(format!(
                "https://discord.com/api/channels/{}/messages",
                channel
            ))
            .header("Authorization", format!("Bot {}", self.bot_token.clone()))
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(&json!({ "content": content })).unwrap())
            .send()?;
        Ok(())
    }

    fn connect(
        &mut self,
    ) -> Result<tungstenite::WebSocket<tungstenite::client::AutoStream>, DiscordManagerError> {
        let (mut socket, _) = connect(Url::parse("wss://gateway.discord.gg/?v=8&encoding=json")?)?;
        socket.write_message(Message::Text(
            json!({
                "op": 2,
                "d": {
                    "token": &self.bot_token,
                    "intents": 1 << 9, // GUILD_MESSAEGS
                    "properties": {
                        "$os": "linux",
                        "$browser": "blitzserver",
                        "$device": "blitzserver"
                    }
                }
            })
            .to_string(),
        ))?;
        Ok(socket)
    }

    fn resume(
        &mut self,
    ) -> Result<tungstenite::WebSocket<tungstenite::client::AutoStream>, DiscordManagerError> {
        let (mut socket, _) = connect(Url::parse("wss://gateway.discord.gg/?v=8&encoding=json")?)?;
        socket.write_message(Message::Text(
            json!({
                "op": 6,
                "d": {
                    "token": &self.bot_token,
                    "session_id": &(self.session_id.as_ref().unwrap()),
                    "seq": self.seq
                }
            })
            .to_string(),
        ))?;
        Ok(socket)
    }

    pub fn send_notice(&mut self, notice: &DiscordConfig) -> Result<(), DiscordManagerError> {
        self.send_message(&notice.discord_channelid, &notice.message)?;
        Ok(())
    }

    pub fn new(
        bus_tx: MsgBusTx,
        db_pool: r2d2::Pool<ConnectionManager<PgConnection>>,
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
            db_pool,
            session_id: None,
            seq: None,
            bus_tx,
            channel_cache: Arc::new(Mutex::new(HashMap::new())),
            server_cache: Arc::new(Mutex::new(HashMap::new())),
        })
    }
    pub fn monitor_bus(mut self, bus_rx: MsgBusRx) -> Result<(), DiscordManagerError> {
        std::thread::spawn(move || loop {
            {
                let db = self.db_pool.get().unwrap();
                for notification in DiscordConfig::get_due_notifications(&db).unwrap() {
                    log::debug!("Send notification");
                    if let Err(e) = self.send_notice(&notification) {
                        log::error!("Failed to send notification of for game {}, {:?}", notification.game_id, e);
                    }
                    notification.mark_sent(&db).unwrap();
                }
                for notification in DiscordConfig::get_due_reminders(&db).unwrap() {
                    if let Err(e) = self.send_notice(&notification) {
                        log::error!("Failed to send reminder for game {}, {:?}", notification.game_id, e);
                    }
                    notification.mark_sent(&db).unwrap();
                }
            }
            let wakeup = {
                let db = self.db_pool.get().unwrap();
                let now = std::time::SystemTime::now();
                DiscordConfig::get_reminders_wakeup(&db)
                    .unwrap_or(None)
                    .unwrap_or(now)
                    .duration_since(now)
                    .unwrap_or(std::time::Duration::from_nanos(1))
            };
            match bus_rx.recv_timeout(wakeup) {
                _ => {}
            }
        });
        Ok(())
    }

    pub fn get_channel_name(&self, server_id: &str, channel_id: &str) -> String {
        if let Some(d) = self
            .channel_cache
            .lock()
            .unwrap()
            .get(&format!("{}:{}", server_id, channel_id))
            .map(|s| s.to_owned())
        {
            return d;
        }
        if let Ok(res) = reqwest::blocking::Client::new()
            .get(format!(
                "https://discord.com/api/guilds/{}/channels",
                server_id
            ))
            .header("Authorization", format!("Bot {}", self.bot_token.clone()))
            .header("Content-Type", "application/json")
            .send()
        {
            let body = res.text().unwrap();
            if let Ok(res) = serde_json::from_str::<Vec<DiscordChannel>>(&body) {
                let mut cache = self.channel_cache.lock().unwrap();
                for channel in res {
                    cache.insert(format!("{}:{}", server_id, channel.id), channel.name);
                }
                drop(cache);
                self.get_channel_name(server_id, channel_id)
            } else {
                let mut cache = self.channel_cache.lock().unwrap();
                cache.insert(format!("{}:{}", server_id, channel_id), "[Deleted]".to_string());
                "[Deleted]".to_owned()
            }
        } else {
            "Unable to fetch name".to_string()
        }
    }

    pub fn get_server_name(&self, server_id: &str) -> String {
        if let Some(d) = self
            .server_cache
            .lock()
            .unwrap()
            .get(server_id)
            .map(|s| s.to_owned())
        {
            return d;
        }
        if let Ok(res) = reqwest::blocking::Client::new()
            .get(format!("https://discord.com/api/guilds/{}", server_id))
            .header("Authorization", format!("Bot {}", self.bot_token.clone()))
            .header("Content-Type", "application/json")
            .send()
        {
            let body = res.text().unwrap();
            let res: DiscordGuild = serde_json::from_str(&body).unwrap_or(DiscordGuild {
                id: server_id.to_string(),
                name: "[Deleted]".to_owned(),
            });
            self.server_cache
                .lock()
                .unwrap()
                .insert(server_id.to_string(), res.name.clone());
            res.name
        } else {
            "Unable to fetch name".to_string()
        }
    }

    pub fn monitor_discord(mut self) -> Result<(), DiscordManagerError> {
        let mut socket = self.connect()?;
        std::thread::spawn(move || {
            let mut me = String::new();

            let reminder_regex: regex::Regex =
                regex::Regex::new(r#"reminder ([a-zA-Z0-9_\-]+) ([0-9]+) (.+)"#).unwrap();
            let notification_regex: regex::Regex =
                regex::Regex::new(r#"notification ([a-zA-Z0-9_\-]+) (.+)"#).unwrap();
            loop {
                let msg = match socket.read_message() {
                    Ok(Message::Text(t)) => t,
                    Err(_) => {
                        socket = self.connect().unwrap();
                        "".to_owned()
                    }
                    _ => "".to_owned(),
                };
                match serde_json::from_str(&msg) {
                    Ok(ProtocolMsg::IdentifyResponse { s, op: _op, d }) => {
                        if let Some(s) = s {
                            self.seq = Some(s);
                        }
                        me = d.user.id;
                        self.session_id = Some(d.session_id);
                    }
                    Ok(ProtocolMsg::Message { s, op: _op, d }) => {
                        if let Some(s) = s {
                            self.seq = Some(s);
                        }
                        if d.content.contains(&format!("<@!{}>", me)) {
                            if d.content.contains("help") {
                                self.send_message(
                                    &d.channel_id,
                                    &format!(
                                        "Hey, <@!{}>, here's some instructions:{}",
                                        d.author.id, HELP_MESSAGE
                                    ),
                                )
                                .unwrap();
                            } else if let Some(captures) = notification_regex.captures(&d.content) {
                                let db = self.db_pool.get().unwrap();
                                let game_name = captures.get(1).unwrap();
                                let message = captures.get(2).unwrap();
                                match Game::get_by_name(game_name.as_str(), &db) {
                                    Ok(game) => {
                                        match (NewDiscordNotification {
                                            game_id: game.id,
                                            discord_channelid: &d.channel_id,
                                            discord_guildid: &d.guild_id,
                                            message: message.as_str(),
                                        }
                                        .insert(&db))
                                        {
                                            Ok(dc) => {
                                                self.bus_tx
                                                    .send(Msg::DiscordConfigCreated(
                                                        DiscordConfigCreatedMsg { id: dc.id },
                                                    ))
                                                    .unwrap();
                                                self.send_message(
                                                    &d.channel_id,
                                                    &format!(
                                                    "<@!{}>, when there is a new turn for {} I will post the following in this channel:\n{}",
                                                    d.author.id, game.name, message.as_str()
                                                    ),
                                                ).unwrap();
                                            }
                                            Err(_) => {}
                                        }
                                    }
                                    Err(_) => {
                                        self.send_message(
                                            &d.channel_id,
                                            &format!(
                                                "<@!{}>, I can't find a game labeled {}, please check your spelling",
                                                d.author.id, game_name.as_str()
                                            ),
                                        ).unwrap();
                                    }
                                }
                            } else if let Some(captures) = reminder_regex.captures(&d.content) {
                                let db = self.db_pool.get().unwrap();
                                let game_name = captures.get(1).unwrap();
                                let hours_remaining =
                                    captures.get(2).unwrap().as_str().parse::<i32>().unwrap();
                                let message = captures.get(3).unwrap();
                                match Game::get_by_name(game_name.as_str(), &db) {
                                    Ok(game) => {
                                        match (NewDiscordReminder {
                                            game_id: game.id,
                                            discord_channelid: &d.channel_id,
                                            discord_guildid: &d.guild_id,
                                            message: message.as_str(),
                                            hours_remaining,
                                        }
                                        .insert(&db))
                                        {
                                            Ok(dc) => {
                                                self.bus_tx
                                                    .send(Msg::DiscordConfigCreated(
                                                        DiscordConfigCreatedMsg { id: dc.id },
                                                    ))
                                                    .unwrap();
                                                self.send_message(
                                                    &d.channel_id,
                                                    &format!(
                                                    "<@!{}>, when there are {} hours remaining to host a turn for {}, I will post the following in this channel:\n{}",
                                                    d.author.id, hours_remaining, game.name, message.as_str()
                                                    ),
                                                ).unwrap();
                                            }
                                            Err(_) => {}
                                        }
                                    }
                                    Err(_) => {
                                        self.send_message(
                                            &d.channel_id,
                                            &format!(
                                                "<@!{}>, I can't find a game labeled {}, please check your spelling",
                                                d.author.id, game_name.as_str()
                                            ),
                                        ).unwrap();
                                    }
                                }
                            }
                        }
                    }
                    Ok(ProtocolMsg::Hello {
                        s,
                        ..
                    }) => {
                        if let Some(s) = s {
                            self.seq = Some(s);
                        }
                        socket
                            .write_message(Message::Text(
                                json!({
                                    "op": 1,
                                    "d": self.seq,
                                })
                                .to_string(),
                            ))
                            .unwrap();
                    }
                    Ok(ProtocolMsg::UnkResponse { .. }) => {}
                    Err(_) => {
                        socket = self.resume().unwrap();
                    }
                }
            }
            // socket.close(None);
        });
        Ok(())
    }
}
