use actix_multipart::Multipart;
use actix_web::{get, http::header, post, web::Data, HttpResponse, Result};
use async_std::fs::create_dir;
use html_node::{html, text};
use libvips::bindings::mkdir;
use smol::stream::StreamExt;

use crate::{models::binary::Binary, views::container::wrap_container};

use super::{utils::ActixHtmlNode, AppData};

struct ListTemplate<'a> {
    binaries: &'a [Binary],
}

impl<'a> ListTemplate<'a> {
    pub fn get_upload_location() -> String {
        return format!(
            "{}/binaries",
            std::env::var("UPLOAD_ADDRESS")
                .expect("GAMES_ADDRESS env varnot specified, needed for binary upload form")
        );
    }
}

#[get("/binaries")]
pub async fn list(app_data: Data<AppData>) -> Result<ActixHtmlNode> {
    let db = app_data.sqlx_pool.clone();
    let binaries = Binary::get_all(&db).await.unwrap();
    Ok(list_view(ListTemplate {
        binaries: &binaries,
    }))
}

#[post("/binaries")]
pub async fn upload((app_data, mut payload): (Data<AppData>, Multipart)) -> Result<HttpResponse> {
    let db = app_data.sqlx_pool.clone();
    println!("Recv request");
    let binary_match = regex::Regex::new("dom._amd64").unwrap();
    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_type = field.content_disposition().clone();
        match content_type.get_name() {
            Some("archive") => {
                let mut contents: Vec<u8> = vec![];
                while let Some(bytes) = field.next().await {
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
            }
            _ => unimplemented!("Not there"),
        }
    }
    return Ok(HttpResponse::Found()
        .append_header((header::LOCATION, "/binaries"))
        .finish());
}

fn list_view(data: ListTemplate) -> ActixHtmlNode {
    ActixHtmlNode(wrap_container(
        None,
        None,
        Some(html! {
            <div class="games-list">
                <div class="pane">
                    <form enctype="multipart/form-data" action={ListTemplate::get_upload_location()} method="post">
                        <h3>Upload Binary</h3>
                        <div class="input-group">
                            <label>Archive</label>
                            <input name="archive" required type="file" />
                        </div>
                        <input class="btn btn-block btn-primary" type="submit" value="Upload"/>
                    </form>
                </div>
                <div>
                    { data.binaries.iter().map(|b| html! {
                        <div>{text!("Dominions {}", b.version)}</div>
                    })}

                </div>
            </div>
        }),
    ))
}
