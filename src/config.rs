use oauth2::{ClientId, ClientSecret, RedirectUrl};
use std::env;

#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub rust_log: String,
    pub client_id: ClientId,
    pub client_secret: ClientSecret,
    pub redirect_url: RedirectUrl,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        dotenvy::dotenv()?;
        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL is not set in .env file");
        let rust_log = env::var("RUST_LOG").unwrap_or_else(|_| "debug".into());
        let client_id = env::var("PLANNING_CENTER_CLIENT_ID")
            .map(ClientId::new)
            .expect("CLIENT_ID should be provided.");
        let client_secret = env::var("PLANNING_CENTER_CLIENT_SECRET")
            .map(ClientSecret::new)
            .expect("CLIENT_SECRET should be provided");
        let redirect_url = env::var("PLANNING_CENTER_REDIRECT_URI")
            .map(RedirectUrl::new)
            .expect("PLANNING_CENTER_REDIRECT_URI should be provided")?;

        Ok(Self {
            database_url,
            rust_log,
            client_id,
            client_secret,
            redirect_url,
        })
    }
}
