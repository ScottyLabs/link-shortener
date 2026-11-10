use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Links::Table)
                    .drop_constraint(Alias::new("links_slug_key"))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_links_slug_lower")
                    .table(Links::Table)
                    .col(Func::lower(Expr::col(Links::Slug)))
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(Index::drop().name("idx_links_slug_lower").to_owned())
            .await?;

        // restores links_slug_key
        manager
            .alter_table(
                Table::alter()
                    .table(Links::Table)
                    .modify_column(ColumnDef::new(Links::Slug).unique_key())
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Links {
    Table,
    Slug,
}
