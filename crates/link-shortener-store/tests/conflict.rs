use entity::links;
use link_shortener_store::{Store, StoreError};
use sea_orm::{ActiveValue::Set, ConnectionTrait, Database};
use uuid::Uuid;

// exercises the real postgres unique index through store.create and store.update
// run with: cargo test -p link-shortener-store -- --ignored
#[tokio::test]
#[ignore = "requires a postgres DATABASE_URL with pg_uuidv7"]
async fn duplicate_slug_yields_slug_conflict() {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let db = Database::connect(&url).await.expect("connect");

    db.execute_unprepared(
        "CREATE EXTENSION IF NOT EXISTS pg_uuidv7;
         CREATE TABLE IF NOT EXISTS links (
             id uuid PRIMARY KEY DEFAULT uuid_generate_v7(),
             slug text NOT NULL UNIQUE,
             target_url text NOT NULL,
             owner_id text NOT NULL,
             created_at timestamp NOT NULL DEFAULT now(),
             updated_at timestamp NOT NULL DEFAULT now()
         )",
    )
    .await
    .expect("create schema");

    let store = Store::new(db);
    let taken = format!("taken-{}", Uuid::now_v7());

    let active = |slug: &str| links::ActiveModel {
        slug: Set(slug.to_owned()),
        target_url: Set("https://example.com".to_owned()),
        owner_id: Set("tester".to_owned()),
        ..Default::default()
    };

    // create path: inserting the same slug twice conflicts
    let created = store
        .links()
        .create(active(&taken))
        .await
        .expect("first insert");
    assert_eq!(created.slug, taken);
    let err = store
        .links()
        .create(active(&taken))
        .await
        .expect_err("duplicate create must conflict");
    assert!(
        matches!(&err, StoreError::SlugConflict(s) if *s == taken),
        "create: expected SlugConflict, got {err:?}",
    );

    // update path: renaming a link onto a taken slug conflicts
    let other = store
        .links()
        .create(active(&format!("other-{}", Uuid::now_v7())))
        .await
        .expect("second insert");
    let mut rename: links::ActiveModel = other.into();
    rename.slug = Set(taken.clone());
    let err = store
        .links()
        .update(rename)
        .await
        .expect_err("duplicate update must conflict");
    assert!(
        matches!(&err, StoreError::SlugConflict(s) if *s == taken),
        "update: expected SlugConflict, got {err:?}",
    );

    // a non-colliding rename still succeeds
    let third = store
        .links()
        .create(active(&format!("third-{}", Uuid::now_v7())))
        .await
        .expect("third insert");
    let fresh = format!("fresh-{}", Uuid::now_v7());
    let mut ok_rename: links::ActiveModel = third.into();
    ok_rename.slug = Set(fresh.clone());
    let updated = store
        .links()
        .update(ok_rename)
        .await
        .expect("non-colliding update");
    assert_eq!(updated.slug, fresh);
}
