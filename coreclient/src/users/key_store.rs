// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::ops::Deref;

use anyhow::{bail, Result};
use phnxbackend::{
    auth_service::credentials::VerifiableClientCredential,
    messages::{client_as::AsQueueMessagePayload, client_ds::QsQueueMessagePayload},
};

use crate::utils::persistance::DataType;

use super::*;

// For now we persist the key store along with the user. Any key material that gets rotated in the future needs to be persisted separately.
#[derive(Serialize, Deserialize)]
pub(crate) struct MemoryUserKeyStore {
    // Client credential secret key
    pub(super) signing_key: ClientSigningKey,
    // AS-specific key material
    pub(super) as_queue_decryption_key: RatchetDecryptionKey,
    pub(super) connection_decryption_key: ConnectionDecryptionKey,
    // QS-specific key material
    pub(super) qs_client_signing_key: QsClientSigningKey,
    pub(super) qs_user_signing_key: QsUserSigningKey,
    pub(super) qs_queue_decryption_key: RatchetDecryptionKey,
    pub(super) qs_client_id_encryption_key: ClientIdEncryptionKey,
    pub(super) push_token_ear_key: PushTokenEarKey,
    // These are keys that we send to our contacts
    pub(super) friendship_token: FriendshipToken,
    pub(super) add_package_ear_key: AddPackageEarKey,
    pub(super) client_credential_ear_key: ClientCredentialEarKey,
    pub(super) signature_ear_key_wrapper_key: SignatureEarKeyWrapperKey,
    pub(super) wai_ear_key: WelcomeAttributionInfoEarKey,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct PersistableAsCredential {
    rowid: Option<i64>,
    own_client_id: Vec<u8>,
    credential: AsCredential,
    fingerprint: CredentialFingerprint,
}

impl PersistableAsCredential {
    pub(crate) fn from_as_credential(
        own_client_id: &AsClientId,
        credential: AsCredential,
    ) -> Result<Self> {
        let fingerprint = credential.fingerprint()?;
        Ok(Self {
            rowid: None,
            own_client_id: own_client_id.tls_serialize_detached()?,
            credential,
            fingerprint,
        })
    }
}

impl Persistable for PersistableAsCredential {
    type Key = CredentialFingerprint;

    type SecondaryKey = Fqdn;

    const DATA_TYPE: DataType = DataType::AsCredential;

    fn own_client_id_bytes(&self) -> Vec<u8> {
        self.own_client_id.clone()
    }

    fn rowid(&self) -> Option<i64> {
        self.rowid
    }

    fn key(&self) -> &Self::Key {
        &self.fingerprint
    }

    fn secondary_key(&self) -> &Self::SecondaryKey {
        self.credential.domain()
    }

    fn set_rowid(&mut self, rowid: i64) {
        self.rowid = Some(rowid);
    }
}

impl Deref for PersistableAsCredential {
    type Target = AsCredential;

    fn deref(&self) -> &Self::Target {
        &self.credential
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct PersistableAsIntermediateCredential {
    rowid: Option<i64>,
    own_client_id: Vec<u8>,
    credential: AsIntermediateCredential,
    fingerprint: CredentialFingerprint,
    domain: Fqdn,
}

impl PersistableAsIntermediateCredential {
    pub(crate) fn from_as_intermediate_credential(
        own_client_id: &AsClientId,
        credential: AsIntermediateCredential,
        domain: Fqdn,
    ) -> Result<Self> {
        let fingerprint = credential.fingerprint()?;
        Ok(Self {
            rowid: None,
            own_client_id: own_client_id.tls_serialize_detached()?,
            credential,
            fingerprint,
            domain,
        })
    }
}

impl PersistableAsIntermediateCredential {
    /// Fetches the credentials of the AS with the given `domain` if they are
    /// not already present in the store.
    async fn fetch_credentials(
        own_client_id: &AsClientId,
        api_clients: &mut ApiClients,
        domain: &Fqdn,
    ) -> Result<()> {
        let as_credentials_response = api_clients.get(&domain)?.as_as_credentials().await?;
        let as_credentials: HashMap<CredentialFingerprint, AsCredential> = as_credentials_response
            .as_credentials
            .into_iter()
            .map(|credential| Ok((credential.fingerprint()?, credential)))
            .collect::<Result<HashMap<_, _>>>()?;
        for as_inter_cred in as_credentials_response.as_intermediate_credentials {
            let as_credential = as_credentials
                .get(as_inter_cred.signer_fingerprint())
                .ok_or(anyhow!(
                    "Can't find AS credential for the given fingerprint"
                ))?;
            let verified_credential: AsIntermediateCredential =
                as_inter_cred.verify(as_credential.verifying_key())?;
            let p_as_inter_cred =
                PersistableAsIntermediateCredential::from_as_intermediate_credential(
                    own_client_id,
                    verified_credential,
                    domain.clone(),
                )?;
            p_as_inter_cred.persist()?;
        }
        for as_credential in as_credentials.values() {
            let p_credential =
                PersistableAsCredential::from_as_credential(own_client_id, as_credential.clone())?;
            p_credential.persist()?;
        }
        Ok(())
    }

    pub async fn get(
        own_client_id: &AsClientId,
        api_clients: &mut ApiClients,
        domain: &Fqdn,
        fingerprint: &CredentialFingerprint,
    ) -> Result<AsIntermediateCredential> {
        if PersistableAsIntermediateCredential::load(own_client_id, fingerprint).is_err() {
            Self::fetch_credentials(own_client_id, api_clients, domain).await?;
        }
        let credential = PersistableAsIntermediateCredential::load(own_client_id, fingerprint)?;
        if &credential.domain != domain {
            bail!("Found credential matching fingerprint, but it does not belong to the requested domain")
        }
        Ok(credential.credential)
    }

    pub async fn verify_client_credential(
        own_client_id: &AsClientId,
        api_clients: &mut ApiClients,
        verifiable_client_credential: VerifiableClientCredential,
    ) -> Result<ClientCredential> {
        let as_intermediate_credential = Self::get(
            own_client_id,
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

impl Persistable for PersistableAsIntermediateCredential {
    type Key = CredentialFingerprint;

    type SecondaryKey = Fqdn;

    const DATA_TYPE: DataType = DataType::AsIntermediateCredential;

    fn own_client_id_bytes(&self) -> Vec<u8> {
        self.own_client_id.clone()
    }

    fn rowid(&self) -> Option<i64> {
        self.rowid
    }

    fn key(&self) -> &Self::Key {
        &self.fingerprint
    }

    fn secondary_key(&self) -> &Self::SecondaryKey {
        &self.domain
    }

    fn set_rowid(&mut self, rowid: i64) {
        self.rowid = Some(rowid);
    }
}

impl Deref for PersistableAsIntermediateCredential {
    type Target = AsIntermediateCredential;

    fn deref(&self) -> &Self::Target {
        &self.credential
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct PersistableLeafKeys {
    rowid: Option<i64>,
    own_client_id: Vec<u8>,
    leaf_signing_key: InfraCredentialSigningKey,
    signature_ear_key: SignatureEarKey,
}

impl PersistableLeafKeys {
    pub(crate) fn from_keys(
        own_client_id: AsClientId,
        leaf_signing_key: InfraCredentialSigningKey,
        signature_ear_key: SignatureEarKey,
    ) -> Result<Self> {
        Ok(Self {
            rowid: None,
            own_client_id: own_client_id.tls_serialize_detached()?,
            leaf_signing_key,
            signature_ear_key,
        })
    }

    pub(crate) fn leaf_signing_key(&self) -> &InfraCredentialSigningKey {
        &self.leaf_signing_key
    }
}

impl Persistable for PersistableLeafKeys {
    type Key = SignaturePublicKey;

    type SecondaryKey = SignaturePublicKey;

    const DATA_TYPE: DataType = DataType::LeafKeys;

    fn own_client_id_bytes(&self) -> Vec<u8> {
        self.own_client_id.clone()
    }

    fn rowid(&self) -> Option<i64> {
        self.rowid
    }

    fn key(&self) -> &Self::Key {
        self.leaf_signing_key.credential().verifying_key()
    }

    fn secondary_key(&self) -> &Self::SecondaryKey {
        self.leaf_signing_key.credential().verifying_key()
    }

    fn set_rowid(&mut self, rowid: i64) {
        self.rowid = Some(rowid);
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct PersistableQsVerifyingKey {
    rowid: Option<i64>,
    own_client_id: Vec<u8>,
    domain: Fqdn,
    qs_verifying_key: QsVerifyingKey,
}

impl PersistableQsVerifyingKey {
    pub(crate) fn from_verifying_key(
        own_client_id: AsClientId,
        domain: Fqdn,
        qs_verifying_key: QsVerifyingKey,
    ) -> Result<Self> {
        Ok(Self {
            rowid: None,
            own_client_id: own_client_id.tls_serialize_detached()?,
            domain,
            qs_verifying_key,
        })
    }

    pub(super) async fn get(
        own_client_id: &AsClientId,
        api_clients: &mut ApiClients,
        domain: &Fqdn,
    ) -> Result<QsVerifyingKey> {
        if let Ok(verifying_key) = PersistableQsVerifyingKey::load(&own_client_id, domain) {
            Ok(verifying_key.qs_verifying_key)
        } else {
            let verifying_key_response = api_clients.get(domain)?.qs_verifying_key().await?;
            let verifying_key = Self::from_verifying_key(
                own_client_id.clone(),
                domain.clone(),
                verifying_key_response.verifying_key,
            )?;
            verifying_key.persist()?;
            Ok(verifying_key.qs_verifying_key)
        }
    }
}

impl Persistable for PersistableQsVerifyingKey {
    type Key = Fqdn;

    type SecondaryKey = Fqdn;

    const DATA_TYPE: DataType = DataType::QsVerifyingKey;

    fn own_client_id_bytes(&self) -> Vec<u8> {
        self.own_client_id.clone()
    }

    fn rowid(&self) -> Option<i64> {
        self.rowid
    }

    fn key(&self) -> &Self::Key {
        &self.domain
    }

    fn secondary_key(&self) -> &Self::SecondaryKey {
        &self.domain
    }

    fn set_rowid(&mut self, rowid: i64) {
        self.rowid = Some(rowid);
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum QueueRatchetType {
    As,
    Qs,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct PersistableAsQueueRatchet {
    rowid: Option<i64>,
    own_client_id: Vec<u8>,
    queue_ratchet: AsQueueRatchet,
}

impl PersistableAsQueueRatchet {
    pub(crate) fn from_ratchet(
        own_client_id: AsClientId,
        queue_ratchet: AsQueueRatchet,
    ) -> Result<Self> {
        Ok(Self {
            rowid: None,
            own_client_id: own_client_id.tls_serialize_detached()?,
            queue_ratchet,
        })
    }

    pub fn decrypt(&mut self, queue_message: QueueMessage) -> Result<AsQueueMessagePayload> {
        let message = self.queue_ratchet.decrypt(queue_message)?;
        self.persist()?;
        Ok(message)
    }
}

impl Persistable for PersistableAsQueueRatchet {
    type Key = QueueRatchetType;

    type SecondaryKey = QueueRatchetType;

    const DATA_TYPE: DataType = DataType::QueueRatchet;

    fn own_client_id_bytes(&self) -> Vec<u8> {
        self.own_client_id.clone()
    }

    fn rowid(&self) -> Option<i64> {
        self.rowid
    }

    fn key(&self) -> &Self::Key {
        &QueueRatchetType::As
    }

    fn secondary_key(&self) -> &Self::SecondaryKey {
        &QueueRatchetType::As
    }

    fn set_rowid(&mut self, rowid: i64) {
        self.rowid = Some(rowid);
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct PersistableQsQueueRatchet {
    rowid: Option<i64>,
    own_client_id: Vec<u8>,
    queue_ratchet: QsQueueRatchet,
}

impl PersistableQsQueueRatchet {
    pub(crate) fn from_ratchet(
        own_client_id: AsClientId,
        queue_ratchet: QsQueueRatchet,
    ) -> Result<Self> {
        Ok(Self {
            rowid: None,
            own_client_id: own_client_id.tls_serialize_detached()?,
            queue_ratchet,
        })
    }

    pub fn decrypt(&mut self, queue_message: QueueMessage) -> Result<QsQueueMessagePayload> {
        let message = self.queue_ratchet.decrypt(queue_message)?;
        self.persist()?;
        Ok(message)
    }
}

impl Persistable for PersistableQsQueueRatchet {
    type Key = QueueRatchetType;

    type SecondaryKey = QueueRatchetType;

    const DATA_TYPE: DataType = DataType::QueueRatchet;

    fn own_client_id_bytes(&self) -> Vec<u8> {
        self.own_client_id.clone()
    }

    fn rowid(&self) -> Option<i64> {
        self.rowid
    }

    fn key(&self) -> &Self::Key {
        &QueueRatchetType::Qs
    }

    fn secondary_key(&self) -> &Self::SecondaryKey {
        &QueueRatchetType::Qs
    }

    fn set_rowid(&mut self, rowid: i64) {
        self.rowid = Some(rowid);
    }
}
