// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Misc. functions

use phnxtypes::identifiers::AsClientId;
use uuid::Uuid;

pub async fn delete_databases(db_path: String) -> anyhow::Result<()> {
    phnxcoreclient::delete_databases(&db_path).await
}

pub async fn delete_client_database(
    db_path: String,
    user_name: String,
    client_id: Uuid,
) -> anyhow::Result<()> {
    let user_name = user_name.parse()?;
    let as_client_id = AsClientId::new(user_name, client_id);
    phnxcoreclient::delete_client_database(&db_path, &as_client_id).await
}
