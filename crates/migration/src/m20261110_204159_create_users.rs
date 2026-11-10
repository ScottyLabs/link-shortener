use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::{DbBackend, Statement};

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20261110_204159_create_users"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Users::Table)
                    .col(ColumnDef::new(Users::Id).text().not_null().primary_key())
                    .col(ColumnDef::new(Users::Name).text().null())
                    .col(
                        ColumnDef::new(Users::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Users::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .get_connection()
            .execute_raw(Statement::from_string(
                DbBackend::Postgres,
                "CREATE TRIGGER update_users_updated_at BEFORE UPDATE ON users \
         FOR EACH ROW EXECUTE FUNCTION update_updated_at_column()"
                    .to_string(),
            ))
            .await?;

        let latest_names = Query::select()
            .distinct_on([Links::OwnerId])
            .columns([Links::OwnerId, Links::OwnerName])
            .from(Links::Table)
            .order_by(Links::OwnerId, Order::Asc)
            // links from before the owner_name migration have a null name
            .order_by_expr(Expr::col(Links::OwnerName).is_null(), Order::Asc)
            .order_by(Links::UpdatedAt, Order::Desc)
            .to_owned();
        manager
            .exec_stmt(
                Query::insert()
                    .into_table(Users::Table)
                    .columns([Users::Id, Users::Name])
                    .select_from(latest_names)
                    .map_err(|err| DbErr::Migration(err.to_string()))?
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Links::Table)
                    .add_foreign_key(
                        TableForeignKey::new()
                            .name("fk_links_owner_id")
                            .from_tbl(Links::Table)
                            .from_col(Links::OwnerId)
                            .to_tbl(Users::Table)
                            .to_col(Users::Id),
                    )
                    .drop_column(Links::OwnerName)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Links::Table)
                    .add_column(ColumnDef::new(Links::OwnerName).text().null())
                    .to_owned(),
            )
            .await?;

        manager
            .exec_stmt(
                Query::update()
                    .table(Links::Table)
                    .value(Links::OwnerName, Expr::col((Users::Table, Users::Name)))
                    .from(Users::Table)
                    .and_where(
                        Expr::col((Users::Table, Users::Id)).equals((Links::Table, Links::OwnerId)),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Links::Table)
                    .drop_foreign_key(Alias::new("fk_links_owner_id"))
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(Users::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
    Name,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Links {
    Table,
    OwnerId,
    OwnerName,
    UpdatedAt,
}
