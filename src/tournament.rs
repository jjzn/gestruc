use rocket::serde::Serialize;
use rocket_db_pools::{Connection, sqlx};
use rocket_db_pools::sqlx::Row;

use crate::AppData;
use crate::error::AppError;
use crate::team::Team;

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct Tournament {
    pub name: String,
    pub edition: String
}

impl Tournament {
    pub async fn try_fetch(name: String, edition: String, db: &mut Connection<AppData>) -> Result<Self, AppError> {
        // Even tournament does only have two fields which we already know, we still query the DB
        // for two reasons: validation of input data (does the tournament exist?) and future schema changes
        let row = sqlx::query("SELECT name, edition FROM tournaments WHERE name = $1 AND edition = $2")
            .bind(name).bind(edition)
            .fetch_one(&mut ***db).await?;

        Ok(Self { name: row.try_get("name")?, edition: row.try_get("edition")? })
    }

    pub async fn get_teams(&self, db: &mut Connection<AppData>) -> Result<Vec<Team>, AppError> {
        let rows = sqlx::query("SELECT id, name, captainId, partnerId, tournamentName, tournamentEdition FROM teams WHERE tournamentName = $1 AND tournamentEdition = $2")
            .bind(&self.name).bind(&self.edition)
            .fetch_all(&mut ***db).await?;

        rows.iter().map(Team::try_from).collect()
    }
}

impl TryFrom<&sqlx::sqlite::SqliteRow> for Tournament {
    type Error = AppError;

    fn try_from(row: &sqlx::sqlite::SqliteRow) -> Result<Self, Self::Error> {
        Ok(Self {
            name: row.try_get("name")?,
            edition: row.try_get("edition")?
        })
    }
}
