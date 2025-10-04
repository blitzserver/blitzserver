#![feature(duration_constants)]
#![feature(let_chains)]
#![recursion_limit = "256"]
#![feature(try_blocks)]
#![feature(async_closure)]
#![feature(type_alias_impl_trait)]
#![feature(get_mut_unchecked)]
#![feature(never_type)]
extern crate image;
extern crate num_derive;
extern crate num_traits;
extern crate sqlx;

use actix_session::SessionMiddleware;
use actix_session::storage::CookieSessionStore;
use actix_web::http::header;
use actix_web::{App, HttpResponse, HttpServer, Result, get, middleware};
use sqlx::migrate::MigrateDatabase;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode};
use std::env;
use std::fs::exists;
use std::str::FromStr;

pub mod dom5_conn;
pub mod dom5_gateway;
pub mod dom5_scheduler;
pub mod dom5_turn;
pub mod files;
pub mod frontend;
pub mod models;
pub mod packets;
pub mod statusdump;
pub mod util;
pub mod views;

use frontend::AppData;
use models::settings::Setting;

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
    let db_loc = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://./db".to_string());
    let sqlx_pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(20)
        .connect_with(
            SqliteConnectOptions::from_str(&db_loc)
                .unwrap()
                .journal_mode(SqliteJournalMode::Wal)
                .create_if_missing(true),
        )
        .await
        .unwrap();
    sqlx::migrate!("./migrations")
        .run(&sqlx_pool)
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
        sqlx_pool: sqlx_pool.clone(),
    };

    // Get settings that are needed for server configuration
    let cookie_key = match Setting::get_or_env(&sqlx_pool, "COOKIE_KEY").await {
        Some(key) => key,
        None => {
            // Generate a random 64-character hex string for the cookie key
            use rand::Rng;
            let mut rng = rand::thread_rng();
            let random_bytes: [u8; 32] = std::array::from_fn(|_| rng.r#gen::<u8>());

            // Convert bytes to hex string manually
            let generated_key = random_bytes
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<String>();

            // Save the generated key to the database
            Setting::update(&sqlx_pool, "COOKIE_KEY", Some(&generated_key))
                .await
                .expect("Failed to save generated COOKIE_KEY to database");

            println!("Generated new COOKIE_KEY and saved to database");
            generated_key
        }
    };
    let web_port = Setting::get_or_env(&sqlx_pool, "WEB_PORT")
        .await
        .unwrap_or_else(|| "8080".to_string());

    // Check if ADMIN_PASSWORD is set, and if not, open browser to admin page
    let admin_password_set = Setting::get_or_env(&sqlx_pool, "ADMIN_PASSWORD")
        .await
        .is_some();
    if !admin_password_set {
        let admin_url = format!("http://localhost:{}/admin", web_port);
        println!("⚠️  ADMIN_PASSWORD not set! Opening browser to admin page...");
        println!("   Admin page: {}", admin_url);

        // Spawn browser opening in background after a short delay
        let url_clone = admin_url.clone();
        std::thread::spawn(move || {
            // Wait a bit for server to start
            std::thread::sleep(std::time::Duration::from_secs(2));

            // Try to open browser with platform-specific commands
            let open_cmd = if cfg!(target_os = "macos") {
                std::process::Command::new("open").arg(&url_clone).spawn()
            } else if cfg!(target_os = "windows") {
                std::process::Command::new("cmd")
                    .args(["/c", "start", &url_clone])
                    .spawn()
            } else {
                // Linux/Unix - try common browsers
                std::process::Command::new("xdg-open")
                    .arg(&url_clone)
                    .spawn()
                    .or_else(|_| {
                        std::process::Command::new("firefox")
                            .arg(&url_clone)
                            .spawn()
                    })
                    .or_else(|_| {
                        std::process::Command::new("chromium")
                            .arg(&url_clone)
                            .spawn()
                    })
                    .or_else(|_| {
                        std::process::Command::new("google-chrome")
                            .arg(&url_clone)
                            .spawn()
                    })
            };

            if let Err(e) = open_cmd {
                println!("Could not open browser automatically: {}", e);
                println!("Please manually open: {}", url_clone);
            }
        });
    }

    HttpServer::new(move || {
        // TODO: Hack here, we create the map thumbnail dir VERY ahead of time since the file
        // watcher needs it to be there otherwise it wont discover new files
        let maps_dir = std::path::PathBuf::from("./images/maps");
        if !maps_dir.exists() {
            std::fs::create_dir_all(&maps_dir).unwrap();
        }
        let secret_key = actix_web::cookie::Key::from(cookie_key.as_bytes());

        App::new()
            .wrap(middleware::Logger::default())
            .wrap(middleware::Compress::default())
            .data(app_data.clone())
            .wrap(SessionMiddleware::new(
                CookieSessionStore::default(),
                secret_key,
            ))
            .app_data(
                serde_qs::actix::QsQueryConfig::default()
                    .qs_config(serde_qs::Config::new(10, false)),
            )
            .service(index)
            .service(styles)
            .service(frontend::maps::details)
            .service(frontend::maps::image)
            .service(frontend::maps::upload_post)
            .service(frontend::maps::list)
            // .service(frontend::maps::download)
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
            .service(frontend::settings::admin)
            .service(frontend::settings::admin_post)
            .service(frontend::settings::admin_upload_binary)
            // .service(frontend::api::game_json)
            .service(actix_files::Files::new("/", "./content"))
    })
    .bind(format!("0.0.0.0:{}", web_port))?
    .run()
    .await
}
