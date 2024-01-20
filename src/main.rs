#![feature(duration_constants)]
#![feature(let_chains)]
#![recursion_limit = "256"]
#![feature(try_blocks)]
#![feature(extract_if)]
#![feature(async_closure)]
#![feature(type_alias_impl_trait)]
#![feature(get_mut_unchecked)]
#![feature(never_type)]
#[macro_use]
extern crate async_std;
extern crate base64;
extern crate byteorder;
extern crate chrono;
extern crate crc;
extern crate crossbeam_channel;
extern crate derive_more;
extern crate encoding;
extern crate fletcher;
extern crate image;
extern crate libvips;
extern crate num_derive;
extern crate num_traits;
extern crate reqwest;
extern crate serde_json;
extern crate smol;
extern crate sqlx;
extern crate thiserror;
extern crate tungstenite;
extern crate url;
extern crate zip;

use actix_session::SessionMiddleware;
use actix_session::storage::CookieSessionStore;
use actix_web::http::header;
use actix_web::{get, middleware, App, HttpResponse, HttpServer, Result};
use dotenv::dotenv;
use std::env;

pub mod discord;
pub mod dom5_conn;
pub mod dom5_gateway;
pub mod dom5_scheduler;
pub mod dom5_turn;
pub mod email_manager;
pub mod files;
pub mod frontend;
pub mod models;
pub mod msgbus;
pub mod packets;
pub mod statusdump;
pub mod util;
pub mod views;

use frontend::AppData;

#[get("/")]
async fn index() -> Result<HttpResponse> {
    Ok(HttpResponse::PermanentRedirect()
        .append_header((header::LOCATION, "/games"))
        .finish())
}

#[get("/styles.css")]
async fn styles() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok()
        .append_header(header::CacheControl(vec![header::CacheDirective::MaxAge(
            86400u32,
        )]))
        .content_type("text/css")
        .body(
            &concat!(
                include_str!("../content/map-list.css"),
                include_str!("../content/mod-list.css"),
                include_str!("../content/styles.css")
            )[..],
        ))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    // Db
    dotenv().ok();
    let bus = msgbus::MsgBus::new();

    let sqlx_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(20)
        .connect(&env::var("DATABASE_URL").expect("DATABASE_URL must be set"))
        .await
        .unwrap();
    let gateway = dom5_gateway::Gateway {
        pool: sqlx_pool.clone(),
    };
    gateway.run();
    let scheduler = dom5_scheduler::Scheduler {
        db_pool: sqlx_pool.clone(),
    };
    scheduler.run();

    let app_data = AppData {
        msgbus_sender: bus.sender.clone(),
        sqlx_pool: sqlx_pool.clone(),
        email_manager: crate::email_manager::EmailManager {
            msgbus_tx: bus.sender.clone(),
            sqlx_pool: sqlx_pool.clone(),
            smtp_user: env::var("SMTP_USER").expect("SMTP_USER must be said to the SMTP user"),
            smtp_pass: env::var("SMTP_PASS").expect("SMTP_PASS must be said to the SMTP password"),
            smtp_server: env::var("SMTP_SERVER")
                .expect("SMTP_SERVER must be said to the SMTP server"),
            hostname: env::var("HOSTNAME").expect("HOSTNAME must be set to accessible address"),
        },
    };
    crate::email_manager::EmailManager {
        msgbus_tx: bus.sender.clone(),
        sqlx_pool: sqlx_pool.clone(),
        smtp_user: env::var("SMTP_USER").expect("SMTP_USER must be said to the SMTP user"),
        smtp_pass: env::var("SMTP_PASS").expect("SMTP_PASS must be said to the SMTP password"),
        smtp_server: env::var("SMTP_SERVER").expect("SMTP_SERVER must be said to the SMTP server"),
        hostname: env::var("HOSTNAME").expect("HOSTNAME must be set to accessible address"),
    }
    .monitor()
    .await;

    HttpServer::new(move || {
        // TODO: Hack here, we create the map thumbnail dir VERY ahead of time since the file
        // watcher needs it to be there otherwise it wont discover new files
        let maps_dir = std::path::PathBuf::from("./images/maps");
        if !maps_dir.exists() {
            std::fs::create_dir_all(&maps_dir).unwrap();
        }
        let secret_key = actix_web::cookie::Key::from(env::var("COOKIE_KEY")
            .expect("Missing COOKIE_KEY phrase, needed for signed cookies!")
            .as_bytes());

        App::new()
            .wrap(middleware::Logger::default())
            .wrap(middleware::Compress::default())
            .data(app_data.clone())
            .wrap(SessionMiddleware::new(
                CookieSessionStore::default(),
                secret_key
            ))
            .app_data(
                serde_qs::actix::QsQueryConfig::default()
                    .qs_config(serde_qs::Config::new(10, false)),
            )
            .service(
                actix_files::Files::new("/images/maps", "./images/maps")
                    .default_handler(crate::frontend::maps::map_thumb_handler),
            )
            .service(index)
            .service(styles)
            .service(frontend::maps::details)
            .service(frontend::maps::image)
            .service(frontend::maps::upload_post)
            .service(frontend::maps::list)
            .service(frontend::maps::download)
            .service(frontend::games::motd)
            .service(frontend::games::watch)
            .service(frontend::games::timer)
            .service(frontend::games::postpone)
            .service(frontend::games::forcehost)
            .service(frontend::games::old_details)
            .service(frontend::games::details)
            .service(frontend::games::launch)
            .service(frontend::games::list)
            .service(frontend::games::create_post)
            .service(frontend::games::remove_post)
            .service(frontend::games::settings_post)
            .service(frontend::games::emails_delete)
            .service(frontend::games::emails_post)
            .service(frontend::games::archive_post)
            .service(frontend::games::rollback_post)
            .service(frontend::games::unstart_post)
            .service(frontend::games::assign_team)
            .service(frontend::games::assign_start)
            .service(frontend::games::download_turn)
            .service(frontend::mods::list)
            .service(frontend::mods::upload_post)
            .service(frontend::mods::details)
            .service(frontend::mods::download)
            .service(frontend::mods::image)
            .service(frontend::help::help)
            .service(frontend::binaries::list)
            .service(frontend::binaries::upload)
            .service(actix_files::Files::new("/", "./content"))
    })
    .bind(format!("0.0.0.0:{}", env::var("WEB_PORT").unwrap_or("8080".to_string())))?
    .run()
    .await
}
