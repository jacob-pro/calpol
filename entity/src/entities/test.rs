//! SeaORM Entity. Generated by sea-orm-codegen 0.9.3

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "test")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub enabled: bool,
    pub config: Json,
    pub failing: bool,
    pub failure_threshold: i16,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::test_result::Entity")]
    TestResult,
}

impl Related<super::test_result::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TestResult.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}