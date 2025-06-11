mod auth;
mod entities;
mod pco;

use auth::user::{AuthSession, Backend};
use axum::{
    Router,
    extract::State,
    response::{Html, IntoResponse},
    routing::{get, get_service},
};
use axum_login::{
    AuthManagerLayerBuilder, login_required,
    tower_sessions::{
        Expiry, MemoryStore, SessionManagerLayer,
        cookie::{SameSite, time},
    },
};
use migration::{Migrator, MigratorTrait};
use minijinja::Environment;
use oauth2::{
    AuthUrl, ClientId, ClientSecret, EndpointNotSet, EndpointSet, RedirectUrl, TokenUrl,
    basic::BasicClient,
};
use reqwest::StatusCode;
use sea_orm::{Database, DatabaseConnection};
use std::env;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::services::ServeDir;
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
    let db = Database::connect(db_url)
        .await
        .expect("Cannot connect to db");
    Migrator::up(&db, None).await.unwrap();

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

    let session_store = MemoryStore::default();
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
        .route("/other", get(other))
        .route("/me", get(me))
        .route_layer(login_required!(Backend, login_url = "/login"))
        .route("/", get(index))
        .with_state(state)
        .merge(auth::router::router())
        .nest_service("/static", get_service(ServeDir::new("static")))
        .layer(auth_layer);

    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await?;

    Ok(())
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
    env
}

async fn index(State(state): State<AppState>) -> Html<String> {
    let tmpl = state.templates.get_template("index.html").unwrap();
    let html = tmpl.render(minijinja::context! {}).unwrap();
    Html(html)
}

async fn me(auth_session: AuthSession) -> impl IntoResponse {
    match auth_session.user {
        Some(_user) => "It's me ya boi!".into_response(),

        None => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

async fn other() -> &'static str {
    "Doesn't need auth"
}
