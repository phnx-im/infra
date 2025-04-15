// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::{
    errors::auth_service::{GetUserProfileError, UpdateUserProfileError},
    messages::client_as_out::{
        GetUserProfileParams, GetUserProfileResponse, UpdateUserProfileParamsTbs,
    },
};

use crate::auth_service::{AuthService, user_record::UserRecord};

impl AuthService {
    pub(crate) async fn as_get_user_profile(
        &self,
        params: GetUserProfileParams,
    ) -> Result<GetUserProfileResponse, GetUserProfileError> {
        let GetUserProfileParams { client_id } = params;

        let user_record = UserRecord::load(&self.db_pool, client_id.user_name())
            .await
            .map_err(|e| {
                tracing::error!("Error loading user record: {:?}", e);
                GetUserProfileError::StorageError
            })?
            .ok_or(GetUserProfileError::UserNotFound)?;

        let user_profile = user_record.into_encrypted_user_profile();

        let response = GetUserProfileResponse {
            encrypted_user_profile: user_profile,
        };

        Ok(response)
    }

    pub(crate) async fn as_update_user_profile(
        &self,
        params: UpdateUserProfileParamsTbs,
    ) -> Result<(), UpdateUserProfileError> {
        let UpdateUserProfileParamsTbs {
            client_id,
            user_profile,
        } = params;

        let mut user_record = UserRecord::load(&self.db_pool, client_id.user_name())
            .await
            .map_err(|e| {
                tracing::error!("Error loading user record: {:?}", e);
                UpdateUserProfileError::StorageError
            })?
            .ok_or(UpdateUserProfileError::UserNotFound)?;

        user_record.set_user_profile(user_profile);

        user_record.update(&self.db_pool).await.map_err(|e| {
            tracing::error!("Error updating user record: {:?}", e);
            UpdateUserProfileError::StorageError
        })?;

        Ok(())
    }
}
