//! `SeaORM` Entity, @generated by sea-orm-codegen 1.1.8

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "pairing_history")]
pub struct Model {
    pub created_at: DateTime,
    pub updated_at: DateTime,
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i32,
    pub potluck_id: i32,
    pub organization_id: i32,
    pub entity_a_person_id: Option<i32>,
    pub entity_a_household_id: Option<i32>,
    pub entity_b_person_id: Option<i32>,
    pub entity_b_household_id: Option<i32>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::household::Entity",
        from = "Column::EntityAHouseholdId",
        to = "super::household::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Household2,
    #[sea_orm(
        belongs_to = "super::household::Entity",
        from = "Column::EntityBHouseholdId",
        to = "super::household::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Household1,
    #[sea_orm(
        belongs_to = "super::organization::Entity",
        from = "Column::OrganizationId",
        to = "super::organization::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Organization,
    #[sea_orm(
        belongs_to = "super::person::Entity",
        from = "Column::EntityAPersonId",
        to = "super::person::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Person2,
    #[sea_orm(
        belongs_to = "super::person::Entity",
        from = "Column::EntityBPersonId",
        to = "super::person::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Person1,
    #[sea_orm(
        belongs_to = "super::potluck::Entity",
        from = "Column::PotluckId",
        to = "super::potluck::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Potluck,
}

impl Related<super::organization::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Organization.def()
    }
}

impl Related<super::potluck::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Potluck.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
