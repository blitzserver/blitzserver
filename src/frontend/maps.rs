use super::AppData;
use crate::models::maps::{MapSummary, PlaneImage};
use crate::models::Map;
use crate::util::calculate_crc;
use ::image::ImageFormat;
use actix_multipart::Multipart;
use actix_web::http::header;
use actix_web::{get, post, web, HttpResponse, Result};
use askama::Template;
use async_std::future;
use core::slice;
use futures::{StreamExt, TryStreamExt};
use regex::Regex;
use serde::Deserialize;
use serde_dom5::D6mFile;
use std::io::Write;
use std::io::{BufRead, Read};
use std::path::{Path, PathBuf};

#[derive(Template)]
#[template(path = "maps/details.html")]
pub struct MapDetailsTemplate<'a> {
    map: &'a Map,
}

#[derive(serde::Deserialize)]
struct ThumbQuery {
    crop_x: Option<i32>,
    crop_y: Option<i32>,
    width: Option<i32>,
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

#[get("/maps/{path_id}/planes/{plane_id}/thumb.webp")]
pub async fn image(
    (app_data, path, query): (
        web::Data<AppData>,
        web::Path<(i32, i32)>,
        web::Query<ThumbQuery>,
    ),
) -> Result<HttpResponse> {
    let (map_id, plane_id) = path.into_inner();
    let db = app_data.sqlx_pool.clone();
    let image = crate::models::maps::Map::get_plane_image(map_id, plane_id, &db)
        .await
        .unwrap();
    let pathbuf = PathBuf::from(image.filename);
    let mut image = match pathbuf.extension().map(|e| e.to_str()).flatten() {
        Some("d6m") => {
            let d6m: D6mFile = bincode::deserialize(&image.bytes).unwrap();
            let mut img =
                crate::image::RgbImage::new(d6m.header.width as u32, d6m.header.height as u32);
            for (i, d) in d6m.heights.into_iter().enumerate() {
                img.put_pixel(
                    (i % d6m.header.width as usize) as u32,
                    (d6m.header.height - 1 - (i as i32 / d6m.header.width)) as u32,
                    crate::image::Rgb([
                        0u8,
                        ((d >> 15 & 1 ^ 1) * 200) as u8,
                        ((d >> 15 & 1) * 200) as u8,
                    ]),
                );
            }
            img
        }
        Some("tga") => {
            let mut image = crate::image::io::Reader::new(std::io::Cursor::new(&image.bytes));
            image.set_format(ImageFormat::Tga);
            image.decode().unwrap().to_rgb8()
        }
        Some("rgb") => {
            let mut image = crate::image::io::Reader::new(std::io::Cursor::new(&image.bytes));
            image.set_format(ImageFormat::Sgi);
            image.decode().unwrap().to_rgb8()
        }
        _ => {
            let image = crate::image::io::Reader::new(std::io::Cursor::new(&image.bytes));
            image.decode().unwrap().to_rgb8()
        }
    };
    if let Some(width) = query.width {
        let aspect_ratio = image.height() as f32 / image.width() as f32;
        image = crate::image::imageops::resize(
            &image,
            width as u32,
            (width as f32 * aspect_ratio) as u32,
            crate::image::imageops::FilterType::Lanczos3,
        );
    }
    let mut stuff = vec![];
    image.write_to(&mut stuff, ImageFormat::Jpeg).unwrap();
    Ok(HttpResponse::Ok()
        .append_header(header::CacheControl(vec![
            header::CacheDirective::Public,
            header::CacheDirective::MaxAge(31536000u32),
        ]))
        .content_type("image/jpeg")
        .body(stuff))
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
                    let image_capture = Regex::new("#imagefile (.+)").unwrap();
                    let mut map = crate::models::maps::Map {
                        is_randommap: false,
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

                        let mut image_names: Vec<String> = vec![];

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
                                if let Some(capture) = image_capture.captures(&line) {
                                    image_names.push(capture.get(1).unwrap().as_str().to_owned());
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
                        for name in image_names {
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
                    if let Err(e) = map.insert(&mut sqlx_db.acquire().await.unwrap()).await {
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
