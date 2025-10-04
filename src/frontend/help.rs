use askama::Template;
use actix_web::{get, HttpRequest, HttpResponse, Result, web};
use crate::models::settings::Setting;
use super::AppData;

#[derive(Template)]
#[template(path = "help.html")]
struct HelpTemplate { hostname: String }

#[get("/help")]
pub async fn help(app_data: web::Data<AppData>, req: HttpRequest) -> Result<HttpResponse> {
    let hostname = req.headers()
        .get("host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("localhost:8080")
        .split(':')
        .next()
        .unwrap_or("localhost")
        .to_string();
    Ok(HttpResponse::Ok().content_type("text/html").body(
        (HelpTemplate { hostname })
        .render()
        .unwrap(),
    ))
}
