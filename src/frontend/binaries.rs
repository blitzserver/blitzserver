use askama::Template;
use actix_web::{get, HttpResponse, Result};

#[derive(Template)]
#[template(path = "binaries/list.html")]
struct HelpTemplate { hostname: String }

#[get("/binaries")]
pub async fn list() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().content_type("text/html").body(
        (HelpTemplate { hostname: std::env::var("GAMES_ADDRESS").expect("GAMES_ADDRESS env var missing") })
        .render()
        .unwrap(),
    ))
}
