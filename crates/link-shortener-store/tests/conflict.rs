mod common;

use entity::links;
use link_shortener_store::StoreError;
use sea_orm::ActiveValue::Set;
use uuid::Uuid;

// exercises the real postgres unique index through store.create and store.update run with: cargo test -p link-shortener-store -- --ignored
#[tokio::test]
#[ignore = "requires a postgres DATABASE_URL with pg_uuidv7"]
async fn duplicate_slug_yields_slug_conflict() {
    let store = common::store().await;
    let taken = format!("taken-{}", Uuid::now_v7());

    let active = |slug: &str| links::ActiveModel {
        slug: Set(slug.to_owned()),
        target_url: Set("https://example.com".to_owned()),
        owner_id: Set(common::OWNER.to_owned()),
        ..Default::default()
    };

    // create path: inserting the same slug twice conflicts
    let created = store
        .links()
        .create(active(&taken), vec![])
        .await
        .expect("first insert")
        .link;
    assert_eq!(created.slug, taken);
    let err = store
        .links()
        .create(active(&taken), vec![])
        .await
        .expect_err("duplicate create must conflict");
    assert!(
        matches!(&err, StoreError::SlugConflict(s) if *s == taken),
        "create: expected SlugConflict, got {err:?}",
    );

    // update path: renaming a link onto a taken slug conflicts
    let other = store
        .links()
        .create(active(&format!("other-{}", Uuid::now_v7())), vec![])
        .await
        .expect("second insert")
        .link;
    let mut rename: links::ActiveModel = other.into();
    rename.slug = Set(taken.clone());
    let err = store
        .links()
        .update(rename, None)
        .await
        .expect_err("duplicate update must conflict");
    assert!(
        matches!(&err, StoreError::SlugConflict(s) if *s == taken),
        "update: expected SlugConflict, got {err:?}",
    );

    // a non-colliding rename still succeeds
    let third = store
        .links()
        .create(active(&format!("third-{}", Uuid::now_v7())), vec![])
        .await
        .expect("third insert")
        .link;
    let fresh = format!("fresh-{}", Uuid::now_v7());
    let mut ok_rename: links::ActiveModel = third.into();
    ok_rename.slug = Set(fresh.clone());
    let updated = store
        .links()
        .update(ok_rename, None)
        .await
        .expect("non-colliding update")
        .link;
    assert_eq!(updated.slug, fresh);
}

#[tokio::test]
#[ignore = "requires a postgres DATABASE_URL with pg_uuidv7"]
async fn slugs_ignore_case() {
    let store = common::store().await;
    let slug = format!("Mixed-{}", Uuid::now_v7());

    let active = |slug: &str| links::ActiveModel {
        slug: Set(slug.to_owned()),
        target_url: Set("https://example.com".to_owned()),
        owner_id: Set(common::OWNER.to_owned()),
        ..Default::default()
    };

    store
        .links()
        .create(active(&slug), vec![])
        .await
        .expect("insert");

    let found = store
        .links()
        .find_by_slug(&slug.to_lowercase())
        .await
        .expect("find")
        .expect("lookup ignores case")
        .link;
    assert_eq!(
        found.slug, slug,
        "the slug keeps the case it was created with"
    );

    let err = store
        .links()
        .create(active(&slug.to_uppercase()), vec![])
        .await
        .expect_err("a slug differing only in case must conflict");
    assert!(
        matches!(err, StoreError::SlugConflict(_)),
        "expected SlugConflict, got {err:?}",
    );
}
