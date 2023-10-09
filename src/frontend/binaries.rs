use actix_multipart::Multipart;
use actix_web::{get, http::header, post, web::Data, HttpResponse, Result};
use html_node::{html, text};
use smol::stream::StreamExt;

use crate::{models::binary::Binary, views::container::wrap_container};

use super::{utils::ActixHtmlNode, AppData};

struct ListTemplate<'a> {
    binaries: &'a [Binary],
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
    let mut name: Option<String> = None;
    let binary_match = regex::Regex::new("dom._amd64").unwrap();
    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_type = field.content_disposition().clone();
        match content_type.get_name() {
            Some("name") => {
                name = Some(
                    std::str::from_utf8(&field.next().await.unwrap().unwrap())
                        .unwrap()
                        .to_owned(),
                );
                println!("Found name");
            }
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

                let archive = zip::ZipArchive::new(std::io::Cursor::new(&contents)).unwrap();
                let fname = archive
                    .file_names()
                    .find(|fname| binary_match.is_match(fname))
                    .unwrap();
                Binary::new(
                    &name.as_ref().unwrap(),
                    &filename,
                    &contents,
                    &fname,
                    &db,
                )
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
                    <form enctype="multipart/form-data" action="/binaries" method="post">
                        <h3>Upload Binary</h3>
                        <div class="input-group">
                            <label>Name</label>
                            <input placeholder="Dominions 5.61" name="name" required type="text" />
                        </div>
                        <div class="input-group">
                            <label>Archive</label>
                            <input name="archive" required type="file" />
                        </div>
                        <input class="btn btn-block btn-primary" type="submit" value="Upload"/>
                    </form>
                </div>
                <div>
                    { data.binaries.iter().map(|b| html! {
                        <div>{text!("{}", b.name)}</div>
                    })}

                </div>
            </div>
        }),
    ))
}
