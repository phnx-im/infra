use serde::{Deserialize, Serialize};
use tls_codec::{TlsDeserialize, TlsSerialize, TlsSize};
use utoipa::ToSchema;

pub mod client_ds;
pub mod client_qs;
pub(crate) mod intra_backend;

#[derive(Serialize, Deserialize, ToSchema, TlsSerialize, TlsDeserialize, TlsSize)]
pub struct FriendshipToken {}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct FriendshipEarKey {}
