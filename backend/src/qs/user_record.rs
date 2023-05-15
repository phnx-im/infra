// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::{crypto::signatures::keys::OwnerVerifyingKey, messages::FriendshipToken};

use super::QsClientId;

#[derive(Debug, Clone)]
pub struct QsUserRecord {
    pub(crate) auth_key: OwnerVerifyingKey,
    pub(crate) friendship_token: FriendshipToken,
    pub(crate) clients: Vec<QsClientId>,
}

impl QsUserRecord {
    pub fn new(
        auth_key: OwnerVerifyingKey,
        friendship_token: FriendshipToken,
        initial_client: QsClientId,
    ) -> Self {
        Self {
            auth_key,
            friendship_token,
            clients: vec![initial_client],
        }
    }

    pub(crate) fn update(
        &mut self,
        auth_key: OwnerVerifyingKey,
        friendship_token: FriendshipToken,
    ) {
        self.auth_key = auth_key;
        self.friendship_token = friendship_token;
    }

    pub fn clients(&self) -> &[QsClientId] {
        self.clients.as_ref()
    }

    pub fn clients_mut(&mut self) -> &mut Vec<QsClientId> {
        &mut self.clients
    }

    pub fn friendship_token(&self) -> &FriendshipToken {
        &self.friendship_token
    }
}
