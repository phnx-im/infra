// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::openmls::prelude::KeyPackageIn;
use tls_codec::{TlsDeserialize, TlsSize};

use crate::{
    auth_service::{
        credentials::{
            AsCredential, CredentialFingerprint, VerifiableAsIntermediateCredential,
            VerifiableClientCredential,
        },
        AsClientId, OpaqueLoginResponse, OpaqueRegistrationResponse,
    },
    crypto::signatures::signable::Signature,
};

use super::client_as::{
    AsDequeueMessagesResponse, Init2FactorAuthResponse, IssueTokensResponse,
    VerifiedAsRequestParams,
};

pub struct ClientCredentialAuthOut {
    pub(crate) client_id: AsClientId,
    pub(crate) payload: Box<VerifiedAsRequestParams>,
    pub(crate) label: &'static str,
    pub(crate) signature: Signature,
}

#[derive(Debug, TlsDeserialize, TlsSize)]
pub struct AsClientKeyPackageResponseIn {
    pub key_package: Option<KeyPackageIn>,
}

#[derive(Debug, TlsDeserialize, TlsSize)]
pub struct UserKeyPackagesResponseIn {
    pub key_packages: Vec<KeyPackageIn>,
}

#[derive(Debug, TlsDeserialize, TlsSize)]
pub struct InitClientAdditionResponseIn {
    pub client_credential: VerifiableClientCredential,
    pub opaque_login_response: OpaqueLoginResponse,
}

#[derive(Debug, TlsDeserialize, TlsSize)]
pub struct UserClientsResponseIn {
    pub client_credentials: Vec<VerifiableClientCredential>,
}

#[derive(Debug, TlsDeserialize, TlsSize)]
pub struct AsCredentialsResponseIn {
    // TODO: We might want a Verifiable... type variant here that ensures that
    // this is matched against the local trust store or something.
    pub as_credentials: Vec<AsCredential>,
    pub as_intermediate_credentials: Vec<VerifiableAsIntermediateCredential>,
    pub revoked_credentials: Vec<CredentialFingerprint>,
}

#[derive(Debug, TlsDeserialize, TlsSize)]
pub struct InitUserRegistrationResponseIn {
    pub client_credential: VerifiableClientCredential,
    pub opaque_registration_response: OpaqueRegistrationResponse,
}

#[derive(Debug, TlsDeserialize, TlsSize)]
#[repr(u8)]
pub enum AsProcessResponseIn {
    Ok,
    Init2FactorAuth(Init2FactorAuthResponse),
    DequeueMessages(AsDequeueMessagesResponse),
    ClientKeyPackage(AsClientKeyPackageResponseIn),
    IssueTokens(IssueTokensResponse),
    UserKeyPackages(UserKeyPackagesResponseIn),
    InitiateClientAddition(InitClientAdditionResponseIn),
    UserClients(UserClientsResponseIn),
    AsCredentials(AsCredentialsResponseIn),
    InitUserRegistration(InitUserRegistrationResponseIn),
}
