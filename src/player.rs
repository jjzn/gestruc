use chrono::Utc;
use rocket::http::Status;
use rocket::request::{FromRequest, Request, Outcome};
use rocket::serde::Serialize;
use rocket_db_pools::{Connection, sqlx};
use rocket_db_pools::sqlx::Row;

use crate::AppData;
use crate::error::AppError;
use crate::team::Team;

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct Player {
    pub id: String,
    pub email: String,
    pub name: String
}

impl Player {
    pub async fn get_teams(&self, db: &mut Connection<AppData>) -> Result<Vec<Team>, AppError> {
        let rows = sqlx::query("SELECT id, name, captainId, partnerId, tournamentName, tournamentEdition FROM teams WHERE captainId = $1 OR partnerId = $1")
            .bind(&self.id)
            .fetch_all(&mut ***db).await?;

        rows.iter().map(Team::try_from).collect()
    }

    pub async fn try_fetch(id: String, db: &mut Connection<AppData>) -> Result<Self, AppError> {
        let row = sqlx::query("SELECT email, name FROM players WHERE id = $1")
            .bind(&id)
            .fetch_one(&mut ***db).await?;

        Ok(Self { id, email: row.try_get("email")?, name: row.try_get("name")? })
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for Player {
    // We do not need to display detailed app errors, we only care about the HTTP status
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let session_id = match req.cookies().get("sessionId") {
            Some(cookie) => cookie.value(),
            None => return Outcome::Forward(Status::Unauthorized)
        };

        let mut db = match req.guard::<Connection<AppData>>().await {
            Outcome::Success(db) => db,
            // We use Outcome::Error instead of Outcome::Forward because we want
            // the user to know that a server error ocurred
            _ => return Outcome::Error((Status::InternalServerError, ()))
        };

        let row = match sqlx::query("SELECT playerId, expires FROM sessions WHERE id = $1").bind(session_id)
            .fetch_one(&mut **db).await {
            Ok(row) => row,
            Err(_) => return Outcome::Forward(Status::Unauthorized)
        };

        let player_id: String = row.get(0);
        let expiration: i64 = row.get(1);

        if Utc::now().timestamp() >= expiration {
            return Outcome::Forward(Status::Unauthorized);
        }

        // TODO: use Player::try_fetch
        let row = match sqlx::query("SELECT email, name FROM players WHERE id = $1").bind(&player_id)
            .fetch_one(&mut **db).await {
            Ok(row) => row,
            Err(_) => return Outcome::Error((Status::InternalServerError, ()))
        };

        Outcome::Success(Player { id: player_id, email: row.get(0), name: row.get(1) })
    }
}
