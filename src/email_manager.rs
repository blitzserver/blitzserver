use crate::diesel::prelude::*;
use crate::models::{EmailConfig, Game, NewEmailConfig, Turn};
use crate::msgbus::{EmailConfigCreatedMsg, EmailConfigDeletedMsg, Msg, MsgBusRx, MsgBusTx};
use diesel::r2d2::ConnectionManager;
use diesel::PgConnection;
use lettre::{transport::smtp::authentication::Credentials, Transport};
use std::env;

#[derive(Clone)]
pub struct EmailManager {
    pub db_pool: r2d2::Pool<ConnectionManager<PgConnection>>,
    pub smtp_user: String,
    pub smtp_pass: String,
    pub smtp_server: String,
    pub hostname: String,
    pub msgbus_tx: MsgBusTx,
}

impl EmailManager {
    pub fn delete_config(&self, email_id: i32, email_address: String) {
        let db = self.db_pool.get().unwrap();
        if EmailConfig::delete(email_id, email_address, &db).unwrap() == 1 {
            self.msgbus_tx
                .send(Msg::EmailConfigDeleted(EmailConfigDeletedMsg {
                    id: email_id,
                }))
                .unwrap();
        }
    }
    pub fn create_config(
        &self,
        game_id: i32,
        address: String,
        subject: String,
        body: String,
        nation: i32,
        hours_remaining: i32,
        is_reminder: bool,
    ) {
        let db = self.db_pool.get().unwrap();
        let email_config = NewEmailConfig {
            nation_id: nation,
            game_id,
            hours_before_host: hours_remaining,
            email_address: address,
            body,
            subject,
            is_reminder,
        }
        .insert(&db)
        .unwrap();
        self.msgbus_tx
            .send(Msg::EmailConfigCreated(EmailConfigCreatedMsg {
                id: email_config.id,
            }))
            .unwrap();
    }

    fn send_notice_email<D>(email_config: &EmailConfig, db: &D)
    where
        D: diesel::Connection<Backend = diesel::pg::Pg>,
    {
        use crate::schema::games::dsl as games_dsl;
        use crate::schema::turns::dsl as turns_dsl;

        let hostname = env::var("HOSTNAME").expect("HOSTNAME must be set to accessible address");
        let game: Game = games_dsl::games
            .filter(games_dsl::id.eq(email_config.game_id))
            .get_result(db)
            .unwrap();
        let turn: Turn = turns_dsl::turns
            .filter(
                turns_dsl::game_id
                    .eq(email_config.game_id)
                    .and(turns_dsl::archived.eq(false)),
            )
            .order(turns_dsl::turn_number.desc())
            .limit(1)
            .get_result(db)
            .unwrap();
        let subject = email_config
            .subject
            .replace("%TURNNUMBER%", &turn.turn_number.to_string())
            .replace(
                "%GAMEIP%",
                &format!("{}:{}", hostname, game.port.unwrap_or(0)),
            )
            .replace(
                "%GAMEURL%",
                &format!("{}/game/{}", hostname, game.id),
            )
            .replace("%GAMENAME%", &game.name)
            .replace("%HOURSREMAINING%", &game.next_turn_string());
        let body = email_config
            .body
            .replace("%TURNNUMBER%", &turn.turn_number.to_string())
            .replace(
                "%GAMEIP%",
                &format!("{}:{}", hostname, game.port.unwrap_or(0)),
            )
            .replace(
                "%GAMEURL%",
                &format!("{}/game/{}", hostname, game.id),
            )
            .replace("%GAMENAME%", &game.name)
            .replace("%HOURSREMAINING%", &game.next_turn_string());
        let email = lettre::Message::builder()
            .from(format!("Admin <admin@{}>", hostname).parse().unwrap())
            .to(format!("<{}>", email_config.email_address).parse().unwrap())
            .subject(subject)
            .body(body)
            .unwrap();
        lettre::transport::smtp::SmtpTransport::relay(
            &env::var("SMTP_SERVER").expect("SMTP_SERVER must be said to the SMTP server"),
        )
        .unwrap()
        .credentials(Credentials::new(
            env::var("SMTP_USER").expect("SMTP_USER must be said to the SMTP user"),
            env::var("SMTP_PASS").expect("SMTP_PASS must be said to the SMTP password"),
        ))
        .build()
        .send(&email)
        .unwrap();
        email_config.mark_sent(turn.turn_number, db).unwrap();
    }

    pub fn monitor(self, msgbus_rx: MsgBusRx) {
        std::thread::spawn(move || loop {
            {
                let db = self.db_pool.get().unwrap();
                for notification in EmailConfig::get_due_reminders(&db).unwrap() {
                    Self::send_notice_email(&notification, &db);
                }
                for notification in EmailConfig::get_due_notifications(&db).unwrap() {
                    Self::send_notice_email(&notification, &db);
                }
            }
            std::thread::sleep(std::time::Duration::from_secs(5));
        });
    }
}
