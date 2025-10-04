use super::AppData;
use crate::models::{File, Mod, ModDefinition, NewFile};
use crate::models::settings::Setting;
use actix_multipart::Multipart;
use actix_web::{get, http::header, post, web, HttpResponse, Result};
use askama::Template;
use futures::{StreamExt, TryStreamExt};
use std::io::Read;

#[derive(Template)]
#[template(path = "mods/list.html")]
pub struct ListModsTemplate<'a> {
    errors: &'a [String],
    mods: Vec<Mod>,
    upload_location: String,
}

impl<'a> ListModsTemplate<'a> {
    pub fn get_upload_location() -> String {
        return "/mods/upload".to_string();
    }
}

#[derive(Template)]
#[template(path = "mods/details.html")]
pub struct ModDetailsTemplate<'a> {
    cmod: &'a Mod,
}

#[get("/mods")]
pub async fn list(app_data: web::Data<AppData>) -> Result<HttpResponse> {
    let db = app_data.sqlx_pool.clone();
    let upload_location = ListModsTemplate::get_upload_location();
    Ok(HttpResponse::Ok().content_type("text/html").body(
        (ListModsTemplate {
            errors: &vec![],
            mods: sqlx::query_as!(Mod,
                r#"SELECT
                    id "id: i32",
                    dm_filename,
                    name,
                    file_id "file_id: i32",
                    icon_file_id "icon_file_id: i32",
                    crc
                FROM mods"#)
                .fetch_all(&db)
                .await
                .unwrap(),
            upload_location,
        })
        .render()
        .unwrap(),
    ))
}

#[get("/images/mods/{id}.jpg")]
pub async fn image(
    (app_data, path_id): (web::Data<AppData>, web::Path<i64>),
) -> Result<HttpResponse> {
    let db = app_data.sqlx_pool.clone();
    if let Ok(file) = sqlx::query_as!(
        File,
        r#"SELECT
            f.id "id: i32",
            f.filename,
            f.filebinary
        FROM mods m
        INNER JOIN files f ON f.id = m.icon_file_id
        WHERE m.id=$1"#,
        *path_id
    )
    .fetch_one(&db)
    .await
    {
        let reader = crate::image::io::Reader::with_format(
            std::io::Cursor::new(file.filebinary),
            crate::image::ImageFormat::Tga,
        )
        .decode()
        .unwrap();
        let mods_dir = std::path::PathBuf::from("./images/mods");
        if !mods_dir.exists() {
            std::fs::create_dir_all(&mods_dir).unwrap();
        }
        let mut file = std::fs::File::create(mods_dir.join(format!("{}.jpg", *path_id))).unwrap();
        reader
            .write_to(&mut file, crate::image::ImageFormat::Jpeg)
            .unwrap();
        let mut jpg_bytes: Vec<u8> = Vec::new();
        reader
            .write_to(
                &mut std::io::Cursor::new(&mut jpg_bytes),
                crate::image::ImageFormat::Jpeg,
            )
            .unwrap();
        Ok(HttpResponse::Ok()
            .content_type("application/jpg")
            .append_header(header::CacheControl(vec![header::CacheDirective::MaxAge(
                86400u32,
            )]))
            .body(jpg_bytes))
    } else {
        Ok(HttpResponse::NotFound().finish())
    }
}

#[post("/mods/upload")]
pub async fn upload_post(
    (app_data, mut payload): (web::Data<AppData>, Multipart),
) -> Result<HttpResponse> {
    let sqlx_db = app_data.sqlx_pool.clone();
    while let Ok(Some(mut field)) = payload.try_next().await {
        let mut mod_name: Option<ModDefinition> = None;
        let content_type = field.content_disposition().clone();
        match content_type.get_name() {
            Some("archive") => {
                let mut contents: Vec<u8> = vec![];
                while let Some(bytes) = field.next().await {
                    contents.extend_from_slice(&bytes.unwrap());
                }
                match zip::ZipArchive::new(std::io::Cursor::new(&contents)) {
                    Ok(mut archive) => {
                        let valid_filenames = archive
                            .file_names()
                            .filter(|fname| fname.ends_with(".dm"))
                            .map(|fname| fname.to_owned())
                            .collect::<Vec<String>>();
                        for fname in valid_filenames {
                            let mut archive_file = archive.by_name(&fname).unwrap();
                            let mut contents: String = String::new();
                            archive_file.read_to_string(&mut contents).unwrap();
                            let icon_filename_captures =
                                regex::Regex::new(r#"#icon +"?(.+?)"?\r?\n"#)
                                    .unwrap()
                                    .captures(&contents);
                            if let Some(modname_captures) =
                                regex::Regex::new(r#"#modname +"?(.+?)"?\r?\n"#)
                                    .unwrap()
                                    .captures(&contents)
                            {
                                mod_name = Some(ModDefinition {
                                    name: modname_captures.get(1).unwrap().as_str().to_owned(),
                                    // 3084600890/./Worthy_Heroes/Worthy_Heroes.tga
                                    icon_filename: icon_filename_captures.map_or(
                                        "".to_string(),
                                        |c| {
                                            std::path::Path::new(&fname)
                                                .parent()
                                                .unwrap()
                                                .join(c.get(1).map_or("".to_string(), |c| {
                                                    c.as_str().to_owned()
                                                }))
                                                .to_string_lossy()
                                                .to_string()
                                        },
                                    ),
                                    dm_filename: archive_file.name().to_owned(),
                                });
                            }
                        }
                        match mod_name {
                            Some(ModDefinition {
                                name,
                                dm_filename,
                                icon_filename,
                            }) => {
                                let banner_file = if icon_filename == "" {
                                    None
                                } else {
                                    let path = std::path::PathBuf::from(&icon_filename);
                                    let cleaned =
                                        path.to_string_lossy().to_string().replace("./", "");
                                    println!("Icon location {:?}", cleaned);
                                    let contents: Vec<u8> = archive
                                        .by_name(&cleaned)
                                        .unwrap()
                                        .bytes()
                                        .collect::<std::io::Result<Vec<u8>>>()
                                        .unwrap();
                                    let new_banner_file = NewFile::new(&icon_filename, &contents);
                                    let banner_file: File =
                                        new_banner_file.insert(&sqlx_db).await.unwrap();
                                    Some(banner_file)
                                };
                                let new_file =
                                    NewFile::new(content_type.get_filename().unwrap(), &contents);
                                let file: File = new_file.insert(&sqlx_db).await.unwrap();
                                let file_id = file.id;
                                let icon_file_id = banner_file.map(|bf| bf.id);
                                let name_ref = &name;
                                let dm_filename_ref = &dm_filename;
                                sqlx::query_as!(Mod,
                                    r#"INSERT INTO mods (name, dm_filename, file_id, icon_file_id)
                                    VALUES ($1, $2, $3, $4)
                                    RETURNING
                                        id "id: i32",
                                        dm_filename,
                                        name,
                                        file_id "file_id: i32",
                                        icon_file_id "icon_file_id: i32",
                                        crc"#,
                                    name_ref, dm_filename_ref, file_id, icon_file_id)
                                    .fetch_one(&sqlx_db).await.unwrap();
                                return Ok(HttpResponse::Found()
                                    .append_header((header::LOCATION, "/mods"))
                                    .finish());
                            }
                            _ => {
                                let upload_location = ListModsTemplate::get_upload_location();
                                return Ok(HttpResponse::Ok().content_type("text/html").body(
                                    (ListModsTemplate {
                                        errors: &vec!["Unable to find mod in zip (make sure there's a file that ends in .dm)".to_string()],
                                        mods: sqlx::query_as!(Mod,
                r#"SELECT
                    id "id: i32",
                    dm_filename,
                    name,
                    file_id "file_id: i32",
                    icon_file_id "icon_file_id: i32",
                    crc
                FROM mods"#)
                                            .fetch_all(&sqlx_db)
                                            .await
                                            .unwrap(),
                                        upload_location,
                                    })
                                    .render()
                                    .unwrap(),
                                ));
                            }
                        }
                    }
                    _ => {
                        let upload_location = ListModsTemplate::get_upload_location();
                        return Ok(HttpResponse::Ok().content_type("text/html").body(
                            (ListModsTemplate {
                                mods: sqlx::query_as!(Mod,
                r#"SELECT
                    id "id: i32",
                    dm_filename,
                    name,
                    file_id "file_id: i32",
                    icon_file_id "icon_file_id: i32",
                    crc
                FROM mods"#)
                                    .fetch_all(&sqlx_db)
                                    .await
                                    .unwrap(),
                                errors: &vec!["File is not a zip".to_string()],
                                upload_location,
                            })
                            .render()
                            .unwrap(),
                        ));
                    }
                }
            }
            _ => {}
        }
    }
    let upload_location = ListModsTemplate::get_upload_location();
    Ok(HttpResponse::Ok().content_type("text/html").body(
        (ListModsTemplate {
            mods: sqlx::query_as!(Mod,
                r#"SELECT
                    id "id: i32",
                    dm_filename,
                    name,
                    file_id "file_id: i32",
                    icon_file_id "icon_file_id: i32",
                    crc
                FROM mods"#)
                .fetch_all(&sqlx_db)
                .await
                .unwrap(),
            errors: &vec!["Form post is complete garbage".to_string()],
            upload_location,
        })
        .render()
        .unwrap(),
    ))
}

#[get("/mod/{id}")]
pub async fn details(
    (app_data, path): (web::Data<AppData>, web::Path<i64>),
) -> Result<HttpResponse> {
    let sqlx_db = app_data.sqlx_pool.clone();
    let cmod = sqlx::query_as!(Mod, r#"SELECT
        id "id: i32",
        dm_filename,
        name,
        file_id "file_id: i32",
        icon_file_id "icon_file_id: i32",
        crc
        FROM mods WHERE id=$1"#, *path)
        .fetch_one(&sqlx_db)
        .await
        .unwrap();
    Ok(HttpResponse::Ok().body((ModDetailsTemplate { cmod: &cmod }).render().unwrap()))
}

#[get("/mod/{id}/download")]
async fn download((app_data, modid): (web::Data<AppData>, web::Path<i64>)) -> Result<HttpResponse> {
    let sqlx_db = app_data.sqlx_pool.clone();
    let zipfile = sqlx::query!(
        "SELECT filebinary FROM files f INNER JOIN mods m ON m.file_id=f.id AND m.id=$1",
        *modid
    )
    .fetch_one(&sqlx_db)
    .await
    .unwrap();

    Ok(HttpResponse::Ok()
        .content_type("application/zip")
        .append_header(header::CacheControl(vec![
            header::CacheDirective::Public,
            header::CacheDirective::MaxAge(86400u32),
        ]))
        .body(zipfile.filebinary))
}
