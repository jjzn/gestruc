#[macro_use] extern crate rocket;

use chrono::{DateTime, Duration, Utc};
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
struct Game {
    date: DateTime<Utc>,
    scores: [u32; 2],
    accepted: [Option<DateTime<Utc>>; 2],
    team_ids: [String; 2]
}

impl TryFrom<&sqlx::sqlite::SqliteRow> for Game {
    type Error = sqlx::Error;

    fn try_from(row: &sqlx::sqlite::SqliteRow) -> Result<Self, Self::Error> {
        let date: String = row.try_get("date")?;
        let accepted_a: String = row.try_get("acceptedByA")?;
        let accepted_b: String = row.try_get("acceptedByB")?;

        Ok(Self {
            date: date.parse().map_err(|_| sqlx::Error::RowNotFound)?, // TODO: use proper error
            scores: [row.try_get("scoresA")?, row.try_get("scoresB")?],
            accepted: [accepted_a.parse().ok(), accepted_b.parse().ok()],
            team_ids: [row.try_get("teamA")?, row.try_get("teamB")?]
        })
    }
}

impl Team {
    async fn get_games(&self, db: &mut Connection<AppData>) -> Result<Vec<Game>, sqlx::Error> {
        let rows = sqlx::query("SELECT date, scoresA, scoresB, acceptedByA, acceptedByB, teamA, teamB FROM games WHERE teamA = $1 OR teamB = $1")
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

#[get("/")]
async fn index_auth(player: Player, mut db: Connection<AppData>) -> Template {
    let teams = player.get_teams(&mut db).await.ok();
    Template::render("index", context! { player, teams })
}

#[get("/", rank = 2)]
async fn index() -> Option<NamedFile> {
    NamedFile::open("public/index.html").await.ok()
}

#[get("/teams/<id>")]
async fn view_team(id: &str, mut db: Connection<AppData>) -> Option<Template> {
    let team = Team::try_fetch(id.to_string(), &mut db).await?;
    let captain = Player::try_fetch(team.captain_id.clone(), &mut db).await?;
    let partner = Player::try_fetch(team.partner_id.clone(), &mut db).await?;
    let games = team.get_games(&mut db).await.ok();

    Some(Template::render("team", context! { team, captain, partner, games }))
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

#[launch]
fn rocket() -> _ {
    rocket::build()
        .attach(AppData::init())
        .attach(Template::fairing())
        .mount("/", routes![index, index_auth, view_team, login])
        .mount("/", FileServer::from(relative!("public/static")))
}
