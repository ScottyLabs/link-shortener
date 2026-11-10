mod common;

use entity::links;
use sea_orm::ActiveValue::Set;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires a postgres DATABASE_URL with pg_uuidv7"]
async fn links_show_the_owners_current_name() {
    let store = common::store().await;
    let owner = format!("owner-{}", Uuid::now_v7());
    let slug = format!("named-{}", Uuid::now_v7());

    store
        .users()
        .upsert(&owner, "Old Name")
        .await
        .expect("create owner");
    store
        .links()
        .create(
            links::ActiveModel {
                slug: Set(slug.clone()),
                target_url: Set("https://example.com".to_owned()),
                owner_id: Set(owner.clone()),
                ..Default::default()
            },
            vec![],
        )
        .await
        .expect("create link");

    store
        .users()
        .upsert(&owner, "New Name")
        .await
        .expect("rename owner");

    let found = store
        .links()
        .find_by_slug(&slug)
        .await
        .expect("find")
        .expect("link exists");
    assert_eq!(
        found.owner.and_then(|owner| owner.name).as_deref(),
        Some("New Name"),
    );

    let listed = store.links().list_by_owner(&owner).await.expect("list");
    assert_eq!(
        listed[0]
            .owner
            .as_ref()
            .and_then(|owner| owner.name.as_deref()),
        Some("New Name"),
    );
}
