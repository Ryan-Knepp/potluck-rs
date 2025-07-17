mod auth;
mod entities;
mod pco;
mod routes;
mod util;

use crate::{
    routes::dashboard::dashboard,
    routes::search::{search, search_partial},
    util::asset_loader::AssetLoader,
};
use auth::user::{AuthSession, Backend};
use axum::{
    Router,
    extract::State,
    response::{Html, IntoResponse, Redirect},
    routing::{get, get_service},
};
use axum_login::{
    AuthManagerLayerBuilder,
    tower_sessions::{
        ExpiredDeletion, Expiry, SessionManagerLayer,
        cookie::{SameSite, time},
    },
};
use migration::{Migrator, MigratorTrait};
use minijinja::Environment;
use oauth2::{
    AuthUrl, ClientId, ClientSecret, EndpointNotSet, EndpointSet, RedirectUrl, TokenUrl,
    basic::BasicClient,
};
use routes::api::api_pco;
use routes::api::api_people;
use routes::me::me;
use sea_orm::{Database, DatabaseConnection, sqlx::PgPool};

use std::env;
use std::sync::Arc;
use tokio::{net::TcpListener, signal, task::AbortHandle};
use tower_http::services::ServeDir;
use tower_sessions_sqlx_store::PostgresStore;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub type OauthClient =
    BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>;



#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv()?;
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL is not set in .env file");
    let db = Database::connect(&db_url)
        .await
        .expect("Cannot connect to db");
    Migrator::up(&db, None).await.unwrap();

    let pool = PgPool::connect(&db_url).await?;
    let session_store = PostgresStore::new(pool);
    session_store.migrate().await?;

    let deletion_task = tokio::task::spawn(
        session_store
            .clone()
            .continuously_delete_expired(tokio::time::Duration::from_secs(60)),
    );

    let client_id = env::var("PLANNING_CENTER_CLIENT_ID")
        .map(ClientId::new)
        .expect("CLIENT_ID should be provided.");
    let client_secret = env::var("PLANNING_CENTER_CLIENT_SECRET")
        .map(ClientSecret::new)
        .expect("CLIENT_SECRET should be provided");
    let redirect_url = env::var("PLANNING_CENTER_REDIRECT_URI")
        .map(RedirectUrl::new)
        .expect("PLANNING_CENTER_REDIRECT_URI should be provided")?;

    let auth_url =
        AuthUrl::new("https://api.planningcenteronline.com/oauth/authorize".to_string())?;
    let token_url = TokenUrl::new("https://api.planningcenteronline.com/oauth/token".to_string())?;
    let client = BasicClient::new(client_id)
        .set_client_secret(client_secret)
        .set_auth_uri(auth_url)
        .set_token_uri(token_url)
        .set_redirect_uri(redirect_url);

    let templates = setup_templates().await;

    let state = AppState {
        db: db.clone(),
        client: client.clone(),
        templates: Arc::new(templates),
    };

    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false)
        .with_same_site(SameSite::Lax) // Ensure we send the cookie from the OAuth redirect.
        .with_expiry(Expiry::OnInactivity(time::Duration::days(1)));

    // Auth service.
    //
    // This combines the session layer with our backend to establish the auth
    // service which will provide the auth session as a request extension.
    let backend = Backend::new(db.clone(), client.clone());
    let auth_layer = AuthManagerLayerBuilder::new(backend, session_layer).build();

    let app = Router::new()
        .route("/dashboard", get(dashboard))
        .route("/me", get(me))
        .route("/search", get(search))
        .route("/search/partial", get(search_partial))
        .route("/api/people", get(api_people))
        .route("/api/pco", get(api_pco))
        .route("/", get(index))
        .merge(auth::router::router())
        .with_state(state)
        .nest_service("/static", get_service(ServeDir::new("static")))
        .layer(auth_layer);

    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(deletion_task.abort_handle()))
        .await?;

    deletion_task.await??;

    Ok(())
}

async fn shutdown_signal(deletion_task_abort_handle: AbortHandle) {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => { deletion_task_abort_handle.abort() },
        _ = terminate => { deletion_task_abort_handle.abort() },
    }
}

#[derive(Clone)]
struct AppState {
    db: DatabaseConnection,
    client: OauthClient,
    templates: Arc<Environment<'static>>,
}

async fn setup_templates() -> Environment<'static> {
    let mut env = Environment::new();
    env.set_loader(minijinja::path_loader("templates"));
    let asset_loader = AssetLoader::new();
    asset_loader.register(&mut env);
    env
}

async fn index(State(state): State<AppState>, auth_session: AuthSession) -> impl IntoResponse {
    if auth_session.user.is_some() {
        Redirect::to("/dashboard").into_response()
    } else {
        let tmpl = state.templates.get_template("index.html").unwrap();
        let html = tmpl.render(minijinja::context! {}).unwrap();
        Html(html).into_response()
    }
}
