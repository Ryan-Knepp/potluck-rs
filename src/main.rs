mod auth;
mod config;
mod database;
mod entities;
mod pco;
mod router;
mod routes;
mod util;

use crate::{
    config::Config,
    database::setup_database,
    router::{create_router, shutdown_signal, OauthClient},
};
use axum_login::tower_sessions::ExpiredDeletion;
use oauth2::{AuthUrl, TokenUrl, basic::BasicClient};
use tokio::net::TcpListener;
use tower_sessions_sqlx_store::PostgresStore;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::from_env()?;
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(&config.rust_log))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let (db, pool) = setup_database(&config.database_url).await?;

    let client = setup_oauth_client(&config)?;

    let session_store = PostgresStore::new(pool);
    session_store.migrate().await?;

    let deletion_task = tokio::task::spawn(
        session_store
            .clone()
            .continuously_delete_expired(tokio::time::Duration::from_secs(60)),
    );

    let app = create_router(db, client, session_store).await?;

    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(deletion_task.abort_handle()))
        .await?;

    deletion_task.await??;

    Ok(())
}

fn setup_oauth_client(config: &Config) -> anyhow::Result<OauthClient> {
    let auth_url =
        AuthUrl::new("https://api.planningcenteronline.com/oauth/authorize".to_string())?;
    let token_url = TokenUrl::new("https://api.planningcenteronline.com/oauth/token".to_string())?;
    let client = BasicClient::new(config.client_id.clone())
        .set_client_secret(config.client_secret.clone())
        .set_auth_uri(auth_url)
        .set_token_uri(token_url)
        .set_redirect_uri(config.redirect_url.clone());
    Ok(client)
}
