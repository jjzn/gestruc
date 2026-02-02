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

mod game;
mod team;
mod player;
mod error;

use crate::error::AppError;
use crate::game::{Game, GameData};
use crate::team::{Team};
use crate::player::Player;

#[derive(Database)]
#[database("appdata")]
struct AppData(sqlx::SqlitePool);

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

#[get("/add-game/<teamid>")]
async fn add_game_form(teamid: &str, player: Player, mut db: Connection<AppData>) -> Result<Template, Status> {
    let team = Team::try_fetch(teamid.to_string(), &mut db).await?;

    if !team.has_member(&player) {
        return Err(Status::Unauthorized);
    }

    let other_teams: Vec<String> = sqlx::query("SELECT name FROM teams WHERE tournamentName = $1 AND tournamentEdition = $2 AND name != $3")
        .bind(&team.tournament_name)
        .bind(&team.tournament_edition)
        .bind(&team.name)
        .fetch_all(&mut **db).await
        .map_err(|_| Status::InternalServerError)?
        .iter().map(|row| row.get("name")).collect();

    Ok(Template::render("add-game", context! { team, other_teams }))
}

#[post("/add-game/<teamid>", data = "<form>")]
async fn add_game(teamid: &str, form: Form<GameData>, player: Player, mut db: Connection<AppData>) -> Result<Redirect, Status> {
    let opponent_name = form.opponentName.clone();

    let team = Team::try_fetch(teamid.to_string(), &mut db).await.map_err(|_| Status::InternalServerError)?;
    let game: Game = form.try_into().map_err(|_| Status::BadRequest)?;

    if !team.has_member(&player) {
        return Err(Status::Unauthorized);
    }

    // TODO: enforce that team names are unique to each tournament
    let opponent_id: String = {
        let row = sqlx::query("SELECT id FROM teams WHERE name = $1 AND tournamentName = $2 AND tournamentEdition = $3")
            .bind(opponent_name)
            .bind(team.tournament_name)
            .bind(team.tournament_edition)
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

async fn view_team(id: &str, mut db: Connection<AppData>, is_team_member: bool, is_captain: bool) -> Result<Template, AppError> {
    let team = Team::try_fetch(id.to_string(), &mut db).await?;
    let captain = Player::try_fetch(team.captain_id.clone(), &mut db).await?;
    let partner = Player::try_fetch(team.partner_id.clone(), &mut db).await?;
    let games = team.get_games(&mut db).await.ok();

    Ok(Template::render("team", context! { team, captain, partner, games, is_team_member, is_captain }))
}

#[get("/teams/<id>")]
async fn view_team_auth(id: &str, mut db: Connection<AppData>, player: Player) -> Result<Template, AppError> {
    let team = Team::try_fetch(id.to_string(), &mut db).await?;
    view_team(id, db, team.has_member(&player), player.id == team.captain_id).await
}

#[get("/teams/<id>", rank = 2)]
async fn view_team_unauth(id: &str, db: Connection<AppData>) -> Result<Template, AppError> {
    view_team(id, db, false, false).await
}

#[get("/games/<id>")]
async fn view_game(id: &str, mut db: Connection<AppData>) -> Result<Template, AppError> {
    let game = Game::try_fetch(id.to_string(), &mut db).await?;
    Ok(Template::render("game", context! { game }))
}

#[post("/login", data = "<form>")]
async fn login(form: Form<LoginData>, cookies: &CookieJar<'_>, config: &State<AppConfig>, mut db: Connection<AppData>) -> Result<Redirect, Status> {
    let row = sqlx::query("SELECT password, id FROM players WHERE email = $1").bind(&form.email)
        .fetch_one(&mut **db).await
        .or(Err(Status::Unauthorized))?;

    if row.get::<&str, usize>(0) != form.password {
        return Err(Status::Unauthorized);
    }

    let player_id: &str = row.get(1);
    let session = uuid::Uuid::new_v4().to_string();
    let expiration = Utc::now() + config.session_duration;

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
    let figment = Figment::from(rocket::Config::figment())
        .merge(Toml::file("App.toml").nested())
        .select(Profile::from_env_or("APP_PROFILE", "default"));

    rocket::custom(figment)
        .attach(AppData::init())
        .attach(Template::fairing())
        .attach(AdHoc::config::<AppConfig>())
        .mount("/", routes![index, index_auth, view_team_unauth, view_team_auth, add_game_form, add_game, view_game, login, logout])
        .mount("/", FileServer::from(relative!("public/static")))
}
