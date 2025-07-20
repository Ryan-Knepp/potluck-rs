use crate::{
    auth::{
        router as auth_router,
        user::{AuthSession, Backend},
    },
    routes::{
        api::{api_pco, api_people},
        dashboard::dashboard,
        me::me,
        search::{search, search_partial, sign_up_household},
    },
    util::asset_loader::AssetLoader,
};
use axum::{
    Router,
    extract::State,
    response::{Html, IntoResponse, Redirect},
    routing::{get, get_service, post},
};
use axum_login::{
    AuthManagerLayerBuilder,
    tower_sessions::{
        Expiry, SessionManagerLayer,
        cookie::{SameSite, time},
    },
};
use minijinja::Environment;
use oauth2::{EndpointNotSet, EndpointSet, basic::BasicClient};
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use tokio::{signal, task::AbortHandle};
use tower_http::services::ServeDir;
use tower_sessions_sqlx_store::PostgresStore;

pub type OauthClient =
    BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>;

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub client: OauthClient,
    pub templates: Arc<Environment<'static>>,
}

pub async fn create_router(
    db: DatabaseConnection,
    client: OauthClient,
    session_store: PostgresStore,
) -> anyhow::Result<Router> {
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
    let backend = Backend::new(db, client);
    let auth_layer = AuthManagerLayerBuilder::new(backend, session_layer).build();

    let app = Router::new()
        .route("/dashboard", get(dashboard))
        .route("/me", get(me))
        .route("/search", get(search))
        .route("/search/partial", get(search_partial))
        .route(
            "/search/sign-up-household/{household_id}",
            post(sign_up_household),
        )
        .route("/api/people", get(api_people))
        .route("/api/pco", get(api_pco))
        .route("/", get(index))
        .merge(auth_router::router())
        .with_state(state)
        .nest_service("/static", get_service(ServeDir::new("static")))
        .layer(auth_layer);
    Ok(app)
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

pub async fn shutdown_signal(deletion_task_abort_handle: AbortHandle) {
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
