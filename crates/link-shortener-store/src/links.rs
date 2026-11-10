use crate::StoreError;
use ::entity::{link_rules, links, prelude::*, users};
use sea_orm::sea_query::{Expr, Func};
use sea_orm::*;
use uuid::Uuid;

/// A link with its owner and its rules in priority order
#[derive(Debug)]
pub struct LinkDetails {
    pub link: links::Model,
    pub owner: Option<users::Model>,
    pub rules: Vec<link_rules::Model>,
}

type DetailsQuery = SelectThree<Links, Users, LinkRules, TopologyStar>;

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
    pub async fn find_by_slug(&self, slug: &str) -> crate::Result<Option<LinkDetails>> {
        let query = details_query().filter(
            Expr::expr(Func::lower(Expr::col((Links, links::Column::Slug))))
                .eq(Func::lower(Expr::val(slug))),
        );

        Ok(self.load(query).await?.pop())
    }

    pub async fn list_by_owner(&self, owner_id: &str) -> crate::Result<Vec<LinkDetails>> {
        self.load(
            details_query()
                .filter(links::Column::OwnerId.eq(owner_id))
                .order_by_desc(links::Column::CreatedAt),
        )
        .await
    }

    pub async fn list_all(&self) -> crate::Result<Vec<LinkDetails>> {
        self.load(details_query().order_by_desc(links::Column::CreatedAt))
            .await
    }

    async fn load(&self, query: DetailsQuery) -> crate::Result<Vec<LinkDetails>> {
        let rows = query
            .order_by_asc(link_rules::Column::Position)
            .consolidate()
            .all(self.db)
            .await?;

        Ok(rows
            .into_iter()
            .map(|(link, owners, rules)| LinkDetails {
                link,
                owner: owners.into_iter().next(),
                rules,
            })
            .collect())
    }

    pub async fn create(
        &self,
        link: links::ActiveModel,
        rules: Vec<link_rules::ActiveModel>,
    ) -> crate::Result<LinkDetails> {
        let slug = active_slug(&link);
        let txn = self.db.begin().await?;

        let model = Links::insert(link)
            .exec_with_returning(&txn)
            .await
            .map_err(|err| slug_conflict(err, slug))?;

        let rules = insert_rules(&txn, model.id, rules).await?;
        let owner = Users::find_by_id(&model.owner_id).one(&txn).await?;
        txn.commit().await?;

        Ok(LinkDetails {
            link: model,
            owner,
            rules,
        })
    }

    pub async fn update(
        &self,
        link: links::ActiveModel,
        rules: Option<Vec<link_rules::ActiveModel>>,
    ) -> crate::Result<LinkDetails> {
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

        let owner = Users::find_by_id(&model.owner_id).one(&txn).await?;
        txn.commit().await?;

        Ok(LinkDetails {
            link: model,
            owner,
            rules,
        })
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

/// Links joined with their owner and rules in a single query
fn details_query() -> DetailsQuery {
    Links::find()
        .find_also_related(Users)
        .find_also_related(LinkRules)
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
