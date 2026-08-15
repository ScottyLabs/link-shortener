use crate::StoreError;
use ::entity::{link_rules, links, prelude::*};
use sea_orm::sea_query::OnConflict;
use sea_orm::*;
use uuid::Uuid;

pub type LinkWithRules = (links::Model, Vec<link_rules::Model>);

pub struct LinkRepository<'a> {
    db: &'a DatabaseConnection,
}

impl<'a> LinkRepository<'a> {
    pub fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn find_by_id(&self, id: Uuid) -> crate::Result<Option<links::Model>> {
        Ok(Links::find_by_id(id).one(self.db).await?)
    }

    pub async fn find_by_slug(&self, slug: &str) -> crate::Result<Option<LinkWithRules>> {
        Ok(Links::find()
            .filter(links::Column::Slug.eq(slug))
            .find_with_related(LinkRules)
            .order_by_asc(link_rules::Column::Position)
            .all(self.db)
            .await?
            .pop())
    }

    pub async fn list_by_owner(&self, owner_id: &str) -> crate::Result<Vec<LinkWithRules>> {
        Ok(Links::find()
            .filter(links::Column::OwnerId.eq(owner_id))
            .find_with_related(LinkRules)
            .order_by_desc(links::Column::CreatedAt)
            .order_by_asc(link_rules::Column::Position)
            .all(self.db)
            .await?)
    }

    pub async fn list_all(&self) -> crate::Result<Vec<LinkWithRules>> {
        Ok(Links::find()
            .find_with_related(LinkRules)
            .order_by_desc(links::Column::CreatedAt)
            .order_by_asc(link_rules::Column::Position)
            .all(self.db)
            .await?)
    }

    pub async fn create(
        &self,
        link: links::ActiveModel,
        rules: Vec<link_rules::ActiveModel>,
    ) -> crate::Result<LinkWithRules> {
        let slug = active_slug(&link);
        let txn = self.db.begin().await?;

        // insert with on-conflict-do-nothing
        // a duplicate slug then yields no returned row which sea-orm maps to RecordNotFound
        let model = match Links::insert(link)
            .on_conflict(
                OnConflict::column(links::Column::Slug)
                    .do_nothing()
                    .to_owned(),
            )
            .exec_with_returning(&txn)
            .await
        {
            Ok(model) => model,
            Err(DbErr::RecordNotFound(_)) => return Err(StoreError::SlugConflict(slug)),
            Err(err) => return Err(err.into()),
        };

        let rules = insert_rules(&txn, model.id, rules).await?;
        txn.commit().await?;

        Ok((model, rules))
    }

    pub async fn update(
        &self,
        link: links::ActiveModel,
        rules: Option<Vec<link_rules::ActiveModel>>,
    ) -> crate::Result<LinkWithRules> {
        let slug = active_slug(&link);
        let txn = self.db.begin().await?;

        // update has no on-conflict form so catch the unique violation on the slug
        let model = match link.update(&txn).await {
            Ok(model) => model,
            Err(err) if matches!(err.sql_err(), Some(SqlErr::UniqueConstraintViolation(_))) => {
                return Err(StoreError::SlugConflict(slug));
            }
            Err(err) => return Err(err.into()),
        };

        let rules = match rules {
            Some(rules) => {
                LinkRules::delete_many()
                    .filter(link_rules::Column::LinkId.eq(model.id))
                    .exec(&txn)
                    .await?;
                insert_rules(&txn, model.id, rules).await?
            }
            None => {
                LinkRules::find()
                    .filter(link_rules::Column::LinkId.eq(model.id))
                    .order_by_asc(link_rules::Column::Position)
                    .all(&txn)
                    .await?
            }
        };

        txn.commit().await?;

        Ok((model, rules))
    }

    pub async fn delete(&self, id: Uuid) -> crate::Result<DeleteResult> {
        Ok(Links::delete_by_id(id).exec(self.db).await?)
    }
}

async fn insert_rules<C: ConnectionTrait>(
    db: &C,
    link_id: Uuid,
    rules: Vec<link_rules::ActiveModel>,
) -> crate::Result<Vec<link_rules::Model>> {
    let mut saved = Vec::with_capacity(rules.len());
    for (position, mut rule) in rules.into_iter().enumerate() {
        rule.link_id = ActiveValue::Set(link_id);
        rule.position = ActiveValue::Set(position as i32);
        saved.push(rule.insert(db).await?);
    }
    Ok(saved)
}

fn active_slug(link: &links::ActiveModel) -> String {
    match &link.slug {
        ActiveValue::Set(slug) | ActiveValue::Unchanged(slug) => slug.clone(),
        ActiveValue::NotSet => String::new(),
    }
}
