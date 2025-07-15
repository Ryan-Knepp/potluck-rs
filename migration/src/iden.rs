use sea_orm_migration::prelude::*;

// Define table names
#[derive(DeriveIden)]
pub enum Organization {
    Table,
    Id,
    PcoId,
    Name,
    AvatarUrl,
}

#[derive(DeriveIden)]
pub enum Person {
    Table,
    Id,
    PcoId,
    OrganizationId,
    Name,
    Email,
    Phone,
    Address,
    AvatarUrl,
    IsSignedUp,
    CanHost,
    IsChild,
    HouseholdId,
}

#[derive(DeriveIden)]
pub enum Household {
    Table,
    Id,
    PcoId,
    OrganizationId,
    Name,
    IsSignedUp,
    CanHost,
    AvatarUrl,
}

#[derive(DeriveIden)]
pub enum PotluckSeries {
    Table,
    Id,
    OrganizationId,
    Name,
    StartDate,
    EndDate,
    Description,
}

#[derive(DeriveIden)]
pub enum Potluck {
    Table,
    Id,
    OrganizationId,
    #[allow(clippy::enum_variant_names)]
    PotluckSeriesId,
    HostPersonId,
    HostHouseholdId,
}

#[derive(DeriveIden)]
pub enum Attendance {
    Table,
    Id,
    PotluckId,
    OrganizationId,
    AttendeePersonId,
    AttendeeHouseholdId,
}

#[derive(DeriveIden)]
pub enum User {
    Table,
    Id,
    PersonId,
    OrganizationId,
    AccessToken,
    RefreshToken,
    TokenExpiresAt,
}

#[derive(DeriveIden)]
pub enum PairingHistory {
    Table,
    Id,
    PotluckId,
    OrganizationId,
    EntityAPersonId,
    EntityAHouseholdId,
    EntityBPersonId,
    EntityBHouseholdId,
}
