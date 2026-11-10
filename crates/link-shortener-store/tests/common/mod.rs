use link_shortener_store::Store;
use migration::{Migrator, MigratorTrait};
use sea_orm::Database;

/// Owner of the links the tests create
pub const OWNER: &str = "tester";

pub async fn store() -> Store {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let db = Database::connect(&url).await.expect("connect");

    Migrator::up(&db, None).await.expect("migrate");

    let store = Store::new(db);
    store
        .users()
        .upsert(OWNER, "Tester")
        .await
        .expect("create owner");

    store
}
