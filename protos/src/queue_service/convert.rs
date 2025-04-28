// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::{
    crypto::{ear, signatures},
    identifiers,
    messages::{self, push_token},
};
use uuid::Uuid;

use crate::{
    common::convert::InvalidNonceLen,
    validation::{MissingFieldError, MissingFieldExt},
};

use super::v1::{
    EncryptedPushToken, FriendshipToken, QsClientId, QsClientVerifyingKey, QsUserId,
    QsUserVerifyingKey,
};

impl From<identifiers::QsUserId> for QsUserId {
    fn from(value: identifiers::QsUserId) -> Self {
        let uuid = *value.as_uuid();
        Self {
            value: Some(uuid.into()),
        }
    }
}

impl TryFrom<QsUserId> for identifiers::QsUserId {
    type Error = MissingFieldError<&'static str>;

    fn try_from(proto: QsUserId) -> Result<Self, Self::Error> {
        Ok(identifiers::QsUserId::from(Uuid::from(
            proto.value.ok_or_missing_field("uuid")?,
        )))
    }
}

impl From<identifiers::QsClientId> for QsClientId {
    fn from(value: identifiers::QsClientId) -> Self {
        let uuid = *value.as_uuid();
        Self {
            value: Some(uuid.into()),
        }
    }
}

impl TryFrom<QsClientId> for identifiers::QsClientId {
    type Error = MissingFieldError<&'static str>;

    fn try_from(proto: QsClientId) -> Result<Self, Self::Error> {
        Ok(Self::from(Uuid::from(
            proto.value.ok_or_missing_field("uuid")?,
        )))
    }
}

impl From<QsUserVerifyingKey> for signatures::keys::QsUserVerifyingKey {
    fn from(proto: QsUserVerifyingKey) -> Self {
        Self::from_bytes(proto.bytes)
    }
}

impl From<signatures::keys::QsUserVerifyingKey> for QsUserVerifyingKey {
    fn from(value: signatures::keys::QsUserVerifyingKey) -> Self {
        Self {
            bytes: value.into_bytes(),
        }
    }
}

impl From<FriendshipToken> for messages::FriendshipToken {
    fn from(proto: FriendshipToken) -> Self {
        Self::from_bytes(proto.bytes)
    }
}

impl From<messages::FriendshipToken> for FriendshipToken {
    fn from(value: messages::FriendshipToken) -> Self {
        Self {
            bytes: value.into_bytes(),
        }
    }
}

impl From<QsClientVerifyingKey> for signatures::keys::QsClientVerifyingKey {
    fn from(proto: QsClientVerifyingKey) -> Self {
        Self::from_bytes(proto.bytes)
    }
}

impl From<signatures::keys::QsClientVerifyingKey> for QsClientVerifyingKey {
    fn from(value: signatures::keys::QsClientVerifyingKey) -> Self {
        Self {
            bytes: value.into_bytes(),
        }
    }
}

impl TryFrom<EncryptedPushToken> for push_token::EncryptedPushToken {
    type Error = InvalidNonceLen;

    fn try_from(proto: EncryptedPushToken) -> Result<Self, Self::Error> {
        let ciphertext: ear::Ciphertext = proto.ciphertext.unwrap_or_default().try_into()?;
        Ok(Self::from(ciphertext))
    }
}

impl From<push_token::EncryptedPushToken> for EncryptedPushToken {
    fn from(value: push_token::EncryptedPushToken) -> Self {
        Self {
            ciphertext: Some(value.into_ciphertext().into()),
        }
    }
}
