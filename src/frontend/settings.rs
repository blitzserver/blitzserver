use actix_web::{get, post, web, HttpResponse, Result, HttpRequest};
use actix_multipart::Multipart;
use futures::TryStreamExt;
use html_node::{html, text};
use serde::Deserialize;

use crate::views::container::wrap_container;
use crate::models::settings::Setting;
use crate::models::binary::Binary;
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

    let binaries = Binary::get_all(&app_data.sqlx_pool).await.unwrap_or_else(|_| Vec::new());

    Ok(HttpResponse::Ok().content_type("text/html").body(admin_view(settings, binaries).0.to_string()))
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

#[post("/admin/upload-binary")]
pub async fn admin_upload_binary(
    app_data: web::Data<AppData>,
    req: HttpRequest,
    mut payload: Multipart,
) -> Result<HttpResponse> {
    // Check authentication
    if let Err(auth_response) = check_admin_auth(&req, &app_data).await {
        return Ok(auth_response);
    }

    let db = app_data.sqlx_pool.clone();
    println!("Recv binary upload request");
    let binary_match = regex::Regex::new("dom._amd64").unwrap();

    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_type = field.content_disposition().clone();
        match content_type.get_name() {
            Some("archive") => {
                let mut contents: Vec<u8> = vec![];
                while let Some(bytes) = smol::stream::StreamExt::next(&mut field).await {
                    contents.extend_from_slice(&bytes.unwrap());
                }
                println!("Found binary archive");
                let filename = field
                    .content_disposition()
                    .get_filename()
                    .unwrap()
                    .to_owned();

                let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&contents)).unwrap();

                let tmp_path = std::env::current_dir()
                    .unwrap()
                    .join("tmp")
                    .join("binaries")
                    .join("landing")
                    .join(std::path::PathBuf::from(rand::random::<u32>().to_string()));
                archive.extract(&tmp_path).unwrap();
                let fname = archive
                    .file_names()
                    .find(|fname| binary_match.is_match(fname))
                    .unwrap();

                std::thread::sleep(std::time::Duration::from_secs(1));

                let command = std::process::Command::new(tmp_path.join(&fname))
                    .arg("--version")
                    .output()
                    .unwrap();
                let stdout = std::str::from_utf8(&command.stdout).unwrap();
                let version = regex::Regex::new("Dominions ., version (.+?) ")
                    .unwrap()
                    .captures(stdout)
                    .unwrap()
                    .get(1)
                    .unwrap()
                    .as_str();
                Binary::new(&version, &filename, &contents, &fname, &db)
                    .await
                    .unwrap();
                println!("Binary saved to database");
            }
            _ => {
                println!("Ignoring field: {:?}", content_type.get_name());
            }
        }
    }

    Ok(HttpResponse::Found()
        .append_header(("Location", "/admin"))
        .finish())
}

fn admin_view(settings_list: Vec<Setting>, binaries: Vec<Binary>) -> ActixHtmlNode {
    ActixHtmlNode(wrap_container(
        Some("Admin"),
        None,
        Some(html! {
            <div style="margin: 15px;">
                <div class="pane" style="margin-bottom: 20px;">
                    <form method="post" action="/admin">
                        <h3>Server Settings</h3>
                        <div class="settings-group">
                            {settings_list.iter().map(|setting| {
                                if setting.key == "ADMIN_PASSWORD" {
                                    let is_empty = setting.value.as_deref().unwrap_or("").is_empty();
                                    html! {
                                        <div class="input-group">
                                            <label for={text!("{}", &setting.key)} style="min-width: 200px; font-weight: bold;">
                                                {text!("{}", &setting.key)}
                                            </label>
                                            <input
                                                type="password"
                                                id={text!("{}", &setting.key)}
                                                name={text!("{}", &setting.key)}
                                                value={text!("{}", setting.value.as_deref().unwrap_or(""))}
                                                placeholder={text!("{}", if is_empty { "⚠️ Enter admin password - page is not protected!" } else { "Password set" })}
                                                style="flex: 1; max-width: 400px;"
                                            />
                                        </div>
                                    }
                                } else {
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
                                }
                            })}
                        </div>
                        <input class="btn btn-block btn-primary" type="submit" value="Save Settings" style="max-width: 200px;"/>
                    </form>
                </div>

                <div class="pane">
                    <form enctype="multipart/form-data" action="/admin/upload-binary" method="post">
                        <h3>Upload Binary</h3>
                        <div class="input-group">
                            <label>Archive</label>
                            <input name="archive" required type="file" />
                        </div>
                        <input class="btn btn-block btn-primary" type="submit" value="Upload"/>
                    </form>

                    <h3 style="margin-top: 20px;">Installed Binaries</h3>
                    <div>
                        {if binaries.is_empty() {
                            html! { <p>"No binaries installed"</p> }
                        } else {
                            html! {
                                <div>
                                    {binaries.iter().map(|b| html! {
                                        <div style="padding: 5px; border: 1px solid #ddd; margin: 5px 0;">
                                            {text!("Dominions {}", b.version)}
                                        </div>
                                    })}
                                </div>
                            }
                        }}
                    </div>
                </div>
            </div>
        }),
    ))
}