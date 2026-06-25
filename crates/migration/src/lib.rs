pub use sea_orm_migration::prelude::*;

mod m20260330_152532_create_links;
mod m20260624_120000_add_owner_name;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260330_152532_create_links::Migration),
            Box::new(m20260624_120000_add_owner_name::Migration),
        ]
    }
}
