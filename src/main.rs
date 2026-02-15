#[macro_use] extern crate rocket;

use chrono::{Duration, Utc};
use regex::Regex;
use rocket::State;
use rocket::fairing::AdHoc;
use rocket::figment::{Figment, Profile};
use rocket::figment::providers::{Toml, Format};
use rocket::form::Form;
use rocket::fs::{FileServer, NamedFile, relative};
use rocket::http::{Cookie, CookieJar, SameSite, Status};
use rocket::response::Redirect;
use rocket::serde::{Deserialize, Deserializer, de::Error};
use rocket_db_pools::{Connection, Database, sqlx};
use rocket_db_pools::sqlx::Row;
use rocket_dyn_templates::{Template, context};

use gestruc::error::AppError;
use gestruc::player::Player;
use gestruc::tournament::Tournament;
use gestruc::AppData;
use gestruc::routes;

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct AppConfig {
    #[serde(deserialize_with = "deserialize_duration")]
    session_duration: Duration
}

fn deserialize_duration<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>
{
    let re = Regex::new(r"^(?:(\d+)h)?(?:(\d+)m)?$").unwrap(); // Should never fail
    let s = String::deserialize(deserializer)?;
    let caps = re.captures(&s)
        .ok_or(D::Error::custom("invalid format"))?;

    let hours = caps.get(1).map_or(Ok(0), |m|
        m.as_str().parse().map_err(|_| D::Error::custom("expected an integer")))?;

    let minutes = caps.get(2).map_or(Ok(0), |m|
        m.as_str().parse().map_err(|_| D::Error::custom("expected an integer")))?;

    Ok(Duration::hours(hours) + Duration::minutes(minutes))
}

#[derive(FromForm)]
struct LoginData {
    email: String,
    password: String
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

#[get("/tournament/<name>/<edition>")]
async fn view_tournament(name: &str, edition: &str, mut db: Connection<AppData>) -> Result<Template, AppError> {
    let tournament = Tournament::try_fetch(name.to_string(), edition.to_string(), &mut db).await?;
    let teams = tournament.get_teams(&mut db).await?;

    // TODO: add players and games
    Ok(Template::render("tournament", context! { tournament, teams }))
}

#[get("/tournaments")]
async fn view_tournaments_all(mut db: Connection<AppData>) -> Result<Template, AppError> {
    let tournaments: Vec<Tournament> = sqlx::query("SELECT * FROM tournaments")
        .fetch_all(&mut **db).await?
        .iter().map(Tournament::try_from)
        .collect::<Result<_, _>>()?;

    Ok(Template::render("tournaments-list", context ! { tournaments }))
}

#[get("/players/<id>")]
async fn view_player(id: &str, mut db: Connection<AppData>) -> Result<Template, AppError> {
    let player = Player::try_fetch(id.to_string(), &mut db).await?;
    let teams = player.get_teams(&mut db).await?;

    Ok(Template::render("player", context! { player, teams }))
}

#[post("/login", data = "<form>")]
async fn login(form: Form<LoginData>, cookies: &CookieJar<'_>, config: &State<AppConfig>, mut db: Connection<AppData>) -> Result<Redirect, Status> {
    let row = sqlx::query("SELECT password, id FROM players WHERE email = $1").bind(&form.email)
        .fetch_one(&mut **db).await
        .map_err(AppError::from)?;

    if row.get::<&str, usize>(0) != form.password {
        return Err(Status::Unauthorized);
    }

    let player_id: &str = row.get(1);
    let session = uuid::Uuid::new_v4().to_string();
    let expiration = Utc::now() + config.session_duration;

    sqlx::query("INSERT INTO sessions (id, playerId, expires) VALUES ($1, $2, $3)")
        .bind(&session).bind(player_id).bind(expiration.timestamp())
        .execute(&mut **db).await
        .map_err(AppError::from)?;

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
    let figment = Figment::from(rocket::Config::figment())
        .merge(Toml::file("App.toml").nested())
        .select(Profile::from_env_or("APP_PROFILE", "default"));

    rocket::custom(figment)
        .attach(AppData::init())
        .attach(Template::fairing())
        .attach(AdHoc::config::<AppConfig>())
        .mount("/", routes::game::routes())
        .mount("/", routes::team::routes())
        .mount("/", routes![index, index_auth, view_tournament, view_tournaments_all, view_player, login, logout])
        .mount("/", FileServer::from(relative!("public/static")))
}
