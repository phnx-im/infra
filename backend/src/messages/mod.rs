use serde::{Deserialize, Serialize};
use tls_codec::{TlsDeserialize, TlsSerialize, TlsSize};
use utoipa::ToSchema;

pub mod client_ds;
pub mod client_qs;
pub(crate) mod intra_backend;

#[derive(Serialize, Deserialize, ToSchema, TlsSerialize, TlsDeserialize, TlsSize)]
pub struct FriendshipToken {}

/// Enum encoding the version of the MlsInfra protocol that was used to create
/// the given message.
#[derive(TlsSerialize, TlsDeserialize, TlsSize)]
#[repr(u8)]
pub(crate) enum MlsInfraVersion {
    Alpha,
}
