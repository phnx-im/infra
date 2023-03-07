use mls_assist::SignaturePublicKey;

use crate::messages::FriendshipToken;

use super::ClientId;

pub struct QsUserRecord {
    auth_key: SignaturePublicKey,
    friendship_token: FriendshipToken,
    client_ids: Vec<ClientId>,
}

impl QsUserRecord {
    pub fn new(
        auth_key: SignaturePublicKey,
        friendship_token: FriendshipToken,
        client_id: ClientId,
    ) -> Self {
        Self {
            auth_key,
            friendship_token,
            client_ids: vec![client_id],
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
