use anyhow::{Ok, Result};
use axum::{Router, routing::get};
use dotenvy::dotenv;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let app = Router::new().route("/", get(root));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await?;

    Ok(())
}

// basic handler that responds with a static string
async fn root() -> &'static str {
    "Hello, World!"
}
