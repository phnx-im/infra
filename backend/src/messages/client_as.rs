use hpke::HpkePublicKey;
use mls_assist::KeyPackage;

use crate::auth_service::*;

// === Authentication ===

struct Initiate2FaAuthenticationParams {
    client_id: AsClientId,
    opaque_ke1: OpaqueKe1,
}

struct Initiate2FaAuthenticationResponse {
    opaque_ke2: OpaqueKe2,
}

// === User ===

struct InitUserRegistrationParams {
    client_csr: ClientCsr,
    opaque_registration_request: OpaqueRegistrationRequest,
}

struct InitUserRegistrationResponse {
    client_credential: ClientCredential,
    opaque_registration_response: OpaqueRegistrationResponse,
}

struct FinishUserRegistrationParams {
    user_name: UserName,
    queue_encryption_key: HpkePublicKey,
    connection_key_package: KeyPackage,
    opaque_registration_record: OpaqueRegistrationRecord,
}

struct UserClientsParams {
    user_name: UserName,
}

struct UserClientsResponse {
    client_credentials: Vec<KeyPackage>,
}

struct DeleteUserParams {
    user_name: UserName,
    opaque_ke3: OpaqueKe3,
}

// === Client ===

struct InitiateClientAdditionParams {
    client_csr: ClientCsr,
    opaque_ke1: OpaqueKe1,
}

struct InitiateClientAdditionResponse {
    client_credential: ClientCredential,
    opaque_ke2: OpaqueKe2,
}

struct FinishClientAdditionParams {
    client_id: AsClientId,
    queue_encryption_key: HpkePublicKey,
    connection_key_package: KeyPackage,
    opaque_ke3: OpaqueKe3,
}

struct DeleteClientParams {
    client_id: AsClientId,
}

struct DequeueMessagesParams {
    client_id: AsClientId,
    sequence_number_start: u64,
    max_message_number: u64,
}

struct AsCredentialsResponse {
    as_credentials: Vec<AsCredentials>,
    as_intermediate_credentials: Vec<AsIntermediateCredential>,
    revoked_certs: Vec<Fingerprint>,
}
