use crate::StoreError;
use ::entity::{link_rules, links, prelude::*};
use sea_orm::sea_query::{Expr, Func};
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

    /// Finds a link by slug, ignoring case to match the unique index on `lower(slug)`
    pub async fn find_by_slug(&self, slug: &str) -> crate::Result<Option<LinkWithRules>> {
        Ok(Links::find()
            .filter(
                Expr::expr(Func::lower(Expr::col((Links, links::Column::Slug))))
                    .eq(Func::lower(Expr::val(slug))),
            )
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

        let model = Links::insert(link)
            .exec_with_returning(&txn)
            .await
            .map_err(|err| slug_conflict(err, slug))?;

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

        let model = link
            .update(&txn)
            .await
            .map_err(|err| slug_conflict(err, slug))?;

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

/// The only unique index a link write can violate is the one on `lower(slug)`
fn slug_conflict(err: DbErr, slug: String) -> StoreError {
    match err.sql_err() {
        Some(SqlErr::UniqueConstraintViolation(_)) => StoreError::SlugConflict(slug),
        _ => err.into(),
    }
}
