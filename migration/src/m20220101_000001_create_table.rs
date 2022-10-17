use crate::bigint_primary_key;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Users::Table)
                    .if_not_exists()
                    .col(&mut bigint_primary_key(Users::Id))
                    .col(ColumnDef::new(Users::Name).string().not_null())
                    .col(
                        ColumnDef::new(Users::Email)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Users::PasswordHash).string().null())
                    .col(
                        ColumnDef::new(Users::PasswordResetToken)
                            .string()
                            .null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(Users::PasswordResetTokenCreation)
                            .date_time()
                            .null(),
                    )
                    .col(ColumnDef::new(Users::PhoneNumber).string().null())
                    .col(ColumnDef::new(Users::SmsNotifications).boolean().not_null())
                    .col(
                        ColumnDef::new(Users::EmailNotifications)
                            .boolean()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Sessions::Table)
                    .if_not_exists()
                    .col(&mut bigint_primary_key(Sessions::Id))
                    .col(ColumnDef::new(Sessions::UserId).big_integer().not_null())
                    .col(
                        ColumnDef::new(Sessions::Token)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Sessions::Created).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(Sessions::LastUsed).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(Sessions::LastIp).string().not_null())
                    .col(
                        ColumnDef::new(Sessions::UserAgent)
                            .string_len(512)
                            .not_null(),
                    )
                    .foreign_key(
                        &mut ForeignKeyCreateStatement::new()
                            .from(Sessions::Table, Sessions::UserId)
                            .to(Users::Table, Users::Id)
                            .to_owned(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Tests::Table)
                    .if_not_exists()
                    .col(&mut bigint_primary_key(Tests::Id))
                    .col(ColumnDef::new(Tests::Enabled).boolean().not_null())
                    .col(ColumnDef::new(Tests::Config).json_binary().not_null())
                    .col(ColumnDef::new(Tests::Failing).boolean().not_null())
                    .col(
                        ColumnDef::new(Tests::FailureThreshold)
                            .small_unsigned()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(TestResults::Table)
                    .if_not_exists()
                    .col(&mut bigint_primary_key(TestResults::Id))
                    .col(ColumnDef::new(TestResults::TestId).big_integer().not_null())
                    .col(ColumnDef::new(TestResults::Failure).string().null())
                    .col(
                        ColumnDef::new(TestResults::TimeStarted)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(TestResults::TimeFinished)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        &mut ForeignKeyCreateStatement::new()
                            .from(TestResults::Table, TestResults::TestId)
                            .to(Tests::Table, Tests::Id)
                            .to_owned(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(RunnerLogs::Table)
                    .if_not_exists()
                    .col(&mut bigint_primary_key(RunnerLogs::Id))
                    .col(
                        ColumnDef::new(RunnerLogs::TimeStarted)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RunnerLogs::TimeFinished)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(RunnerLogs::Failure).string().null())
                    .col(ColumnDef::new(RunnerLogs::TestsPassed).unsigned().null())
                    .col(ColumnDef::new(RunnerLogs::TestsFailed).unsigned().null())
                    .col(ColumnDef::new(RunnerLogs::TestsSkipped).unsigned().null())
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(RunnerLogs::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(TestResults::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Tests::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Sessions::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Users::Table).to_owned())
            .await?;
        Ok(())
    }
}

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
enum Users {
    Table,
    Id,
    Name,
    Email,
    PasswordHash,
    PasswordResetToken,
    PasswordResetTokenCreation,
    PhoneNumber,
    SmsNotifications,
    EmailNotifications,
}

#[derive(Iden)]
enum Sessions {
    Table,
    Id,
    UserId,
    Token,
    Created,
    LastUsed,
    LastIp,
    UserAgent,
}

#[derive(Iden)]
enum Tests {
    Table,
    Id,
    Enabled,
    Config,
    Failing,
    FailureThreshold,
}

#[derive(Iden)]
enum TestResults {
    Table,
    Id,
    TestId,
    Failure,
    TimeStarted,
    TimeFinished,
}

#[derive(Iden)]
enum RunnerLogs {
    Table,
    Id,
    TimeStarted,
    TimeFinished,
    Failure,
    TestsPassed,
    TestsFailed,
    TestsSkipped,
}
