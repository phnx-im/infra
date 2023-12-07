// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::{crypto::signatures::keys::QsUserVerifyingKey, messages::FriendshipToken};

#[derive(Debug, Clone, PartialEq)]
pub struct QsUserRecord {
    pub(crate) verifying_key: QsUserVerifyingKey,
    pub(crate) friendship_token: FriendshipToken,
}

impl QsUserRecord {
    pub fn new(verifying_key: QsUserVerifyingKey, friendship_token: FriendshipToken) -> Self {
        Self {
            verifying_key,
            friendship_token,
        }
    }

    pub(crate) fn update(
        &mut self,
        verifying_key: QsUserVerifyingKey,
        friendship_token: FriendshipToken,
    ) {
        self.verifying_key = verifying_key;
        self.friendship_token = friendship_token;
    }

    pub fn friendship_token(&self) -> &FriendshipToken {
        &self.friendship_token
    }

    pub fn verifying_key(&self) -> &QsUserVerifyingKey {
        &self.verifying_key
    }
}
