pub mod error;
pub mod links;

pub use error::{Result, StoreError};

use sea_orm::DatabaseConnection;

pub struct Store {
    db: DatabaseConnection,
}

impl Store {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub fn db(&self) -> &DatabaseConnection {
        &self.db
    }

    pub fn links(&self) -> links::LinkRepository<'_> {
        links::LinkRepository::new(&self.db)
    }
}
