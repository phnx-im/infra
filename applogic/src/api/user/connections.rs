// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::Result;
use phnxtypes::identifiers::{SafeTryInto, UserName};

use crate::api::types::{UiContact, UiUserProfile};

use super::creation::User;

impl User {
    #[tokio::main(flavor = "current_thread")]
    pub async fn create_connection(&self, user_name: String) -> Result<()> {
        let mut user = self.user.lock().await;
        let conversation_id = user.add_contact(&user_name).await?;
        self.dispatch_conversation_notifications(vec![conversation_id])
            .await;
        Ok(())
    }

    pub async fn get_contacts(&self) -> Vec<UiContact> {
        let user = self.user.lock().await;
        user.contacts()
            .unwrap_or_default()
            .into_iter()
            .map(|c| c.into())
            .collect()
    }

    pub async fn contact(&self, user_name: String) -> Option<UiContact> {
        let user = self.user.lock().await;
        let user_name = <String as SafeTryInto<UserName>>::try_into(user_name).unwrap();
        user.contact(&user_name).map(|c| c.into())
    }

    /// Get the user profile of the user with the given [`UserName`].
    pub async fn user_profile(&self, user_name: String) -> Result<Option<UiUserProfile>> {
        let user = self.user.lock().await;
        let user_name = SafeTryInto::try_into(user_name)?;
        let user_profile = user.user_profile(&user_name)?.map(UiUserProfile::from);
        Ok(user_profile)
    }
}
