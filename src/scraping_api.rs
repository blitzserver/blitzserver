use actix_web::get;
use actix_web::http::header;
use actix_web::web;
use actix_web::HttpResponse;
use actix_web::Result;
use chrono::serde::ts_seconds_option;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::to_string;

use crate::frontend::AppData;

#[derive(Serialize, Deserialize)]
struct ApiScrapeGame {
    #[serde(with = "ts_seconds_option")]
    seconds_to_next_turn: Option<i32>,
    game_id: i32,
}

#[get("/state")]
pub async fn state(app_data: web::Data<AppData>) -> Result<HttpResponse> {
    let data = sqlx::query_as!(
        ApiScrapeGame,
        r#"
SELECT 
    id "game_id",
    EXTRACT(EPOCH FROM (next_turn - NOW()))::int "seconds_to_next_turn?"
    FROM games
"#
    )
    .fetch_all(&app_data.sqlx_pool)
    .await
    .unwrap();
    return Ok(HttpResponse::Ok()
        .append_header(header::ContentType::json())
        .body(to_string(&data).unwrap()));
}
