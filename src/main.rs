#[macro_use] extern crate rocket;

use chrono::{Duration, Utc};
use rocket::form::Form;
use rocket::fs::NamedFile;
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

impl TryFrom<&sqlx::sqlite::SqliteRow> for Team {
    type Error = sqlx::Error;

    fn try_from(row: &sqlx::sqlite::SqliteRow) -> Result<Self, Self::Error> {
        Ok(Team {
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
    async fn get_teams(&self, mut db: Connection<AppData>) -> Result<Vec<Team>, sqlx::Error> {
        let rows = sqlx::query("SELECT id, name, captainId, partnerId, tournamentName, tournamentYear FROM teams WHERE captainId = $1 OR partnerId = $1")
            .bind(&self.id)
            .fetch_all(&mut **db).await?;

        rows.iter().map(Team::try_from).collect()
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

        let row = match sqlx::query("SELECT email, name FROM players WHERE id = $1").bind(&player_id)
            .fetch_one(&mut **db).await {
            Ok(row) => row,
            Err(_) => return Outcome::Error((Status::InternalServerError, ()))
        };

        Outcome::Success(Player { id: player_id, email: row.get(0), name: row.get(1) })
    }
}

#[get("/")]
async fn index_auth(player: Player, db: Connection<AppData>) -> Template {
    let teams = player.get_teams(db).await.ok();
    Template::render("index", context! { player, teams })
}

#[get("/", rank = 2)]
async fn index() -> Option<NamedFile> {
    NamedFile::open("public/index.html").await.ok()
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
        .mount("/", routes![index, index_auth, login])
}
