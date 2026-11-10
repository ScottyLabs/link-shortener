use ::entity::{prelude::*, users};
use sea_orm::sea_query::{Expr, OnConflict};
use sea_orm::*;

pub struct UserRepository<'a> {
    db: &'a DatabaseConnection,
}

impl<'a> UserRepository<'a> {
    pub fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }

    /// Records the user's current display name, creating the user the first time they are seen
    pub async fn upsert(&self, id: &str, name: &str) -> crate::Result<()> {
        let user = users::ActiveModel {
            id: ActiveValue::Set(id.to_owned()),
            name: ActiveValue::Set(Some(name.to_owned())),
            ..Default::default()
        };

        // skip the write, and the updated_at bump, when the name is unchanged
        Users::insert(user)
            .on_conflict(
                OnConflict::column(users::Column::Id)
                    .update_column(users::Column::Name)
                    .action_and_where(Expr::cust("users.name IS DISTINCT FROM excluded.name"))
                    .to_owned(),
            )
            .exec_without_returning(self.db)
            .await?;

        Ok(())
    }
}
