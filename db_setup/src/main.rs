use sqlx::sqlite::{SqlitePoolOptions, SqliteConnectOptions};
use std::str::FromStr;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Connect to db and create if missing
    let connect_options = SqliteConnectOptions::from_str("sqlite://../korucha-fund.db")?
        .create_if_missing(true);

    let db = SqlitePoolOptions::new()
        .connect_with(connect_options)
        .await?;

    // Run migrations
    sqlx::migrate!("../migrations")
        .run(&db)
        .await?;

    println!("Database setup successful!");
    Ok(())
}
