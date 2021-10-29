use super::AppData;
use crate::diesel::prelude::*;
use crate::models::{File, Mod, ModDefinition, NewFile, NewMod};
use actix_multipart::Multipart;
use actix_web::{get, http::header, post, web, HttpResponse, Result};
use askama::Template;
use diesel::RunQueryDsl;
use futures::{StreamExt, TryStreamExt};
use std::io::Read;

#[derive(Template)]
#[template(path = "mods/list.html")]
pub struct ListModsTemplate<'a> {
    errors: &'a [String],
    mods: Vec<Mod>,
}

impl<'a> ListModsTemplate<'a> {
    pub fn get_upload_location() -> String {
        return format!(
            "{}/mods/upload",
            std::env::var("UPLOAD_ADDRESS").unwrap_or(
                std::env::var("ADDRESS")
                    .expect("GAMES_ADDRESS env varnot specified, needed for map upload form")
            )
        );
    }
}

#[derive(Template)]
#[template(path = "mods/details.html")]
pub struct ModDetailsTemplate<'a> {
    cmod: &'a Mod,
}

#[get("/mods")]
pub async fn list(app_data: web::Data<AppData>) -> Result<HttpResponse> {
    let db = app_data.pool.get().expect("Unable to connect to database");
    Ok(HttpResponse::Ok().content_type("text/html").body(
        (ListModsTemplate {
            errors: &vec![],
            mods: crate::schema::mods::dsl::mods.load::<Mod>(&db).unwrap(),
        })
        .render()
        .unwrap(),
    ))
}

#[get("/images/mods/{id}.jpg")]
pub async fn image(
    (app_data, web::Path(path_id)): (web::Data<AppData>, web::Path<i32>),
) -> Result<HttpResponse> {
    let db = app_data.pool.get().unwrap();
    use crate::schema::files::dsl as files_dsl;
    use crate::schema::mods::dsl as mods_dsl;
    if let Ok((cmod, file)) = mods_dsl::mods
        .filter(mods_dsl::id.eq(path_id))
        .inner_join(
            crate::schema::files::dsl::files
                .on(files_dsl::id.nullable().eq(mods_dsl::icon_file_id)),
        )
        .get_result::<(Mod, File)>(&db)
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
        let mut file = std::fs::File::create(mods_dir.join(format!("{}.jpg", cmod.id))).unwrap();
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
            .set(header::CacheControl(vec![header::CacheDirective::MaxAge(
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
    let db = app_data.pool.get().expect("Unable to connect to database");
    while let Ok(Some(mut field)) = payload.try_next().await {
        let mut mod_name: Option<ModDefinition> = None;
        let content_type = field.content_disposition().unwrap();
        match content_type.get_name() {
            Some("archive") => {
                let mut contents: Vec<u8> = vec![];
                while let Some(bytes) = field.next().await {
                    contents.extend_from_slice(&bytes.unwrap());
                }
                match zip::ZipArchive::new(std::io::Cursor::new(&contents)) {
                    Ok(mut archive) => {
                        for i in 0..archive.len() {
                            let mut archive_file = archive.by_index(i).unwrap();
                            if archive_file.is_file()
                                && std::path::PathBuf::from(archive_file.name()).extension()
                                    == Some(std::ffi::OsStr::new("dm"))
                            {
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
                                        icon_filename: icon_filename_captures.map_or(
                                            "".to_string(),
                                            |c| {
                                                c.get(1).map_or("".to_string(), |c| {
                                                    c.as_str().to_owned()
                                                })
                                            },
                                        ),
                                        dm_filename: archive_file.name().to_owned(),
                                    });
                                }
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
                                    let cleaned = path.strip_prefix("./").unwrap_or(&path);
                                    let contents: Vec<u8> = archive
                                        .by_name(cleaned.to_str().unwrap())
                                        .unwrap()
                                        .bytes()
                                        .collect::<std::io::Result<Vec<u8>>>()
                                        .unwrap();
                                    let new_banner_file = NewFile::new(&icon_filename, &contents);
                                    let banner_file: File = new_banner_file.insert(&db);
                                    Some(banner_file)
                                };
                                let new_file =
                                    NewFile::new(content_type.get_filename().unwrap(), &contents);
                                let file: File = new_file.insert(&db);
                                let _inserted_mod: Mod =
                                    diesel::insert_into(crate::schema::mods::table)
                                        .values(NewMod {
                                            name: &name,
                                            dm_filename: &dm_filename,
                                            file_id: file.id,
                                            icon_file_id: banner_file
                                                .map_or(None, |bf| Some(bf.id)),
                                        })
                                        .on_conflict(crate::schema::mods::file_id)
                                        .do_update()
                                        .set(
                                            crate::schema::mods::file_id
                                                .eq(crate::schema::mods::file_id),
                                        )
                                        .get_result(&db)
                                        .unwrap();
                                let address = std::env::var("ADDRESS")
                                    .expect("ADDRESS env var not specified");
                                return Ok(HttpResponse::Found()
                                    .header(header::LOCATION, format!("{}/mods", address))
                                    .finish());
                            }
                            _ => {
                                return Ok(HttpResponse::Ok().content_type("text/html").body(
                                    (ListModsTemplate {
                                        errors: &vec!["Unable to find mod in zip".to_string()],
                                        mods: crate::schema::mods::dsl::mods.load(&db).unwrap(),
                                    })
                                    .render()
                                    .unwrap(),
                                ));
                            }
                        }
                    }
                    _ => {
                        return Ok(HttpResponse::Ok().content_type("text/html").body(
                            (ListModsTemplate {
                                mods: crate::schema::mods::dsl::mods.load(&db).unwrap(),
                                errors: &vec!["File is not a zip".to_string()],
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
    Ok(HttpResponse::Ok().content_type("text/html").body(
        (ListModsTemplate {
            mods: crate::schema::mods::dsl::mods.load(&db).unwrap(),
            errors: &vec!["Form post is complete garbage".to_string()],
        })
        .render()
        .unwrap(),
    ))
}

#[get("/mod/{id}")]
pub async fn details(
    (app_data, path): (web::Data<AppData>, web::Path<i32>),
) -> Result<HttpResponse> {
    let db = app_data.pool.get().expect("Unable to connect to database");
    let cmod = crate::schema::mods::dsl::mods
        .filter(crate::schema::mods::dsl::id.eq(*path))
        .get_result::<Mod>(&db)
        .unwrap();
    Ok(HttpResponse::Ok().body((ModDetailsTemplate { cmod: &cmod }).render().unwrap()))
}

#[get("/mod/{id}/download")]
async fn download((app_data, modid): (web::Data<AppData>, web::Path<i32>)) -> Result<HttpResponse> {
    let db = app_data.pool.get().expect("Unable to connect to database");
    let (_cmod, zipfile) = crate::schema::mods::dsl::mods
        .filter(crate::schema::mods::dsl::id.eq(*modid))
        .inner_join(
            crate::schema::files::dsl::files
                .on(crate::schema::files::dsl::id.eq(crate::schema::mods::dsl::file_id)),
        )
        .get_result::<(Mod, File)>(&db)
        .unwrap();

    Ok(HttpResponse::Ok()
        .content_type("application/zip")
        .body(zipfile.filebinary))
}
