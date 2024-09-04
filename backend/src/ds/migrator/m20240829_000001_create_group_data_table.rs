// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use sea_orm_migration::prelude::*;

pub(super) struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20240829_000001_create_group_data_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(EncryptedGroupData::Table)
                    .col(
                        ColumnDef::new(EncryptedGroupData::GroupId)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(EncryptedGroupData::EncryptedGroupState)
                            .blob()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(EncryptedGroupData::LastUsed)
                            .timestamp()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(EncryptedGroupData::DeletedQueues)
                            .blob()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await
    }

    // Define how to rollback this migration: Drop the Bakery table.
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(EncryptedGroupData::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum EncryptedGroupData {
    Table,
    GroupId,
    EncryptedGroupState,
    LastUsed,
    DeletedQueues,
}
