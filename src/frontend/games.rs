use super::utils::*;
use super::AppData;
use chrono::Utc;
use itertools::Itertools;
use std::env;
use std::io::Write;

use crate::discord::DiscordManager;
use crate::dom5_turn::Dom5Turn;
use crate::models::maps::MapSummary;
use crate::models::{
    AdminLog, Disciple, DiscordConfig, EmailConfig, Game, Map, MapProvince, Mod, Nation,
    NewAdminLog, NewGame, Player, PlayerTurn, Turn,
};
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
#[cfg_attr(rustfmt, rustfmt_skip)]
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
struct GameSettings {
    #[serde(default="default_one",     deserialize_with ="from_str")     ] era:                    i32,
    #[serde(default="default_five",    deserialize_with ="from_str")     ] thrones_t1:             i32,
    #[serde(default,                   deserialize_with ="from_str")     ] thrones_t2:             i32,
    #[serde(default,                   deserialize_with ="from_str")     ] thrones_t3:             i32,
    #[serde(default="default_five",    deserialize_with ="from_str")     ] throne_points_required: i32,
    #[serde(default="default_two",     deserialize_with ="from_str")     ] research_diff:          i32,
    #[serde(default="default_one",     deserialize_with ="from_str")     ] research_rand:          i32,
    #[serde(default="default_ten",     deserialize_with ="from_str")     ] hof_size:               i32,
    #[serde(default="default_five",    deserialize_with ="from_str")     ] global_size:            i32,
    #[serde(default="default_five",    deserialize_with ="from_str")     ] indepstr:               i32,
    #[serde(default="default_forty",   deserialize_with ="from_str")     ] magicsites:             i32,
    #[serde(default="default_two",     deserialize_with ="from_str")     ] eventrarity:            i32,
    #[serde(default="default_hundred", deserialize_with ="from_str")     ] richness:               i32,
    #[serde(default="default_hundred", deserialize_with ="from_str")     ] resources:              i32,
    #[serde(default="default_hundred", deserialize_with ="from_str")     ] recruitment:            i32,
    #[serde(default="default_hundred", deserialize_with ="from_str")     ] supplies:               i32,
    #[serde(default="default_one",     deserialize_with ="from_str")     ] startprov:              i32,
    #[serde(default,                   deserialize_with ="from_str")     ] renaming:               i32,
    #[serde(default="default_one",     deserialize_with ="from_str")     ] scoregraphs:            i32,
    #[serde(default="default_one",     deserialize_with ="from_str")     ] nationinfo:             i32,
    #[serde(default="default_one",     deserialize_with ="from_str")     ] artrest:                i32,
    #[serde(default,                   deserialize_with ="from_str")     ] teamgame:               i32,
    #[serde(default,                   deserialize_with ="from_str")     ] clustered:              i32,
    #[serde(default="default_one",     deserialize_with ="from_str")     ] storyevents:            i32,
    #[serde(default="default_two",     deserialize_with ="from_str")     ] newailvl:               i32,
    #[serde(default="default_one",     deserialize_with ="from_str")     ] map:                    i32,
    #[serde(default)                                                     ] mapfilter:              String,
    #[serde(default,                   deserialize_with ="from_str_seq") ] cmods:                  Vec<i32>,
    #[serde(default)                                                     ] modfilter:              String,
    #[serde(default)                                                     ] masterpass:             Option<String>,
    #[serde(default,                   deserialize_with ="from_str")     ] private:                i32,
    #[serde(default,                   deserialize_with ="from_str")     ] require_password:       i32,
    #[serde(default,                   deserialize_with ="from_str")     ] select_start:           i32,
    #[serde(default,                   deserialize_with ="from_str_opt") ] cataclysm:              Option<i32>,
    #[serde(default,                   deserialize_with ="from_str_opt") ] max_stales:             Option<i32>,
    #[serde(default,                   deserialize_with ="from_str")     ] conq_all:               i32,
    #[serde(default,                   deserialize_with ="from_str")     ] formal_diplo:           i32,
    #[serde(default,                   deserialize_with ="from_str")     ] research_9_slow:        i32,
//Markertotellwhethertheformisonitsfirstload
    #[serde(default)]loaded:Option<String>,
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
            map: game.maps_dom6_id.unwrap_or(1),
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
            conq_all: game.conq_all as i32,
            formal_diplo: game.formal_diplo as i32,
            research_9_slow: game.research_9_slow as i32,
        }
    }
}
#[cfg_attr(rustfmt, rustfmt_skip)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmailForm {
    #[serde(default)                               ] email_address:   String,
    #[serde(default, deserialize_with ="from_str") ] nation:          i32,
    #[serde(default)                               ] subject:         String,
    #[serde(default)                               ] body:            String,
    #[serde(default, deserialize_with ="from_str") ] hours_remaining: i32,
    #[serde(default, deserialize_with ="from_str") ] is_reminder:     bool,
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
pub struct WatchForm {
    player_id: i32,
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
    log_output: Option<i32>,
}
pub fn deserialize_option_ignore_error<'de, T, D>(d: D) -> Result<Option<T>, D::Error>
where
    T: serde::de::Deserialize<'de>,
    D: serde::de::Deserializer<'de>,
{
    Ok(T::deserialize(d).ok())
}
#[derive(Deserialize)]
struct StartGame {
    #[serde(default, deserialize_with = "deserialize_option_ignore_error")]
    countdown: Option<u64>,
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
    password: String,
}

#[derive(Debug)]
struct PlayerSummary {
    name: String,
    player_id: i32,
    epithet: String,
    pretender_name: String,
    id: i32,
    status: i32,
    is_disciple: bool,
    team: i32,
    has_pass: bool,
    pub playing: bool,
    pub unsubmitted: bool,
    pub duplicate_cdkey_nationnames: Option<Vec<String>>,
}
#[derive(Debug)]
struct GameDetailsPlayer {
    pub id: i32,
    pub nationid: i32,
    pub game_id: i32,
    pub file_id: i32,
    pub name: String,
    pub password: Option<String>,
    pub startprov: Option<i32>,
    pub cdkey: Option<i64>,
    pub playing: Option<bool>,
    pub turn_status: Option<i32>,
    pub duplicate_cdkey_nationnames: Option<Vec<String>>,
}
#[derive(Debug)]
struct ActiveJob {
    turn_number: i32,
    started_at: chrono::DateTime<chrono::Utc>,
    label: Option<String>,
}
#[derive(Debug)]
struct GameJobDisplay {
    job_id: i32,
    game_logs: Option<String>,
    turn_number: i32,
    started_at: chrono::DateTime<chrono::Utc>,
    finished_at: Option<chrono::DateTime<chrono::Utc>>,
    reaped: bool,
}
impl GameJobDisplay {
    pub fn started_at_str(&self) -> String {
        self.started_at.to_rfc2822()
    }
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
    players: &'a Vec<GameDetailsPlayer>,
    disciples: &'a Vec<Disciple>,
    nations: &'a Vec<Nation>,
    mods: &'a Vec<Mod>,
    maps: &'a Vec<MapSummary>,
    provinces: &'a Vec<MapProvince>,
    authed: AuthStatus,
    logs: Vec<GameJobDisplay>,
    admin_logs: &'a Vec<AdminLog>,
    staling_players: Vec<i32>,
    winner_name: Option<String>,
    active_job: Option<ActiveJob>,
    unhealthy: bool,
}
pub struct PNT<'a> {
    player: &'a GameDetailsPlayer,
    nation: &'a Nation,
}
impl<'a> GameDetailsTemplate<'a> {
    pub fn get_duplicate_players_string(&self, nation: &PlayerSummary) -> String {
        format!(
            "This cdkey is also playing {}",
            nation
                .duplicate_cdkey_nationnames
                .as_ref()
                .map(|n| n.iter().join(", "))
                .unwrap_or(String::new())
        )
    }
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
                        player_id: p.id,
                        playing: p.playing.unwrap_or(false),
                        unsubmitted: p.turn_status.unwrap_or(2) < 2,
                        duplicate_cdkey_nationnames: p.duplicate_cdkey_nationnames.clone(),
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
                        player_id: 0,
                        playing: false,
                        unsubmitted: false,
                        duplicate_cdkey_nationnames: None,
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
                player_id: 0,
                playing: false,
                unsubmitted: false,
                duplicate_cdkey_nationnames: None,
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
    pub fn next_turn_string(&self) -> String {
        if self.game.archived {
            return "Never".to_owned();
        }
        match self.game.next_turn {
            Some(next_turn) => {
                let duration = next_turn - chrono::Utc::now();
                let seconds = duration.num_seconds();
                let minutes = (duration.num_seconds() / 60) % 60;
                let hours = (duration.num_seconds() / 60) / 60;
                format!(
                    "{}{}{}",
                    if hours > 0 {
                        // More than one hour
                        format!("{} hours, ", hours)
                    } else {
                        "".to_string()
                    },
                    if minutes > 0 {
                        // More than one minute within the hour
                        format!("{} minutes", minutes)
                    } else {
                        "".to_string()
                    },
                    if seconds < 60 {
                        format!("{} seconds", seconds)
                    } else {
                        "".to_string()
                    }
                )
            }
            _ => "On submission".to_owned(),
        }
    }
    fn get_fail_count(&self) -> i32 {
        self.logs.iter().rev().fold(0, |a, i| {
            println!("a {}", a);
            if i.reaped {
                a + 1
            } else if i.finished_at.is_some() {
                0
            } else {
                a
            }
        })
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
    fn get_current_map(&self) -> Option<&MapSummary> {
        if self.settings.map != 0 {
            self.maps.iter().find(|m| m.map_id == self.settings.map)
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
    map_id: i32,
    plane_count: i32,
    hosting: bool,
    fail_count: i32,
    turn_status: Option<i32>,
}
impl ActiveGame {
    pub fn pane_class(&self) -> String {
        match self.turn_status.unwrap_or(3) {
            2 => "playing ".to_owned(),
            0 | 1 => "unsubmitted ".to_owned(),
            _ => "".to_owned(),
        }
    }
    pub fn next_turn_string(&self) -> String {
        match self.turn_summary.next_turn {
            Some(nt) => {
                let duration = nt - chrono::Utc::now();
                let minutes = (duration.num_seconds() / 60) % 60;
                let hours = (duration.num_seconds() / 60) / 60;
                let seconds = duration.num_seconds();
                if seconds < 60 {
                    format!("Next turn {} seconds", seconds)
                } else {
                    format!("Next turn {} hours, {} minutes", hours, minutes)
                }
            }
            None => "Next turn when all orders are submitted".to_owned(),
        }
    }
}
struct PendingGame {
    id: i32,
    name: String,
    address: String,
    players: i32,
    map_id: i32,
    plane_count: i32,
    hosting: bool,
    fail_count: i32,
    joined: bool,
}
struct AbandonedGame {
    id: i32,
    name: String,
    address: String,
    map_id: i32,
    plane_count: i32,
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
    (app_data, path_id, timer_form): (web::Data<AppData>, web::Path<i32>, web::Form<SetTimer>),
) -> Result<HttpResponse> {
    let db = app_data.sqlx_pool.clone();
    sqlx::query!(
        r#"UPDATE games set next_turn=COALESCE(next_turn,NOW()) + interval '1 minute' * $2
        WHERE id=$1"#,
        *path_id,
        timer_form.timer as f64
    )
    .execute(&app_data.sqlx_pool)
    .await
    .unwrap();
    NewAdminLog {
        game_id: *path_id,
        datetime: Utc::now().naive_utc(),
        action: format!("Postponed timer to {} minutes", timer_form.timer),
    }
    .insert(&db)
    .await
    .unwrap();
    Ok(HttpResponse::Found()
        .append_header((header::LOCATION, format!("/game/{}#schedule", path_id)))
        .finish())
}
#[post("/game/{id}/forcehost")]
async fn forcehost(
    (app_data, path_id): (web::Data<AppData>, web::Path<i32>),
) -> Result<HttpResponse> {
    let db = app_data.sqlx_pool.clone();
    let sqlx_db = app_data.sqlx_pool.clone();
    let game = Game::get(*path_id, &sqlx_db).await.unwrap();
    game.schedule_turn(chrono::Utc::now(), &sqlx_db)
        .await
        .unwrap();
    NewAdminLog {
        game_id: *path_id,
        datetime: Utc::now().naive_utc(),
        action: "Forcehosted".to_string(),
    }
    .insert(&db)
    .await
    .unwrap();
    Ok(HttpResponse::Found()
        .append_header((header::LOCATION, format!("/game/{}#schedule", game.id)))
        .finish())
}
#[post("/game/{id}/timer")]
async fn timer(
    (app_data, path_id, timer_form): (web::Data<AppData>, web::Path<i32>, web::Form<SetTimer>),
) -> Result<HttpResponse> {
    let db = app_data.sqlx_pool.clone();
    sqlx::query!(
        "
    UPDATE games SET timer=$2 WHERE id=$1 RETURNING *",
        *path_id,
        timer_form.timer
    )
    .fetch_one(&db)
    .await
    .unwrap();
    sqlx::query!("
    UPDATE games g SET next_turn=NOW() + interval '1 minute' * g.timer FROM turns t WHERE g.id=$1 AND t.game_id=g.id  AND t.archived=FALSE RETURNING g.*", *path_id).fetch_one(&db).await.unwrap();
    NewAdminLog {
        game_id: *path_id,
        datetime: Utc::now().naive_utc(),
        action: format!("Set timer to {} minutes", timer_form.timer),
    }
    .insert(&db)
    .await
    .unwrap();
    Ok(HttpResponse::Found()
        .append_header((header::LOCATION, format!("/game/{}#schedule", path_id)))
        .finish())
}
#[get("/game/{id}/status")]
async fn old_details(path_id: web::Path<String>) -> Result<HttpResponse> {
    return Ok(HttpResponse::PermanentRedirect()
        .append_header((header::LOCATION, format!("/game/{}#status", path_id)))
        .finish());
}
#[get("/game/{id}")]
async fn details(
    (app_data, path_id, session, payload): (
        web::Data<AppData>,
        web::Path<i32>,
        actix_session::Session,
        serde_qs::actix::QsQuery<GameDetailsPayload>,
    ),
) -> Result<HttpResponse> {
    let game_settings = &payload.game_settings;
    let email_form = &payload.email_form;
    let sqlx_db = app_data.sqlx_pool.clone();
    let game = Game::get(*path_id, &sqlx_db).await.unwrap();
    if let Some(p) = &payload.password {
        if *p == game.password {
            session
                .insert(&format!("auth_{}", game.id), AuthStatus::AuthSuccess)
                .unwrap();
        } else {
            session
                .insert(&format!("auth_{}", game.id), AuthStatus::AuthFail)
                .unwrap();
        }
        return Ok(HttpResponse::TemporaryRedirect()
            .append_header((header::LOCATION, format!("/game/{}#status", game.id)))
            .finish());
    }
    let authed: AuthStatus = session
        .get(&format!("auth_{}", game.id))
        .unwrap_or(Some(AuthStatus::Unauthed))
        .unwrap_or(AuthStatus::Unauthed);
    let watch_player_ids: std::collections::HashSet<i32> = session
        .get("watches")
        .unwrap()
        .unwrap_or(std::collections::HashSet::new());
    let players = sqlx::query_as!(
        GameDetailsPlayer,
        r#"
        WITH topturn as (select MAX(id) as id,nation_id,game_id FROM player_turns WHERE archived IS NOT TRUE GROUP BY nation_id, game_id)
        SELECT 
            (SELECT array_agg(name) FROM players pd WHERE pd.cdkey=p.cdkey AND pd.game_id=p.game_ID AND pd.id!=p.id) duplicate_cdkey_nationnames,
            p.*,
            p.id=ANY($2) as "playing", 
            pt.status as "turn_status?"
        FROM players p
        LEFT JOIN topturn tt ON tt.game_id=$1 AND tt.nation_id=p.nationid
        LEFT JOIN player_turns pt ON pt.id=tt.id
        WHERE p.game_id=$1"#,
        game.id,
        &watch_player_ids.into_iter().collect::<Vec<i32>>()[..]
    )
    .fetch_all(&sqlx_db)
    .await
    .unwrap();
    let nations = Nation::get_all(game.id, &sqlx_db).await.unwrap();
    let disciples = sqlx::query_as!(Disciple,
                                    "
                                    WITH top as (SELECT game_id, MAX(turn_number) as top FROM turns where archived is not true group by game_id)
                                    SELECT d.id, d.game_id, d.nation_id, d.is_disciple, d.team 
                                    FROM disciples d 
                                    LEFT JOIN players p 
                                    ON p.nationid=d.nation_id AND p.game_id=d.game_id
                                    LEFT JOIN top 
                                    ON top.game_id=d.game_id
                                    WHERE d.game_id=$1 AND (p.id IS NOT NULL OR top.top is null)
                                    ", game.id).fetch_all(&sqlx_db).await.unwrap();
    let player_turn_map = crate::models::PlayerTurn::get_player_turns(game.id, &sqlx_db)
        .await
        .unwrap();
    let turns = Turn::get_all(game.id, &sqlx_db).await.unwrap();
    let settings: GameSettings = if let Some(_) = game_settings.loaded {
        game_settings.clone()
    } else {
        (&game, &game.get_mods(&sqlx_db).await.unwrap()[..]).into()
    };
    let (maps, mods): (Vec<MapSummary>, Vec<Mod>) = if authed == AuthStatus::AuthSuccess {
        (
            sqlx::query_as!(
                MapSummary,
                "SELECT  map_id, name
                    FROM maps_dom6 m 
                    WHERE 
                        m.name ILIKE $1 OR 
                        EXISTS (SELECT g.* FROM games g WHERE g.id=$2 AND g.maps_dom6_id=m.map_id)
                    ORDER BY (SELECT COUNT(*) FROM games g WHERE g.maps_dom6_id=m.map_id) DESC",
                format!("%{}%", game_settings.mapfilter),
                game.id
            )
            .fetch_all(&sqlx_db)
            .await
            .unwrap(),
            sqlx::query_as!(
                Mod,
                "SELECT * 
                    FROM mods m 
                    WHERE 
                        m.name ILIKE $1 OR 
                        EXISTS (
                            SELECT m.* FROM mods mi 
                            INNER JOIN game_mods gm ON 
                                gm.mod_id=mi.id AND gm.game_id=$2 AND 
                                m.id=mi.id
                        )
                    ORDER BY (SELECT COUNT(*) FROM game_mods gmo WHERE gmo.mod_id=m.id) DESC",
                format!("%{}%", game_settings.modfilter),
                game.id
            )
            .fetch_all(&sqlx_db)
            .await
            .unwrap(),
        )
    } else {
        (
            sqlx::query_as!(
                MapSummary,
                "SELECT m.* 
                FROM maps_dom6 m
                INNER JOIN games g ON
                    g.maps_dom6_id =m.map_id AND
                    g.id=$1",
                game.id
            )
            .fetch_all(&sqlx_db)
            .await
            .unwrap(),
            sqlx::query_as!(
                Mod,
                "SELECT m.* 
                FROM mods m 
                INNER JOIN game_mods gm ON 
                    gm.mod_id=m.id AND 
                    gm.game_id=$1",
                game.id
            )
            .fetch_all(&sqlx_db)
            .await
            .unwrap(),
        )
    };
    let email_configs = sqlx::query_as!(
        EmailConfig,
        "SELECT * FROM email_configs WHERE game_id=$1 AND email_address=$2",
        *path_id,
        email_form.email_address
    )
    .fetch_all(&sqlx_db)
    .await
    .unwrap();
    let logs = sqlx::query_as!(GameJobDisplay, r#"
    WITH _insert as (INSERT INTO admin_logs (game_id, datetime, action) 
    SELECT $1 as "game_id", NOW() as "datetime", concat('Viewed turn logs for turn ', j.turn_number) as "action"
    FROM jobs j WHERE j.job_id=$2)
    SELECT DISTINCT ON (j.started_at) 
        j.job_id, 
        j.turn_number, 
        j.started_at, 
        j.finished_at, 
        j.reaped, 
        string_agg(sj.label || E'\n' || sj.logs, E'\n-------\n' ORDER BY sj.started_at) as "game_logs?"
    FROM jobs j
    LEFT JOIN subjobs sj  ON sj.job_id=j.job_id AND j.job_id=$2 AND $2 IS NOT NULL
    WHERE j.game_id=$1 
    GROUP BY j.job_id
    ORDER BY j.started_at DESC"#, game.id, payload.log_output).fetch_all(&sqlx_db).await.unwrap();
    // let map = maps.iter().find(|m| m.map_id == game.map_id).unwrap();
    /*
    let mut provinces = map.get_provinces(&sqlx_db).await.unwrap_or(vec![]);
    if provinces.len() == 0 {
        let (_, tga, _) = Map::get_files(map.id, &sqlx_db).await.unwrap();
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
        provinces = map.insert_provinces(new_provinces, &sqlx_db).await.unwrap();
    }
    */
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
    .fetch_all(&sqlx_db)
    .await
    .unwrap()
    .into_iter()
    .map(|r| r.nation_id)
    .collect();
    let admin_logs = AdminLog::get_all(game.id, &sqlx_db).await.unwrap();
    let current_jobs: Vec<ActiveJob> = sqlx::query_as!(
        ActiveJob,
        r#"
    SELECT j.turn_number, j.started_at, sj.label as "label?"
    FROM jobs j 
    LEFT JOIN subjobs sj ON sj.job_id=j.job_id AND sj.finished_at IS NULL
    WHERE j.game_id=$1 AND j.finished_at IS NULL AND reaped!=TRUE"#,
        game.id
    )
    .fetch_all(&app_data.sqlx_pool)
    .await
    .unwrap();
    Ok(HttpResponse::Ok().content_type("text/html").body(
        (GameDetailsTemplate {
            active_job: current_jobs.into_iter().next(),
            logs,
            staling_players,
            settings,
            authed,
            provinces: &vec![],
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
            unhealthy: sqlx::query!(r#"WITH top as (select MAX(job_id) as last FROM jobs where finished_at is not null and game_id=$1)
            SELECT reaped as "reaped" FROM jobs INNER JOIN top ON top.last=jobs.job_id"#,game.id).fetch_all(&sqlx_db).await.unwrap().into_iter().next().map_or(false, |t| t.reaped),
            game,
        })
        .render()
        .unwrap(),
    ))
}
#[post("/game/{id}/launch")]
async fn launch(
    (app_data, path_id, mut body): (web::Data<AppData>, web::Path<i32>, web::Payload),
) -> Result<HttpResponse> {
    let mut bytes = web::BytesMut::new();
    while let Some(item) = body.next().await {
        bytes.extend_from_slice(&item?);
    }
    let config = serde_qs::Config::new(10, false);
    let form: StartGame = config.deserialize_bytes(&*bytes).unwrap();
    let sqlx_db = app_data.sqlx_pool.clone();
    let sched = chrono::Utc::now().add(chrono::Duration::hours(form.countdown.unwrap_or(0) as i64));
    let game = Game::get(*path_id, &sqlx_db).await.unwrap();
    sqlx::query!(
        "UPDATE games SET next_turn=$2 WHERE id=$1 RETURNING *",
        *path_id,
        sched
    )
    .fetch_one(&sqlx_db)
    .await
    .unwrap();
    NewAdminLog {
        game_id: *path_id,
        datetime: Utc::now().naive_utc(),
        action: format!("Set game to start in {} hours", form.countdown.unwrap_or(0)),
    }
    .insert(&sqlx_db)
    .await
    .unwrap();
    Ok(HttpResponse::Found()
        .append_header((header::LOCATION, format!("/game/{}#schedule", game.id)))
        .finish())
}
#[post("/games/create")]
async fn create_post(
    (app_data, mut body): (web::Data<AppData>, web::Payload),
) -> Result<HttpResponse> {
    let sqlx_db = app_data.sqlx_pool.clone();
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
                .append_header((header::LOCATION, "/games?antibot_failed=1"))
                .finish());
        }
    }

    let mut new_game = NewGame::default();
    let port_var = env::var("PORT_RANGE").expect("PORT_RANGE must be set (ie. '10000,10999')");
    let range: Vec<&str> = port_var.split(",").collect();
    let new_port: i32 = sqlx::query!(
                    "SELECT num FROM generate_series($1::integer, $2::integer) num LEFT OUTER JOIN games g ON g.port=num WHERE g.id IS NULL ORDER BY num ASC LIMIT 1",
                    range[0].parse::<i32>().unwrap(), range[1].parse::<i32>().unwrap()
                )
                .fetch_one(&sqlx_db)
                .await
                .unwrap().num.unwrap();
    new_game.port = new_port;
    new_game.name = params.name;
    new_game.password = params.password.clone();
    new_game.masterpass = Some(params.password);
    new_game.map_id = sqlx::query!("SELECT map_id FROM maps_dom6 ORDER BY map_id DESC LIMIT 1")
        .fetch_one(&sqlx_db)
        .await
        .unwrap()
        .map_id;

    let game: Game = new_game.insert(&sqlx_db).await.unwrap();

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
    session: actix_session::Session,
    query: serde_qs::actix::QsQuery<GamesListPayload>,
) -> Result<HttpResponse> {
    let sqlx_db = app_data.sqlx_pool.clone();

    let watch_player_ids: std::collections::HashSet<i32> = session
        .get("watches")
        .unwrap()
        .unwrap_or(std::collections::HashSet::new());

    let mut results = sqlx::query!(r#"
        WITH last_success as (SELECT game_id,MAX(job_id) as job_id FROM jobs WHERE finished_at IS NOT NULL AND reaped IS NOT TRUE GROUP BY game_id),
        topturn AS (SELECT MAX(turn_number) as max, game_id FROM turns WHERE archived IS NOT TRUE GROUP BY game_id)
        SELECT g.*,mp.plane_count, COUNT(j.*) as fail_count, MAX(cj.job_id) as "current_job?", MAX(p.nationid) AS "playing?", MIN(pt.status) as "turn_status?"
        FROM games g 
        LEFT JOIN topturn tt ON tt.game_id=g.id
        LEFT JOIN players p ON p.id=ANY($1) AND p.game_id=g.id
        LEFT JOIN player_turns pt ON pt.nation_id=p.nationid AND pt.game_id=g.id AND pt.turn_number=tt.max AND pt.archived IS NOT TRUE
        LEFT JOIN last_success ls ON ls.game_id=g.id 
        LEFT JOIN jobs j ON j.job_id>ls.job_id AND j.game_id=g.id AND j.reaped IS TRUE
        LEFT JOIN jobs cj ON cj.game_id=g.id AND cj.finished_at IS NULL
        LEFT JOIN LATERAL (SELECT COUNT(*) plane_count FROM map_planes mp WHERE g.maps_dom6_id=mp.map_id) mp ON 1=1
        WHERE g.private!=TRUE
        GROUP BY g.id, mp.plane_count
        ORDER BY MIN(pt.status), MAX(p.nationid), g.id
        "#, &watch_player_ids.into_iter().collect::<Vec<i32>>()[..]).fetch_all(&sqlx_db).await.unwrap();
    let player_counts = crate::models::Game::get_player_count(
        &results.iter().map(|g| g.id).collect::<Vec<i32>>(),
        &sqlx_db,
    )
    .await
    .unwrap();
    let turn_summaries = crate::models::Turn::turn_summary(
        &results.iter().map(|g| g.id).collect::<Vec<i32>>(),
        &sqlx_db,
    );
    let archived_games: Vec<ArchivedGame> = results
        .extract_if(|g| g.archived)
        .map(|g| ArchivedGame { id: g.id })
        .collect();
    let abandoned_games: Vec<AbandonedGame> = results
        .extract_if(|g| {
            chrono::Utc::now().signed_duration_since(g.last_activity) > chrono::Duration::days(7)
        })
        .map(|g| AbandonedGame {
            id: g.id,
            map_id: g.maps_dom6_id.unwrap_or(0),
            plane_count: g.plane_count.unwrap_or(1) as i32,
            name: g.name.clone(),
            address: format!(
                "{}:{}",
                std::env::var("GAMES_ADDRESS").unwrap(),
                g.port.unwrap_or(0)
            ),
            auto_archive_in: {
                let duration = g
                    .last_activity
                    .checked_add_signed(chrono::Duration::days(14))
                    .map(|d| d.signed_duration_since(chrono::Utc::now()))
                    .unwrap();
                let hours = duration.num_seconds() / 60 / 60 % 24;
                let days = duration.num_seconds() / 60 / 60 / 24;
                if days > 0 {
                    format!("{} days", days)
                } else {
                    format!("{} hours", hours)
                }
            },
        })
        .collect();
    let active_games: Vec<ActiveGame> = results
        .extract_if(|g| turn_summaries.iter().find(|t| t.game_id == g.id).is_some())
        .map(|g| ActiveGame {
            id: g.id,
            fail_count: g.fail_count.unwrap_or(0) as i32,
            map_id: g.maps_dom6_id.unwrap_or(0),
            plane_count: g.plane_count.unwrap_or(1) as i32,
            turn_status: g.turn_status,
            hosting: g.current_job.is_some(),
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
            fail_count: g.fail_count.unwrap_or(0) as i32,
            map_id: g.maps_dom6_id.unwrap_or(0),
            plane_count: g.plane_count.unwrap_or(1) as i32,
            joined: g.playing.is_some(),
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
            hosting: g.current_job.is_some(),
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
    let sqlx_db = app_data.sqlx_pool.clone();
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
    let old_game = Game::get(*path_id, &sqlx_db).await.unwrap();
    let mut game = old_game.clone();
    let old_mods = old_game.get_mods(&sqlx_db).await.unwrap();

    game.era = body.era;
    game.thrones_t1 = body.thrones_t1;
    game.thrones_t2 = body.thrones_t2;
    game.thrones_t3 = body.thrones_t3;
    game.throne_points_required = body.throne_points_required;
    game.research_diff = body.research_diff;
    game.research_rand = body.research_rand > 0;
    game.hof_size = body.hof_size;
    game.global_size = body.global_size;
    game.indepstr = body.indepstr;
    game.magicsites = body.magicsites;
    game.eventrarity = body.eventrarity;
    game.richness = body.richness;
    game.resources = body.resources;
    game.recruitment = body.recruitment;
    game.supplies = body.supplies;
    game.startprov = body.startprov;
    game.renaming = body.renaming > 0;
    game.scoregraphs = body.scoregraphs > 0;
    game.nationinfo = body.nationinfo > 0;
    game.artrest = body.artrest > 0;
    game.teamgame = body.teamgame > 0;
    game.clustered = body.clustered > 0;
    game.storyevents = body.storyevents;
    game.newailvl = body.newailvl;
    game.maps_dom6_id = Some(body.map);
    game.masterpass = body.masterpass;
    game.private = body.private > 0;
    game.require_password = body.require_password > 0;
    game.select_start = body.select_start;
    game.cataclysm = body.cataclysm;
    game.max_stales = body.max_stales;
    game.conq_all = body.conq_all > 0;
    game.formal_diplo = body.formal_diplo;
    game.research_9_slow = body.research_9_slow > 0;
    game = game.update(&body.cmods, &sqlx_db).await.unwrap();
    let mods = game.get_mods(&sqlx_db).await.unwrap();
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
        dom5_proc
            .update_nations()
            .expect("Failed to refresh nations after mod change");
    }
    if game.map_id != old_game.map_id || (game.select_start == 0 && old_game.select_start != 0) {
        sqlx::query!(
            "WITH res as (UPDATE players SET startprov=NULL WHERE game_id=$1)
            UPDATE games SET modified_mapcrc=NULL WHERE id=$1",
            game.id
        )
        .execute(&app_data.sqlx_pool)
        .await
        .unwrap();
    }
    Ok(HttpResponse::Found()
        .header(header::LOCATION, format!("/game/{}#settings", game.id))
        .finish())
}
#[post("/game/{id}/email")]
async fn emails_post(
    (app_data, bytes, game_id): (web::Data<AppData>, actix_web::web::Bytes, web::Path<i32>),
) -> Result<HttpResponse> {
    let cfg = serde_qs::Config::new(10, false);
    match cfg.deserialize_bytes::<EmailForm>(&bytes) {
        Ok(email_form) => {
            app_data.email_manager.create_config(
                *game_id,
                email_form.email_address.clone(),
                email_form.subject,
                email_form.body,
                email_form.nation,
                email_form.hours_remaining,
                email_form.is_reminder,
            );
            Ok(HttpResponse::Found()
                .append_header((
                    header::LOCATION,
                    format!(
                        "/game/{}?email_address={}#emails",
                        game_id, email_form.email_address
                    ),
                ))
                .finish())
        }
        Err(_) => Ok(HttpResponse::BadRequest()
            .header(header::LOCATION, format!("/game/{}#email", game_id))
            .finish()),
    }
}
#[post("/game/{id}/emails/{emailid}/delete")]
async fn emails_delete(
    (app_data, bytes, path): (
        web::Data<AppData>,
        actix_web::web::Bytes,
        web::Path<(i32, i32)>,
    ),
) -> Result<HttpResponse> {
    let (game_id, email_id) = path.into_inner();
    let cfg = serde_qs::Config::new(10, false);
    match cfg.deserialize_bytes::<EmailForm>(&bytes) {
        Ok(email_form) => {
            app_data
                .email_manager
                .delete_config(email_id, email_form.email_address.clone());
            Ok(HttpResponse::Found()
                .append_header((
                    header::LOCATION,
                    format!(
                        "/game/{}#emails?email_address={}",
                        game_id, email_form.email_address
                    ),
                ))
                .finish())
        }
        Err(_) => Ok(HttpResponse::BadRequest()
            .header(header::LOCATION, format!("/game/{}#email", game_id))
            .finish()),
    }
}

#[post("/game/{id}/archive")]
pub async fn archive_post(
    (app_data, path_id, session, form): (
        web::Data<AppData>,
        web::Path<i32>,
        actix_session::Session,
        web::Form<ArchiveForm>,
    ),
) -> Result<HttpResponse> {
    if session
        .get(&format!("auth_{}", *path_id))
        .unwrap_or(Some(AuthStatus::Unauthed))
        .unwrap_or(AuthStatus::Unauthed)
        == AuthStatus::Unauthed
    {
        return Ok(HttpResponse::Unauthorized()
            .append_header((header::LOCATION, format!("/game/{}#schedule", *path_id)))
            .finish());
    }
    let sqlx_db = app_data.sqlx_pool.clone();
    match form.archived {
        true => Game::archive(*path_id, &sqlx_db).await.unwrap(),
        false => Game::unarchive(*path_id, &sqlx_db).await.unwrap(),
    }

    return Ok(HttpResponse::Found()
        .header(header::LOCATION, format!("/game/{}#schedule", path_id))
        .finish());
}

#[post("/game/{id}/remove/{playerid}")]
pub async fn remove_post(
    (app_data, path, session, web::Form(RemovePlayerForm { password } )): (
        web::Data<AppData>,
        web::Path<(i32, i32)>,
        actix_session::Session,
        web::Form<RemovePlayerForm>,
    ),
) -> Result<HttpResponse> {
    let (path_id, nation_id) = path.into_inner();
    let is_authed = session
        .get(&format!("auth_{}", path_id))
        .unwrap_or(Some(AuthStatus::Unauthed))
        .unwrap_or(AuthStatus::Unauthed)
        == AuthStatus::AuthSuccess;
    let sqlx_db = app_data.sqlx_pool.clone();
    let turns = crate::models::Turn::current_turn(&vec![path_id], &sqlx_db)
        .await
        .unwrap();
    // Check if there were no turns found for the vector of games containing only our game, *not*
    // that there were zero games for just ours, this is not a logic error, though it looks odd.
    if turns.len() == 0 {
        let players = Player::get_players(path_id, &sqlx_db).await.unwrap();
        if let Some(player) = players.iter().find(|p| p.nationid == nation_id) {
            log::debug!("Attempted pass {:?} {:?}", password, player.password);
            if player.password.is_some() && password == player.password {
                player.remove(&sqlx_db).await.unwrap();
            } else if is_authed {
                player.remove(&sqlx_db).await.unwrap();
                let nation = Nation::get(path_id, player.nationid, &sqlx_db)
                    .await
                    .unwrap();
                NewAdminLog {
                    game_id: path_id,
                    datetime: Utc::now().naive_utc(),
                    action: format!("Removed {}", nation.name),
                }
                .insert(&sqlx_db)
                .await
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
    (app_data, path_id, session): (
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
    let sqlx_db = app_data.sqlx_pool.clone();
    Game::rollback(*path_id, &sqlx_db).await.unwrap();
    return Ok(HttpResponse::Found()
        .header(header::LOCATION, format!("/game/{}#schedule", path_id))
        .finish());
}

#[post("/game/{id}/unstart")]
pub async fn unstart_post(
    (app_data, path_id, session): (
        web::Data<AppData>,
        web::Path<i32>,
        actix_session::Session,
    ),
) -> Result<HttpResponse> {
    if session
        .get(&format!("auth_{}", *path_id))
        .unwrap_or(Some(AuthStatus::Unauthed))
        .unwrap_or(AuthStatus::Unauthed)
        == AuthStatus::Unauthed
    {
        return Ok(HttpResponse::Unauthorized()
            .header(header::LOCATION, format!("/game/{}#schedule", *path_id))
            .finish());
    }
    let sqlx_db = app_data.sqlx_pool.clone();
    Game::unstart(*path_id, &sqlx_db).await.unwrap();
    Game::remove_timer(*path_id, &sqlx_db).await.unwrap();
    return Ok(HttpResponse::Found()
        .header(header::LOCATION, format!("/game/{}#schedule", *path_id))
        .finish());
}
#[post("/game/{id}/assign-team")]
pub async fn assign_team(
    (app_data, path_id, session, form): (
        web::Data<AppData>,
        web::Path<i32>,
        actix_session::Session,
        web::Form<TeamForm>,
    ),
) -> Result<HttpResponse> {
    if session
        .get(&format!("auth_{}", *path_id))
        .unwrap_or(Some(AuthStatus::Unauthed))
        .unwrap_or(AuthStatus::Unauthed)
        == AuthStatus::Unauthed
    {
        return Ok(HttpResponse::Unauthorized()
            .header(header::LOCATION, format!("/game/{}", *path_id))
            .finish());
    }
    let sqlx_db = app_data.sqlx_pool.clone();
    let disciple = Disciple::get(*path_id, form.nation, &sqlx_db).await.unwrap();
    if form.team == 0 {
        if form.disciple == 0 {
            disciple.set_team(None, &sqlx_db).await.unwrap();
        } else {
            disciple.remove(&sqlx_db).await.unwrap();
        }
    } else {
        disciple
            .set_team(Some(form.team), &sqlx_db)
            .await
            .unwrap()
            .set_disc(form.disciple, &sqlx_db)
            .await
            .unwrap();
    }
    return Ok(HttpResponse::Found()
        .header(header::LOCATION, format!("/game/{}#status", *path_id))
        .finish());
}
#[post("/game/{id}/motd")]
pub async fn motd(
    (app_data, path_id, session, mut form): (
        web::Data<AppData>,
        web::Path<i32>,
        actix_session::Session,
        web::Form<MotdForm>,
    ),
) -> Result<HttpResponse> {
    if session
        .get(&format!("auth_{}", *path_id))
        .unwrap_or(Some(AuthStatus::Unauthed))
        .unwrap_or(AuthStatus::Unauthed)
        == AuthStatus::Unauthed
    {
        return Ok(HttpResponse::Unauthorized()
            .header(header::LOCATION, format!("/game/{}", *path_id))
            .finish());
    }
    let sqlx_pool = app_data.sqlx_pool.clone();
    if form.motd == Some("".to_owned()) {
        form.motd = None;
    }
    sqlx::query!("UPDATE games SET motd=$2 WHERE id=$1", *path_id, form.motd)
        .execute(&sqlx_pool)
        .await
        .unwrap();
    return Ok(HttpResponse::Found()
        .header(header::LOCATION, format!("/game/{}#schedule", *path_id))
        .finish());
}
#[post("/game/{id}/watch")]
pub async fn watch(
    (path_id, session, form): (
        web::Path<i32>,
        actix_session::Session,
        web::Form<WatchForm>,
    ),
) -> Result<HttpResponse> {
    let mut watches: std::collections::HashSet<i32> = session
        .get("watches")
        .unwrap()
        .unwrap_or(std::collections::HashSet::new());
    match watches.get(&form.player_id) {
        Some(_) => {
            watches.remove(&form.player_id);
        }
        None => {
            watches.insert(form.player_id);
        }
    }
    session.insert("watches", watches).unwrap();

    return Ok(HttpResponse::Found()
        .append_header((header::LOCATION, format!("/game/{}#status", *path_id)))
        .finish());
}
#[post("/game/{id}/startloc/{nationid}")]
pub async fn assign_start(
    (app_data, path, session, bytes): (
        web::Data<AppData>,
        web::Path<(i32, i32)>,
        actix_session::Session,
        actix_web::web::Bytes,
    ),
) -> Result<HttpResponse> {
    let (path_id, nation_id) = path.into_inner();
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
        .append_header((header::LOCATION, format!("/game/{}#status", path_id)))
        .finish());
}

#[post("/game/{id}/turns/{turn}/download/{nation_id}")]
pub async fn download_turn(
    (app_data, path, form): (
        web::Data<AppData>,
        web::Path<(i32, i32, i32)>,
        web::Form<DownloadTurnForm>,
    ),
) -> Result<HttpResponse> {
    let (game_id, turn, nation_id) = path.into_inner();
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
            .append_header((header::LOCATION, format!("/game/{}#schedule", game_id)))
            .finish());
    }
    let mut bytes: Vec<u8> = vec![];
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut bytes));
    zip.start_file(
        &format!("{}.trn", files[0].trnfilename),
        zip::write::FileOptions::default(),
    )
    .unwrap();
    zip.write_all(&files[0].trnfilebinary).unwrap();
    if let (Some(twohfn), Some(twohfb)) = (
        files[0].twohfilename.as_ref(),
        files[0].twohfilebinary.as_ref(),
    ) {
        zip.start_file(twohfn, zip::write::FileOptions::default())
            .unwrap();
        zip.write_all(twohfb).unwrap();
    }
    zip.finish().unwrap();
    drop(zip);
    return Ok(HttpResponse::Ok()
        .append_header((
            "content-disposition",
            format!(
                r#"attachment; filename="{} {} turn {}.zip""#,
                files[0].game_name, files[0].nation_name, turn
            ),
        ))
        .content_type("application/zip")
        .body(bytes));
}
