use sea_orm_migration::{prelude::*, schema::*};

use crate::iden::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let table = table_auto(PairingHistory::Table)
            .col(pk_auto(PairingHistory::Id))
            .col(integer(PairingHistory::PotluckId))
            .col(integer(PairingHistory::OrganizationId))
            .col(integer_null(PairingHistory::EntityAPersonId))
            .col(integer_null(PairingHistory::EntityAHouseholdId))
            .col(integer_null(PairingHistory::EntityBPersonId))
            .col(integer_null(PairingHistory::EntityBHouseholdId))
            .foreign_key(
                ForeignKey::create()
                    .name("fk_pairing_potluck")
                    .from(PairingHistory::Table, PairingHistory::PotluckId)
                    .to(Potluck::Table, Potluck::Id)
                    .on_delete(ForeignKeyAction::Cascade),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("fk_pairing_organization")
                    .from(PairingHistory::Table, PairingHistory::OrganizationId)
                    .to(Organization::Table, Organization::Id)
                    .on_delete(ForeignKeyAction::Cascade),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("fk_pairing_person_a")
                    .from(PairingHistory::Table, PairingHistory::EntityAPersonId)
                    .to(Person::Table, Person::Id)
                    .on_delete(ForeignKeyAction::Cascade),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("fk_pairing_household_a")
                    .from(PairingHistory::Table, PairingHistory::EntityAHouseholdId)
                    .to(Household::Table, Household::Id)
                    .on_delete(ForeignKeyAction::Cascade),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("fk_pairing_person_b")
                    .from(PairingHistory::Table, PairingHistory::EntityBPersonId)
                    .to(Person::Table, Person::Id)
                    .on_delete(ForeignKeyAction::Cascade),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("fk_pairing_household_b")
                    .from(PairingHistory::Table, PairingHistory::EntityBHouseholdId)
                    .to(Household::Table, Household::Id)
                    .on_delete(ForeignKeyAction::Cascade),
            )
            .check(
                Expr::col(PairingHistory::EntityAPersonId)
                    .is_not_null()
                    .and(Expr::col(PairingHistory::EntityAHouseholdId).is_null())
                    .or(Expr::col(PairingHistory::EntityAPersonId)
                        .is_null()
                        .and(Expr::col(PairingHistory::EntityAHouseholdId).is_not_null())),
            )
            .check(
                Expr::col(PairingHistory::EntityBPersonId)
                    .is_not_null()
                    .and(Expr::col(PairingHistory::EntityBHouseholdId).is_null())
                    .or(Expr::col(PairingHistory::EntityBPersonId)
                        .is_null()
                        .and(Expr::col(PairingHistory::EntityBHouseholdId).is_not_null())),
            )
            .to_owned();
        manager.create_table(table).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PairingHistory::Table).to_owned())
            .await?;

        Ok(())
    }
}
