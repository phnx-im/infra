// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use opaque_ke::{
    CredentialFinalization, CredentialRequest, CredentialResponse, RegistrationRequest,
    RegistrationResponse, RegistrationUpload,
};
use tls_codec::Size;

use crate::crypto::{
    OPAQUE_LOGIN_FINISH_SIZE, OPAQUE_LOGIN_REQUEST_SIZE, OPAQUE_LOGIN_RESPONSE_SIZE,
    OPAQUE_REGISTRATION_RECORD_SIZE, OPAQUE_REGISTRATION_REQUEST_SIZE,
    OPAQUE_REGISTRATION_RESPONSE_SIZE,
};

use super::{
    OpaqueLoginFinish, OpaqueLoginRequest, OpaqueLoginResponse, OpaqueRegistrationRecord,
    OpaqueRegistrationRequest, OpaqueRegistrationResponse,
};

impl Size for OpaqueRegistrationRequest {
    fn tls_serialized_len(&self) -> usize {
        OPAQUE_REGISTRATION_REQUEST_SIZE
    }
}

impl tls_codec::Serialize for OpaqueRegistrationRequest {
    fn tls_serialize<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, tls_codec::Error> {
        let serialized_request = self.client_message.serialize().to_vec();
        let len = writer.write(&serialized_request)?;
        Ok(len)
    }
}

impl tls_codec::Deserialize for OpaqueRegistrationRequest {
    fn tls_deserialize<R: std::io::Read>(bytes: &mut R) -> Result<Self, tls_codec::Error>
    where
        Self: Sized,
    {
        let mut serialized_request = vec![0u8; OPAQUE_REGISTRATION_REQUEST_SIZE];
        bytes
            .read(&mut serialized_request)
            .map_err(|_| tls_codec::Error::InvalidInput)?;
        let client_message = RegistrationRequest::deserialize(&serialized_request)
            .map_err(|_| tls_codec::Error::InvalidInput)?;
        Ok(Self { client_message })
    }
}

impl Size for OpaqueRegistrationResponse {
    fn tls_serialized_len(&self) -> usize {
        OPAQUE_REGISTRATION_RESPONSE_SIZE
    }
}

impl tls_codec::Serialize for OpaqueRegistrationResponse {
    fn tls_serialize<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, tls_codec::Error> {
        let serialized_request = self.server_message.serialize().to_vec();
        let len = writer.write(&serialized_request)?;
        Ok(len)
    }
}

impl tls_codec::Deserialize for OpaqueRegistrationResponse {
    fn tls_deserialize<R: std::io::Read>(bytes: &mut R) -> Result<Self, tls_codec::Error>
    where
        Self: Sized,
    {
        let mut serialized_request = vec![0u8; OPAQUE_REGISTRATION_RESPONSE_SIZE];
        bytes
            .read(&mut serialized_request)
            .map_err(|_| tls_codec::Error::InvalidInput)?;
        let server_message = RegistrationResponse::deserialize(&serialized_request)
            .map_err(|_| tls_codec::Error::InvalidInput)?;
        Ok(Self { server_message })
    }
}

impl Size for OpaqueRegistrationRecord {
    fn tls_serialized_len(&self) -> usize {
        OPAQUE_REGISTRATION_RECORD_SIZE
    }
}

impl tls_codec::Serialize for OpaqueRegistrationRecord {
    fn tls_serialize<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, tls_codec::Error> {
        let serialized_request = self.client_message.serialize().to_vec();
        let len = writer.write(&serialized_request)?;
        Ok(len)
    }
}

impl tls_codec::Deserialize for OpaqueRegistrationRecord {
    fn tls_deserialize<R: std::io::Read>(bytes: &mut R) -> Result<Self, tls_codec::Error>
    where
        Self: Sized,
    {
        let mut serialized_request = vec![0u8; OPAQUE_REGISTRATION_RESPONSE_SIZE];
        bytes
            .read(&mut serialized_request)
            .map_err(|_| tls_codec::Error::InvalidInput)?;
        let client_message = RegistrationUpload::deserialize(&serialized_request)
            .map_err(|_| tls_codec::Error::InvalidInput)?;
        Ok(Self { client_message })
    }
}

impl Size for OpaqueLoginRequest {
    fn tls_serialized_len(&self) -> usize {
        OPAQUE_LOGIN_REQUEST_SIZE
    }
}

impl tls_codec::Serialize for OpaqueLoginRequest {
    fn tls_serialize<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, tls_codec::Error> {
        let serialized_request = self.client_message.serialize().to_vec();
        let len = writer.write(&serialized_request)?;
        Ok(len)
    }
}

impl tls_codec::Deserialize for OpaqueLoginRequest {
    fn tls_deserialize<R: std::io::Read>(bytes: &mut R) -> Result<Self, tls_codec::Error>
    where
        Self: Sized,
    {
        let mut serialized_request = vec![0u8; OPAQUE_REGISTRATION_RESPONSE_SIZE];
        bytes
            .read(&mut serialized_request)
            .map_err(|_| tls_codec::Error::InvalidInput)?;
        let client_message = CredentialRequest::deserialize(&serialized_request)
            .map_err(|_| tls_codec::Error::InvalidInput)?;
        Ok(Self { client_message })
    }
}

impl Size for OpaqueLoginResponse {
    fn tls_serialized_len(&self) -> usize {
        OPAQUE_LOGIN_RESPONSE_SIZE
    }
}

impl tls_codec::Serialize for OpaqueLoginResponse {
    fn tls_serialize<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, tls_codec::Error> {
        let serialized_request = self.server_message.serialize().to_vec();
        let len = writer.write(&serialized_request)?;
        Ok(len)
    }
}

impl tls_codec::Deserialize for OpaqueLoginResponse {
    fn tls_deserialize<R: std::io::Read>(bytes: &mut R) -> Result<Self, tls_codec::Error>
    where
        Self: Sized,
    {
        let mut serialized_request = vec![0u8; OPAQUE_REGISTRATION_RESPONSE_SIZE];
        bytes
            .read(&mut serialized_request)
            .map_err(|_| tls_codec::Error::InvalidInput)?;
        let server_message = CredentialResponse::deserialize(&serialized_request)
            .map_err(|_| tls_codec::Error::InvalidInput)?;
        Ok(Self { server_message })
    }
}

impl Size for OpaqueLoginFinish {
    fn tls_serialized_len(&self) -> usize {
        OPAQUE_LOGIN_FINISH_SIZE
    }
}

impl tls_codec::Serialize for OpaqueLoginFinish {
    fn tls_serialize<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, tls_codec::Error> {
        let serialized_request = self.client_message.serialize().to_vec();
        let len = writer.write(&serialized_request)?;
        Ok(len)
    }
}

impl tls_codec::Deserialize for OpaqueLoginFinish {
    fn tls_deserialize<R: std::io::Read>(bytes: &mut R) -> Result<Self, tls_codec::Error>
    where
        Self: Sized,
    {
        let mut serialized_request = vec![0u8; OPAQUE_REGISTRATION_RESPONSE_SIZE];
        bytes
            .read(&mut serialized_request)
            .map_err(|_| tls_codec::Error::InvalidInput)?;
        let client_message = CredentialFinalization::deserialize(&serialized_request)
            .map_err(|_| tls_codec::Error::InvalidInput)?;
        Ok(Self { client_message })
    }
}
