use anyhow::{Context, Result, ensure};
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub admin_user: String,
    pub admin_pass: String,
    pub port: u16,
    pub scheduler_cron: String,
    pub public_base_url: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let admin_user = env::var("ADMIN_USER").context("ADMIN_USER is not set")?;
        let admin_pass = env::var("ADMIN_PASS").context("ADMIN_PASS is not set")?;
        ensure!(!admin_user.is_empty(), "ADMIN_USER must not be empty");
        ensure!(!admin_pass.is_empty(), "ADMIN_PASS must not be empty");

        let public_base_url =
            env::var("PUBLIC_BASE_URL").unwrap_or_else(|_| "https://fund.korucha.com".to_string());
        let parsed_base_url = url::Url::parse(&public_base_url)
            .context("PUBLIC_BASE_URL must be a valid absolute URL")?;
        ensure!(
            matches!(parsed_base_url.scheme(), "http" | "https"),
            "PUBLIC_BASE_URL must use http or https"
        );
        ensure!(
            parsed_base_url.host_str().is_some()
                && parsed_base_url.username().is_empty()
                && parsed_base_url.password().is_none()
                && parsed_base_url.path() == "/"
                && parsed_base_url.query().is_none()
                && parsed_base_url.fragment().is_none(),
            "PUBLIC_BASE_URL must contain only scheme, host, and optional port"
        );

        Ok(Self {
            database_url: env::var("DATABASE_URL").context("DATABASE_URL is not set")?,
            admin_user,
            admin_pass,
            port: env::var("PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .context("PORT must be a valid u16")?,
            scheduler_cron: env::var("SCHEDULER_CRON")
                .unwrap_or_else(|_| "0 0 23 * * *".to_string()),
            public_base_url: parsed_base_url.origin().ascii_serialization(),
        })
    }
}
