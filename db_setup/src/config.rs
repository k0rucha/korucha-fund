use anyhow::{Context, Result};
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub admin_user: String,
    pub admin_pass: String,
    pub port: u16,
    pub scheduler_cron: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            database_url: env::var("DATABASE_URL")
                .context("DATABASE_URL is not set")?,
            admin_user: env::var("ADMIN_USER").context("ADMIN_USER is not set")?,
            admin_pass: env::var("ADMIN_PASS").context("ADMIN_PASS is not set")?,
            port: env::var("PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .context("PORT must be a valid u16")?,
            scheduler_cron: env::var("SCHEDULER_CRON")
                .unwrap_or_else(|_| "0 0 23 * * *".to_string()),
        })
    }
}
