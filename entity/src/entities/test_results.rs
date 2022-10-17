//! SeaORM Entity. Generated by sea-orm-codegen 0.9.3

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "test_results")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub test_id: i64,
    pub failure: Option<String>,
    pub time_started: DateTimeWithTimeZone,
    pub time_finished: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::tests::Entity",
        from = "Column::TestId",
        to = "super::tests::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    Tests,
}

impl Related<super::tests::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Tests.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
