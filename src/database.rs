use migration::{Migrator, MigratorTrait};
use sea_orm::{sqlx::PgPool, Database, DatabaseConnection};

pub async fn setup_database(db_url: &str) -> anyhow::Result<(DatabaseConnection, PgPool)> {
    let db = Database::connect(db_url)
        .await
        .expect("Cannot connect to db");
    Migrator::up(&db, None).await.unwrap();

    let pool = PgPool::connect(db_url).await?;

    Ok((db, pool))
}
