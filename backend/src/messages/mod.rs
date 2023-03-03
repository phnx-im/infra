use mls_assist::KeyPackage;
use serde::{Deserialize, Serialize};
use tls_codec::{TlsDeserialize, TlsSerialize, TlsSize};
use utoipa::ToSchema;

pub mod client_backend;
pub(crate) mod intra_backend;

#[derive(Serialize, Deserialize, ToSchema, TlsSerialize, TlsDeserialize, TlsSize)]
pub struct FriendshipToken {}

#[derive(Serialize, Deserialize, ToSchema, TlsSerialize, TlsDeserialize, TlsSize)]
pub struct QsUid {}

#[derive(Serialize, Deserialize, ToSchema, TlsSerialize, TlsDeserialize, TlsSize)]
pub struct QsCid {}

#[derive(Debug, Serialize, Deserialize, ToSchema, TlsSerialize, TlsDeserialize, TlsSize)]
pub struct AddPackage {
    key_package: KeyPackage,
    icc_ciphertext: Vec<u8>,
}
