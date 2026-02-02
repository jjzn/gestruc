use chrono::{DateTime, NaiveDateTime, Utc};
use chrono_tz::Europe;
use rocket::{form::Form, serde::Serialize};
use rocket_db_pools::{Connection, sqlx::{self, Row}};

use crate::{AppData, error::AppError};

#[derive(FromForm)]
#[allow(non_snake_case)]
pub struct GameData {
    pub opponentName: String,
    pub date: String,
    pub score1A: u8,
    pub score1B: u8,
    pub score2A: u8,
    pub score2B: u8,
    pub score3A: u8,
    pub score3B: u8,
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct GameScore(u8, u8, u8);

impl From<u32> for GameScore {
    fn from(score: u32) -> Self {
        Self(score as u8, (score >> 8) as u8, (score >> 16) as u8)
    }
}

impl From<GameScore> for u32 {
    fn from(score: GameScore) -> Self {
        (score.0 as u32) | ((score.1 as u32) << 8) | ((score.2 as u32) << 16)
    }
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct Game {
    pub id: String,
    pub date: DateTime<Utc>,
    pub scores: [GameScore; 2],
    pub accepted: [Option<DateTime<Utc>>; 2],
    pub team_names: [String; 2],
    pub team_ids: [String; 2]
}

impl Game {
    pub async fn try_fetch(id: String, db: &mut Connection<AppData>) -> Result<Self, AppError> {
        let row = sqlx::query("SELECT games.id, date, scoresA, scoresB, acceptedByA, acceptedByB, a.name AS teamNameA, b.name AS teamNameB, a.id AS teamIdA, b.id AS teamIdB FROM games JOIN teams AS a ON a.id = teamA JOIN teams AS b ON b.id = teamB WHERE games.id = $1")
            .bind(id)
            .fetch_one(&mut ***db).await?;

        Game::try_from(&row)
    }
}

struct Timestamp(i64); // Silly little wrapper so that I impl From<i64>

impl From<i64> for Timestamp {
    fn from(ts: i64) -> Self {
        Self(ts)
    }
}

impl TryFrom<Timestamp> for DateTime<Utc> {
    type Error = AppError;

    fn try_from(ts: Timestamp) -> Result<Self, Self::Error> {
        DateTime::from_timestamp_secs(ts.0).ok_or(AppError::InvalidTimestamp(ts.0))
    }
}

impl TryFrom<&sqlx::sqlite::SqliteRow> for Game {
    type Error = AppError;

    fn try_from(row: &sqlx::sqlite::SqliteRow) -> Result<Self, Self::Error> {
        let date: i64 = row.try_get("date")?;
        let scores_a: u32 = row.try_get("scoresA")?;
        let scores_b: u32 = row.try_get("scoresB")?;

        let accepted_a: Option<Timestamp>  = {
            let val: i64 = row.try_get("acceptedByA")?;
            (val != 0).then_some(val.into())
        };

        let accepted_b: Option<Timestamp> = {
            let val: i64 = row.try_get("acceptedByB")?;
            (val != 0).then_some(val.into())
        };

        Ok(Self {
            id: row.try_get("id")?,
            date: Timestamp::from(date).try_into()?,
            scores: [scores_a.into(), scores_b.into()],
            accepted: [
                accepted_a.map(DateTime::try_from).transpose()?,
                accepted_b.map(DateTime::try_from).transpose()?],
            team_names: [row.try_get("teamNameA")?, row.try_get("teamNameB")?],
            team_ids: [row.try_get("teamIdA")?, row.try_get("teamIdB")?]
        })
    }
}

impl TryFrom<Form<GameData>> for Game {
    type Error = AppError;

    fn try_from(form: Form<GameData>) -> Result<Self, Self::Error> {
        let fmt = "%Y-%m-%dT%H:%M";

        let date = NaiveDateTime::parse_from_str(&form.date, "%Y-%m-%dT%H:%M")
            .map_err(|_| AppError::InvalidDateTime(form.date.clone(), fmt.to_string()))?
            .and_local_timezone(Europe::Madrid)
            .unwrap().to_utc();

        Ok(Self {
            id: uuid::Uuid::new_v4().to_string(),
            date,
            scores: [GameScore(form.score1A, form.score2A, form.score3A), GameScore(form.score1B, form.score2B, form.score3B)],
            accepted: [None, None],
            team_names: ["".to_string(), form.opponentName.to_string()],
            team_ids: ["".to_string(), "".to_string()]
        })
    }
}
