use std::collections::HashMap;

use thiserror::Error;

use gestruc::game::{Game, GameScore};

#[derive(Error, Debug)]
enum Error {
    #[error("missing argument: {0}")]
    MissingArgument(&'static str),

    #[error("could not parse datetime: {0}")]
    DatetimeParseError(#[from] chrono::ParseError),

    #[error("could not parse integer: {0}")]
    IntegerParseError(#[from] std::num::ParseIntError),

    #[error("CSV error")]
    CsvError(#[from] csv::Error)
}

fn main() -> Result<(), Error> {
    let path = std::env::args().nth(1).ok_or(Error::MissingArgument("path"))?;

    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_path(path)?;

    let mut teams = HashMap::new();
    let mut statements = Vec::new();

    for record in reader.records() {
        let record = record?;

        let field: Vec<_> = record.iter().collect(); // TODO: check correct number of fields

        if !teams.contains_key(field[0]) {
            let id = uuid::Uuid::new_v4().to_string();
            teams.insert(field[0].to_string(), id);
        }

        if !teams.contains_key(field[1]) {
            let id = uuid::Uuid::new_v4().to_string();
            teams.insert(field[1].to_string(), id);
        }

        let id = uuid::Uuid::new_v4().to_string();

        let date = chrono::NaiveDate::parse_from_str(field[8], "%d-%m-%Y")?
            .and_hms_opt(12, 0, 0).unwrap().and_utc();

        let game = Game {
            id,
            date,
            scores: [
                GameScore(field[2].parse()?, field[4].parse()?, field[6].parse()?),
                GameScore(field[3].parse()?, field[5].parse()?, field[7].parse()?)
            ],
            accepted: [None, None], // TODO: currently not supported
            team_names: [field[0].to_string(), field[1].to_string()],
            team_ids: [teams.get(field[0]).unwrap().to_string(), teams.get(field[1]).unwrap().to_string()]
        };

        statements.push(game.to_sql_insert().to_string());
    }

    println!("{}", statements.join("\n"));

    Ok(())
}
