pub use sea_orm_migration::prelude::*;

mod iden;
mod m20220101_000001_create_table;
mod m20250405_153121_user_table;
mod m20250715_040011_add_pairing_history_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_table::Migration),
            Box::new(m20250405_153121_user_table::Migration),
            Box::new(m20250715_040011_add_pairing_history_table::Migration),
        ]
    }
}
