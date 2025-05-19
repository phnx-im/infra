// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Misc. functions

use super::types::UiUserId;

pub async fn delete_databases(db_path: String) -> anyhow::Result<()> {
    phnxcoreclient::delete_databases(&db_path).await
}

pub async fn delete_client_database(db_path: String, client_id: UiUserId) -> anyhow::Result<()> {
    phnxcoreclient::delete_client_database(&db_path, &client_id.into()).await
}
