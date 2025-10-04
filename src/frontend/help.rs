use askama::Template;
use actix_web::{get, HttpResponse, Result, web};
use crate::models::settings::Setting;
use super::AppData;

#[derive(Template)]
#[template(path = "help.html")]
struct HelpTemplate { hostname: String }

#[get("/help")]
pub async fn help(app_data: web::Data<AppData>) -> Result<HttpResponse> {
    let hostname = Setting::get_or_env(&app_data.sqlx_pool, "GAMES_ADDRESS").await
        .expect("GAMES_ADDRESS env var missing");
    Ok(HttpResponse::Ok().content_type("text/html").body(
        (HelpTemplate { hostname })
        .render()
        .unwrap(),
    ))
}
