use crate::{crypto::signatures::keys::OwnerVerifyingKey, messages::FriendshipToken};

pub struct QsUserRecord {
    pub(crate) auth_key: OwnerVerifyingKey,
    pub(crate) friendship_token: FriendshipToken,
}

impl QsUserRecord {
    pub fn new(auth_key: OwnerVerifyingKey, friendship_token: FriendshipToken) -> Self {
        Self {
            auth_key,
            friendship_token,
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
}
