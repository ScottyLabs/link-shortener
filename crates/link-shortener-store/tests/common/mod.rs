use sea_orm::{ConnectionTrait, Database, DatabaseConnection};

pub async fn connect() -> DatabaseConnection {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let db = Database::connect(&url).await.expect("connect");

    db.execute_unprepared(
        "CREATE EXTENSION IF NOT EXISTS pg_uuidv7;
         CREATE TABLE IF NOT EXISTS links (
             id uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
             slug text NOT NULL UNIQUE,
             target_url text NOT NULL,
             owner_id text NOT NULL,
             owner_name text,
             created_at timestamp NOT NULL DEFAULT now(),
             updated_at timestamp NOT NULL DEFAULT now()
         );
         CREATE TABLE IF NOT EXISTS link_rules (
             id uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
             link_id uuid NOT NULL REFERENCES links (id) ON DELETE CASCADE,
             position integer NOT NULL,
             kind text NOT NULL,
             pattern text NOT NULL,
             target_url text NOT NULL,
             created_at timestamp NOT NULL DEFAULT now(),
             updated_at timestamp NOT NULL DEFAULT now(),
             UNIQUE (link_id, position)
         )",
    )
    .await
    .expect("create schema");

    db
}
