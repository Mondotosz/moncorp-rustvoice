use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m00001_create_guilds"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Guilds::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Guilds::Id)
                            .big_integer()
                            .not_null()
                            .primary_key(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Guilds::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum Guilds {
    Table,
    Id,
}
