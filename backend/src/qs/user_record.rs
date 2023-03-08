use mls_assist::SignaturePublicKey;

use crate::messages::FriendshipToken;

pub struct QsUserRecord {
    auth_key: SignaturePublicKey,
    friendship_token: FriendshipToken,
}

impl QsUserRecord {
    pub fn new(auth_key: SignaturePublicKey, friendship_token: FriendshipToken) -> Self {
        Self {
            auth_key,
            friendship_token,
        }
    }

    pub(crate) fn update(
        &mut self,
        auth_key: SignaturePublicKey,
        friendship_token: FriendshipToken,
    ) {
        self.auth_key = auth_key;
        self.friendship_token = friendship_token;
    }
}
