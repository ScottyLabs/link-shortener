use thiserror::Error;

#[derive(Error, Debug)]
pub enum StoreError {
    #[error("database error: {0}")]
    Database(#[from] sea_orm::DbErr),

    #[error("link not found: {0}")]
    LinkNotFound(uuid::Uuid),

    #[error("slug already exists: {0}")]
    SlugConflict(String),
}

pub type Result<T> = std::result::Result<T, StoreError>;
