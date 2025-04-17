// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::{
    errors::DsProcessingError, messages::client_ds::UserProfileKeyUpdateParams, time::TimeStamp,
};

use super::group_state::DsGroupState;

impl DsGroupState {
    pub(super) fn update_user_profile_key(
        &mut self,
        params: UserProfileKeyUpdateParams,
    ) -> Result<(), DsProcessingError> {
        let client_profile = self
            .member_profiles
            .get_mut(&params.sender_index)
            .ok_or(DsProcessingError::UnknownSender)?;
        client_profile.encrypted_user_profile_key = params.user_profile_key;
        client_profile.activity_time = TimeStamp::now();
        Ok(())
    }
}
