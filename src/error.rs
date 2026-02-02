use rocket::{http::Status, response::Responder};
use rocket_db_pools::sqlx;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("internal database error")]
    Database(sqlx::Error),

    #[error("no rows returned from query")]
    RowNotFound,

    #[error("timestamp is out of bounds")]
    InvalidTimestamp(i64),

    #[error("datetime string ({0}) does not match format ({1})")]
    InvalidDateTime(String, String) // TODO should probably be tagged
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => Self::RowNotFound,
            other => Self::Database(other)
        }
    }
}

impl From<AppError> for Status {
    fn from(err: AppError) -> Self {
        match err {
            AppError::Database(_) => Self::InternalServerError,
            AppError::RowNotFound => Self::NotFound,
            AppError::InvalidTimestamp(_) => Self::BadRequest,
            AppError::InvalidDateTime(_, _) => Self::BadRequest
        }
    }
}

// Allows us to return Result<T, AppError> from route handlers
impl<'r> Responder<'r, 'static> for AppError {
    fn respond_to(self, _: &'r rocket::Request<'_>) -> rocket::response::Result<'static> {
        Err(self.into())
    }
}
