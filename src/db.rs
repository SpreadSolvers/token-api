use diesel::prelude::*;
use dotenv::dotenv;
use log::error;
use std::env;

pub fn establish_connection() -> SqliteConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    SqliteConnection::establish(&database_url).unwrap_or_else(|e| {
        error!(
            "Error connecting to SQLite via URL: {} with error: {}",
            database_url, e
        );
        panic!("Failed to connect to SQLite database");
    })
}
