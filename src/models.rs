use std::env;
use diesel::{Queryable, Selectable, SqliteConnection, Connection};
use dotenvy::dotenv;
use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Queryable, Selectable, Serialize, TS)]
#[ts(export, export_to = "rust_bindings.ts")]
#[serde(rename_all = "camelCase")]
#[diesel(table_name = crate::schema::games)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Game {
    pub id: String,
    pub title: String,
    #[serde(skip_serializing)]
    pub cover: Option<Vec<u8>>,
    pub vr_backend: String,
    pub steam_app_id: Option<i64>,
    pub proton_version: Option<String>,
    pub command_line: Option<String>,
    pub total_playtime_sec: i32,
}

pub fn establish_connection() -> SqliteConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    SqliteConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}