use rocket::serde::json::Json;
use rocket::{get, routes};
use rocket_db_pools::Connection;

use crate::AppData;
use crate::error::AppError;
use crate::game::Game;
use crate::team::Team;

pub fn routes() -> Vec<rocket::Route> {
    routes![view_team]
}

#[get("/teams/<id>")]
async fn view_team(id: &str, mut db: Connection<AppData>) -> Result<Json<(Team, Vec<Game>)>, AppError> {
    let team = Team::try_fetch(id.to_string(), &mut db).await?;
    let games = team.get_games(&mut db).await?;

    Ok(Json((team, games)))
}
