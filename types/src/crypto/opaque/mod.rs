// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use argon2::Argon2;
use opaque_ke::{
    CipherSuite, CredentialFinalization, CredentialRequest, CredentialResponse,
    RegistrationRequest, RegistrationResponse, RegistrationUpload,
};
use serde::{Deserialize, Serialize};

mod codec;

/// Default ciphersuite we use for OPAQUE
pub struct OpaqueCiphersuite;

impl CipherSuite for OpaqueCiphersuite {
    type OprfCs = opaque_ke::Ristretto255;
    type KeGroup = opaque_ke::Ristretto255;
    type KeyExchange = opaque_ke::key_exchange::tripledh::TripleDh;

    type Ksf = Argon2<'static>;
}

// The OPAQUE ciphersuite's Noe: The size of a serialized OPRF group element output from SerializeElement.
const OPAQUE_NOE: usize = 32;
// The OPAQUE ciphersuite's Nok: The size of an OPRF private key as output from DeriveKeyPair.
const OPAQUE_NOK: usize = 32;
// The OPAQUE ciphersuite's Nn: Nonce length.
const OPAQUE_NN: usize = 32;
// The OPAQUE ciphersuite's Nm: MAC length.
const OPAQUE_NM: usize = 64;
// The OPAQUE ciphersuite's Nh: Hash length.
const OPAQUE_NH: usize = 64;
// The OPAQUE ciphersuite's Npk: Public key length.
const OPAQUE_NPK: usize = 32;

// The size of an OPAQUE envelope (Nn + nM)
const OPAQUE_ENVELOPE_SIZE: usize = OPAQUE_NN + OPAQUE_NM;
const OPAQUE_CREDENTIAL_REQUEST_SIZE: usize = OPAQUE_NOE;
const OPAQUE_CREDENTIAL_RESPONSE_SIZE: usize =
    OPAQUE_NOE + OPAQUE_NN + OPAQUE_NPK + OPAQUE_NN + OPAQUE_NM;
const OPAQUE_AUTH_REQUEST_SIZE: usize = OPAQUE_NN + OPAQUE_NPK;
const OPAQUE_AUTH_RESPONSE_SIZE: usize = OPAQUE_NN + OPAQUE_NPK + OPAQUE_NM;

// The size of the blinded message, i.e. a serialized OPRF group element using the
// ciphersuite defined above.
pub(crate) const OPAQUE_REGISTRATION_REQUEST_SIZE: usize = OPAQUE_NOE;
// The size of the evaluated message, i.e. a serialized OPRF group element, plus that of the server public key using the
// ciphersuite defined above.
pub(crate) const OPAQUE_REGISTRATION_RESPONSE_SIZE: usize = OPAQUE_NOE + OPAQUE_NOK;
// The size of the client upload after successful registration: The client public key, as well as a masking key and an envelope.
pub(crate) const OPAQUE_REGISTRATION_RECORD_SIZE: usize =
    OPAQUE_NPK + OPAQUE_NH + OPAQUE_ENVELOPE_SIZE;

// The size of the KE1 struct
pub(crate) const OPAQUE_LOGIN_REQUEST_SIZE: usize =
    OPAQUE_CREDENTIAL_REQUEST_SIZE + OPAQUE_AUTH_REQUEST_SIZE;
// The size of the KE2 struct
pub(crate) const OPAQUE_LOGIN_RESPONSE_SIZE: usize =
    OPAQUE_CREDENTIAL_RESPONSE_SIZE + OPAQUE_AUTH_RESPONSE_SIZE;
// The size of the KE3 struct
pub(crate) const OPAQUE_LOGIN_FINISH_SIZE: usize = OPAQUE_NM;

#[derive(Debug, Serialize, Deserialize)]
pub struct OpaqueLoginRequest {
    pub client_message: CredentialRequest<OpaqueCiphersuite>,
}

#[derive(Debug)]
pub struct OpaqueLoginResponse {
    pub server_message: CredentialResponse<OpaqueCiphersuite>,
}

#[derive(Clone, Debug)]
pub struct OpaqueLoginFinish {
    pub client_message: CredentialFinalization<OpaqueCiphersuite>,
}

/// Registration request containing the OPAQUE payload.
///
/// The TLS serialization implementation of this
#[derive(Debug)]
pub struct OpaqueRegistrationRequest {
    pub client_message: RegistrationRequest<OpaqueCiphersuite>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpaqueRegistrationResponse {
    pub server_message: RegistrationResponse<OpaqueCiphersuite>,
}

impl From<RegistrationResponse<OpaqueCiphersuite>> for OpaqueRegistrationResponse {
    fn from(value: RegistrationResponse<OpaqueCiphersuite>) -> Self {
        Self {
            server_message: value,
        }
    }
}

#[derive(Debug)]
pub struct OpaqueRegistrationRecord {
    pub client_message: RegistrationUpload<OpaqueCiphersuite>,
}
