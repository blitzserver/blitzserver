use actix_web::{get, post, web, HttpResponse, Result, HttpRequest};
use html_node::{html, text};
use serde::Deserialize;

use crate::views::container::wrap_container;
use crate::models::settings::Setting;
use super::{AppData, utils::ActixHtmlNode};

#[derive(Deserialize)]
pub struct AdminForm {
    #[serde(flatten)]
    settings: std::collections::HashMap<String, Option<String>>,
}

async fn check_admin_auth(req: &HttpRequest, app_data: &web::Data<AppData>) -> Result<(), HttpResponse> {
    // Check if there's an admin password set
    let admin_password = Setting::get_or_env(&app_data.sqlx_pool, "ADMIN_PASSWORD").await;

    // If no admin password is set, allow access
    if admin_password.is_none() {
        return Ok(());
    }

    // Parse Authorization header
    if let Some(auth_header) = req.headers().get("authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if auth_str.starts_with("Basic ") {
                if let Ok(decoded) = base64::decode(&auth_str[6..]) {
                    if let Ok(credentials) = String::from_utf8(decoded) {
                        if let Some((_, password)) = credentials.split_once(':') {
                            if password == admin_password.unwrap() {
                                return Ok(());
                            }
                        }
                    }
                }
            }
        }
    }

    // Authentication failed, return 401 with WWW-Authenticate header
    Err(HttpResponse::Unauthorized()
        .append_header(("WWW-Authenticate", "Basic realm=\"Admin Access\""))
        .body("Authentication required"))
}

#[get("/admin")]
pub async fn admin(app_data: web::Data<AppData>, req: HttpRequest) -> Result<HttpResponse> {
    // Check authentication
    if let Err(auth_response) = check_admin_auth(&req, &app_data).await {
        return Ok(auth_response);
    }

    let settings = Setting::get_all(&app_data.sqlx_pool)
        .await
        .unwrap_or_else(|_| Vec::new());

    Ok(HttpResponse::Ok().content_type("text/html").body(admin_view(settings).0.to_string()))
}

#[post("/admin")]
pub async fn admin_post(
    app_data: web::Data<AppData>,
    req: HttpRequest,
    form: web::Form<AdminForm>,
) -> Result<HttpResponse> {
    // Check authentication
    if let Err(auth_response) = check_admin_auth(&req, &app_data).await {
        return Ok(auth_response);
    }

    let db = &app_data.sqlx_pool;

    for (key, value) in &form.settings {
        let value_str = value.as_deref().filter(|v| !v.is_empty());
        Setting::update(db, key, value_str).await.ok();
    }

    Ok(HttpResponse::Found()
        .append_header(("Location", "/admin"))
        .finish())
}

fn admin_view(settings_list: Vec<Setting>) -> ActixHtmlNode {
    ActixHtmlNode(wrap_container(
        Some("Admin"),
        None,
        Some(html! {
            <div class="pane" style="margin: 15px;">
                <form method="post" action="/admin">
                    <h3>Server Settings</h3>
                    <div class="settings-group">
                        {settings_list.iter().map(|setting| {
                            html! {
                                <div class="input-group">
                                    <label for={text!("{}", &setting.key)} style="min-width: 200px;">
                                        {text!("{}", &setting.key)}
                                    </label>
                                    <input
                                        type="text"
                                        id={text!("{}", &setting.key)}
                                        name={text!("{}", &setting.key)}
                                        value={text!("{}", setting.value.as_deref().unwrap_or(""))}
                                        placeholder="Not configured"
                                        style="flex: 1; max-width: 400px;"
                                    />
                                </div>
                            }
                        })}
                    </div>
                    <input class="btn btn-block btn-primary" type="submit" value="Save Settings" style="max-width: 200px;"/>
                </form>
            </div>
        }),
    ))
}