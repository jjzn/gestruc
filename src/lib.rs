pub mod error;
pub mod game;
pub mod player;
pub mod team;
pub mod tournament;

use rocket_db_pools::{Database, sqlx};

#[derive(Database)]
#[database("appdata")]
pub struct AppData(sqlx::SqlitePool);
