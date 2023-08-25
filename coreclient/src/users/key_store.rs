// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::Result;
use phnxbackend::auth_service::credentials::VerifiableClientCredential;

use super::*;

pub(crate) struct MemoryUserKeyStore {
    // Client credential secret key
    pub(super) signing_key: ClientSigningKey,
    // AS-specific key material
    pub(super) as_queue_decryption_key: RatchetDecryptionKey,
    pub(super) as_queue_ratchet: AsQueueRatchet,
    pub(super) connection_decryption_key: ConnectionDecryptionKey,
    pub(super) as_credentials: AsCredentials,
    // QS-specific key material
    pub(super) qs_client_signing_key: QsClientSigningKey,
    pub(super) qs_user_signing_key: QsUserSigningKey,
    pub(super) qs_queue_decryption_key: RatchetDecryptionKey,
    pub(super) qs_queue_ratchet: QsQueueRatchet,
    pub(super) qs_verifying_keys: HashMap<Fqdn, QsVerifyingKey>,
    pub(super) qs_client_id_encryption_key: ClientIdEncryptionKey,
    pub(super) push_token_ear_key: PushTokenEarKey,
    // These are keys that we send to our contacts
    pub(super) friendship_token: FriendshipToken,
    pub(super) add_package_ear_key: AddPackageEarKey,
    pub(super) client_credential_ear_key: ClientCredentialEarKey,
    pub(super) signature_ear_key_wrapper_key: SignatureEarKeyWrapperKey,
    pub(super) wai_ear_key: WelcomeAttributionInfoEarKey,
    // Leaf credentials in KeyPackages the active ones are stored in the group
    // that they belong to.
    pub(super) leaf_signers:
        HashMap<SignaturePublicKey, (InfraCredentialSigningKey, SignatureEarKey)>,
}

pub(crate) struct AsCredentials {
    credentials: HashMap<Fqdn, HashMap<CredentialFingerprint, AsCredential>>,
    intermediate_credentials:
        HashMap<Fqdn, HashMap<CredentialFingerprint, AsIntermediateCredential>>,
}

impl AsCredentials {
    pub(crate) async fn new(api_clients: &mut ApiClients, domain: &Fqdn) -> Result<Self> {
        let mut as_credentials = Self {
            credentials: HashMap::new(),
            intermediate_credentials: HashMap::new(),
        };
        as_credentials
            .fetch_credentials(api_clients, &domain)
            .await?;
        Ok(as_credentials)
    }

    /// Fetches the credentials of the AS with the given `domain` if they are
    /// not already present in the store.
    async fn fetch_credentials(
        &mut self,
        api_clients: &mut ApiClients,
        domain: &Fqdn,
    ) -> Result<()> {
        let as_credentials_response = api_clients.get(&domain)?.as_as_credentials().await?;
        let as_credentials: HashMap<CredentialFingerprint, AsCredential> = as_credentials_response
            .as_credentials
            .into_iter()
            .map(|credential| Ok((credential.fingerprint()?, credential)))
            .collect::<Result<HashMap<_, _>>>()?;
        let as_intermediate_credentials: HashMap<CredentialFingerprint, AsIntermediateCredential> =
            as_credentials_response
                .as_intermediate_credentials
                .into_iter()
                .map(|as_inter_cred| {
                    let as_credential = as_credentials
                        .get(as_inter_cred.signer_fingerprint())
                        .ok_or(anyhow!(
                            "Can't find AS credential for the given fingerprint"
                        ))?;
                    let verified_credential: AsIntermediateCredential =
                        as_inter_cred.verify(as_credential.verifying_key())?;
                    Ok((verified_credential.fingerprint()?, verified_credential))
                })
                .collect::<Result<HashMap<_, _>>>()?;
        self.credentials.insert(domain.clone(), as_credentials);
        self.intermediate_credentials
            .insert(domain.clone(), as_intermediate_credentials);
        Ok(())
    }

    pub async fn get(
        &mut self,
        api_clients: &mut ApiClients,
        domain: &Fqdn,
        fingerprint: &CredentialFingerprint,
    ) -> Result<&AsIntermediateCredential> {
        if !self.credentials.contains_key(domain) {
            self.fetch_credentials(api_clients, domain).await?
        }
        self.intermediate_credentials
            .get(domain)
            // We can unwrap here because we just fetched the corresponding
            // credential.
            .unwrap()
            .get(fingerprint)
            .ok_or(anyhow!("Can't find credential for the given fingerprint"))
    }

    pub async fn verify_client_credential(
        &mut self,
        api_clients: &mut ApiClients,
        verifiable_client_credential: VerifiableClientCredential,
    ) -> Result<ClientCredential> {
        let as_intermediate_credential = self
            .get(
                api_clients,
                &verifiable_client_credential.domain(),
                verifiable_client_credential.signer_fingerprint(),
            )
            .await?;
        let client_credential =
            verifiable_client_credential.verify(as_intermediate_credential.verifying_key())?;
        Ok(client_credential)
    }
}
