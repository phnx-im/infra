// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Deserialize, Serialize};
use tls_codec::{TlsDeserializeBytes, TlsSerialize, TlsSize};

use super::private_keys::{SigningKey, VerifyingKey};

#[derive(Debug)]
pub struct LeafVerifyingKeyType;
pub type LeafVerifyingKey = VerifyingKey<LeafVerifyingKeyType>;

#[derive(
    Clone, PartialEq, Eq, Serialize, Deserialize, Debug, TlsSerialize, TlsDeserializeBytes, TlsSize,
)]
pub struct QsClientVerifyingKeyType;
pub type QsClientVerifyingKey = VerifyingKey<QsClientVerifyingKeyType>;

pub type QsClientSigningKey = SigningKey<QsClientVerifyingKeyType>;

#[derive(
    Clone, PartialEq, Serialize, Deserialize, Debug, TlsSerialize, TlsDeserializeBytes, TlsSize,
)]
pub struct QsUserVerifyingKeyType;
pub type QsUserVerifyingKey = VerifyingKey<QsUserVerifyingKeyType>;

pub type QsUserSigningKey = SigningKey<QsUserVerifyingKeyType>;

#[cfg(test)]
mod test {
    use crate::codec::PhnxCodec;

    use super::*;

    #[test]
    fn qs_client_verifying_key_serde_codec() {
        let key = QsClientVerifyingKey::new_for_test(vec![1, 2, 3]);
        insta::assert_binary_snapshot!(".cbor", PhnxCodec::to_vec(&key).unwrap());
    }

    #[test]
    fn qs_client_verifying_key_serde_json() {
        let key = QsClientVerifyingKey::new_for_test(vec![1, 2, 3]);
        insta::assert_json_snapshot!(key);
    }
}
