use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::{DbBackend, Statement};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(LinkRules::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(LinkRules::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT uuid_generate_v7()".to_string()),
                    )
                    .col(ColumnDef::new(LinkRules::LinkId).uuid().not_null())
                    .col(ColumnDef::new(LinkRules::Position).integer().not_null())
                    .col(ColumnDef::new(LinkRules::Kind).text().not_null())
                    .col(ColumnDef::new(LinkRules::Pattern).text().not_null())
                    .col(ColumnDef::new(LinkRules::TargetUrl).text().not_null())
                    .col(
                        ColumnDef::new(LinkRules::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(LinkRules::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_link_rules_link_id")
                            .from(LinkRules::Table, LinkRules::LinkId)
                            .to(Links::Table, Links::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_link_rules_link_id_position")
                    .table(LinkRules::Table)
                    .col(LinkRules::LinkId)
                    .col(LinkRules::Position)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .get_connection()
            .execute(Statement::from_string(
                DbBackend::Postgres,
                "CREATE TRIGGER update_link_rules_updated_at BEFORE UPDATE ON link_rules \
                 FOR EACH ROW EXECUTE FUNCTION update_updated_at_column()"
                    .to_string(),
            ))
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(LinkRules::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum LinkRules {
    Table,
    Id,
    LinkId,
    Position,
    Kind,
    Pattern,
    TargetUrl,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Links {
    Table,
    Id,
}
