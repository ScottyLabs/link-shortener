use crate::StoreError;
use ::entity::{links, prelude::*};
use sea_orm::sea_query::OnConflict;
use sea_orm::*;
use uuid::Uuid;

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

    pub async fn find_by_slug(&self, slug: &str) -> crate::Result<Option<links::Model>> {
        Ok(Links::find()
            .filter(links::Column::Slug.eq(slug))
            .one(self.db)
            .await?)
    }

    pub async fn list_by_owner(&self, owner_id: &str) -> crate::Result<Vec<links::Model>> {
        Ok(Links::find()
            .filter(links::Column::OwnerId.eq(owner_id))
            .order_by_desc(links::Column::CreatedAt)
            .all(self.db)
            .await?)
    }

    pub async fn list_all(&self) -> crate::Result<Vec<links::Model>> {
        Ok(Links::find()
            .order_by_desc(links::Column::CreatedAt)
            .all(self.db)
            .await?)
    }

    pub async fn create(&self, link: links::ActiveModel) -> crate::Result<links::Model> {
        let slug = active_slug(&link);

        // insert with on-conflict-do-nothing
        // a duplicate slug then yields no returned row which sea-orm maps to RecordNotFound
        match Links::insert(link)
            .on_conflict(
                OnConflict::column(links::Column::Slug)
                    .do_nothing()
                    .to_owned(),
            )
            .exec_with_returning(self.db)
            .await
        {
            Ok(model) => Ok(model),
            Err(DbErr::RecordNotFound(_)) => Err(StoreError::SlugConflict(slug)),
            Err(err) => Err(err.into()),
        }
    }

    pub async fn update(&self, link: links::ActiveModel) -> crate::Result<links::Model> {
        let slug = active_slug(&link);

        // update has no on-conflict form so catch the unique violation on the slug
        match link.update(self.db).await {
            Ok(model) => Ok(model),
            Err(err) if matches!(err.sql_err(), Some(SqlErr::UniqueConstraintViolation(_))) => {
                Err(StoreError::SlugConflict(slug))
            }
            Err(err) => Err(err.into()),
        }
    }

    pub async fn delete(&self, id: Uuid) -> crate::Result<DeleteResult> {
        Ok(Links::delete_by_id(id).exec(self.db).await?)
    }
}

fn active_slug(link: &links::ActiveModel) -> String {
    match &link.slug {
        ActiveValue::Set(slug) | ActiveValue::Unchanged(slug) => slug.clone(),
        ActiveValue::NotSet => String::new(),
    }
}
