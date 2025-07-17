use sea_orm_migration::{prelude::*, schema::*};

use crate::iden::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create Organization Table
        let table = table_auto(Organization::Table)
            .col(pk_uuid(Organization::Id))
            .col(string_uniq(Organization::PcoId))
            .col(string(Organization::Name))
            .col(string_null(Organization::AvatarUrl))
            .to_owned();
        manager.create_table(table).await?;

        // Create Person Table
        let table = table_auto(Person::Table)
            .col(pk_uuid(Person::Id))
            .col(string_uniq(Person::PcoId))
            .col(uuid(Person::OrganizationId))
            .col(string(Person::Name))
            .col(string_null(Person::Email))
            .col(string_null(Person::Phone))
            .col(json(Person::Address))
            .col(string_null(Person::AvatarUrl))
            .col(boolean(Person::IsSignedUp).default(false))
            .col(boolean(Person::CanHost).default(false))
            .col(boolean(Person::IsChild).default(false))
            .col(uuid_null(Person::HouseholdId))
            .to_owned();
        manager.create_table(table).await?;

        // Create Household Table
        let table = table_auto(Household::Table)
            .col(pk_uuid(Household::Id))
            .col(string_uniq(Household::PcoId))
            .col(uuid(Household::OrganizationId))
            .col(string(Household::Name))
            .col(boolean(Household::IsSignedUp).default(false))
            .col(boolean(Household::CanHost).default(false))
            .col(string_null(Household::AvatarUrl))
            .foreign_key(
                ForeignKey::create()
                    .name("fk_household_organization")
                    .from(Household::Table, Household::OrganizationId)
                    .to(Organization::Table, Organization::Id)
                    .on_delete(ForeignKeyAction::Cascade),
            )
            .to_owned();
        manager.create_table(table).await?;

        // Add Foreign Key to Person Table for Household
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_person_household")
                    .from(Person::Table, Person::HouseholdId)
                    .to(Household::Table, Household::Id)
                    .on_delete(ForeignKeyAction::SetNull)
                    .to_owned(),
            )
            .await?;

        // Create PotluckSeries Table
        let table = table_auto(PotluckSeries::Table)
            .col(pk_uuid(PotluckSeries::Id))
            .col(uuid(PotluckSeries::OrganizationId))
            .col(string(PotluckSeries::Name))
            .col(date(PotluckSeries::StartDate))
            .col(date(PotluckSeries::EndDate))
            .col(string_null(PotluckSeries::Description))
            .foreign_key(
                ForeignKey::create()
                    .name("fk_potluckseries_organization")
                    .from(PotluckSeries::Table, PotluckSeries::OrganizationId)
                    .to(Organization::Table, Organization::Id)
                    .on_delete(ForeignKeyAction::Cascade),
            )
            .check(Expr::col(PotluckSeries::EndDate).gt(Expr::col(PotluckSeries::StartDate)))
            .to_owned();
        manager.create_table(table).await?;

        // Create Potluck Table
        let table = table_auto(Potluck::Table)
            .col(pk_uuid(Potluck::Id))
            .col(uuid(Potluck::OrganizationId))
            .col(uuid(Potluck::PotluckSeriesId))
            .col(uuid_null(Potluck::HostPersonId))
            .col(uuid_null(Potluck::HostHouseholdId))
            .foreign_key(
                ForeignKey::create()
                    .name("fk_potluck_organization")
                    .from(Potluck::Table, Potluck::OrganizationId)
                    .to(Organization::Table, Organization::Id)
                    .on_delete(ForeignKeyAction::Cascade),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("fk_potluck_series")
                    .from(Potluck::Table, Potluck::PotluckSeriesId)
                    .to(PotluckSeries::Table, PotluckSeries::Id)
                    .on_delete(ForeignKeyAction::Cascade),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("fk_potluck_host_person")
                    .from(Potluck::Table, Potluck::HostPersonId)
                    .to(Person::Table, Person::Id)
                    .on_delete(ForeignKeyAction::SetNull),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("fk_potluck_host_household")
                    .from(Potluck::Table, Potluck::HostHouseholdId)
                    .to(Household::Table, Household::Id)
                    .on_delete(ForeignKeyAction::SetNull),
            )
            .check(
                Expr::col(Potluck::HostPersonId)
                    .is_not_null()
                    .and(Expr::col(Potluck::HostHouseholdId).is_null())
                    .or(Expr::col(Potluck::HostPersonId)
                        .is_null()
                        .and(Expr::col(Potluck::HostHouseholdId).is_not_null())),
            )
            .to_owned();
        manager.create_table(table).await?;

        // Create Attendance Table
        let table = table_auto(Attendance::Table)
            .col(pk_uuid(Attendance::Id))
            .col(uuid(Attendance::PotluckId))
            .col(uuid(Attendance::OrganizationId))
            .col(uuid_null(Attendance::AttendeePersonId))
            .col(uuid_null(Attendance::AttendeeHouseholdId))
            .foreign_key(
                ForeignKey::create()
                    .name("fk_attendance_potluck")
                    .from(Attendance::Table, Attendance::PotluckId)
                    .to(Potluck::Table, Potluck::Id)
                    .on_delete(ForeignKeyAction::Cascade),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("fk_attendance_organization")
                    .from(Attendance::Table, Attendance::OrganizationId)
                    .to(Organization::Table, Organization::Id)
                    .on_delete(ForeignKeyAction::Cascade),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("fk_attendance_person")
                    .from(Attendance::Table, Attendance::AttendeePersonId)
                    .to(Person::Table, Person::Id)
                    .on_delete(ForeignKeyAction::Cascade),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("fk_attendance_household")
                    .from(Attendance::Table, Attendance::AttendeeHouseholdId)
                    .to(Household::Table, Household::Id)
                    .on_delete(ForeignKeyAction::Cascade),
            )
            .check(
                Expr::col(Attendance::AttendeePersonId)
                    .is_not_null()
                    .and(Expr::col(Attendance::AttendeeHouseholdId).is_null())
                    .or(Expr::col(Attendance::AttendeePersonId)
                        .is_null()
                        .and(Expr::col(Attendance::AttendeeHouseholdId).is_not_null())),
            )
            .to_owned();
        manager.create_table(table).await?;

        // Create indices for common lookups
        manager
            .create_index(
                Index::create()
                    .name("idx_person_organization")
                    .table(Person::Table)
                    .col(Person::OrganizationId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_household_organization")
                    .table(Household::Table)
                    .col(Household::OrganizationId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_potluck_series")
                    .table(Potluck::Table)
                    .col(Potluck::PotluckSeriesId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_attendance_potluck")
                    .table(Attendance::Table)
                    .col(Attendance::PotluckId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_person_household")
                    .table(Person::Table)
                    .to_owned(),
            )
            .await?;

        // Drop all tables in reverse order to avoid foreign key constraints
        manager
            .drop_table(Table::drop().table(Attendance::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Potluck::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(PotluckSeries::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Household::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Person::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Organization::Table).to_owned())
            .await?;

        Ok(())
    }
}
