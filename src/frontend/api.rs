use crate::frontend::AppData;
use crate::models::Game;
use actix_web::guard::Header;
use actix_web::http::header::{self, ContentType, HeaderName};
use actix_web::{get, Result};
use actix_web::{web, HttpResponse};

/*
#[get("/json/game/{id}")]
pub async fn game_json(
    (app_data, path_id): (web::Data<AppData>, web::Path<i32>),
) -> Result<HttpResponse> {
    let mut summary = Game::json_summary(*path_id, &app_data.sqlx_pool)
        .await
        .unwrap();
    summary.as_object_mut().unwrap().insert(
        "address".to_owned(),
        std::env::var("ADDRESS").unwrap().into(),
    );
    Ok(HttpResponse::Ok()
        .insert_header(header::ContentType(mime::APPLICATION_JSON))
        .body(serde_json::to_string(&summary).unwrap()))
}
*/
