use sea_orm::EnumIter;
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
    HouseholdId,
}

#[derive(DeriveIden)]
pub enum Household {
    Table,
    Id,
    PcoId,
    OrganizationId,
    Name,
    PrimaryContactId,
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
    PotluckSeriesId,
    HostType,
    HostId,
}

#[derive(DeriveIden)]
pub enum Attendance {
    Table,
    Id,
    PotluckId,
    OrganizationId,
    AttendeeType,
    AttendeeId,
}

#[derive(DeriveIden)]
pub struct AttendeeTypeEnum;

#[derive(DeriveIden, EnumIter)]
pub enum AttendeeType {
    Person,
    Household,
}
