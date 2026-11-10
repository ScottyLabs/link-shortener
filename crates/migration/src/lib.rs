pub use sea_orm_migration::prelude::*;

mod m20260330_152532_create_links;
mod m20260624_120000_add_owner_name;
mod m20260814_120000_create_link_rules;
mod m20261110_203329_case_insensitive_slugs;
mod m20261110_204159_create_users;
mod m20261110_204520_add_link_pause_and_expiry;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260330_152532_create_links::Migration),
            Box::new(m20260624_120000_add_owner_name::Migration),
            Box::new(m20260814_120000_create_link_rules::Migration),
            Box::new(m20261110_203329_case_insensitive_slugs::Migration),
            Box::new(m20261110_204159_create_users::Migration),
            Box::new(m20261110_204520_add_link_pause_and_expiry::Migration),
        ]
    }
}
