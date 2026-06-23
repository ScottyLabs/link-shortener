use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::{DbBackend, Statement};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute(Statement::from_string(
            DbBackend::Postgres,
            "CREATE EXTENSION IF NOT EXISTS pg_uuidv7".to_string(),
        ))
        .await?;

        manager
            .create_table(
                Table::create()
                    .table(Links::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Links::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT uuid_generate_v7()".to_string()),
                    )
                    .col(ColumnDef::new(Links::Slug).text().not_null().unique_key())
                    .col(ColumnDef::new(Links::TargetUrl).text().not_null())
                    .col(ColumnDef::new(Links::OwnerId).text().not_null())
                    .col(
                        ColumnDef::new(Links::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Links::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_links_owner_id")
                    .table(Links::Table)
                    .col(Links::OwnerId)
                    .to_owned(),
            )
            .await?;

        // updated_at trigger
        db.execute(Statement::from_string(
            DbBackend::Postgres,
            r#"
            CREATE OR REPLACE FUNCTION update_updated_at_column()
            RETURNS TRIGGER AS $$
            BEGIN
                NEW.updated_at = now();
                RETURN NEW;
            END;
            $$ language 'plpgsql';
            "#
            .to_string(),
        ))
        .await?;

        db.execute(Statement::from_string(
            DbBackend::Postgres,
            "CREATE TRIGGER update_links_updated_at BEFORE UPDATE ON links \
             FOR EACH ROW EXECUTE FUNCTION update_updated_at_column()"
                .to_string(),
        ))
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute(Statement::from_string(
            DbBackend::Postgres,
            "DROP TRIGGER IF EXISTS update_links_updated_at ON links".to_string(),
        ))
        .await?;

        db.execute(Statement::from_string(
            DbBackend::Postgres,
            "DROP FUNCTION IF EXISTS update_updated_at_column()".to_string(),
        ))
        .await?;

        manager
            .drop_table(Table::drop().table(Links::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Links {
    Table,
    Id,
    Slug,
    TargetUrl,
    OwnerId,
    CreatedAt,
    UpdatedAt,
}
