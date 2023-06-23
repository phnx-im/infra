// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use opaque_ke::{
    CredentialFinalization, CredentialRequest, CredentialResponse, RegistrationRequest,
    RegistrationResponse, RegistrationUpload,
};
use tls_codec::{DeserializeBytes, Size};

use crate::{
    auth_service::client_api::client,
    crypto::{
        OpaqueCiphersuite, OPAQUE_LOGIN_FINISH_SIZE, OPAQUE_LOGIN_REQUEST_SIZE,
        OPAQUE_LOGIN_RESPONSE_SIZE, OPAQUE_REGISTRATION_RECORD_SIZE,
        OPAQUE_REGISTRATION_REQUEST_SIZE, OPAQUE_REGISTRATION_RESPONSE_SIZE,
    },
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

impl tls_codec::DeserializeBytes for OpaqueRegistrationRequest {
    fn tls_deserialize(bytes: &[u8]) -> Result<(Self, &[u8]), tls_codec::Error>
    where
        Self: Sized,
    {
        let serialized_request = bytes
            .get(..OPAQUE_REGISTRATION_REQUEST_SIZE)
            .ok_or(tls_codec::Error::EndOfStream)?;

        let client_message = RegistrationRequest::deserialize(serialized_request)
            .map_err(|_| tls_codec::Error::InvalidInput)?;
        let remainder = bytes
            .get(OPAQUE_REGISTRATION_REQUEST_SIZE..)
            .ok_or(tls_codec::Error::EndOfStream)?;
        Ok((Self { client_message }, remainder))
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

impl tls_codec::DeserializeBytes for OpaqueRegistrationResponse {
    fn tls_deserialize(bytes: &[u8]) -> Result<(Self, &[u8]), tls_codec::Error>
    where
        Self: Sized,
    {
        let serialized_request = bytes
            .get(..OPAQUE_REGISTRATION_RESPONSE_SIZE)
            .ok_or(tls_codec::Error::EndOfStream)?;
        let server_message = RegistrationResponse::deserialize(serialized_request)
            .map_err(|_| tls_codec::Error::InvalidInput)?;
        let remainder = bytes
            .get(OPAQUE_REGISTRATION_RESPONSE_SIZE..)
            .ok_or(tls_codec::Error::EndOfStream)?;
        Ok((Self { server_message }, remainder))
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

impl tls_codec::DeserializeBytes for OpaqueRegistrationRecord {
    fn tls_deserialize(bytes: &[u8]) -> Result<(Self, &[u8]), tls_codec::Error>
    where
        Self: Sized,
    {
        let serialized_request = bytes
            .get(..OPAQUE_REGISTRATION_RECORD_SIZE)
            .ok_or(tls_codec::Error::EndOfStream)?;
        let client_message = RegistrationUpload::deserialize(serialized_request)
            .map_err(|_| tls_codec::Error::InvalidInput)?;
        let remainder = bytes
            .get(OPAQUE_REGISTRATION_RECORD_SIZE..)
            .ok_or(tls_codec::Error::EndOfStream)?;
        Ok((Self { client_message }, remainder))
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

impl tls_codec::DeserializeBytes for OpaqueLoginRequest {
    fn tls_deserialize(bytes: &[u8]) -> Result<(Self, &[u8]), tls_codec::Error>
    where
        Self: Sized,
    {
        let serialized_request = bytes
            .get(..OPAQUE_LOGIN_REQUEST_SIZE)
            .ok_or(tls_codec::Error::EndOfStream)?;
        let client_message = CredentialRequest::deserialize(serialized_request)
            .map_err(|_| tls_codec::Error::InvalidInput)?;
        let remainder = bytes
            .get(OPAQUE_LOGIN_REQUEST_SIZE..)
            .ok_or(tls_codec::Error::EndOfStream)?;
        Ok((Self { client_message }, remainder))
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

impl tls_codec::DeserializeBytes for OpaqueLoginResponse {
    fn tls_deserialize(bytes: &[u8]) -> Result<(Self, &[u8]), tls_codec::Error>
    where
        Self: Sized,
    {
        let serialized_request = bytes
            .get(..OPAQUE_LOGIN_RESPONSE_SIZE)
            .ok_or(tls_codec::Error::EndOfStream)?;
        let server_message = CredentialResponse::deserialize(serialized_request)
            .map_err(|_| tls_codec::Error::InvalidInput)?;
        let remainder = bytes
            .get(OPAQUE_LOGIN_RESPONSE_SIZE..)
            .ok_or(tls_codec::Error::EndOfStream)?;
        Ok((Self { server_message }, remainder))
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

impl tls_codec::DeserializeBytes for OpaqueLoginFinish {
    fn tls_deserialize(bytes: &[u8]) -> Result<(Self, &[u8]), tls_codec::Error>
    where
        Self: Sized,
    {
        let serialized_request = bytes
            .get(..OPAQUE_LOGIN_FINISH_SIZE)
            .ok_or(tls_codec::Error::EndOfStream)?;
        let client_message = CredentialFinalization::deserialize(serialized_request)
            .map_err(|_| tls_codec::Error::InvalidInput)?;
        let remainder = bytes
            .get(OPAQUE_LOGIN_FINISH_SIZE..)
            .ok_or(tls_codec::Error::EndOfStream)?;
        Ok((Self { client_message }, remainder))
    }
}

#[test]
fn test_opaque_codec() {
    use tls_codec::Serialize;

    use opaque_ke::*;
    use rand::rngs::OsRng;

    let mut rng = OsRng;
    let server_setup = ServerSetup::<OpaqueCiphersuite>::new(&mut rng);

    let mut client_rng = OsRng;
    let client_registration_start_result =
        ClientRegistration::<OpaqueCiphersuite>::start(&mut client_rng, b"password").unwrap();

    let server_registration_start_result = ServerRegistration::<OpaqueCiphersuite>::start(
        &server_setup,
        client_registration_start_result.message,
        b"alice@example.com",
    )
    .unwrap();

    let client_registration_finish_result = client_registration_start_result
        .state
        .finish(
            &mut client_rng,
            b"password",
            server_registration_start_result.message,
            ClientRegistrationFinishParameters::default(),
        )
        .unwrap();

    let opaque_registration_record = OpaqueRegistrationRecord {
        client_message: client_registration_finish_result.message,
    };

    println!(
        "opaque_registration_record: {:?}",
        opaque_registration_record
    );

    let bytes = opaque_registration_record.tls_serialize_detached().unwrap();

    assert_eq!(bytes.len(), OPAQUE_REGISTRATION_RECORD_SIZE);
    let _ = OpaqueRegistrationRecord::tls_deserialize(bytes.as_slice()).unwrap();
}
