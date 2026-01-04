use rocket::serde::Serialize;
use rocket_db_pools::{Connection, sqlx};
use rocket_db_pools::sqlx::Row;

use crate::AppData;
use crate::error::AppError;
use crate::game::Game;
use crate::player::Player;

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct Team {
    pub id: String,
    pub name: String,
    pub captain_id: String,
    pub partner_id: String,
    pub tournament_name: String,
    pub tournament_year: u16
}

impl Team {
    pub async fn get_games(&self, db: &mut Connection<AppData>) -> Result<Vec<Game>, AppError> {
        let rows = sqlx::query("SELECT games.id, date, scoresA, scoresB, acceptedByA, acceptedByB, a.name AS teamNameA, b.name AS teamNameB, a.id AS teamIdA, b.id AS teamIdB FROM games JOIN teams AS a ON a.id = teamA JOIN teams AS b ON b.id = teamB WHERE teamA = $1 OR teamB = $1")
            .bind(&self.id)
            .fetch_all(&mut ***db).await?;

        rows.iter().map(Game::try_from).collect()
    }

    // TODO: should probably return Result<Self, AppError>
    pub async fn try_fetch(id: String, db: &mut Connection<AppData>) -> Option<Self> {
        let row = sqlx::query("SELECT name, captainId, partnerId, tournamentName, tournamentYear FROM teams WHERE id = $1")
            .bind(&id)
            .fetch_one(&mut ***db).await.ok()?;

        Some(Self {
            id,
            name: row.try_get("name").ok()?,
            captain_id: row.try_get("captainId").ok()?,
            partner_id: row.try_get("partnerId").ok()?,
            tournament_name: row.try_get("tournamentName").ok()?,
            tournament_year: row.try_get("tournamentYear").ok()?
        })
    }

    pub fn has_member(&self, player: Player) -> bool {
        player.id == self.captain_id || player.id == self.partner_id
    }
}

impl TryFrom<&sqlx::sqlite::SqliteRow> for Team {
    type Error = AppError;

    fn try_from(row: &sqlx::sqlite::SqliteRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            captain_id: row.try_get("captainId")?,
            partner_id: row.try_get("partnerId")?,
            tournament_name: row.try_get("tournamentName")?,
            tournament_year: row.try_get("tournamentYear")?
        })
    }
}
