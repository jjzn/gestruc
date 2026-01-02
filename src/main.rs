#[macro_use] extern crate rocket;

use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use chrono_tz::Europe;
use rocket::form::Form;
use rocket::fs::{FileServer, NamedFile, relative};
use rocket::http::{Cookie, CookieJar, SameSite, Status};
use rocket::request::{Request, FromRequest, Outcome};
use rocket::response::Redirect;
use rocket::serde::Serialize;
use rocket_db_pools::{Connection, Database, sqlx};
use rocket_db_pools::sqlx::Row;
use rocket_dyn_templates::{Template, context};

#[derive(Database)]
#[database("appdata")]
struct AppData(sqlx::SqlitePool);

#[derive(FromForm)]
struct LoginData {
    email: String,
    password: String
}

#[derive(FromForm, Debug)]
#[allow(non_snake_case)]
struct GameData {
    opponentName: String,
    date: String,
    score1A: u8,
    score1B: u8,
    score2A: u8,
    score2B: u8,
    score3A: u8,
    score3B: u8,
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct Team {
    id: String,
    name: String,
    captain_id: String,
    partner_id: String,
    tournament_name: String,
    tournament_year: u16
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct GameScore(u8, u8, u8);

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
struct Game {
    id: String,
    date: DateTime<Utc>,
    scores: [GameScore; 2],
    accepted: [Option<DateTime<Utc>>; 2],
    team_names: [String; 2],
    team_ids: [String; 2]
}

impl TryFrom<&sqlx::sqlite::SqliteRow> for Game {
    type Error = sqlx::Error;

    fn try_from(row: &sqlx::sqlite::SqliteRow) -> Result<Self, Self::Error> {
        let date = row.try_get("date")?;
        let scores_a: u32 = row.try_get("scoresA")?;
        let scores_b: u32 = row.try_get("scoresB")?;

        let accepted_a  = {
            let val = row.try_get("acceptedByA")?;
            (val != 0).then_some(val)
        };

        let accepted_b = {
            let val = row.try_get("acceptedByB")?;
            (val != 0).then_some(val)
        };

        Ok(Self {
            id: row.try_get("id")?,
            date: DateTime::from_timestamp(date, 0).ok_or(sqlx::Error::RowNotFound)?, // TODO: use proper error
            scores: [scores_a.into(), scores_b.into()],
            accepted: [accepted_a.map(DateTime::from_timestamp_secs).flatten(), accepted_b.map(DateTime::from_timestamp_secs).flatten()],
            team_names: [row.try_get("teamNameA")?, row.try_get("teamNameB")?],
            team_ids: [row.try_get("teamIdA")?, row.try_get("teamIdB")?]
        })
    }
}

impl TryFrom<Form<GameData>> for Game {
    type Error = ();

    fn try_from(form: Form<GameData>) -> Result<Self, Self::Error> {
        let date = NaiveDateTime::parse_from_str(&form.date, "%Y-%m-%dT%H:%M")
            .map_err(|_| ())?
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

impl Team {
    async fn get_games(&self, db: &mut Connection<AppData>) -> Result<Vec<Game>, sqlx::Error> {
        let rows = sqlx::query("SELECT games.id, date, scoresA, scoresB, acceptedByA, acceptedByB, a.name AS teamNameA, b.name AS teamNameB, a.id AS teamIdA, b.id AS teamIdB FROM games JOIN teams AS a ON a.id = teamA JOIN teams AS b ON b.id = teamB WHERE teamA = $1 OR teamB = $1")
            .bind(&self.id)
            .fetch_all(&mut ***db).await?;

        rows.iter().map(Game::try_from).collect()
    }

    async fn try_fetch(id: String, db: &mut Connection<AppData>) -> Option<Self> {
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

    fn has_member(&self, player: Player) -> bool {
        player.id == self.captain_id || player.id == self.partner_id
    }
}

impl TryFrom<&sqlx::sqlite::SqliteRow> for Team {
    type Error = sqlx::Error;

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

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct Player {
    id: String,
    email: String,
    name: String
}

impl Player {
    async fn get_teams(&self, db: &mut Connection<AppData>) -> Result<Vec<Team>, sqlx::Error> {
        let rows = sqlx::query("SELECT id, name, captainId, partnerId, tournamentName, tournamentYear FROM teams WHERE captainId = $1 OR partnerId = $1")
            .bind(&self.id)
            .fetch_all(&mut ***db).await?;

        rows.iter().map(Team::try_from).collect()
    }

    async fn try_fetch(id: String, db: &mut Connection<AppData>) -> Option<Self> {
        let row = sqlx::query("SELECT email, name FROM players WHERE id = $1")
            .bind(&id)
            .fetch_one(&mut ***db).await.ok()?;

        Some(Self { id, email: row.try_get("email").ok()?, name: row.try_get("name").ok()? })
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for Player {
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

#[get("/add-game/<teamid>")]
async fn add_game_form(teamid: &str, player: Player, mut db: Connection<AppData>) -> Result<Template, Status> {
    let team = Team::try_fetch(teamid.to_string(), &mut db).await.ok_or(Status::InternalServerError)?;

    if !team.has_member(player) {
        return Err(Status::Unauthorized);
    }

    Ok(Template::render("add-game", context! { team }))
}

#[post("/add-game/<teamid>", data = "<form>")]
async fn add_game(teamid: &str, form: Form<GameData>, player: Player, mut db: Connection<AppData>) -> Result<Redirect, Status> {
    let opponent_name = form.opponentName.clone();

    println!("{:?}", form);

    let team = Team::try_fetch(teamid.to_string(), &mut db).await.ok_or(Status::InternalServerError)?;
    let game: Game = form.try_into().map_err(|_| Status::BadRequest)?;

    if !team.has_member(player) {
        return Err(Status::Unauthorized);
    }

    // TODO: enforce that team names are unique to each tournament
    let opponent_id: String = {
        let row = sqlx::query("SELECT id FROM teams WHERE name = $1 AND tournamentName = $2 AND tournamentYear = $3")
            .bind(opponent_name)
            .bind(team.tournament_name)
            .bind(team.tournament_year)
            .fetch_one(&mut **db).await
            .map_err(|_| Status::BadRequest)?; // TODO: check on the client side first + add nicer
                                               // error message

        row.try_get("id").map_err(|_| Status::InternalServerError)?
    };

    let scores = game.scores.map(u32::from);

    sqlx::query("INSERT INTO games (id, date, scoresA, scoresB, acceptedByA, acceptedByB, teamA, teamB) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)")
        .bind(game.id)
        .bind(game.date.timestamp())
        .bind(scores[0])
        .bind(scores[1])
        .bind(game.accepted[0].map(|dt| dt.timestamp()))
        .bind(game.accepted[1].map(|dt| dt.timestamp()))
        .bind(&team.id)
        .bind(opponent_id)
        .execute(&mut **db).await
        .map_err(|_| Status::InternalServerError)?;

    Ok(Redirect::to(format!("/teams/{}", team.id)))
}

#[get("/")]
async fn index_auth(player: Player, mut db: Connection<AppData>) -> Template {
    let teams = player.get_teams(&mut db).await.ok();
    Template::render("index", context! { player, teams })
}

#[get("/", rank = 2)]
async fn index() -> Option<NamedFile> {
    NamedFile::open("public/index.html").await.ok()
}

async fn view_team(id: &str, mut db: Connection<AppData>, is_team_member: bool) -> Option<Template> {
    let team = Team::try_fetch(id.to_string(), &mut db).await?;
    let captain = Player::try_fetch(team.captain_id.clone(), &mut db).await?;
    let partner = Player::try_fetch(team.partner_id.clone(), &mut db).await?;
    let games = team.get_games(&mut db).await.ok();

    Some(Template::render("team", context! { team, captain, partner, games, is_team_member }))
}

#[get("/teams/<id>")]
async fn view_team_auth(id: &str, mut db: Connection<AppData>, player: Player) -> Option<Template> {
    let team = Team::try_fetch(id.to_string(), &mut db).await?;
    view_team(id, db, team.has_member(player)).await
}

#[get("/teams/<id>", rank = 2)]
async fn view_team_unauth(id: &str, db: Connection<AppData>) -> Option<Template> {
    view_team(id, db, false).await
}

#[post("/login", data = "<form>")]
async fn login(form: Form<LoginData>, cookies: &CookieJar<'_>, mut db: Connection<AppData>) -> Result<Redirect, Status> {
    let row = sqlx::query("SELECT password, id FROM players WHERE email = $1").bind(&form.email)
        .fetch_one(&mut **db).await
        .or(Err(Status::Unauthorized))?;

    if row.get::<&str, usize>(0) != form.password {
        return Err(Status::Unauthorized);
    }

    let player_id: &str = row.get(1);
    let session = uuid::Uuid::new_v4().to_string();
    let expiration = Utc::now() + Duration::hours(1);

    sqlx::query("INSERT INTO sessions (id, playerId, expires) VALUES ($1, $2, $3)")
        .bind(&session).bind(player_id).bind(expiration.timestamp())
        .execute(&mut **db).await
        .or(Err(Status::InternalServerError))?;

    let cookie = Cookie::build(("sessionId", session))
        .secure(true)
        .http_only(true)
        .same_site(SameSite::Strict);

    cookies.add(cookie);
    Ok(Redirect::to("/"))
}

#[get("/logout")]
async fn logout(player: Player, mut db: Connection<AppData>) -> Redirect {
    let _ = sqlx::query("DELETE FROM sessions WHERE playerId = $1").bind(player.id)
        .execute(&mut **db).await;

    Redirect::to("/")
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .attach(AppData::init())
        .attach(Template::fairing())
        .mount("/", routes![index, index_auth, view_team_unauth, view_team_auth, add_game_form, add_game, login, logout])
        .mount("/", FileServer::from(relative!("public/static")))
}
