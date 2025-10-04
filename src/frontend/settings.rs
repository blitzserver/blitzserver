use actix_web::{get, post, web, HttpResponse, Result};
use html_node::{html, text};
use serde::Deserialize;

use crate::views::container::wrap_container;
use crate::models::settings::Setting;
use super::{AppData, utils::ActixHtmlNode};

#[derive(Deserialize)]
pub struct SettingsForm {
    #[serde(flatten)]
    settings: std::collections::HashMap<String, Option<String>>,
}

#[get("/settings")]
pub async fn settings(app_data: web::Data<AppData>) -> Result<ActixHtmlNode> {
    let settings = Setting::get_all(&app_data.sqlx_pool)
        .await
        .unwrap_or_else(|_| Vec::new());

    Ok(settings_view(settings))
}

#[post("/settings")]
pub async fn settings_post(
    app_data: web::Data<AppData>,
    form: web::Form<SettingsForm>,
) -> Result<HttpResponse> {
    let db = &app_data.sqlx_pool;

    for (key, value) in &form.settings {
        let value_str = value.as_deref().filter(|v| !v.is_empty());
        Setting::update(db, key, value_str).await.ok();
    }

    Ok(HttpResponse::Found()
        .append_header(("Location", "/settings"))
        .finish())
}

fn settings_view(settings_list: Vec<Setting>) -> ActixHtmlNode {
    ActixHtmlNode(wrap_container(
        Some("Settings"),
        None,
        Some(html! {
            <div class="pane" style="margin: 15px;">
                <form method="post" action="/settings">
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