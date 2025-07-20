use sea_orm_migration::{prelude::*, schema::*};

use crate::iden::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let table = table_auto(User::Table)
            .col(pk_auto(User::Id))
            .col(integer(User::PersonId))
            .col(integer(User::OrganizationId))
            .col(string(User::AccessToken))
            .col(string_null(User::RefreshToken))
            .col(timestamp(User::TokenExpiresAt))
            .foreign_key(
                ForeignKey::create()
                    .name("fk_user_person")
                    .from(User::Table, User::PersonId)
                    .to(Person::Table, Person::Id)
                    .on_delete(ForeignKeyAction::Cascade),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("fk_user_organizaation")
                    .from(User::Table, User::OrganizationId)
                    .to(Organization::Table, Organization::Id)
                    .on_delete(ForeignKeyAction::Cascade),
            )
            .to_owned();
        manager.create_table(table).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(User::Table).to_owned())
            .await?;

        Ok(())
    }
}
