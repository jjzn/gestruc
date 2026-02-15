use rocket::{get, routes};
use rocket_db_pools::Connection;
use rocket_dyn_templates::{Template, context};

use crate::AppData;
use crate::error::AppError;
use crate::player::Player;
use crate::team::Team;

pub fn routes() -> Vec<rocket::Route> {
    routes![view_team_auth, view_team_unauth]
}

// TODO: probably redundant, view_team could accept a Option<Player> to discriminate between the
// authorized and unauthorized cases
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
