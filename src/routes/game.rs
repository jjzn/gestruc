use chrono::Utc;
use rocket::{get, post, routes};
use rocket::form::Form;
use rocket::http::Status;
use rocket::response::Redirect;
use rocket_db_pools::{Connection, sqlx};
use rocket_db_pools::sqlx::Row;
use rocket_dyn_templates::{Template, context};

use crate::AppData;
use crate::error::AppError;
use crate::game::{Game, GameData};
use crate::player::Player;
use crate::team::Team;

pub fn routes() -> Vec<rocket::Route> {
    routes![view_game, accept_game_results, decline_game_results, add_game_form, add_game]
}

#[get("/games/<id>")]
async fn view_game(id: &str, mut db: Connection<AppData>, player: Option<Player>) -> Result<Template, AppError> {
    let game = Game::try_fetch(id.to_string(), &mut db).await?;
    let teams = game.get_teams(&mut db).await?;

    let players = [
        Player::try_fetch(teams[0].captain_id.clone(), &mut db).await?,
        Player::try_fetch(teams[0].partner_id.clone(), &mut db).await?,
        Player::try_fetch(teams[1].captain_id.clone(), &mut db).await?,
        Player::try_fetch(teams[1].partner_id.clone(), &mut db).await?
    ];

    // If player is None, both values are false, else they depend on matching IDs
    let is_captain = [
        player.as_ref().map_or(false, |p| p.id == teams[0].captain_id),
        player.as_ref().map_or(false, |p| p.id == teams[1].captain_id)
    ];

    Ok(Template::render("game", context! { game, teams, players, is_captain }))
}

#[get("/games/<id>/accept")]
async fn accept_game_results(id: &str, mut db: Connection<AppData>, player: Player) -> Result<Redirect, Status> {
    let game = Game::try_fetch(id.to_string(), &mut db).await?;
    let teams = game.get_teams(&mut db).await?;

    let player_team = teams
        .iter().position(|team| team.captain_id == player.id)
        .ok_or(Status::Unauthorized)?;

    // Prevent accepting the same results again
    if game.accepted[player_team].is_some() {
        return Err(Status::Conflict);
    }

    let statement = if player_team == 0 {
        "UPDATE games SET acceptedByA = $1 WHERE id = $2"
    } else {
        "UPDATE games SET acceptedByB = $1 WHERE id = $2"
    };

    sqlx::query(statement)
        .bind(Utc::now().timestamp()).bind(&game.id)
        .execute(&mut **db).await
        .map_err(AppError::from)?;

    Ok(Redirect::to(format!("/games/{}", game.id)))
}

#[get("/games/<id>/decline")]
async fn decline_game_results(id: &str, mut db: Connection<AppData>, player: Player) -> Result<Redirect, Status> {
    let game = Game::try_fetch(id.to_string(), &mut db).await?;
    let teams = game.get_teams(&mut db).await?;

    if player.id != teams[0].captain_id && player.id != teams[1].captain_id {
        return Err(Status::Unauthorized);
    }

    todo!()
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
