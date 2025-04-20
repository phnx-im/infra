// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::{crypto::opaque, identifiers};
use tls_codec::{DeserializeBytes, Serialize};
use tonic::Status;

use crate::{
    common::convert::QualifiedUserNameError,
    convert::{RefInto, TryFromRef},
    validation::{MissingFieldError, MissingFieldExt},
};

use super::v1::{AsClientId, OpaqueLoginRequest, OpaqueLoginResponse};

impl From<identifiers::AsClientId> for AsClientId {
    fn from(value: identifiers::AsClientId) -> Self {
        AsClientId {
            user_name: Some(value.user_name().ref_into()),
            client_id: Some(value.client_id().into()),
        }
    }
}

impl TryFrom<AsClientId> for identifiers::AsClientId {
    type Error = AsClientIdError;

    fn try_from(proto: AsClientId) -> Result<Self, Self::Error> {
        Ok(identifiers::AsClientId::new(
            proto
                .user_name
                .ok_or_missing_field(AsClientIdField::UserName)?
                .try_into()?,
            proto
                .client_id
                .ok_or_missing_field(AsClientIdField::ClientId)?
                .into(),
        ))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AsClientIdError {
    #[error(transparent)]
    MissingField(#[from] MissingFieldError<AsClientIdField>),
    #[error(transparent)]
    QualifiedUserNameError(#[from] QualifiedUserNameError),
}

#[derive(Debug, derive_more::Display)]
pub enum AsClientIdField {
    #[display(fmt = "user_name")]
    UserName,
    #[display(fmt = "client_id")]
    ClientId,
}

impl From<AsClientIdError> for Status {
    fn from(error: AsClientIdError) -> Self {
        Status::invalid_argument(format!("invalid client id: {error}"))
    }
}

impl TryFromRef<'_, opaque::OpaqueLoginRequest> for OpaqueLoginRequest {
    type Error = tls_codec::Error;

    fn try_from_ref(value: &opaque::OpaqueLoginRequest) -> Result<Self, Self::Error> {
        Ok(Self {
            tls: value.tls_serialize_detached()?,
        })
    }
}

impl TryFromRef<'_, OpaqueLoginRequest> for opaque::OpaqueLoginRequest {
    type Error = tls_codec::Error;

    fn try_from_ref(proto: &OpaqueLoginRequest) -> Result<Self, Self::Error> {
        DeserializeBytes::tls_deserialize_exact_bytes(&proto.tls)
    }
}

impl TryFromRef<'_, opaque::OpaqueLoginResponse> for OpaqueLoginResponse {
    type Error = tls_codec::Error;

    fn try_from_ref(value: &opaque::OpaqueLoginResponse) -> Result<Self, Self::Error> {
        Ok(Self {
            tls: value.tls_serialize_detached()?,
        })
    }
}

impl TryFromRef<'_, OpaqueLoginResponse> for opaque::OpaqueLoginResponse {
    type Error = tls_codec::Error;

    fn try_from_ref(proto: &OpaqueLoginResponse) -> Result<Self, Self::Error> {
        DeserializeBytes::tls_deserialize_exact_bytes(&proto.tls)
    }
}
