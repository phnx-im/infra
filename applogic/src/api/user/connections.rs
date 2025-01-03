// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::Result;
use phnxtypes::identifiers::{QualifiedUserName, SafeTryInto};

use crate::api::types::{UiContact, UiUserProfile};

use super::User;

impl User {
    pub async fn get_contacts(&self) -> Vec<UiContact> {
        self.user
            .contacts()
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|c| c.into())
            .collect()
    }

    pub async fn contact(&self, user_name: String) -> Option<UiContact> {
        let user_name = <String as SafeTryInto<QualifiedUserName>>::try_into(user_name).unwrap();
        self.user.contact(&user_name).await.map(|c| c.into())
    }

    /// Get the user profile of the user with the given [`QualifiedUserName`].
    pub async fn user_profile(&self, user_name: String) -> Result<Option<UiUserProfile>> {
        let user_name = SafeTryInto::try_into(user_name)?;
        let user_profile = self
            .user
            .user_profile(&user_name)
            .await?
            .map(|profile| UiUserProfile::from_profile(&profile));
        Ok(user_profile)
    }
}
