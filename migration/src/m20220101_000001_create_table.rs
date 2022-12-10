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
                    .table(User::Table)
                    .if_not_exists()
                    .col(&mut bigint_primary_key(User::Id))
                    .col(ColumnDef::new(User::Name).string().not_null())
                    .col(ColumnDef::new(User::Email).string().not_null().unique_key())
                    .col(ColumnDef::new(User::PasswordHash).string().null())
                    .col(
                        ColumnDef::new(User::PasswordResetToken)
                            .string()
                            .null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(User::PasswordResetTokenCreation)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(ColumnDef::new(User::PhoneNumber).string().null())
                    .col(ColumnDef::new(User::SmsNotifications).boolean().not_null())
                    .col(
                        ColumnDef::new(User::EmailNotifications)
                            .boolean()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Session::Table)
                    .if_not_exists()
                    .col(&mut bigint_primary_key(Session::Id))
                    .col(ColumnDef::new(Session::UserId).big_integer().not_null())
                    .col(
                        ColumnDef::new(Session::Token)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(Session::Created)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Session::LastUsed)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Session::LastIp).string().not_null())
                    .col(
                        ColumnDef::new(Session::UserAgent)
                            .string_len(512)
                            .not_null(),
                    )
                    .foreign_key(
                        &mut ForeignKeyCreateStatement::new()
                            .from(Session::Table, Session::UserId)
                            .to(User::Table, User::Id)
                            .to_owned(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Test::Table)
                    .if_not_exists()
                    .col(&mut bigint_primary_key(Test::Id))
                    .col(ColumnDef::new(Test::Name).string().not_null().unique_key())
                    .col(ColumnDef::new(Test::Enabled).boolean().not_null())
                    .col(ColumnDef::new(Test::Config).json_binary().not_null())
                    .col(ColumnDef::new(Test::Failing).boolean().not_null())
                    .col(
                        ColumnDef::new(Test::FailureThreshold)
                            .small_unsigned()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(TestResult::Table)
                    .if_not_exists()
                    .col(&mut bigint_primary_key(TestResult::Id))
                    .col(ColumnDef::new(TestResult::TestId).big_integer().not_null())
                    .col(ColumnDef::new(TestResult::Failure).string().null())
                    .col(
                        ColumnDef::new(TestResult::TimeStarted)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(TestResult::TimeFinished)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        &mut ForeignKeyCreateStatement::new()
                            .from(TestResult::Table, TestResult::TestId)
                            .to(Test::Table, Test::Id)
                            .to_owned(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(RunnerLog::Table)
                    .if_not_exists()
                    .col(&mut bigint_primary_key(RunnerLog::Id))
                    .col(
                        ColumnDef::new(RunnerLog::TimeStarted)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RunnerLog::TimeFinished)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(RunnerLog::Failure).string().null())
                    .col(ColumnDef::new(RunnerLog::TestsPassed).unsigned().null())
                    .col(ColumnDef::new(RunnerLog::TestsFailed).unsigned().null())
                    .col(ColumnDef::new(RunnerLog::TestsSkipped).unsigned().null())
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(RunnerLog::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(TestResult::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Test::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Session::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(User::Table).to_owned())
            .await?;
        Ok(())
    }
}

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
enum User {
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
enum Session {
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
enum Test {
    Table,
    Id,
    Name,
    Enabled,
    Config,
    Failing,
    FailureThreshold,
}

#[derive(Iden)]
enum TestResult {
    Table,
    Id,
    TestId,
    Failure,
    TimeStarted,
    TimeFinished,
}

#[derive(Iden)]
enum RunnerLog {
    Table,
    Id,
    TimeStarted,
    TimeFinished,
    Failure,
    TestsPassed,
    TestsFailed,
    TestsSkipped,
}
