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
    let linux_binary_match = regex::Regex::new("dom._amd64").unwrap();
    let windows_binary_match = regex::Regex::new("Dominions6\\.exe").unwrap();

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
                // Find both Windows and Linux executables
                let linux_executable = archive
                    .file_names()
                    .find(|fname| linux_binary_match.is_match(fname));
                let windows_executable = archive
                    .file_names()
                    .find(|fname| windows_binary_match.is_match(fname));

                // We need both executables
                if linux_executable.is_none() || windows_executable.is_none() {
                    return Ok(HttpResponse::BadRequest()
                        .content_type("text/html")
                        .body("Archive must contain both Linux ('dom6_amd64') and Windows ('Dominions6.exe') executables."));
                }

                let linux_path = linux_executable.unwrap().to_string();
                let windows_path = windows_executable.unwrap().to_string();

                // Use the first available executable for version detection
                let test_executable = linux_executable.or(windows_executable).unwrap();

                // Set executable permission on the test executable
                let test_executable_path = tmp_path.join(test_executable);
                if test_executable_path.exists() {
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let mut perms = std::fs::metadata(&test_executable_path).unwrap().permissions();
                        perms.set_mode(perms.mode() | 0o111); // Add execute permission for owner, group, and others
                        std::fs::set_permissions(&test_executable_path, perms).unwrap();
                    }
                }

                std::thread::sleep(std::time::Duration::from_secs(1));

                let output = match std::process::Command::new(&test_executable_path)
                    .arg("--version")
                    .output() {
                    Ok(output) => output,
                    Err(e) => {
                        return Ok(HttpResponse::BadRequest()
                            .content_type("text/html")
                            .body(format!("Failed to execute binary for version detection: {}\n\nThis is often caused by missing system libraries required by the Dominions binary.", e)));
                    }
                };

                let stdout = std::str::from_utf8(&output.stdout).unwrap_or("<invalid UTF-8>");
                let stderr = std::str::from_utf8(&output.stderr).unwrap_or("<invalid UTF-8>");

                let version_regex = regex::Regex::new("Dominions ., version (.+?) ").unwrap();
                let captures = match version_regex.captures(stdout) {
                    Some(captures) => captures,
                    None => {
                        return Ok(HttpResponse::BadRequest()
                            .content_type("text/html")
                            .body(format!("Could not find version in command output.\n\nStdout:\n{}\n\nStderr:\n{}", stdout, stderr)));
                    }
                };

                let version = match captures.get(1) {
                    Some(version_match) => version_match.as_str(),
                    None => {
                        return Ok(HttpResponse::BadRequest()
                            .content_type("text/html")
                            .body(format!("Could not parse version from command output.\n\nStdout:\n{}\n\nStderr:\n{}", stdout, stderr)));
                    }
                };

                if let Err(e) = Binary::new(&version, &filename, &contents, &linux_path, &windows_path, &db).await {
                    return Ok(HttpResponse::BadRequest()
                        .content_type("text/html")
                        .body(format!("Database error while saving binary: {}", e)));
                }

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
                                            <label for={text!("{}", &setting.key)} style={text!("min-width: 200px;{}", if is_empty { " font-weight: bold;" } else { "" })}>
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
                        <p style="margin-bottom: 15px; color: #666; font-size: 14px;">
                            "To upload a Dominions game binary, create a ZIP archive containing the game contents
                            (including the dominions executable) and upload it here. The system will automatically
                            extract and detect the game version."
                        </p>
                        <div class="input-group">
                            <label>Archive</label>
                            <input name="archive" required type="file" accept=".zip" />
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