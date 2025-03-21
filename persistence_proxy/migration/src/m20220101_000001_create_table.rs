use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Consumer::Table)
                    .if_not_exists()
                    .col(pk_auto(Consumer::Id))
                    .col(string(Consumer::Name))
                    .col(string(Consumer::TokenHash))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Log::Table)
                    .if_not_exists()
                    .col(pk_auto(Log::Id))
                    .col(integer(Log::ConsumerId))
                    .col(json(Log::Request))
                    .col(json(Log::Response))
                    .foreign_key(
                        ForeignKey::create()
                            .from_tbl(Log::Table)
                            .from_col(Log::ConsumerId)
                            .to_tbl(Consumer::Table)
                            .to_col(Consumer::Id),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Consumer::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Log::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Consumer {
    Table,
    Id,
    Name,
    TokenHash,
}

#[derive(DeriveIden)]
enum Log {
    Table,
    Id,
    ConsumerId,
    Request,
    Response,
}
