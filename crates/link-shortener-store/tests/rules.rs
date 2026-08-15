mod common;

use entity::{link_rules, links};
use link_shortener_store::Store;
use sea_orm::{ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires a postgres DATABASE_URL with pg_uuidv7"]
async fn rules_round_trip_in_order_and_follow_their_link() {
    let db = common::connect().await;
    let store = Store::new(db);
    let slug = format!("app-{}", Uuid::now_v7());

    let link = links::ActiveModel {
        slug: Set(slug.clone()),
        target_url: Set("https://example.com/web".to_owned()),
        owner_id: Set("tester".to_owned()),
        ..Default::default()
    };
    let rule = |kind: &str, pattern: &str, target: &str| link_rules::ActiveModel {
        kind: Set(kind.to_owned()),
        pattern: Set(pattern.to_owned()),
        target_url: Set(target.to_owned()),
        ..Default::default()
    };

    let (created, rules) = store
        .links()
        .create(
            link,
            vec![
                rule("regex", "CriOS", "https://example.com/chrome-ios"),
                rule("platform", "ios", "https://apps.apple.com/app"),
                rule("platform", "android", "https://play.google.com/app"),
            ],
        )
        .await
        .expect("create with rules");

    assert_eq!(
        rules.iter().map(|r| r.position).collect::<Vec<_>>(),
        [0, 1, 2],
        "positions follow the given order",
    );

    let (_, fetched) = store
        .links()
        .find_by_slug(&slug)
        .await
        .expect("find")
        .expect("link exists");
    assert_eq!(
        fetched
            .iter()
            .map(|r| r.pattern.as_str())
            .collect::<Vec<_>>(),
        ["CriOS", "ios", "android"],
    );

    let mut rename: links::ActiveModel = created.clone().into();
    rename.target_url = Set("https://example.com/new".to_owned());
    let (_, untouched) = store
        .links()
        .update(rename, None)
        .await
        .expect("update without rules");
    assert_eq!(untouched.len(), 3);

    let (_, replaced) = store
        .links()
        .update(
            created.clone().into(),
            Some(vec![rule(
                "platform",
                "mobile",
                "https://example.com/mobile",
            )]),
        )
        .await
        .expect("update with rules");
    assert_eq!(replaced.len(), 1);
    assert_eq!(replaced[0].position, 0);
    assert_eq!(replaced[0].pattern, "mobile");

    store.links().delete(created.id).await.expect("delete");
    assert!(
        store
            .links()
            .find_by_slug(&slug)
            .await
            .expect("find")
            .is_none(),
        "link is gone",
    );
    let orphans = link_rules::Entity::find()
        .filter(link_rules::Column::LinkId.eq(created.id))
        .all(store.db())
        .await
        .expect("count orphans");
    assert!(orphans.is_empty(), "rules cascade with the link");
}
