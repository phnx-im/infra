// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

pub(crate) mod client_auth_info;
pub(crate) mod diff;
pub(crate) mod error;
pub(crate) mod openmls_provider;
pub(crate) mod persistence;
pub(crate) mod process;

pub(crate) use error::*;

use anyhow::{anyhow, bail, Result};
use mls_assist::messages::AssistedMessageOut;
use openmls_provider::PhnxOpenMlsProvider;
use openmls_traits::storage::StorageProvider;
use phnxtypes::{
    credentials::{
        keys::{ClientSigningKey, InfraCredentialSigningKey},
        ClientCredential, EncryptedClientCredential,
    },
    crypto::{
        ear::{
            keys::{
                ClientCredentialEarKey, EncryptedSignatureEarKey, GroupStateEarKey,
                SignatureEarKey, SignatureEarKeyWrapperKey, WelcomeAttributionInfoEarKey,
            },
            EarDecryptable, EarEncryptable,
        },
        hpke::{HpkeDecryptable, JoinerInfoDecryptionKey},
        signatures::{
            keys::{UserAuthSigningKey, UserAuthVerifyingKey},
            signable::{Signable, Verifiable},
        },
    },
    identifiers::{
        AsClientId, QsClientReference, QualifiedUserName, QS_CLIENT_REFERENCE_EXTENSION_TYPE,
    },
    keypackage_batch::{KeyPackageBatch, VERIFIED},
    messages::{
        client_ds::{
            DsJoinerInformationIn, GroupOperationParamsAad, InfraAadMessage, InfraAadPayload,
            UpdateClientParamsAad, WelcomeBundle,
        },
        client_ds_out::{
            AddUsersInfoOut, CreateGroupParamsOut, DeleteGroupParamsOut, ExternalCommitInfoIn,
            GroupOperationParamsOut, SelfRemoveClientParamsOut, SendMessageParamsOut,
            UpdateClientParamsOut,
        },
        welcome_attribution_info::{
            WelcomeAttributionInfo, WelcomeAttributionInfoPayload, WelcomeAttributionInfoTbs,
        },
    },
    time::TimeStamp,
};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};

use crate::{
    clients::api_clients::ApiClients, contacts::ContactAddInfos,
    conversations::messages::TimestampedMessage, key_stores::leaf_keys::LeafKeys,
    mimi_content::MimiContent, utils::persistence::SqliteConnection, SystemMessage,
};
use std::collections::HashSet;

use openmls::{
    group::ProcessedWelcome,
    key_packages::KeyPackageBundle,
    prelude::{
        tls_codec::Serialize as TlsSerializeTrait, Capabilities, Ciphersuite, Credential,
        CredentialType, CredentialWithKey, Extension, ExtensionType, Extensions, GroupId,
        KeyPackage, LeafNodeIndex, MlsGroup, MlsGroupJoinConfig, MlsMessageOut, OpenMlsProvider,
        Proposal, ProposalType, ProtocolVersion, QueuedProposal, RequiredCapabilitiesExtension,
        Sender, StagedCommit, UnknownExtension, PURE_PLAINTEXT_WIRE_FORMAT_POLICY,
    },
    treesync::{LeafNodeParameters, RatchetTree},
};

use self::{
    client_auth_info::{ClientAuthInfo, GroupMembership, StorableClientCredential},
    diff::{GroupDiff, StagedGroupDiff},
};

pub const FRIENDSHIP_PACKAGE_PROPOSAL_TYPE: u16 = 0xff00;
pub const GROUP_DATA_EXTENSION_TYPE: u16 = 0xff01;

pub const DEFAULT_MLS_VERSION: ProtocolVersion = ProtocolVersion::Mls10;
pub const DEFAULT_CIPHERSUITE: Ciphersuite =
    Ciphersuite::MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519;

pub const REQUIRED_EXTENSION_TYPES: [ExtensionType; 3] = [
    ExtensionType::Unknown(QS_CLIENT_REFERENCE_EXTENSION_TYPE),
    ExtensionType::Unknown(GROUP_DATA_EXTENSION_TYPE),
    ExtensionType::LastResort,
];
pub const REQUIRED_PROPOSAL_TYPES: [ProposalType; 1] =
    [ProposalType::Custom(FRIENDSHIP_PACKAGE_PROPOSAL_TYPE)];
pub const REQUIRED_CREDENTIAL_TYPES: [CredentialType; 1] = [CredentialType::Basic];

pub fn default_required_capabilities() -> RequiredCapabilitiesExtension {
    RequiredCapabilitiesExtension::new(
        &REQUIRED_EXTENSION_TYPES,
        &REQUIRED_PROPOSAL_TYPES,
        &REQUIRED_CREDENTIAL_TYPES,
    )
}

// Default capabilities for every leaf node we create.
pub const SUPPORTED_PROTOCOL_VERSIONS: [ProtocolVersion; 1] = [DEFAULT_MLS_VERSION];
pub const SUPPORTED_CIPHERSUITES: [Ciphersuite; 1] = [DEFAULT_CIPHERSUITE];
pub const SUPPORTED_EXTENSIONS: [ExtensionType; 3] = REQUIRED_EXTENSION_TYPES;
pub const SUPPORTED_PROPOSALS: [ProposalType; 1] = REQUIRED_PROPOSAL_TYPES;
pub const SUPPORTED_CREDENTIALS: [CredentialType; 1] = REQUIRED_CREDENTIAL_TYPES;

pub fn default_capabilities() -> Capabilities {
    Capabilities::new(
        Some(&SUPPORTED_PROTOCOL_VERSIONS),
        Some(&SUPPORTED_CIPHERSUITES),
        Some(&SUPPORTED_EXTENSIONS),
        Some(&SUPPORTED_PROPOSALS),
        Some(&SUPPORTED_CREDENTIALS),
    )
}

pub(crate) struct PartialCreateGroupParams {
    pub(crate) group_id: GroupId,
    ratchet_tree: RatchetTree,
    group_info: MlsMessageOut,
    user_auth_key: UserAuthVerifyingKey,
    encrypted_signature_ear_key: EncryptedSignatureEarKey,
}

impl PartialCreateGroupParams {
    pub(crate) fn into_params(
        self,
        encrypted_client_credential: EncryptedClientCredential,
        client_reference: QsClientReference,
    ) -> CreateGroupParamsOut {
        CreateGroupParamsOut {
            group_id: self.group_id,
            ratchet_tree: self.ratchet_tree,
            encrypted_client_credential,
            encrypted_signature_ear_key: self.encrypted_signature_ear_key,
            creator_client_reference: client_reference,
            creator_user_auth_key: self.user_auth_key,
            group_info: self.group_info,
        }
    }
}

pub(crate) struct GroupData {
    bytes: Vec<u8>,
}

impl GroupData {
    pub(crate) fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

impl From<Vec<u8>> for GroupData {
    fn from(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }
}

#[derive(Debug)]
pub(crate) struct Group {
    group_id: GroupId,
    leaf_signer: InfraCredentialSigningKey,
    signature_ear_key_wrapper_key: SignatureEarKeyWrapperKey,
    credential_ear_key: ClientCredentialEarKey,
    group_state_ear_key: GroupStateEarKey,
    // This needs to be set after initially joining a group.
    user_auth_signing_key_option: Option<UserAuthSigningKey>,
    mls_group: MlsGroup,
    pending_diff: Option<StagedGroupDiff>,
}

impl Group {
    fn mls_group(&self) -> &MlsGroup {
        &self.mls_group
    }

    fn default_mls_group_join_config() -> MlsGroupJoinConfig {
        MlsGroupJoinConfig::builder()
            // This is turned on for now, as it makes OpenMLS return GroupInfos
            // with every commit. At some point, there should be a dedicated
            // config flag for this.
            .use_ratchet_tree_extension(true)
            .wire_format_policy(PURE_PLAINTEXT_WIRE_FORMAT_POLICY)
            .build()
    }

    /// Create a group.
    pub(super) fn create_group(
        provider: &impl OpenMlsProvider,
        signer: &ClientSigningKey,
        group_id: GroupId,
        group_data: GroupData,
    ) -> Result<(Self, GroupMembership, PartialCreateGroupParams)> {
        let credential_ear_key = ClientCredentialEarKey::random()?;
        let user_auth_key = UserAuthSigningKey::generate()?;
        let group_state_ear_key = GroupStateEarKey::random()?;
        let signature_ear_key_wrapper_key = SignatureEarKeyWrapperKey::random()?;

        let signature_ear_key = SignatureEarKey::random()?;
        let leaf_signer = InfraCredentialSigningKey::generate(signer, &signature_ear_key);

        let required_capabilities =
            Extension::RequiredCapabilities(default_required_capabilities());
        let leaf_node_capabilities = default_capabilities();

        let credential_with_key = CredentialWithKey {
            credential: Credential::try_from(leaf_signer.credential())?,
            signature_key: leaf_signer.credential().verifying_key().clone(),
        };
        let group_data_extension = Extension::Unknown(
            GROUP_DATA_EXTENSION_TYPE,
            UnknownExtension(group_data.bytes),
        );
        let gc_extensions =
            Extensions::from_vec(vec![group_data_extension, required_capabilities])?;

        let mls_group = MlsGroup::builder()
            .with_group_id(group_id.clone())
            // This is turned on for now, as it makes OpenMLS return GroupInfos
            // with every commit. At some point, there should be a dedicated
            // config flag for this.
            .with_capabilities(leaf_node_capabilities)
            .use_ratchet_tree_extension(true)
            .with_group_context_extensions(gc_extensions)?
            .with_wire_format_policy(PURE_PLAINTEXT_WIRE_FORMAT_POLICY)
            .build(provider, &leaf_signer, credential_with_key)
            .map_err(|e| anyhow!("Error while creating group: {:?}", e))?;

        let encrypted_signature_ear_key =
            signature_ear_key.encrypt(&signature_ear_key_wrapper_key)?;
        let params = PartialCreateGroupParams {
            group_id: group_id.clone(),
            ratchet_tree: mls_group.export_ratchet_tree(),
            group_info: mls_group.export_group_info(provider, &leaf_signer, true)?,
            user_auth_key: user_auth_key.verifying_key().clone(),
            encrypted_signature_ear_key,
        };

        let group_membership = GroupMembership::new(
            signer.credential().identity(),
            group_id.clone(),
            LeafNodeIndex::new(0), // We just created the group so we're at index 0.
            signature_ear_key,
            signer.credential().fingerprint(),
        );

        let group = Self {
            group_id,
            leaf_signer,
            signature_ear_key_wrapper_key,
            mls_group,
            credential_ear_key,
            group_state_ear_key: group_state_ear_key.clone(),
            user_auth_signing_key_option: Some(user_auth_key),
            pending_diff: None,
        };

        Ok((group, group_membership, params))
    }

    /// Join a group with the provided welcome message. If there exists a group
    /// with the same ID, checks if that group is inactive and if so deletes the
    /// old group.
    ///
    /// Returns the group name.
    pub(super) async fn join_group(
        welcome_bundle: WelcomeBundle,
        // This is our own key that the sender uses to encrypt to us. We should
        // be able to retrieve it from the client's key store.
        welcome_attribution_info_ear_key: &WelcomeAttributionInfoEarKey,
        connection_mutex: SqliteConnection,
        api_clients: &ApiClients,
    ) -> Result<Self> {
        let serialized_welcome = welcome_bundle.welcome.tls_serialize_detached()?;

        let mls_group_config = Self::default_mls_group_join_config();

        // Phase 1: Fetch the right KeyPackageBundle from storage s.t. we can
        // decrypt the encrypted credentials
        let (mls_group, joiner_info, welcome_attribution_info) = {
            let mut connection = connection_mutex.lock().await;
            let mut transaction = connection.transaction()?;
            let provider = PhnxOpenMlsProvider::new(&transaction);
            let key_package_bundle: KeyPackageBundle = welcome_bundle
                .welcome
                .welcome
                .secrets()
                .iter()
                .find_map(|egs| {
                    let kp_hash = egs.new_member();
                    match provider.storage().key_package(&kp_hash) {
                        Ok(Some(kpb)) => Some(kpb),
                        _ => None,
                    }
                })
                .ok_or(GroupOperationError::MissingKeyPackage)?;

            let private_key = key_package_bundle.init_private_key();
            let info = &[];
            let aad = &[];
            let decryption_key = JoinerInfoDecryptionKey::from((
                private_key.clone(),
                key_package_bundle.key_package().hpke_init_key().clone(),
            ));
            let joiner_info = DsJoinerInformationIn::decrypt(
                welcome_bundle.encrypted_joiner_info,
                &decryption_key,
                info,
                aad,
            )?;

            let processed_welcome = ProcessedWelcome::new_from_welcome(
                &provider,
                &mls_group_config,
                welcome_bundle.welcome.welcome,
            )?;
            // Check if there is already a group with the same ID.
            let group_id = processed_welcome.unverified_group_info().group_id().clone();
            if let Some(group) = Self::load(&transaction, &group_id)? {
                // If the group is active, we can't join it.
                if group.mls_group().is_active() {
                    bail!("We can't join a group that is still active.");
                }
                // Otherwise, we delete the old group.
                Self::delete_from_db(&mut transaction, &group_id)?;
            }

            let provider = PhnxOpenMlsProvider::new(&transaction);
            let staged_welcome = processed_welcome.into_staged_welcome(&provider, None)?;

            let mls_group = staged_welcome.into_group(&provider)?;

            // Decrypt WelcomeAttributionInfo
            let verifiable_attribution_info = WelcomeAttributionInfo::decrypt(
                welcome_attribution_info_ear_key,
                &welcome_bundle.encrypted_attribution_info,
            )?
            .into_verifiable(mls_group.group_id().clone(), serialized_welcome);

            let sender_client_id = verifiable_attribution_info.sender();
            let sender_client_credential =
                StorableClientCredential::load_by_client_id(&transaction, &sender_client_id)?
                    .ok_or(anyhow!(
                        "Could not find client credential of sender in database."
                    ))?;
            transaction.commit()?;
            drop(connection);

            let welcome_attribution_info: WelcomeAttributionInfoPayload =
                verifiable_attribution_info.verify(sender_client_credential.verifying_key())?;

            (mls_group, joiner_info, welcome_attribution_info)
        };

        let encrypted_client_information = mls_group
            .members()
            .map(|m| m.index)
            .zip(joiner_info.encrypted_client_information.into_iter());

        // Phase 2: Decrypt and verify the client credentials. This can involve
        // queries to the clients' AS.
        let client_information = ClientAuthInfo::decrypt_and_verify_all(
            connection_mutex.clone(),
            api_clients,
            mls_group.group_id(),
            welcome_attribution_info.client_credential_encryption_key(),
            welcome_attribution_info.signature_ear_key_wrapper_key(),
            encrypted_client_information,
        )
        .await?;

        let verifying_key = mls_group
            .own_leaf_node()
            .ok_or(anyhow!("Group has no own leaf node"))?
            .signature_key();

        // Phase 3: Decrypt and verify the infra credentials.
        let connection = connection_mutex.lock().await;
        for (m, client_auth_info) in mls_group.members().zip(client_information.iter()) {
            client_auth_info.verify_infra_credential(&m.credential)?;
            client_auth_info.store(&connection)?;
        }

        let leaf_keys = LeafKeys::load(&connection, verifying_key)?
            .ok_or(anyhow!("Couldn't find matching leaf keys."))?;
        // Delete the leaf signer from the keys store as it now gets persisted as part of the group.
        LeafKeys::delete(&connection, verifying_key)?;
        drop(connection);

        let leaf_signer = leaf_keys.into_leaf_signer();

        let group = Self {
            group_id: mls_group.group_id().clone(),
            mls_group,
            leaf_signer,
            signature_ear_key_wrapper_key: welcome_attribution_info
                .signature_ear_key_wrapper_key()
                .clone(),
            credential_ear_key: welcome_attribution_info
                .client_credential_encryption_key()
                .clone(),
            group_state_ear_key: joiner_info.group_state_ear_key,
            // This one needs to be rolled fresh.
            user_auth_signing_key_option: None,
            pending_diff: None,
        };

        Ok(group)
    }

    /// Join a group using an external commit.
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn join_group_externally(
        connection_mutex: SqliteConnection,
        api_clients: &ApiClients,
        external_commit_info: ExternalCommitInfoIn,
        leaf_signer: InfraCredentialSigningKey,
        signature_ear_key: SignatureEarKey,
        group_state_ear_key: GroupStateEarKey,
        signature_ear_key_wrapper_key: SignatureEarKeyWrapperKey,
        credential_ear_key: ClientCredentialEarKey,
        aad: InfraAadMessage,
        own_client_credential: &ClientCredential,
    ) -> Result<(Self, MlsMessageOut, MlsMessageOut)> {
        // TODO: We set the ratchet tree extension for now, as it is the only
        // way to make OpenMLS return a GroupInfo. This should change in the
        // future.
        let mls_group_config = Self::default_mls_group_join_config();
        let credential_with_key = CredentialWithKey {
            credential: leaf_signer.credential().try_into()?,
            signature_key: leaf_signer.credential().verifying_key().clone(),
        };
        let ExternalCommitInfoIn {
            verifiable_group_info,
            ratchet_tree_in,
            encrypted_client_info,
        } = external_commit_info;

        // Let's create the group first so that we can access the GroupId.
        // Phase 1: Create and store the group
        let (mls_group, commit, group_info_option) = {
            let connection = connection_mutex.lock().await;
            let provider = PhnxOpenMlsProvider::new(&connection);
            let (mut mls_group, commit, group_info_option) = MlsGroup::join_by_external_commit(
                &provider,
                &leaf_signer,
                Some(ratchet_tree_in),
                verifiable_group_info,
                &mls_group_config,
                Some(default_capabilities()),
                None,
                &aad.tls_serialize_detached()?,
                credential_with_key,
            )?;
            mls_group.merge_pending_commit(&provider)?;
            drop(connection);
            (mls_group, commit, group_info_option)
        };

        let group_info = group_info_option.ok_or(anyhow!("Commit didn't return a group info"))?;
        let group_id = group_info.group_context().group_id();

        let encrypted_client_information = mls_group
            .members()
            .map(|m| m.index)
            .zip(encrypted_client_info.into_iter());

        // Phase 2: Decrypt and verify the client credentials.
        let mut client_information = ClientAuthInfo::decrypt_and_verify_all(
            connection_mutex.clone(),
            api_clients,
            group_id,
            &credential_ear_key,
            &signature_ear_key_wrapper_key,
            encrypted_client_information,
        )
        .await?;

        // We still have to add ourselves to the encrypted client credentials.
        let own_index = mls_group.own_leaf_index().usize();
        let own_group_membership = GroupMembership::new(
            own_client_credential.identity(),
            group_info.group_context().group_id().clone(),
            LeafNodeIndex::new(own_index as u32),
            signature_ear_key.clone(),
            own_client_credential.fingerprint(),
        );

        let own_auth_info =
            ClientAuthInfo::new(own_client_credential.clone(), own_group_membership);
        client_information.push(own_auth_info);

        // Phase 3: Verify and store the infra credentials.
        let connection = connection_mutex.lock().await;
        for (m, client_auth_info) in mls_group.members().zip(client_information.iter()) {
            client_auth_info.verify_infra_credential(&m.credential)?;
            // Store client auth info.
            client_auth_info.store(&connection)?;
        }
        drop(connection);

        // TODO: Once we support multiple clients, this should be synchronized
        // across clients.
        let user_auth_key = UserAuthSigningKey::generate()?;

        let group = Self {
            group_id: mls_group.group_id().clone(),
            mls_group,
            leaf_signer,
            signature_ear_key_wrapper_key,
            credential_ear_key,
            group_state_ear_key,
            user_auth_signing_key_option: Some(user_auth_key),
            pending_diff: None,
        };

        Ok((group, commit, group_info.into()))
    }

    /// Invite the given list of contacts to join the group.
    ///
    /// Returns the [`AddUserParamsOut`] as input for the API client.
    pub(super) fn invite(
        &mut self,
        connection: &Connection,
        signer: &ClientSigningKey,
        // The following three vectors have to be in sync, i.e. of the same length
        // and refer to the same contacts in order.
        add_infos: Vec<ContactAddInfos>,
        wai_keys: Vec<WelcomeAttributionInfoEarKey>,
        client_credentials: Vec<Vec<ClientCredential>>,
    ) -> Result<GroupOperationParamsOut> {
        let Some(user_auth_key) = &self.user_auth_signing_key_option else {
            bail!("No user auth key");
        };
        let client_credentials = client_credentials.into_iter().flatten().collect::<Vec<_>>();
        debug_assert!(add_infos.len() == client_credentials.len());
        // Prepare KeyPackageBatches and KeyPackages
        let (key_package_vecs, key_package_batches): (
            Vec<Vec<(KeyPackage, SignatureEarKey)>>,
            Vec<KeyPackageBatch<VERIFIED>>,
        ) = add_infos
            .into_iter()
            .map(|add_info| (add_info.key_packages, add_info.key_package_batch))
            .unzip();

        let (key_packages, signature_ear_keys): (Vec<KeyPackage>, Vec<SignatureEarKey>) =
            key_package_vecs.into_iter().flatten().unzip();

        let ecc = client_credentials
            .iter()
            .zip(signature_ear_keys.iter())
            .map(|(client_credential, sek)| {
                let ecc = client_credential.encrypt(&self.credential_ear_key)?;
                let esek = sek.encrypt(&self.signature_ear_key_wrapper_key)?;
                Ok((ecc, esek))
            })
            .collect::<Result<Vec<_>>>()?;
        let aad_message: InfraAadMessage =
            InfraAadPayload::GroupOperation(GroupOperationParamsAad {
                new_encrypted_credential_information: ecc,
                credential_update_option: None,
            })
            .into();

        // Set Aad to contain the encrypted client credentials.
        let provider = PhnxOpenMlsProvider::new(connection);
        self.mls_group
            .set_aad(aad_message.tls_serialize_detached()?);
        let (mls_commit, welcome, group_info_option) =
            self.mls_group
                .add_members(&provider, &self.leaf_signer, key_packages.as_slice())?;

        // Groups should always have the flag set that makes them return groupinfos with every Commit.
        // Or at least with Add commits for now.
        let group_info = group_info_option.ok_or(anyhow!("Commit didn't return a group info"))?;
        let commit = AssistedMessageOut::new(mls_commit, Some(group_info.into()))?;

        let encrypted_welcome_attribution_infos = wai_keys
            .iter()
            .map(|wai_key| {
                // WAI = WelcomeAttributionInfo
                let wai_payload = WelcomeAttributionInfoPayload::new(
                    signer.credential().identity(),
                    self.credential_ear_key.clone(),
                    self.signature_ear_key_wrapper_key.clone(),
                );

                let wai = WelcomeAttributionInfoTbs {
                    payload: wai_payload,
                    group_id: self.group_id().clone(),
                    welcome: welcome.tls_serialize_detached()?,
                }
                .sign(signer)?;
                Ok(wai.encrypt(wai_key)?)
            })
            .collect::<Result<Vec<_>>>()?;

        // Stage removals
        for remove in self
            .mls_group()
            .pending_commit()
            .ok_or(anyhow!("No pending commit after commit operation"))?
            .remove_proposals()
        {
            GroupMembership::stage_removal(
                connection,
                self.group_id(),
                remove.remove_proposal().removed(),
            )?;
        }

        // Stage the adds in the DB.
        let free_indices = GroupMembership::free_indices(connection, self.group_id())?;
        for (leaf_index, (client_credential, signature_ear_key)) in free_indices.zip(
            client_credentials
                .into_iter()
                .zip(signature_ear_keys.into_iter()),
        ) {
            let fingerprint = client_credential.fingerprint();
            let group_membership = GroupMembership::new(
                client_credential.identity(),
                self.group_id.clone(),
                leaf_index,
                signature_ear_key,
                fingerprint,
            );
            let client_auth_info = ClientAuthInfo::new(client_credential, group_membership);
            client_auth_info.stage_add(connection)?;
        }

        let add_users_info = AddUsersInfoOut {
            welcome,
            encrypted_welcome_attribution_infos,
            key_package_batches,
        };

        let params = GroupOperationParamsOut {
            commit,
            sender: user_auth_key.verifying_key().hash(),
            add_users_info_option: Some(add_users_info),
        };

        Ok(params)
    }

    pub(super) fn remove(
        &mut self,
        connection: &Connection,
        members: Vec<AsClientId>,
    ) -> Result<GroupOperationParamsOut> {
        let provider = &PhnxOpenMlsProvider::new(connection);
        let Some(user_auth_key) = &self.user_auth_signing_key_option else {
            bail!("No user auth key")
        };
        let remove_indices =
            GroupMembership::client_indices(connection, self.group_id(), &members)?;
        let aad_payload = InfraAadPayload::GroupOperation(GroupOperationParamsAad {
            new_encrypted_credential_information: vec![],
            credential_update_option: None,
        });
        let aad = InfraAadMessage::from(aad_payload).tls_serialize_detached()?;
        self.mls_group.set_aad(aad);
        let (mls_message, _welcome_option, group_info_option) = self.mls_group.remove_members(
            provider,
            &self.leaf_signer,
            remove_indices.as_slice(),
        )?;
        // There shouldn't be a welcome
        debug_assert!(_welcome_option.is_none());
        let group_info = group_info_option.ok_or(anyhow!("No group info after commit"))?;
        let commit = AssistedMessageOut::new(mls_message, Some(group_info.into()))?;

        for remove in self
            .mls_group()
            .pending_commit()
            .ok_or(anyhow!("No pending commit after commit operation"))?
            .remove_proposals()
        {
            GroupMembership::stage_removal(
                connection,
                self.group_id(),
                remove.remove_proposal().removed(),
            )?;
        }

        let params = GroupOperationParamsOut {
            commit,
            sender: user_auth_key.verifying_key().hash(),
            add_users_info_option: None,
        };
        Ok(params)
    }

    pub(super) fn delete(&mut self, connection: &Connection) -> Result<DeleteGroupParamsOut> {
        let provider = &PhnxOpenMlsProvider::new(connection);
        let Some(user_auth_key) = &self.user_auth_signing_key_option else {
            bail!("No user auth key")
        };
        let remove_indices = self
            .mls_group()
            .members()
            .filter_map(|m| {
                if m.index != self.mls_group().own_leaf_index() {
                    Some(m.index)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        // There shouldn't be a welcome
        let aad_payload = InfraAadPayload::DeleteGroup;
        let aad = InfraAadMessage::from(aad_payload).tls_serialize_detached()?;
        self.mls_group.set_aad(aad);
        let (mls_message, _welcome_option, group_info_option) = self.mls_group.remove_members(
            provider,
            &self.leaf_signer,
            remove_indices.as_slice(),
        )?;
        debug_assert!(_welcome_option.is_none());
        let group_info =
            group_info_option.ok_or(anyhow!("No group info after commit operation"))?;
        let commit = AssistedMessageOut::new(mls_message, Some(group_info.into()))?;

        for remove in self
            .mls_group()
            .pending_commit()
            .ok_or(anyhow!("No pending commit after commit operation"))?
            .remove_proposals()
        {
            GroupMembership::stage_removal(
                connection,
                self.group_id(),
                remove.remove_proposal().removed(),
            )?;
        }

        let params = DeleteGroupParamsOut {
            commit,
            sender: user_auth_key.verifying_key().hash(),
        };
        Ok(params)
    }

    /// If a [`StagedCommit`] is given, merge it and apply the pending group
    /// diff. If no [`StagedCommit`] is given, merge any pending commit and
    /// apply the pending group diff.
    pub(super) fn merge_pending_commit(
        &mut self,
        connection: &Connection,
        staged_commit_option: impl Into<Option<StagedCommit>>,
        ds_timestamp: TimeStamp,
    ) -> Result<Vec<TimestampedMessage>> {
        let provider = &PhnxOpenMlsProvider::new(connection);
        let free_indices = GroupMembership::free_indices(connection, self.group_id())?;
        let staged_commit_option: Option<StagedCommit> = staged_commit_option.into();

        let event_messages = if let Some(staged_commit) = staged_commit_option {
            // Compute the messages we want to emit from the staged commit and the
            // client info diff.
            let staged_commit_messages = TimestampedMessage::from_staged_commit(
                connection,
                self.group_id(),
                free_indices,
                &staged_commit,
                ds_timestamp,
            )?;

            self.mls_group
                .merge_staged_commit(provider, staged_commit)?;
            staged_commit_messages
        } else {
            // If we're merging a pending commit, we need to check if we have
            // committed a remove proposal by reference. If we have, we need to
            // create a notification message.
            let staged_commit_messages =
                if let Some(staged_commit) = self.mls_group.pending_commit() {
                    TimestampedMessage::from_staged_commit(
                        connection,
                        self.group_id(),
                        free_indices,
                        staged_commit,
                        ds_timestamp,
                    )?
                } else {
                    vec![]
                };
            self.mls_group.merge_pending_commit(provider)?;
            staged_commit_messages
        };

        // We now apply the diff (if present)
        if let Some(diff) = self.pending_diff.take() {
            if let Some(leaf_signer) = diff.leaf_signer {
                self.leaf_signer = leaf_signer;
            }
            if let Some(signature_ear_key) = diff.signature_ear_key {
                self.signature_ear_key_wrapper_key = signature_ear_key;
            }
            if let Some(credential_ear_key) = diff.credential_ear_key {
                self.credential_ear_key = credential_ear_key;
            }
            if let Some(group_state_ear_key) = diff.group_state_ear_key {
                self.group_state_ear_key = group_state_ear_key;
            }
            if let Some(user_auth_key) = diff.user_auth_key {
                self.user_auth_signing_key_option = Some(user_auth_key);
            }
        }

        GroupMembership::merge_for_group(connection, self.group_id())?;
        self.pending_diff = None;
        // Debug sanity checks after merging.
        #[cfg(debug_assertions)]
        {
            let mls_group_members = self
                .mls_group
                .members()
                .map(|m| m.index)
                .collect::<Vec<_>>();
            let infra_group_members = GroupMembership::group_members(connection, self.group_id())?;
            if mls_group_members.len() != infra_group_members.len() {
                info!(?mls_group_members, "Group members according to OpenMLS");
                info!(?infra_group_members, "Group members according to Infra");
                panic!("Group members don't match up");
            }
            let infra_indices =
                GroupMembership::client_indices(connection, self.group_id(), &infra_group_members)?;
            self.mls_group.members().for_each(|m| {
                let index = m.index;
                debug_assert!(infra_indices.contains(&index));
            });
        }
        Ok(event_messages)
    }

    /// Send an application message to the group.
    pub(super) fn create_message(
        &mut self,
        connection: &Connection,
        content: MimiContent,
    ) -> Result<SendMessageParamsOut, GroupOperationError> {
        let provider = &PhnxOpenMlsProvider::new(connection);
        let mls_message = self.mls_group.create_message(
            provider,
            &self.leaf_signer,
            &content.tls_serialize_detached()?,
        )?;

        let message = AssistedMessageOut::new(mls_message, None)?;

        let send_message_params = SendMessageParamsOut {
            sender: self.mls_group.own_leaf_index(),
            message,
        };

        Ok(send_message_params)
    }

    /// Get a reference to the group's group id.
    pub(crate) fn group_id(&self) -> &GroupId {
        self.mls_group().group_id()
    }

    pub(crate) fn user_auth_key(&self) -> Option<&UserAuthSigningKey> {
        self.user_auth_signing_key_option.as_ref()
    }

    pub(crate) fn group_state_ear_key(&self) -> &GroupStateEarKey {
        &self.group_state_ear_key
    }

    /// Returns the [`AsClientId`] of the clients owned by the given user.
    pub(crate) fn user_client_ids(
        &self,
        connection: &Connection,
        user_name: &QualifiedUserName,
    ) -> Vec<AsClientId> {
        match GroupMembership::user_client_ids(connection, self.group_id(), user_name) {
            Ok(user_client_ids) => user_client_ids,
            Err(error) => {
                error!(%error, "Could not retrieve user client IDs");
                Vec::new()
            }
        }
    }

    pub fn client_by_index(
        &self,
        connection: &Connection,
        index: LeafNodeIndex,
    ) -> Option<AsClientId> {
        GroupMembership::load(connection, self.group_id(), index)
            .ok()
            .flatten()
            .map(|group_membership| group_membership.client_id().clone())
    }

    pub(crate) fn credential_ear_key(&self) -> &ClientCredentialEarKey {
        &self.credential_ear_key
    }

    pub(crate) fn signature_ear_key_wrapper_key(&self) -> &SignatureEarKeyWrapperKey {
        &self.signature_ear_key_wrapper_key
    }

    /// Returns a set containing the [`UserName`] of the members of the group.
    pub(crate) fn members(&self, connection: &Connection) -> HashSet<QualifiedUserName> {
        let Ok(group_members) = GroupMembership::group_members(connection, self.group_id()) else {
            error!("Could not retrieve group members");
            return HashSet::new();
        };
        group_members
            .into_iter()
            .map(|client_id| client_id.user_name())
            // Collecting to a HashSet first to deduplicate.
            .collect::<HashSet<QualifiedUserName>>()
    }

    pub(super) fn update(&mut self, connection: &Connection) -> Result<UpdateClientParamsOut> {
        let provider = &PhnxOpenMlsProvider::new(connection);
        // We don't expect there to be a welcome.
        let aad_payload = UpdateClientParamsAad {
            option_encrypted_signature_ear_key: None,
            option_encrypted_client_credential: None,
        };
        let aad = InfraAadMessage::from(InfraAadPayload::UpdateClient(aad_payload))
            .tls_serialize_detached()?;
        self.mls_group.set_aad(aad);
        let (mls_message, _welcome_option, group_info_option) = self
            .mls_group
            .self_update(provider, &self.leaf_signer, LeafNodeParameters::default())
            .map_err(|e| anyhow!("Error performing group update: {:?}", e))?
            .into_messages();
        let group_info = group_info_option.ok_or(anyhow!("No group info after commit"))?;

        for remove in self
            .mls_group()
            .pending_commit()
            .ok_or(anyhow!("No pending commit after commit operation"))?
            .remove_proposals()
        {
            GroupMembership::stage_removal(
                connection,
                self.group_id(),
                remove.remove_proposal().removed(),
            )?;
        }
        let commit = AssistedMessageOut::new(mls_message, Some(group_info))?;
        Ok(UpdateClientParamsOut {
            commit,
            sender: self.mls_group.own_leaf_index(),
            new_user_auth_key_option: None,
        })
    }

    /// Update or set the user's auth key in this group.
    pub(super) fn update_user_key(
        &mut self,
        connection: &Connection,
    ) -> Result<UpdateClientParamsOut> {
        let provider = &PhnxOpenMlsProvider::new(connection);
        let aad_payload = UpdateClientParamsAad {
            option_encrypted_signature_ear_key: None,
            option_encrypted_client_credential: None,
        };
        let aad = InfraAadMessage::from(InfraAadPayload::UpdateClient(aad_payload))
            .tls_serialize_detached()?;
        self.mls_group.set_aad(aad);
        let (commit, _welcome_option, group_info_option) = self
            .mls_group
            .self_update(provider, &self.leaf_signer, LeafNodeParameters::default())
            .map_err(|e| anyhow!("Error performing group update: {:?}", e))?
            .into_messages();
        let group_info = group_info_option.ok_or(anyhow!("No group info after commit"))?;

        for remove in self
            .mls_group()
            .pending_commit()
            .ok_or(anyhow!("No pending commit after commit operation"))?
            .remove_proposals()
        {
            GroupMembership::stage_removal(
                connection,
                self.group_id(),
                remove.remove_proposal().removed(),
            )?;
        }

        let mut diff = GroupDiff::new();

        let user_auth_signing_key = UserAuthSigningKey::generate()?;
        let verifying_key = user_auth_signing_key.verifying_key().clone();

        diff.user_auth_key = Some(user_auth_signing_key);
        self.pending_diff = Some(diff.stage());

        let commit = AssistedMessageOut::new(commit, Some(group_info))?;
        let params = UpdateClientParamsOut {
            commit,
            sender: self.mls_group.own_leaf_index(),
            new_user_auth_key_option: Some(verifying_key),
        };
        Ok(params)
    }

    pub(super) fn leave_group(
        &mut self,
        connection: &Connection,
    ) -> Result<SelfRemoveClientParamsOut> {
        let provider = &PhnxOpenMlsProvider::new(connection);
        let Some(user_auth_key) = &self.user_auth_signing_key_option else {
            bail!("User auth key not set")
        };
        let proposal = self.mls_group.leave_group(provider, &self.leaf_signer)?;

        let assisted_message = AssistedMessageOut::new(proposal, None)?;
        let params = SelfRemoveClientParamsOut {
            remove_proposal: assisted_message,
            sender: user_auth_key.verifying_key().hash(),
        };
        Ok(params)
    }

    pub(crate) fn leaf_signer(&self) -> &InfraCredentialSigningKey {
        &self.leaf_signer
    }

    pub(super) fn store_proposal(
        &mut self,
        connection: &Connection,
        proposal: QueuedProposal,
    ) -> Result<()> {
        let provider = &PhnxOpenMlsProvider::new(connection);
        self.mls_group
            .store_pending_proposal(provider.storage(), proposal)?;
        Ok(())
    }

    pub(crate) fn pending_removes(&self, connection: &Connection) -> Vec<QualifiedUserName> {
        self.mls_group()
            .pending_proposals()
            .filter_map(|proposal| match proposal.proposal() {
                Proposal::Remove(rp) => self
                    .client_by_index(connection, rp.removed())
                    .map(|c| c.user_name()),
                _ => None,
            })
            .collect()
    }

    pub(crate) fn group_data(&self) -> Option<GroupData> {
        self.mls_group().extensions().iter().find_map(|e| match e {
            Extension::Unknown(GROUP_DATA_EXTENSION_TYPE, extension_bytes) => {
                Some(GroupData::from(extension_bytes.0.clone()))
            }
            _ => None,
        })
    }
}

impl TimestampedMessage {
    /// Turn a staged commit into a list of messages based on the proposals it
    /// includes.
    fn from_staged_commit(
        connection: &Connection,
        group_id: &GroupId,
        free_indices: impl Iterator<Item = LeafNodeIndex>,
        staged_commit: &StagedCommit,
        ds_timestamp: TimeStamp,
    ) -> Result<Vec<Self>> {
        // Collect the remover/removed pairs into a set to avoid duplicates.
        let removed_set = staged_commit
            .remove_proposals()
            .map(|remove_proposal| {
                let Sender::Member(sender_index) = remove_proposal.sender() else {
                    bail!("Only member proposals are supported for now")
                };
                let remover = if let Some(remover) =
                    ClientAuthInfo::load(connection, group_id, *sender_index)?
                {
                    remover
                } else {
                    // This is in case we removed ourselves.
                    ClientAuthInfo::load_staged(connection, group_id, *sender_index)?
                        .ok_or(anyhow!("Could not find client credential of remover"))?
                }
                .client_credential()
                .identity()
                .user_name();
                let removed_index = remove_proposal.remove_proposal().removed();
                let removed = ClientAuthInfo::load_staged(connection, group_id, removed_index)?
                    .ok_or(anyhow!("Could not find client credential of removed"))?
                    .client_credential()
                    .identity()
                    .user_name();
                Ok((remover, removed))
            })
            .collect::<Result<HashSet<_>>>()?;
        let remove_messages = removed_set.into_iter().map(|(remover, removed)| {
            TimestampedMessage::system_message(
                SystemMessage::Remove(remover, removed),
                ds_timestamp,
            )
        });

        // Collect adder and addee names and filter out duplicates
        let adds_set = staged_commit
            .add_proposals()
            .zip(free_indices)
            .map(|(staged_add_proposal, free_index)| {
                let Sender::Member(sender_index) = staged_add_proposal.sender() else {
                    // We don't support non-member adds.
                    bail!("Non-member add proposal")
                };
                // Get the name of the sender from the list of existing clients
                let sender_name = ClientAuthInfo::load(connection, group_id, *sender_index)?
                    .ok_or(anyhow!("Could not find client credential of sender"))?
                    .client_credential()
                    .identity()
                    .user_name();
                // Get the name of the added member from the diff containing
                // the new clients.
                let addee_name = ClientAuthInfo::load_staged(connection, group_id, free_index)?
                    .ok_or(anyhow!(
                        "Could not find client credential of added client at index {}",
                        free_index
                    ))?
                    .client_credential()
                    .identity()
                    .user_name();
                Ok((sender_name, addee_name))
            })
            .collect::<Result<HashSet<_>>>()?;
        let add_messages = adds_set.into_iter().map(|(adder, addee)| {
            TimestampedMessage::system_message(SystemMessage::Add(adder, addee), ds_timestamp)
        });

        let event_messages = remove_messages.chain(add_messages).collect();

        // Emit log messages for updates.
        staged_commit
            .update_proposals()
            .try_for_each(|staged_update_proposal| {
                let Sender::Member(sender_index) = staged_update_proposal.sender() else {
                    // Update proposals have to be sent by group members.
                    bail!("Invalid proposal")
                };
                let user_name = ClientAuthInfo::load(connection, group_id, *sender_index)?
                    .ok_or(anyhow!("Could not find client credential of sender"))?
                    .client_credential()
                    .identity()
                    .user_name();
                debug!(
                    %user_name,
                    %sender_index, "Client has updated their key material",
                );
                Ok(())
            })?;

        Ok(event_messages)
    }
}
