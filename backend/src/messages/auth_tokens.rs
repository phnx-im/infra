use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    crypto::{
        signatures::keys::LeafSignatureKey,
        signatures::signable::{Signable, Signature, SignedStruct, Verifiable, VerifiedStruct},
    },
    ds::group_state::UserKeyHash,
};

use mls_assist::{GroupId, LeafNodeIndex};

/// Wrapper struct to implement tls codec functions.
#[derive(Clone, Serialize, Deserialize)]
pub struct Timestamp {
    time: DateTime<Utc>,
}

/// Types of authentication an endpoint can require from a client.
#[derive(Clone, Serialize, Deserialize)]
#[repr(u8)]
pub enum DsSenderId {
    LeafIndex(LeafNodeIndex),
    // TODO: This is a preliminary change. We should discuss this on the
    // mls-assist level.
    LeafSignatureKey(LeafSignatureKey),
    UserKeyHash(UserKeyHash),
}
