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
    pub tournament_edition: String
}

// Only used for serializing to templates, so we have team and player info all in the same place
#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct TeamInfo {
    team: Team,
    captain: Player,
    partner: Player
}

impl Team {
    pub async fn get_games(&self, db: &mut Connection<AppData>) -> Result<Vec<Game>, AppError> {
        let rows = sqlx::query("SELECT games.id, date, scoresA, scoresB, acceptedByA, acceptedByB, a.name AS teamNameA, b.name AS teamNameB, a.id AS teamIdA, b.id AS teamIdB FROM games JOIN teams AS a ON a.id = teamA JOIN teams AS b ON b.id = teamB WHERE teamA = $1 OR teamB = $1")
            .bind(&self.id)
            .fetch_all(&mut ***db).await?;

        rows.iter().map(Game::try_from).collect()
    }

    pub async fn get_players(&self, db: &mut Connection<AppData>) -> Result<(Player, Player), AppError> {
        let captain = Player::try_fetch(self.captain_id.clone(), db).await?;
        let partner = Player::try_fetch(self.partner_id.clone(), db).await?;

        Ok((captain, partner))
    }

    pub async fn get_team_info(self, db: &mut Connection<AppData>) -> Result<TeamInfo, AppError> {
        let (captain, partner) = self.get_players(db).await?;

        Ok(TeamInfo { team: self, captain, partner })
    }

    pub async fn try_fetch(id: String, db: &mut Connection<AppData>) -> Result<Self, AppError> {
        let row = sqlx::query("SELECT id, name, captainId, partnerId, tournamentName, tournamentEdition FROM teams WHERE id = $1")
            .bind(&id)
            .fetch_one(&mut ***db).await?;

        (&row).try_into()
    }

    pub fn has_member(&self, player: &Player) -> bool {
        player.id == self.captain_id || player.id == self.partner_id
    }

    pub fn to_sql_insert(&self) -> String {
        format!("INSERT INTO teams (id, name, captainId, partnerId, tournamentName, tournamentEdition) VALUES ('{}', '{}', '{}', '{}', '{}', '{}');",
            self.id,
            self.name,
            self.captain_id,
            self.partner_id,
            self.tournament_name,
            self.tournament_edition)
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
            tournament_edition: row.try_get("tournamentEdition")?
        })
    }
}
