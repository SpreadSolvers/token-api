pub mod sqlite;

use std::{
    error::Error,
    fmt::{self, Display},
};

use tap_caip::AccountId;

#[derive(Debug)]
pub enum RepoError {
    NotFound,
    Backend(String),
    Diesel(diesel::result::Error),
    R2d2(diesel::r2d2::Error),
}

impl Display for RepoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RepoError: {}", self)
    }
}

impl Error for RepoError {}

impl From<diesel::result::Error> for RepoError {
    fn from(error: diesel::result::Error) -> Self {
        RepoError::Diesel(error)
    }
}

impl From<diesel::r2d2::Error> for RepoError {
    fn from(error: diesel::r2d2::Error) -> Self {
        RepoError::R2d2(error)
    }
}

pub trait Repository<T> {
    fn get(&self, id: AccountId) -> Result<Option<T>, RepoError>;
    fn save(&self, token: &T) -> Result<(), RepoError>;
}
