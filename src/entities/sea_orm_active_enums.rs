//! `SeaORM` Entity, @generated by sea-orm-codegen 1.1.8

use sea_orm::entity::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "attendee_type_enum")]
pub enum AttendeeTypeEnum {
    #[sea_orm(string_value = "person")]
    Person,
    #[sea_orm(string_value = "household")]
    Household,
}
