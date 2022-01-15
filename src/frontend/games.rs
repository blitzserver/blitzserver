use super::utils::*;
use std::io::Write;
use super::AppData;
use crate::diesel::prelude::*;
use std::env;

use crate::discord::DiscordManager;
use crate::dom5_turn::Dom5Turn;
use crate::models::{
    AdminLog, Disciple, DiscordConfig, EmailConfig, Game, GameLogLite, Map, MapProvince, Mod,
    Nation, NewAdminLog, NewGame, NewGameMod, Player, PlayerTurn, Turn,
};
use crate::msgbus::{GameScheduleMsg, Msg};
use crate::StartGame;
use actix_web::http::header;
use actix_web::{get, post, web, HttpResponse, Result};
use askama::Template;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::Add;

// === Payloads ===
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
struct GamesListPayload {
    antibot_failed: Option<i32>,
}
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
struct GameSettings {
    #[serde(default = "default_one")]
    #[serde(deserialize_with = "from_str")]
    era: i32,
    #[serde(default = "default_five")]
    #[serde(deserialize_with = "from_str")]
    thrones_t1: i32,
    #[serde(default)]
    #[serde(deserialize_with = "from_str")]
    thrones_t2: i32,
    #[serde(default)]
    #[serde(deserialize_with = "from_str")]
    thrones_t3: i32,
    #[serde(default = "default_five")]
    #[serde(deserialize_with = "from_str")]
    throne_points_required: i32,
    #[serde(default = "default_two")]
    #[serde(deserialize_with = "from_str")]
    research_diff: i32,
    #[serde(default = "default_one")]
    #[serde(deserialize_with = "from_str")]
    research_rand: i32,
    #[serde(default = "default_ten")]
    #[serde(deserialize_with = "from_str")]
    hof_size: i32,
    #[serde(default = "default_five")]
    #[serde(deserialize_with = "from_str")]
    global_size: i32,
    #[serde(default = "default_five")]
    #[serde(deserialize_with = "from_str")]
    indepstr: i32,
    #[serde(default = "default_forty")]
    #[serde(deserialize_with = "from_str")]
    magicsites: i32,
    #[serde(default = "default_two")]
    #[serde(deserialize_with = "from_str")]
    eventrarity: i32,
    #[serde(default = "default_hundred")]
    #[serde(deserialize_with = "from_str")]
    richness: i32,
    #[serde(default = "default_hundred")]
    #[serde(deserialize_with = "from_str")]
    resources: i32,
    #[serde(default = "default_hundred")]
    #[serde(deserialize_with = "from_str")]
    recruitment: i32,
    #[serde(default = "default_hundred")]
    #[serde(deserialize_with = "from_str")]
    supplies: i32,
    #[serde(default = "default_one")]
    #[serde(deserialize_with = "from_str")]
    startprov: i32,
    #[serde(default)]
    #[serde(deserialize_with = "from_str")]
    renaming: i32,
    #[serde(default = "default_one")]
    #[serde(deserialize_with = "from_str")]
    scoregraphs: i32,
    #[serde(default = "default_one")]
    #[serde(deserialize_with = "from_str")]
    nationinfo: i32,
    #[serde(default = "default_one")]
    #[serde(deserialize_with = "from_str")]
    artrest: i32,
    #[serde(default)]
    #[serde(deserialize_with = "from_str")]
    teamgame: i32,
    #[serde(default)]
    #[serde(deserialize_with = "from_str")]
    clustered: i32,
    #[serde(default = "default_one")]
    #[serde(deserialize_with = "from_str")]
    storyevents: i32,
    #[serde(default = "default_two")]
    #[serde(deserialize_with = "from_str")]
    newailvl: i32,
    #[serde(deserialize_with = "from_str")]
    #[serde(default = "default_one")]
    map: i32,
    #[serde(default)]
    mapfilter: String,
    #[serde(default)]
    #[serde(deserialize_with = "from_str_seq")]
    cmods: Vec<i32>,
    #[serde(default)]
    modfilter: String,
    masterpass: Option<String>,
    #[serde(default)]
    private: i32,
    #[serde(default)]
    require_password: i32,
    #[serde(default)]
    select_start: i32,
    cataclysm: Option<i32>,
    max_stales: Option<i32>,
    // Marker to tell whether the form is on its first load
    loaded: Option<String>,
}
impl From<(&Game, &[Mod])> for GameSettings {
    fn from((game, mods): (&Game, &[Mod])) -> GameSettings {
        GameSettings {
            era: game.era,
            thrones_t1: game.thrones_t1,
            thrones_t2: game.thrones_t2,
            thrones_t3: game.thrones_t3,
            throne_points_required: game.throne_points_required,
            research_diff: game.research_diff,
            research_rand: game.research_rand as i32,
            hof_size: game.hof_size,
            global_size: game.global_size,
            indepstr: game.indepstr,
            magicsites: game.magicsites,
            eventrarity: game.eventrarity,
            richness: game.richness,
            resources: game.resources,
            recruitment: game.recruitment,
            supplies: game.supplies,
            startprov: game.startprov,
            renaming: game.renaming as i32,
            scoregraphs: game.scoregraphs as i32,
            nationinfo: game.nationinfo as i32,
            artrest: game.artrest as i32,
            teamgame: game.teamgame as i32,
            clustered: game.clustered as i32,
            storyevents: game.storyevents,
            newailvl: game.newailvl,
            map: game.map_id,
            mapfilter: "".to_string(),
            modfilter: "".to_string(),
            cmods: mods.iter().map(|m| m.id).collect(),
            masterpass: game.masterpass.clone(),
            loaded: None,
            require_password: game.require_password as i32,
            select_start: game.select_start,
            private: game.private as i32,
            cataclysm: game.cataclysm,
            max_stales: game.max_stales,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmailForm {
    #[serde(default)]
    email_address: String,
    #[serde(default)]
    #[serde(deserialize_with = "from_str")]
    nation: i32,
    #[serde(default)]
    subject: String,
    #[serde(default)]
    body: String,
    #[serde(default)]
    #[serde(deserialize_with = "from_str")]
    hours_remaining: i32,
    #[serde(default)]
    is_reminder: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TeamForm {
    team: i32,
    nation: i32,
    disciple: i32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MotdForm {
    motd: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StartLocForm {
    province_start: i32,
    #[serde(default)]
    password: std::collections::HashMap<i32, String>,
}
impl Default for EmailForm {
    fn default() -> Self {
        Self {
            email_address: "".to_string(),
            nation: 0,
            subject: "".to_string(),
            body: "".to_string(),
            hours_remaining: 0,
            is_reminder: false,
        }
    }
}
#[derive(PartialEq, Debug, Serialize, Deserialize)]
enum AuthStatus {
    Unauthed = 0,
    AuthSuccess = 1,
    AuthFail = 2,
}
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
struct GameDetailsPayload {
    #[serde(flatten)]
    game_settings: GameSettings,
    #[serde(flatten)]
    email_form: EmailForm,
    password: Option<String>,
    log_errors: Option<i32>,
    log_output: Option<i32>,
}
#[derive(Debug, Deserialize)]
struct CreateGame {
    name: String,
    password: String,
    antibot: Option<String>,
}
#[derive(Debug, Deserialize)]
struct SetTimer {
    timer: i32,
}
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
struct EmailFormEntry {
    pub selected_nation: i32,
    pub remaining_hours: Option<i32>,
}
#[derive(Debug, Deserialize)]
pub struct ArchiveForm {
    archived: bool,
}
#[derive(Debug, Deserialize)]
pub struct RemovePlayerForm {
    password: Option<String>,
}
#[derive(Debug, Deserialize)]
pub struct DownloadTurnForm {
    #[serde(default)]
    password: String
}

#[derive(Debug)]
struct PlayerSummary {
    name: String,
    epithet: String,
    pretender_name: String,
    id: i32,
    status: i32,
    is_disciple: bool,
    team: i32,
    has_pass: bool,
}
#[derive(Template)]
#[template(path = "games/details.html")]
struct GameDetailsTemplate<'a> {
    game: Game,
    settings: GameSettings,
    email_form: EmailForm,
    email_configs: &'a [EmailConfig],
    turn_number: i32,
    turns: HashMap<i32, Vec<PlayerTurn>>,
    games_address: String,
    players: &'a Vec<Player>,
    disciples: &'a Vec<Disciple>,
    nations: &'a Vec<Nation>,
    mods: &'a Vec<Mod>,
    maps: &'a Vec<Map>,
    provinces: &'a Vec<MapProvince>,
    authed: AuthStatus,
    logs: Vec<GameLogLite>,
    focused_log: (i32, String, String),
    admin_logs: &'a Vec<AdminLog>,
    discord_clientid: Option<String>,
    discord_notifications: Vec<DiscordConfig>,
    discord_manager: Option<&'a DiscordManager>,
    staling_players: Vec<i32>,
    winner_name: Option<String>,
}
pub struct PNT<'a> {
    player: &'a Player,
    nation: &'a Nation,
}
impl<'a> GameDetailsTemplate<'a> {
    pub fn turn_color_class(turn_status: &i32) -> String {
        match turn_status {
            0 => "bg-unsubmitted".to_owned(),
            1 => "bg-unfinished".to_owned(),
            2 => "bg-submitted".to_owned(),
            3 => "bg-computer".to_owned(),
            4 => "bg-dead".to_owned(),
            5 => "bg-dead".to_owned(),
            _ => "bg-unsubmitted".to_owned(),
        }
    }
    pub fn nation_staling(&self, nation: &i32) -> bool {
        return self.staling_players.iter().find(|n| *n == nation).is_some();
    }
    pub fn motd_str(&self) -> String {
        self.game.motd.as_ref().unwrap_or(&"".to_owned()).to_owned()
    }
    pub fn cataclysm_str(&self) -> String {
        self.settings
            .cataclysm
            .map(|c| format!("{}", c))
            .unwrap_or("".to_owned())
    }
    pub fn max_stales_str(&self) -> String {
        self.settings
            .max_stales
            .map(|c| format!("{}", c))
            .unwrap_or("".to_owned())
    }
    pub fn get_player_at_start(&self, province_id: &i32) -> Option<PNT> {
        self.players
            .iter()
            .find(|p| match p.startprov {
                Some(prov) => {
                    if prov == *province_id {
                        true
                    } else {
                        false
                    }
                }
                None => false,
            })
            .map(|p| PNT {
                player: p,
                nation: self
                    .nations
                    .iter()
                    .find(|n| n.nation_id == p.nationid)
                    .unwrap(),
            })
    }
    pub fn get_channel_name(&self, server: &str, channel: &str) -> String {
        match self.discord_manager {
            None => "Discord not enabled".to_string(),
            Some(d) => d.get_channel_name(server, channel),
        }
    }
    pub fn get_server_name(&self, server: &str) -> String {
        match self.discord_manager {
            None => "Discord not enabled".to_string(),
            Some(d) => d.get_server_name(server),
        }
    }
    fn get_team_leaders(&self) -> Vec<(i32, PlayerSummary)> {
        let mut teams = self
            .disciples
            .iter()
            .filter_map(|d| {
                if d.team.unwrap_or(0) > 0 {
                    d.team
                } else {
                    None
                }
            })
            .collect::<Vec<i32>>();
        teams.sort_unstable();
        teams.dedup();
        let mut disc: Vec<(i32, PlayerSummary)> = teams
            .into_iter()
            .filter_map(|t| {
                match self
                    .disciples
                    .iter()
                    .find(|d| d.team.unwrap_or(0) == t && d.is_disciple == 0)
                {
                    Some(d) => Some((d.team.unwrap_or(0), self.get_player_summary(t, d.nation_id))),
                    None => Some((t, self.get_player_summary(t, 0))),
                }
            })
            .collect();
        disc.sort_unstable_by(|a, b| match a.0 - b.0 {
            a if a > 0 => std::cmp::Ordering::Greater,
            a if a < 0 => std::cmp::Ordering::Less,
            a if a == 0 => std::cmp::Ordering::Equal,
            _ => std::cmp::Ordering::Greater,
        });
        disc
    }
    fn get_team_disciples(&self, team: &i32) -> Vec<PlayerSummary> {
        self.disciples
            .iter()
            .filter_map(|d| {
                if self.turn_number != 0
                    && !self
                        .players
                        .iter()
                        .find(|p| p.nationid == d.nation_id)
                        .is_some()
                    || d.is_disciple == 0
                    || d.team.unwrap_or(0) != *team
                {
                    return None;
                }
                Some(self.get_player_summary(d.team.unwrap_or(0), d.nation_id))
            })
            .collect()
    }
    fn get_player_summary(&self, team: i32, nation_id: i32) -> PlayerSummary {
        match self.nations.iter().find(|n| n.nation_id == nation_id) {
            Some(n) => {
                let is_disciple = match self.disciples.iter().find(|d| d.nation_id == nation_id) {
                    Some(d) => d.is_disciple > 0,
                    None => true,
                };
                match self.players.iter().find(|p| p.nationid == nation_id) {
                    Some(p) => PlayerSummary {
                        name: n.name.clone(),
                        epithet: n.epithet.clone(),
                        pretender_name: p.name.clone(),
                        id: n.nation_id,
                        team,
                        status: self
                            .turns
                            .get(&n.nation_id)
                            .map_or(0, |ts| ts.last().map_or(0, |t| t.status)),
                        is_disciple,
                        has_pass: p.password.is_some(),
                    },
                    None => PlayerSummary {
                        name: n.name.clone(),
                        epithet: n.epithet.clone(),
                        pretender_name: "Nation has not joined".to_string(),
                        id: n.nation_id,
                        status: -1,
                        team,
                        is_disciple,
                        has_pass: false,
                    },
                }
            }
            None => PlayerSummary {
                name: "Not Selected".to_string(),
                epithet: "".to_string(),
                pretender_name: "Nation has not joined".to_string(),
                id: 0,
                team,
                status: -1,
                is_disciple: false,
                has_pass: false,
            },
        }
    }
    fn get_joined_nations(&self) -> Vec<&Nation> {
        self.nations
            .iter()
            .filter(|n| {
                self.players
                    .iter()
                    .find(|p| p.nationid == n.nation_id)
                    .is_some()
            })
            .collect()
    }
    fn get_player_summaries(&self) -> Vec<PlayerSummary> {
        self.players
            .iter()
            .map(|p| self.get_player_summary(0, p.nationid))
            .collect()
    }
    fn get_email_config_nation_name(&self, id: &i32) -> &str {
        self.email_configs
            .iter()
            .find(|e| e.id == *id)
            .map_or("", |e| {
                &self
                    .nations
                    .iter()
                    .find(|n| n.nation_id == e.nation_id)
                    .unwrap()
                    .name
            })
    }
    fn get_turn_status(&self, nation_id: &i32, turn_number: &i32) -> i32 {
        self.turns.get(nation_id).map_or(0, |turns| {
            turns
                .iter()
                .find(|t| t.turn_number == *turn_number)
                .map_or(0, |t| t.status)
        })
    }
    fn get_turn_pips(&self, nation_id: &i32) -> Vec<&PlayerTurn> {
        match self.turns.get(&nation_id) {
            Some(turns) => {
                let len = turns.len();
                if len > 0 {
                    turns
                        .iter()
                        .rev()
                        .take(10)
                        // The final turn is not shown as a pip
                        .skip(1)
                        .rev()
                        .collect()
                } else {
                    vec![]
                }
            }
            None => vec![],
        }
    }
    fn get_current_mods(&self) -> Vec<&Mod> {
        if self.settings.cmods.len() == 0 {
            vec![]
        } else {
            self.mods
                .iter()
                .filter(|m| self.settings.cmods.iter().find(|cm| **cm == m.id).is_some())
                .collect()
        }
    }
    fn get_status_string(&self) -> String {
        match (self.game.archived, self.winner_name.as_ref()) {
            (true, None) => "Archived".to_string(),
            (true, Some(w)) => format!("Won by {} (Archived)", w),
            (false, None) => {
                if self.turn_number == 0 {
                    "Waiting for pretenders".to_string()
                } else {
                    format!("Turn {}", self.turn_number)
                }
            }
            (false, Some(w)) => {
                format!("Won by {} (Turn {})", w, self.turn_number)
            }
        }
    }
    fn get_shorthand_description(&self) -> String {
        format!(
            "{}\n{}{}",
            &self.get_status_string(),
            match self.game.port {
                Some(p) => format!("{}:{}\n", self.games_address, p),
                None => "".to_owned(),
            },
            match self.game.next_turn {
                Some(_t) => format!("Next turn in {}\n", self.game.next_turn_string()),
                None => "".to_owned(),
            }
        )
    }

    fn get_current_map(&self) -> Option<&Map> {
        if self.settings.map != 0 {
            self.maps.iter().find(|m| m.id == self.settings.map)
        } else {
            None
        }
    }
}
struct ActiveGame {
    id: i32,
    name: String,
    address: String,
    turn_summary: crate::models::TurnSummary,
    next_turn_string: String,
    map_id: i32,
}
struct PendingGame {
    id: i32,
    name: String,
    address: String,
    players: i32,
    map_id: i32,
}
struct AbandonedGame {
    id: i32,
    name: String,
    address: String,
    map_id: i32,
    auto_archive_in: String,
}
struct ArchivedGame {
    id: i32,
}
#[derive(Template)]
#[template(path = "games/list.html")]
struct GamesTemplate<'a> {
    pending_games: &'a [PendingGame],
    active_games: &'a [ActiveGame],
    abandoned_games: &'a [AbandonedGame],
    archived_games: &'a [ArchivedGame],
    antibot_question: Option<String>,
    antibot_failed: Option<i32>,
}
impl<'a> GamesTemplate<'a> {
    pub fn question_string(&self) -> String {
        match &self.antibot_question {
            Some(s) => s.to_owned(),
            None => "".to_owned(),
        }
    }
}

// === Routes ====
#[post("/game/{id}/postpone")]
async fn postpone(
    (app_data, web::Path(path_id), timer_form): (
        web::Data<AppData>,
        web::Path<i32>,
        web::Form<SetTimer>,
    ),
) -> Result<HttpResponse> {
    let db = app_data.pool.get().expect("Unable to connect to database");
    let game: Game = {
        use crate::schema::games::dsl::*;
        games
            .filter(id.eq(path_id))
            .get_result::<Game>(&db)
            .unwrap()
    };
    sqlx::query!(
        r#"UPDATE games set next_turn=COALESCE(next_turn,NOW()) + interval '1 minute' * $2
        WHERE id=$1"#, game.id, timer_form.timer as f64).execute(&app_data.sqlx_pool).await.unwrap();
    NewAdminLog {
        game_id: path_id,
        datetime: std::time::SystemTime::now(),
        action: format!("Postponed timer to {} minutes", timer_form.timer),
    }
    .insert(&db)
    .unwrap();
    Ok(HttpResponse::Found()
        .header(header::LOCATION, format!("/game/{}#schedule", game.id))
        .finish())
}
#[post("/game/{id}/forcehost")]
async fn forcehost(
    (app_data, web::Path(path_id)): (web::Data<AppData>, web::Path<i32>),
) -> Result<HttpResponse> {
    let db = app_data.pool.get().expect("Unable to connect to database");
    let game = Game::get(path_id, &db).unwrap();
    game.schedule_turn(std::time::SystemTime::now(), &db)
        .unwrap();
    app_data
        .msgbus_sender
        .send(Msg::GameSchedule(GameScheduleMsg {
            game_id: game.id,
            schedule: std::time::SystemTime::now(),
        }))
        .unwrap();
    NewAdminLog {
        game_id: path_id,
        datetime: std::time::SystemTime::now(),
        action: "Forcehosted".to_string(),
    }
    .insert(&db)
    .unwrap();
    Ok(HttpResponse::Found()
        .header(header::LOCATION, format!("/game/{}#schedule", game.id))
        .finish())
}
#[post("/game/{id}/timer")]
async fn timer(
    (app_data, web::Path(path_id), timer_form): (
        web::Data<AppData>,
        web::Path<i32>,
        web::Form<SetTimer>,
    ),
) -> Result<HttpResponse> {
    let db = app_data.pool.get().expect("Unable to connect to database");
    use crate::schema::games::dsl as games_dsl;
    use crate::schema::turns::dsl as turns_dsl;
    let sched = std::time::SystemTime::now()
        .add(std::time::Duration::from_secs(60 * timer_form.timer as u64));
    let game: Game = games_dsl::games
        .filter(games_dsl::id.eq(path_id))
        .get_result::<Game>(&db)
        .unwrap();
    let turns: Vec<Turn> = turns_dsl::turns
        .filter(
            turns_dsl::game_id
                .eq(game.id)
                .and(turns_dsl::archived.eq(false)),
        )
        .get_results(&db)
        .unwrap();
    if turns.len() > 0 {
        diesel::update(games_dsl::games.filter(games_dsl::id.eq(game.id)))
            .set((
                games_dsl::timer.eq(Some(timer_form.timer)),
                games_dsl::next_turn.eq(sched),
            ))
            .execute(&db)
            .unwrap();
    } else {
        diesel::update(games_dsl::games.filter(games_dsl::id.eq(game.id)))
            .set(games_dsl::timer.eq(Some(timer_form.timer)))
            .execute(&db)
            .unwrap();
    }
    app_data
        .msgbus_sender
        .send(Msg::GameSchedule(GameScheduleMsg {
            game_id: game.id,
            schedule: sched,
        }))
        .unwrap();
    NewAdminLog {
        game_id: path_id,
        datetime: std::time::SystemTime::now(),
        action: format!("Set timer to {} minutes", timer_form.timer),
    }
    .insert(&db)
    .unwrap();
    Ok(HttpResponse::Found()
        .header(header::LOCATION, format!("/game/{}#schedule", game.id))
        .finish())
}
#[get("/game/{id}/status")]
async fn old_details(web::Path(path_id): web::Path<String>) -> Result<HttpResponse> {
    return Ok(HttpResponse::PermanentRedirect()
        .header(header::LOCATION, format!("/game/{}#status", path_id))
        .finish());
}
#[get("/game/{id}")]
async fn details(
    (app_data, web::Path(path_id), session, payload): (
        web::Data<AppData>,
        web::Path<String>,
        actix_session::Session,
        serde_qs::actix::QsQuery<GameDetailsPayload>,
    ),
) -> Result<HttpResponse> {
    let game_settings = &payload.game_settings;
    let email_form = &payload.email_form;
    let db = app_data.pool.get().expect("Unable to connect to database");
    let game = if let Ok(id_i32) = path_id.parse::<i32>() {
        use crate::schema::games::dsl::*;
        games
            .filter(id.eq(id_i32))
            .get_result::<Game>(&*db)
            .unwrap()
    } else {
        use crate::schema::games::dsl::*;
        let game: Game = games.filter(name.ilike(path_id)).get_result(&*db).unwrap();
        return Ok(HttpResponse::PermanentRedirect()
            .header(header::LOCATION, format!("/game/{}#status", game.id))
            .finish());
    };
    if let Some(p) = &payload.password {
        if *p == game.password {
            session
                .set(&format!("auth_{}", game.id), AuthStatus::AuthSuccess)
                .unwrap();
        } else {
            session
                .set(&format!("auth_{}", game.id), AuthStatus::AuthFail)
                .unwrap();
        }
        return Ok(HttpResponse::TemporaryRedirect()
            .header(header::LOCATION, format!("/game/{}#status", game.id))
            .finish());
    }
    let authed: AuthStatus = session
        .get(&format!("auth_{}", game.id))
        .unwrap_or(Some(AuthStatus::Unauthed))
        .unwrap_or(AuthStatus::Unauthed);
    let players = Player::get_players(game.id, &db).unwrap();
    let nations = Nation::get_all(game.id, &db).unwrap();
    let disciples = Disciple::get_all(game.id, &db).unwrap();
    let player_turn_map = crate::models::PlayerTurn::get_player_turns(game.id, &db);
    let turns = Turn::get_all(game.id, &db).unwrap();
    let settings: GameSettings = if let Some(_) = game_settings.loaded {
        game_settings.clone()
    } else {
        (&game, &game.get_mods(&db).unwrap()[..]).into()
    };
    let (maps, mods): (Vec<Map>, Vec<Mod>) = (
        { Map::get_available_maps(game.id, authed == AuthStatus::AuthSuccess, &db).unwrap() },
        {
            if authed == AuthStatus::AuthSuccess {
                use crate::schema::mods::dsl::*;
                mods.filter(name.ilike(&format!("%{}%", game_settings.modfilter)))
                    .get_results(&db)
                    .unwrap()
            } else {
                game.get_mods(&db).unwrap()
            }
        },
    );
    use crate::schema::email_configs::dsl as email_dsl;
    let email_configs: Vec<EmailConfig> = email_dsl::email_configs
        .filter(
            email_dsl::email_address
                .eq(&email_form.email_address)
                .and(email_dsl::game_id.eq(game.id)),
        )
        .get_results(&db)
        .unwrap();
    let logs = GameLogLite::get_all(game.id, &db).unwrap();
    let logs_detail = if let Some(output_id) = payload.log_output {
        logs.iter()
            .find(|l| l.id == output_id)
            .unwrap()
            .get_output(&db)
            .unwrap()
    } else if let Some(errors_id) = payload.log_errors {
        logs.iter()
            .find(|l| l.id == errors_id)
            .unwrap()
            .get_errors(&db)
            .unwrap()
    } else {
        (0, String::new(), String::new())
    };
    let map = maps.iter().find(|m| m.id == game.map_id).unwrap();
    let mut provinces = map.get_provinces(&db).unwrap_or(vec![]);
    if provinces.len() == 0 {
        let (_, tga, _) = map.get_files(&db).unwrap();
        let img = crate::image::io::Reader::with_format(
            std::io::Cursor::new(&tga.filebinary),
            crate::image::ImageFormat::Tga,
        )
        .decode()
        .unwrap_or_else(|_| {
            crate::image::io::Reader::with_format(
                std::io::Cursor::new(&tga.filebinary),
                crate::image::ImageFormat::Sgi,
            )
            .decode()
            .unwrap()
        });
        let new_provinces = crate::files::MapFile::load_provinces(&img).unwrap();
        provinces = map.insert_provinces(new_provinces, &db).unwrap();
    }
    let staling_players: Vec<i32> = sqlx::query!(
        r#"
        WITH tt AS (SELECT MAX(turn_number) as tt FROM turns WHERE game_id=$1 AND archived!=TRUE)
        SELECT nation_id
        FROM player_turns pt
        LEFT JOIN games g ON g.id=$1
        INNER JOIN tt ON tt.tt=pt.turn_number
        WHERE pt.game_id=$1
        AND pt.status IN (0,1)
        AND pt.archived != TRUE
        AND COALESCE((
            SELECT COUNT(*) 
            FROM player_turns pt2 
            WHERE pt2.turn_number < tt.tt
              AND pt2.turn_number >= (tt.tt - g.max_stales) 
              AND pt2.archived!=TRUE 
              AND pt2.status=0 
              AND pt2.game_id=g.id 
              AND pt2.nation_id=pt.nation_id
            ) >= g.max_stales, FALSE)"#,
        game.id
    )
    .fetch_all(&app_data.sqlx_pool)
    .await
    .unwrap()
    .into_iter()
    .map(|r| r.nation_id)
    .collect();
    let admin_logs = AdminLog::get_all(game.id, &db).unwrap();
    Ok(HttpResponse::Ok().content_type("text/html").body(
        (GameDetailsTemplate {
            logs: GameLogLite::get_all(game.id, &db).unwrap(),
            staling_players,
            focused_log: logs_detail,
            settings,
            authed,
            provinces: &provinces,
            email_form: (*email_form).clone(),
            email_configs: &email_configs,
            turn_number: turns.len() as i32,
            winner_name: turns.last().and_then(|t| {
                nations
                    .iter()
                    .find(|n| Some(n.nation_id) == t.winner)
                    .and_then(|n| Some(n.name.clone()))
            }),
            maps: &maps,
            mods: &mods,
            players: &players,
            disciples: &disciples,
            nations: &nations,
            turns: player_turn_map,
            games_address: std::env::var("GAMES_ADDRESS").unwrap_or(
                std::env::var("ADDRESS").expect("ADDRESS required for showing game location"),
            ),
            admin_logs: &admin_logs,
            discord_clientid: app_data
                .discord_manager
                .as_ref()
                .and_then(|d| Some(d.client_id.clone())),
            discord_notifications: DiscordConfig::get_notifications(game.id, &db).unwrap(),
            discord_manager: app_data.discord_manager.as_ref(),
            game,
        })
        .render()
        .unwrap(),
    ))
}
#[post("/game/{id}/launch")]
async fn launch(
    (app_data, web::Path(path_id), form): (
        web::Data<AppData>,
        web::Path<i32>,
        web::Form<StartGame>,
    ),
) -> Result<HttpResponse> {
    let db = app_data.pool.get().expect("Unable to connect to database");
    use crate::schema::games::dsl::*;
    let sched = std::time::SystemTime::now().add(std::time::Duration::from_secs(form.countdown));
    let game = games
        .filter(id.eq(path_id))
        .get_result::<Game>(&*db)
        .unwrap();

    diesel::update(games)
        .filter(id.eq(path_id))
        .set(next_turn.eq(sched))
        .execute(&db)
        .unwrap();

    app_data
        .msgbus_sender
        .send(Msg::GameSchedule(GameScheduleMsg {
            game_id: game.id,
            schedule: sched,
        }))
        .unwrap();
    NewAdminLog {
        game_id: path_id,
        datetime: std::time::SystemTime::now(),
        action: format!("Set game to start in {} seconds", form.countdown),
    }
    .insert(&db)
    .unwrap();
    Ok(HttpResponse::Found()
        .header(header::LOCATION, format!("/game/{}#schedule", game.id))
        .finish())
}
#[post("/games/create")]
async fn create_post(
    (app_data, mut body): (web::Data<AppData>, web::Payload),
) -> Result<HttpResponse> {
    let db = app_data.pool.get().expect("Unable to connect to database");
    use crate::schema::maps::dsl as maps_dsl;
    let mut bytes = web::BytesMut::new();
    while let Some(item) = body.next().await {
        bytes.extend_from_slice(&item?);
    }
    let config = serde_qs::Config::new(10, false);
    let params: CreateGame = config.deserialize_bytes(&*bytes).unwrap();
    if AppData::get_antibot_question().is_some() {
        if params.antibot.is_none()
            || AppData::get_antibot_answer()
                .as_ref()
                .unwrap()
                .to_ascii_lowercase()
                != params.antibot.unwrap().to_ascii_lowercase()
        {
            return Ok(HttpResponse::Found()
                .header(header::LOCATION, "/games?antibot_failed=1")
                .finish());
        }
    }

    let mut new_game = NewGame::default();
    let port_var = env::var("PORT_RANGE").expect("PORT_RANGE must be set (ie. '10000,10999')");
    let range: Vec<&str> = port_var.split(",").collect();
    let new_port: i32 = diesel::dsl::sql::<diesel::sql_types::Int4>(&format!(
                    "SELECT num FROM generate_series({}, {}) num LEFT OUTER JOIN games g ON g.port=num WHERE g.id IS NULL ORDER BY num ASC LIMIT 1",
                    range[0], range[1]
                ))
                .get_result(&db)
                .unwrap();
    new_game.port = new_port;
    new_game.name = params.name;
    new_game.password = params.password.clone();
    new_game.masterpass = Some(params.password);
    new_game.map_id = maps_dsl::maps
        .order(maps_dsl::id.desc())
        .limit(1)
        .get_result::<Map>(&db)
        .unwrap()
        .id;

    let game: Game = diesel::insert_into(crate::schema::games::table)
        .values(&new_game)
        .get_result(&db)
        .expect("Error saving new game");

    let addr = format!("/game/{}?password={}#settings", game.id, new_game.password);

    let dom5_proc = Dom5Turn {
        db_pool: app_data.sqlx_pool.clone(),
        game_id: game.id,
    };
    dom5_proc.update_nations();

    Ok(HttpResponse::Found()
        .header(header::LOCATION, addr)
        .finish())
}
#[get("/games")]
async fn list(
    app_data: web::Data<AppData>,
    query: serde_qs::actix::QsQuery<GamesListPayload>,
) -> Result<HttpResponse> {
    let db = app_data.pool.get().expect("Unable to connect to database");
    let sqlx_db = app_data.sqlx_pool.clone();

    // Create game
    let mut results = Game::get_public(&db).unwrap();
    let player_counts = crate::models::Game::get_player_count(
        results.iter().map(|g| g.id).collect::<Vec<i32>>(),
        &db,
    );
    let turn_summaries = crate::models::Turn::turn_summary(
        &results.iter().map(|g| g.id).collect::<Vec<i32>>(),
        &sqlx_db,
    );
    let archived_games: Vec<ArchivedGame> = results
        .drain_filter(|g| g.archived)
        .map(|g| ArchivedGame { id: g.id })
        .collect();
    let abandoned_games: Vec<AbandonedGame> = results
        .drain_filter(|g| {
            std::time::SystemTime::now()
                .duration_since(g.last_activity)
                .unwrap()
                > std::time::Duration::from_secs(60 * 60 * 24 * 7)
        })
        .map(|g| AbandonedGame {
            id: g.id,
            map_id: g.map_id,
            name: g.name.clone(),
            address: format!(
                "{}:{}",
                std::env::var("GAMES_ADDRESS").unwrap(),
                g.port.unwrap_or(0)
            ),
            auto_archive_in: {
                let duration = g
                    .last_activity
                    .add(std::time::Duration::from_secs(60 * 60 * 24 * 7 * 2))
                    .duration_since(std::time::SystemTime::now())
                    .unwrap_or(std::time::Duration::from_secs(0));
                let hours = duration.as_secs() / 60 / 60 % 24;
                let days = duration.as_secs() / 60 / 60 / 24;
                if days > 0 {
                    format!("{} days", days)
                } else {
                    format!("{} hours", hours)
                }
            },
        })
        .collect();
    let active_games: Vec<ActiveGame> = results
        .drain_filter(|g| turn_summaries.iter().find(|t| t.game_id == g.id).is_some())
        .map(|g| ActiveGame {
            next_turn_string: g.next_turn_string(),
            id: g.id,
            map_id: g.map_id,
            turn_summary: turn_summaries
                .iter()
                .find(|t| t.game_id == g.id)
                .unwrap()
                .clone(),
            name: g.name.clone(),
            address: format!(
                "{}:{}",
                std::env::var("GAMES_ADDRESS").unwrap(),
                g.port.unwrap_or(0)
            ),
        })
        .collect();
    let pending_games: Vec<PendingGame> = results
        .iter()
        .map(|g| PendingGame {
            id: g.id,
            map_id: g.map_id,
            players: match player_counts.contains_key(&g.id) {
                true => player_counts[&g.id],
                false => 0,
            },
            address: format!(
                "{}:{}",
                std::env::var("GAMES_ADDRESS").unwrap(),
                g.port.unwrap_or(0)
            ),
            name: g.name.clone(),
        })
        .collect();
    Ok(HttpResponse::Ok().content_type("text/html").body(
        (GamesTemplate {
            active_games: &active_games,
            pending_games: &pending_games,
            abandoned_games: &abandoned_games,
            archived_games: &archived_games,
            antibot_failed: query.antibot_failed,
            antibot_question: AppData::get_antibot_question(),
        })
        .render()
        .unwrap(),
    ))
}
#[post("/game/{id}/settings")]
async fn settings_post(
    (app_data, path_id, mut body, session): (
        web::Data<AppData>,
        web::Path<i32>,
        web::Payload,
        actix_session::Session,
    ),
) -> Result<HttpResponse> {
    let db = app_data.pool.get().unwrap();
    let mut bytes = web::BytesMut::new();
    while let Some(item) = body.next().await {
        bytes.extend_from_slice(&item?);
    }
    let config = serde_qs::Config::new(10, false);
    log::debug!("Game settings {:?}", bytes);
    let body: GameSettings = config.deserialize_bytes(&*bytes).unwrap();
    if session
        .get(&format!("auth_{}", path_id))
        .unwrap_or(Some(AuthStatus::Unauthed))
        .unwrap_or(AuthStatus::Unauthed)
        == AuthStatus::Unauthed
    {
        return Ok(HttpResponse::Unauthorized()
            .header(header::LOCATION, format!("/game/{}#settings", path_id))
            .finish());
    }
    let old_game = Game::get(*path_id, &db).unwrap();
    let old_mods = old_game.get_mods(&db).unwrap();
    let game: Game = db
        .transaction::<_, diesel::result::Error, _>(|| {
            let game: Game = {
                use crate::schema::games::dsl::*;
                diesel::update(games.filter(id.eq(*path_id)))
                    .set((
                        era.eq(body.era),
                        thrones_t1.eq(body.thrones_t1),
                        thrones_t2.eq(body.thrones_t2),
                        thrones_t3.eq(body.thrones_t3),
                        throne_points_required.eq(body.throne_points_required),
                        research_diff.eq(body.research_diff),
                        research_rand.eq(body.research_rand > 0),
                        hof_size.eq(body.hof_size),
                        global_size.eq(body.global_size),
                        indepstr.eq(body.indepstr),
                        magicsites.eq(body.magicsites),
                        eventrarity.eq(body.eventrarity),
                        richness.eq(body.richness),
                        resources.eq(body.resources),
                        recruitment.eq(body.recruitment),
                        supplies.eq(body.supplies),
                        startprov.eq(body.startprov),
                        renaming.eq(body.renaming > 0),
                        scoregraphs.eq(body.scoregraphs > 0),
                        nationinfo.eq(body.nationinfo > 0),
                        artrest.eq(body.artrest > 0),
                        teamgame.eq(body.teamgame > 0),
                        clustered.eq(body.clustered > 0),
                        storyevents.eq(body.storyevents),
                        newailvl.eq(body.newailvl),
                        map_id.eq(body.map),
                        masterpass.eq(body.masterpass),
                        private.eq(body.private > 0),
                        require_password.eq(body.require_password > 0),
                        select_start.eq(body.select_start),
                        cataclysm.eq(body.cataclysm),
                        max_stales.eq(body.max_stales),
                    ))
                    .get_result(&db)?
            };
            {
                use crate::schema::game_mods::dsl::*;
                diesel::delete(game_mods)
                    .filter(game_id.eq(game.id))
                    .execute(&db)?;
                diesel::insert_into(game_mods)
                    .values(
                        body.cmods
                            .iter()
                            .map(|cm| NewGameMod {
                                game_id: game.id,
                                mod_id: *cm,
                            })
                            .collect::<Vec<NewGameMod>>(),
                    )
                    .execute(&db)?;
            };
            Ok(game)
        })
        .unwrap();
    let mods = old_game.get_mods(&db).unwrap();
    if mods.len() != old_mods.len()
        || mods
            .iter()
            .any(|m| !old_mods.iter().any(|om| om.id == m.id))
        || game.era != old_game.era
    {
        let dom5_proc = Dom5Turn {
            db_pool: app_data.sqlx_pool.clone(),
            game_id: game.id,
        };
        dom5_proc.update_nations();
    }
    Ok(HttpResponse::Found()
        .header(header::LOCATION, format!("/game/{}#settings", game.id))
        .finish())
}
#[post("/game/{id}/email")]
async fn emails_post(
    (app_data, bytes, web::Path(game_id)): (
        web::Data<AppData>,
        actix_web::web::Bytes,
        web::Path<i32>,
    ),
) -> Result<HttpResponse> {
    let cfg = serde_qs::Config::new(10, false);
    match cfg.deserialize_bytes::<EmailForm>(&bytes) {
        Ok(email_form) => {
            app_data.email_manager.create_config(
                game_id,
                email_form.email_address.clone(),
                email_form.subject,
                email_form.body,
                email_form.nation,
                email_form.hours_remaining,
                email_form.is_reminder,
            );
            Ok(HttpResponse::Found()
                .header(
                    header::LOCATION,
                    format!(
                        "/game/{}?email_address={}#emails",
                        game_id, email_form.email_address
                    ),
                )
                .finish())
        }
        Err(_) => Ok(HttpResponse::BadRequest()
            .header(header::LOCATION, format!("/game/{}#email", game_id))
            .finish()),
    }
}
#[post("/game/{id}/emails/{emailid}/delete")]
async fn emails_delete(
    (app_data, bytes, web::Path((game_id, email_id))): (
        web::Data<AppData>,
        actix_web::web::Bytes,
        web::Path<(i32, i32)>,
    ),
) -> Result<HttpResponse> {
    let cfg = serde_qs::Config::new(10, false);
    match cfg.deserialize_bytes::<EmailForm>(&bytes) {
        Ok(email_form) => {
            app_data
                .email_manager
                .delete_config(email_id, email_form.email_address.clone());
            Ok(HttpResponse::Found()
                .header(
                    header::LOCATION,
                    format!(
                        "/game/{}#emails?email_address={}",
                        game_id, email_form.email_address
                    ),
                )
                .finish())
        }
        Err(_) => Ok(HttpResponse::BadRequest()
            .header(header::LOCATION, format!("/game/{}#email", game_id))
            .finish()),
    }
}

#[post("/game/{id}/archive")]
pub async fn archive_post(
    (app_data, web::Path(path_id), session, form): (
        web::Data<AppData>,
        web::Path<i32>,
        actix_session::Session,
        web::Form<ArchiveForm>,
    ),
) -> Result<HttpResponse> {
    if session
        .get(&format!("auth_{}", path_id))
        .unwrap_or(Some(AuthStatus::Unauthed))
        .unwrap_or(AuthStatus::Unauthed)
        == AuthStatus::Unauthed
    {
        return Ok(HttpResponse::Unauthorized()
            .header(header::LOCATION, format!("/game/{}#schedule", path_id))
            .finish());
    }
    let db = app_data.pool.get().expect("Unable to connect to database");
    let game = Game::get(path_id, &db).unwrap();
    game.archive(form.archived, &db).unwrap();
    if form.archived {
        NewAdminLog {
            game_id: path_id,
            datetime: std::time::SystemTime::now(),
            action: "Archived game".to_owned(),
        }
        .insert(&db)
        .unwrap();
    } else {
        NewAdminLog {
            game_id: path_id,
            datetime: std::time::SystemTime::now(),
            action: "Reinstated game".to_owned(),
        }
        .insert(&db)
        .unwrap();
    }

    return Ok(HttpResponse::Found()
        .header(header::LOCATION, format!("/game/{}#schedule", path_id))
        .finish());
}

#[post("/game/{id}/remove/{playerid}")]
pub async fn remove_post(
    (app_data, web::Path((path_id, nation_id)), session, web::Form(RemovePlayerForm { password } )): (
        web::Data<AppData>,
        web::Path<(i32, i32)>,
        actix_session::Session,
        web::Form<RemovePlayerForm>,
    ),
) -> Result<HttpResponse> {
    let is_authed = session
        .get(&format!("auth_{}", path_id))
        .unwrap_or(Some(AuthStatus::Unauthed))
        .unwrap_or(AuthStatus::Unauthed)
        == AuthStatus::AuthSuccess;
    let db = app_data.pool.get().expect("Unable to connect to database");
    let turns = crate::models::Turn::current_turn(&vec![path_id], &db);
    // Check if there were no turns found for the vector of games containing only our game, *not*
    // that there were zero games for just ours, this is not a logic error, though it looks odd.
    if turns.len() == 0 {
        let players = Player::get_players(path_id, &db).unwrap();
        if let Some(player) = players.iter().find(|p| p.nationid == nation_id) {
            log::debug!("Attempted pass {:?} {:?}", password, player.password);
            if player.password.is_some() && password == player.password {
                player.remove(&db).unwrap();
            } else if is_authed {
                player.remove(&db).unwrap();
                let nation = Nation::get(path_id, player.nationid, &db).unwrap();
                NewAdminLog {
                    game_id: path_id,
                    datetime: std::time::SystemTime::now(),
                    action: format!("Removed {}", nation.name),
                }
                .insert(&db)
                .unwrap();
            }
        }
    }
    return Ok(HttpResponse::Found()
        .header(header::LOCATION, format!("/game/{}#status", path_id))
        .finish());
}

#[post("/game/{id}/rollback")]
pub async fn rollback_post(
    (app_data, web::Path(path_id), session): (
        web::Data<AppData>,
        web::Path<i32>,
        actix_session::Session,
    ),
) -> Result<HttpResponse> {
    if session
        .get(&format!("auth_{}", path_id))
        .unwrap_or(Some(AuthStatus::Unauthed))
        .unwrap_or(AuthStatus::Unauthed)
        == AuthStatus::Unauthed
    {
        return Ok(HttpResponse::Unauthorized()
            .header(header::LOCATION, format!("/game/{}#schedule", path_id))
            .finish());
    }
    let db = app_data.pool.get().expect("Unable to connect to database");
    use crate::schema::games::dsl as games_dsl;
    let game: Game = games_dsl::games
        .filter(games_dsl::id.eq(path_id))
        .get_result(&db)
        .unwrap();
    game.rollback(&db).unwrap();
    return Ok(HttpResponse::Found()
        .header(header::LOCATION, format!("/game/{}#schedule", path_id))
        .finish());
}

#[post("/game/{id}/unstart")]
pub async fn unstart_post(
    (app_data, web::Path(path_id), session): (
        web::Data<AppData>,
        web::Path<i32>,
        actix_session::Session,
    ),
) -> Result<HttpResponse> {
    if session
        .get(&format!("auth_{}", path_id))
        .unwrap_or(Some(AuthStatus::Unauthed))
        .unwrap_or(AuthStatus::Unauthed)
        == AuthStatus::Unauthed
    {
        return Ok(HttpResponse::Unauthorized()
            .header(header::LOCATION, format!("/game/{}#schedule", path_id))
            .finish());
    }
    let db = app_data.pool.get().expect("Unable to connect to database");
    use crate::schema::games::dsl as games_dsl;
    let game: Game = games_dsl::games
        .filter(games_dsl::id.eq(path_id))
        .get_result(&db)
        .unwrap();
    game.unstart(&db).unwrap();
    let _game = game.remove_timer(&db).unwrap();
    return Ok(HttpResponse::Found()
        .header(header::LOCATION, format!("/game/{}#schedule", path_id))
        .finish());
}
#[post("/game/{id}/assign-team")]
pub async fn assign_team(
    (app_data, web::Path(path_id), session, form): (
        web::Data<AppData>,
        web::Path<i32>,
        actix_session::Session,
        web::Form<TeamForm>,
    ),
) -> Result<HttpResponse> {
    if session
        .get(&format!("auth_{}", path_id))
        .unwrap_or(Some(AuthStatus::Unauthed))
        .unwrap_or(AuthStatus::Unauthed)
        == AuthStatus::Unauthed
    {
        return Ok(HttpResponse::Unauthorized()
            .header(header::LOCATION, format!("/game/{}", path_id))
            .finish());
    }
    let db = app_data.pool.get().expect("Unable to connect to database");
    let disciple = Disciple::get(path_id, form.nation, &db).unwrap();
    if form.team == 0 {
        if form.disciple == 0 {
            disciple.create_team(&db).unwrap();
        } else {
            disciple.remove(&db).unwrap();
        }
    } else {
        disciple
            .set_team(form.team, &db)
            .unwrap()
            .set_disc(form.disciple, &db)
            .unwrap();
    }
    return Ok(HttpResponse::Found()
        .header(header::LOCATION, format!("/game/{}#status", path_id))
        .finish());
}
#[post("/game/{id}/motd")]
pub async fn motd(
    (app_data, web::Path(path_id), session, mut form): (
        web::Data<AppData>,
        web::Path<i32>,
        actix_session::Session,
        web::Form<MotdForm>,
    ),
) -> Result<HttpResponse> {
    if session
        .get(&format!("auth_{}", path_id))
        .unwrap_or(Some(AuthStatus::Unauthed))
        .unwrap_or(AuthStatus::Unauthed)
        == AuthStatus::Unauthed
    {
        return Ok(HttpResponse::Unauthorized()
            .header(header::LOCATION, format!("/game/{}", path_id))
            .finish());
    }
    let sqlx_pool = app_data.sqlx_pool.clone();
    if form.motd == Some("".to_owned()) {
        form.motd = None;
    }
    sqlx::query!("UPDATE games SET motd=$2 WHERE id=$1", path_id, form.motd)
        .execute(&sqlx_pool)
        .await
        .unwrap();
    return Ok(HttpResponse::Found()
        .header(header::LOCATION, format!("/game/{}#schedule", path_id))
        .finish());
}
#[post("/game/{id}/startloc/{nationid}")]
pub async fn assign_start(
    (app_data, web::Path((path_id, nation_id)), session, bytes): (
        web::Data<AppData>,
        web::Path<(i32, i32)>,
        actix_session::Session,
        actix_web::web::Bytes,
    ),
) -> Result<HttpResponse> {
    let cfg = serde_qs::Config::new(10, false);
    let form = cfg.deserialize_bytes::<StartLocForm>(&bytes).unwrap();
    match form.password.get(&nation_id) {
        Some(password) => {
            sqlx::query!("
            WITH game AS (UPDATE games SET modified_mapcrc=NULL WHERE id=$3 AND select_start=1 RETURNING *)
                UPDATE players p SET startprov=$1
                FROM game g 
                WHERE p.game_id=g.id AND p.nationid=$2 AND p.password=$4",
                form.province_start,
                nation_id,
                path_id,
                password
            )
            .execute(&app_data.sqlx_pool)
            .await
            .unwrap();
        }
        None if session
            .get(&format!("auth_{}", path_id))
            .unwrap_or(Some(AuthStatus::Unauthed))
            .unwrap_or(AuthStatus::Unauthed)
            == AuthStatus::AuthSuccess =>
        {
            sqlx::query!("
            WITH game AS (UPDATE games SET modified_mapcrc=NULL WHERE id=$3 AND select_start=2 RETURNING *)
                UPDATE players p SET startprov=$1
                FROM game g 
                WHERE p.game_id=g.id AND p.nationid=$2",
                form.province_start,
                nation_id,
                path_id
            )
            .execute(&app_data.sqlx_pool)
            .await
            .unwrap();
        }
        _ => {}
    }
    return Ok(HttpResponse::Found()
        .header(header::LOCATION, format!("/game/{}#status", path_id))
        .finish());
}

#[post("/game/{id}/turns/{turn}/download/{nation_id}")]
pub async fn download_turn(
    (app_data, web::Path((game_id, turn, nation_id)), form): (
        web::Data<AppData>,
        web::Path<(i32, i32, i32)>,
        web::Form<DownloadTurnForm>,
    ),
) -> Result<HttpResponse> {
    let db = app_data.sqlx_pool.clone();
    let files = sqlx::query!(
        r#"
    SELECT 
        g.name as game_name, n.name as nation_name,
        trn.filename as "trnfilename", trn.filebinary as "trnfilebinary", 
        twoh.filename as "twohfilename?", twoh.filebinary as "twohfilebinary?"
    FROM player_turns pt 
    INNER JOIN games g ON g.id=$1
    INNER JOIN nations n ON n.nation_id=pt.nation_id AND n.game_id=pt.game_id
    INNER JOIN players p ON p.password=$4 AND p.game_id=$1 AND p.nationid=$2
    INNER JOIN files trn ON trn.id=pt.trnfile_id
    LEFT JOIN files twoh ON twoh.id=pt.twohfile_id
    WHERE pt.game_id=$1 AND pt.nation_id=$2 AND pt.turn_number=$3 AND pt.archived != TRUE
    "#,
        game_id,
        nation_id,
        turn,
        form.password
    )
    .fetch_all(&db)
    .await
    .unwrap();
    if files.len() == 0 {
        return Ok(HttpResponse::Found()
            .header(header::LOCATION, format!("/game/{}#schedule", game_id))
            .finish());
    }
    let mut bytes: Vec<u8> = vec![];
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut bytes));
    zip
        .start_file(&format!("{}.trn", files[0].trnfilename), zip::write::FileOptions::default())
        .unwrap();
    zip.write_all(&files[0].trnfilebinary).unwrap();
    if let (Some(twohfn), Some(twohfb)) = (files[0].twohfilename.as_ref(), files[0].twohfilebinary.as_ref()) {
        zip
            .start_file(twohfn, zip::write::FileOptions::default())
            .unwrap();
        zip.write_all(twohfb).unwrap();
    }
    zip.finish().unwrap();
    drop(zip);
    return Ok(HttpResponse::Ok()
              .set_header("content-disposition", format!(r#"attachment; filename="{} {} turn {}.zip""#, files[0].game_name, files[0].nation_name, turn))
              .content_type("application/zip")
              .body(bytes))
}
