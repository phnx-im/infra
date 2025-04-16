// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::{messages::client_ds::UserProfileKeyUpdateParams, time::TimeStamp};

use super::group_state::DsGroupState;

impl DsGroupState {
    pub(super) fn update_user_profile_key(&mut self, params: UserProfileKeyUpdateParams) {
        let client_profile = self
            .member_profiles
            .get_mut(&params.sender_index)
            .expect("Sender not found in group state");
        client_profile.encrypted_user_profile_key = params.user_profile_key;
        client_profile.activity_time = TimeStamp::now();
        client_profile.activity_epoch = params.epoch;
    }
}
