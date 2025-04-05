mod entities;

use axum::{Router, routing::get};
use migration::{Migrator, MigratorTrait};
use sea_orm::{Database, DatabaseConnection};
use std::env;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv()?;
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL is not set in .env file");
    let db = Database::connect(db_url)
        .await
        .expect("Cannot connect to db");
    Migrator::up(&db, None).await.unwrap();

    let state = AppState { db };

    let app = Router::new().route("/", get(root)).with_state(state);

    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await?;

    Ok(())
}

#[derive(Clone)]
struct AppState {
    db: DatabaseConnection,
}

// basic handler that responds with a static string
async fn root() -> &'static str {
    "Hello, World!"
}
