pub use sea_orm_migration::prelude::*;

mod m20220101_000001_create_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20220101_000001_create_table::Migration)]
    }
}

pub fn bigint_primary_key<T: 'static>(name: T) -> ColumnDef
where
    T: Iden,
{
    ColumnDef::new(name)
        .big_integer()
        .not_null()
        .auto_increment()
        .primary_key()
        .to_owned()
}
