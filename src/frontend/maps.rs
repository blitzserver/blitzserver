use super::AppData;
use crate::models::maps::{MapSummary, PlaneImage};
use crate::models::Map;
use crate::util::calculate_crc;
use actix_multipart::Multipart;
use actix_web::http::header;
use actix_web::{get, post, web, HttpResponse, Result};
use askama::Template;
use futures::{StreamExt, TryStreamExt};
use libvips::ops;
use regex::Regex;
use std::io::Write;
use std::io::{BufRead, Read};
use std::path::Path;

#[derive(Template)]
#[template(path = "maps/details.html")]
pub struct MapDetailsTemplate<'a> {
    map: &'a Map,
}

#[get("/map/{id}")]
pub async fn details(
    (app_data, path_id): (web::Data<AppData>, web::Path<i32>),
) -> Result<HttpResponse> {
    let sqlx_pool = app_data.sqlx_pool.clone();
    let map = sqlx::query_as!(Map, "SELECT * FROM maps WHERE id=$1", *path_id)
        .fetch_one(&sqlx_pool)
        .await
        .unwrap();
    Ok(HttpResponse::Ok()
        .content_type("text/html")
        .body((MapDetailsTemplate { map: &map }).render().unwrap()))
}

#[get("/images/maps/{id}.jpg")]
pub async fn image(
    (app_data, path_id): (web::Data<AppData>, web::Path<i32>),
) -> Result<HttpResponse> {
    let db = app_data.sqlx_pool.clone();
    let (_, image, _) = Map::get_files(*path_id, &db).await.unwrap();
    let mut jpg_bytes: Vec<u8> = Vec::new();
    let reader = crate::image::io::Reader::with_format(
        std::io::Cursor::new(image.filebinary),
        crate::image::ImageFormat::Tga,
    )
    .decode()
    .unwrap();
    reader
        .write_to(&mut jpg_bytes, crate::image::ImageFormat::Jpeg)
        .unwrap();
    Ok(HttpResponse::Ok()
        .content_type("application/jpg")
        .body(jpg_bytes))
}

#[derive(Template)]
#[template(path = "maps/list.html")]
struct ListMapsTemplate<'a> {
    maps: &'a [MapSummary],
    errors: &'a [String],
}

impl<'a> ListMapsTemplate<'a> {
    pub fn get_upload_location() -> String {
        return format!(
            "{}/maps/upload",
            std::env::var("UPLOAD_ADDRESS").unwrap_or(
                std::env::var("ADDRESS")
                    .expect("GAMES_ADDRESS env varnot specified, needed for map upload form")
            )
        );
    }
}

#[get("/map/{id}/download")]
async fn download((app_data, mapid): (web::Data<AppData>, web::Path<i32>)) -> Result<HttpResponse> {
    let db = app_data.sqlx_pool.clone();

    let archive = crate::models::maps::Map::get_archive(*mapid, &db)
        .await
        .unwrap();
    println!("Fetched");
    Ok(HttpResponse::Ok()
        .content_type("application/zip")
        .append_header(header::CacheControl(vec![
            header::CacheDirective::Public,
            header::CacheDirective::MaxAge(86400u32),
        ]))
        .append_header(header::ContentDisposition::attachment(archive.filename))
        .body(archive.file_binary))
}

#[get("/maps")]
async fn list(app_data: web::Data<AppData>) -> Result<HttpResponse> {
    let sqlx_db = app_data.sqlx_pool.clone();
    let result_maps = MapSummary::get_map_list(&sqlx_db).await.unwrap();
    Ok(HttpResponse::Ok().content_type("text/html").body(
        (ListMapsTemplate {
            errors: &vec![],
            maps: &result_maps,
        })
        .render()
        .unwrap(),
    ))
}
#[post("/maps/upload")]
async fn upload_post(
    (app_data, mut payload): (web::Data<AppData>, Multipart),
) -> Result<HttpResponse> {
    let sqlx_db = app_data.sqlx_pool.clone();

    let mut errors: Vec<String> = vec![];
    {
        'outer: while let Ok(Some(mut field)) = payload.try_next().await {
            if errors.len() > 0 {
                continue;
            }
            let content_type = field.content_disposition().clone();
            match content_type.get_name() {
                Some("map") => {
                    let mut contents: Vec<u8> = vec![];
                    while let Some(bytes) = field.next().await {
                        contents.extend_from_slice(&bytes.unwrap());
                    }
                    let Ok(mut archive) = zip::ZipArchive::new(std::io::Cursor::new(&contents))
                    else {
                        errors.push("Uploaded map is not a .zip archive".to_string());
                        break 'outer;
                    };

                    let Some(central_dir) =
                        archive.file_names().find(|f| f.contains(".map")).map(|f| {
                            std::path::PathBuf::from(f.to_owned())
                                .parent()
                                .map(|p| p.to_owned())
                        })
                    else {
                        errors.push("Zip does not contain any .map files".to_owned());
                        break 'outer;
                    };

                    let mut mapfiles = vec![];
                    for name in archive.file_names() {
                        if name.ends_with(".map") {
                            mapfiles.push(name.to_owned());
                        }
                    }
                    mapfiles.sort();
                    let name_capture = Regex::new("#dom2title (.+)").unwrap();
                    let mut map = crate::models::maps::Map {
                        name: "Unnamed Map".to_owned(),
                        planes: vec![],
                    };
                    for plane_filename in mapfiles {
                        let mut plane = crate::models::maps::Plane {
                            name: "".to_string(),
                            images: vec![],
                            bytes: vec![],
                            hash: 0,
                        };

                        {
                            let base_plane_file = archive.by_name(&plane_filename).unwrap();
                            let reader = std::io::BufReader::new(base_plane_file);
                            plane.name = std::path::PathBuf::from(&plane_filename)
                                .file_name()
                                .unwrap()
                                .to_string_lossy()
                                .to_string();
                            for line in reader.lines().flatten() {
                                if let Some(capture) = name_capture.captures(&line) {
                                    map.name = capture.get(1).unwrap().as_str().to_owned();
                                }
                            }
                        }
                        {
                            let mut base_plane_file = archive.by_name(&plane_filename).unwrap();
                            base_plane_file.read_to_end(&mut plane.bytes).unwrap();
                        }
                        {
                            let base_plane_file = archive.by_name(&plane_filename).unwrap();
                            plane.hash = calculate_crc(base_plane_file);
                        }
                        let mapwithoutext = Path::new(&plane.name).file_stem().unwrap();
                        let planeimage_regex = Regex::new(&format!(
                            r#"{}([a-zA-Z]+?)?\.(tga|d6m|rgb)"#,
                            mapwithoutext.to_string_lossy()
                        ))
                        .unwrap();
                        let names = archive
                            .file_names()
                            .map(|filename| {
                                println!("Regex {:?}", planeimage_regex);
                                let fname = std::path::PathBuf::from(filename)
                                    .file_name()
                                    .unwrap()
                                    .to_string_lossy()
                                    .to_string();
                                planeimage_regex
                                    .captures(&fname)
                                    .map(|c| c.get(0).unwrap().as_str().to_owned())
                            })
                            .flatten()
                            .collect::<Vec<String>>();
                        for name in names {
                            let hash = {
                                let hash_file = match &central_dir {
                                    Some(cd) => cd.join(&name).to_string_lossy().to_string(),
                                    None => name.clone(),
                                };
                                let img = match archive.by_name(&hash_file) {
                                    Err(_) => {
                                        errors
                                            .push(format!("Missing file in archive {}", hash_file));
                                        break 'outer;
                                    }
                                    Ok(i) => i,
                                };
                                calculate_crc(img)
                            };
                            let mut thing = PlaneImage {
                                filename: name.clone(),
                                hash,
                                bytes: vec![],
                            };
                            {
                                let hash_file = match &central_dir {
                                    Some(cd) => cd.join(&name).to_string_lossy().to_string(),
                                    None => name,
                                };
                                let mut img = match archive.by_name(&hash_file) {
                                    Err(_) => {
                                        errors
                                            .push(format!("Missing file in archive {}", hash_file));
                                        break 'outer;
                                    }
                                    Ok(i) => i,
                                };
                                img.read_to_end(&mut thing.bytes).unwrap();
                            };
                            plane.images.push(thing);
                        }
                        map.planes.push(plane);
                    }
                    if let Err(e) = map.insert(sqlx_db.acquire().await.unwrap()).await {
                        println!("Failed to create new map {:?}", e);
                        errors.push(format!("Failed inserting map: {:?}", e));
                        break 'outer;
                    }
                }
                _ => {}
            };
        }
    }
    if errors.len() > 0 {
        let result_maps = MapSummary::get_map_list(&sqlx_db).await.unwrap();
        return Ok(HttpResponse::Ok().content_type("text/html").body(
            (ListMapsTemplate {
                errors: &errors,
                maps: &result_maps,
            })
            .render()
            .unwrap(),
        ));
    } else {
        let address = std::env::var("ADDRESS").expect("ADDRESS env var not specified");
        return Ok(HttpResponse::Found()
            .append_header((header::LOCATION, format!("{}/maps", address)))
            .finish());
    }
}

pub async fn map_thumb_handler(
    req: actix_web::dev::ServiceRequest,
) -> actix_web::Result<actix_web::dev::ServiceResponse> {
    let app_data = req
        .app_data::<actix_web::web::Data<crate::frontend::AppData>>()
        .unwrap();
    let db_pool = app_data.sqlx_pool.clone();
    let (http_req, _payload) = req.into_parts();
    let map_id_regex = regex::Regex::new(r#"/images/maps/([0-9]+)_([0-9]+).webp"#).unwrap();
    if let Some(captures) = map_id_regex.captures(http_req.path()) {
        let id_i32: i32 = captures.get(1).unwrap().as_str().parse().unwrap();
        let desired_width: f64 = captures.get(2).unwrap().as_str().parse().unwrap();
        let map = sqlx::query!(
            "
        SELECT m.id, filebinary, filename
        FROM maps m 
        LEFT JOIN files f ON f.id=m.tgafile_id
        WHERE m.id=$1",
            id_i32
        )
        .fetch_one(&db_pool)
        .await
        .unwrap();
        let mut vips_image = ops::magickload_buffer(&map.filebinary).unwrap();
        let cur_width = vips_image.get_width() as f64;
        let scale = desired_width / cur_width;
        vips_image = ops::resize(&vips_image, scale).unwrap();
        vips_image = ops::autorot(&vips_image).unwrap();
        let file_loc = format!("./images/maps/{}_{}.webp", map.id, desired_width);
        let mut options = ops::WebpsaveOptions::default();
        options.preset = ops::ForeignWebpPreset::Icon;
        options.q = 100;
        ops::webpsave_with_opts(&vips_image, &file_loc, &options).unwrap();
        let mut webp_file = std::fs::File::open(file_loc).unwrap();
        let mut buff = vec![];
        webp_file.read_to_end(&mut buff).unwrap();
        Ok(actix_web::dev::ServiceResponse::new(
            http_req,
            HttpResponse::Ok().content_type("image/webp").body(buff),
        ))
    } else {
        Ok(actix_web::dev::ServiceResponse::new(
            http_req,
            HttpResponse::NotFound().finish(),
        ))
    }
}
