use askama::Template;
use actix_web::{get, HttpResponse, Result};

#[derive(Template)]
#[template(path = "help.html")]
struct HelpTemplate { hostname: String }

#[get("/help")]
pub async fn help() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().content_type("text/html").body(
        (HelpTemplate { hostname: std::env::var("HOSTNAME").expect("HOSTNAME env var missing") })
        .render()
        .unwrap(),
    ))
}
