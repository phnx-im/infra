// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

pub(crate) mod diff;
pub(crate) mod error;
pub(crate) mod store;

pub(crate) use error::*;

use openmls_memory_keystore::MemoryKeyStore;
use phnxbackend::{
    auth_service::{
        credentials::{
            keys::{
                ClientSigningKey, InfraCredentialPlaintext, InfraCredentialSigningKey,
                InfraCredentialTbs,
            },
            AsIntermediateCredential, ClientCredential, VerifiableClientCredential,
        },
        AsClientId, UserName,
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
    ds::{
        api::QS_CLIENT_REFERENCE_EXTENSION_TYPE, WelcomeAttributionInfo,
        WelcomeAttributionInfoPayload, WelcomeAttributionInfoTbs,
    },
    messages::{
        client_ds::{
            AddUsersParamsAad, DsJoinerInformationIn, InfraAadMessage, InfraAadPayload,
            UpdateClientParamsAad, WelcomeBundle,
        },
        client_ds_out::{
            AddUsersParamsOut, DeleteGroupParamsOut, ExternalCommitInfoIn, RemoveUsersParamsOut,
            SelfRemoveClientParamsOut, SendMessageParamsOut, UpdateClientParamsOut,
        },
    },
    qs::{KeyPackageBatch, VERIFIED},
    AssistedGroupInfo, AssistedMessageOut,
};
pub(crate) use store::*;
use tls_codec::DeserializeBytes;

use crate::{
    contacts::{Contact, ContactAddInfos},
    conversations::*,
    types::MessageContentType,
    types::*,
};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    panic::panic_any,
};

use openmls::{prelude::*, treesync::RatchetTree};

use self::diff::GroupDiff;

pub const FRIENDSHIP_PACKAGE_PROPOSAL_TYPE: u16 = 0xff00;

pub const REQUIRED_EXTENSION_TYPES: [ExtensionType; 0] = [];
//pub const REQUIRED_EXTENSION_TYPES: [ExtensionType; 1] =
//    [ExtensionType::Unknown(QS_CLIENT_REFERENCE_EXTENSION_TYPE)];
pub const REQUIRED_PROPOSAL_TYPES: [ProposalType; 0] = [];
pub const REQUIRED_CREDENTIAL_TYPES: [CredentialType; 1] = [CredentialType::Infra];

// Default capabilities for every leaf node we create.
pub const SUPPORTED_PROTOCOL_VERSIONS: [ProtocolVersion; 1] = [ProtocolVersion::Mls10];
pub const SUPPORTED_CIPHERSUITES: [Ciphersuite; 1] =
    [Ciphersuite::MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519];
pub const SUPPORTED_EXTENSIONS: [ExtensionType; 1] =
    [ExtensionType::Unknown(QS_CLIENT_REFERENCE_EXTENSION_TYPE)];
pub const SUPPORTED_PROPOSALS: [ProposalType; 1] =
    [ProposalType::Unknown(FRIENDSHIP_PACKAGE_PROPOSAL_TYPE)];
pub const SUPPORTED_CREDENTIALS: [CredentialType; 1] = [CredentialType::Infra];

pub(crate) struct PartialCreateGroupParams {
    pub group_id: GroupId,
    pub ratchet_tree: RatchetTree,
    pub group_info: MlsMessageOut,
    pub user_auth_key: UserAuthVerifyingKey,
    pub encrypted_signature_ear_key: EncryptedSignatureEarKey,
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
    client_information: BTreeMap<usize, (ClientCredential, SignatureEarKey)>,
    pending_diff: Option<GroupDiff>,
}

impl Group {
    pub(crate) fn mls_group(&self) -> &MlsGroup {
        &self.mls_group
    }

    fn default_mls_group_config() -> MlsGroupConfig {
        let required_capabilities = RequiredCapabilitiesExtension::new(
            &REQUIRED_EXTENSION_TYPES,
            &REQUIRED_PROPOSAL_TYPES,
            &REQUIRED_CREDENTIAL_TYPES,
        );

        MlsGroupConfig::builder()
            // This is turned on for now, as it makes OpenMLS return GroupInfos
            // with every commit. At some point, there should be a dedicated
            // config flag for this.
            .leaf_node_capabilities(Capabilities::new(
                Some(&SUPPORTED_PROTOCOL_VERSIONS),
                Some(&SUPPORTED_CIPHERSUITES),
                Some(&SUPPORTED_EXTENSIONS),
                Some(&SUPPORTED_PROPOSALS),
                Some(&SUPPORTED_CREDENTIALS),
            ))
            .use_ratchet_tree_extension(true)
            .required_capabilities(required_capabilities)
            .wire_format_policy(PURE_PLAINTEXT_WIRE_FORMAT_POLICY)
            .build()
    }

    /// Create a group.
    pub fn create_group(
        backend: &impl OpenMlsCryptoProvider,
        signer: &ClientSigningKey,
        group_id: GroupId,
    ) -> Group {
        let credential_ear_key = ClientCredentialEarKey::random().unwrap();
        let user_auth_key = UserAuthSigningKey::generate().unwrap();
        let group_state_ear_key = GroupStateEarKey::random().unwrap();
        let signature_ear_key_wrapper_key = SignatureEarKeyWrapperKey::random().unwrap();

        let signature_ear_key = SignatureEarKey::random().unwrap();
        let leaf_signer = InfraCredentialSigningKey::generate(signer, &signature_ear_key);

        let mls_group_config = Self::default_mls_group_config();

        let credential_with_key = CredentialWithKey {
            credential: Credential::from(leaf_signer.credential().clone()),
            signature_key: leaf_signer.credential().verifying_key().clone(),
        };

        let mls_group = MlsGroup::new_with_group_id(
            backend,
            &leaf_signer,
            &mls_group_config,
            group_id.clone(),
            credential_with_key,
        )
        .unwrap();
        Group {
            group_id,
            leaf_signer,
            signature_ear_key_wrapper_key,
            mls_group,
            credential_ear_key,
            group_state_ear_key: group_state_ear_key.clone(),
            user_auth_signing_key_option: Some(user_auth_key),
            client_information: [(0, (signer.credential().clone(), signature_ear_key))].into(),
            pending_diff: None,
        }
    }

    pub(crate) fn create_group_params(
        &self,
        backend: &impl OpenMlsCryptoProvider,
    ) -> PartialCreateGroupParams {
        let Some(user_auth_key) = &self.user_auth_signing_key_option else {
            panic!("User auth key not set")
        };
        let (_own_credential, signature_ear_key) = self
            .client_information
            .get(&self.mls_group.own_leaf_index().usize())
            .unwrap();
        let encrypted_signature_ear_key = signature_ear_key
            .encrypt(self.signature_ear_key_wrapper_key())
            .unwrap();
        PartialCreateGroupParams {
            group_id: self.group_id.clone(),
            ratchet_tree: self.mls_group.export_ratchet_tree(),
            group_info: self
                .mls_group
                .export_group_info(backend, &self.leaf_signer, true)
                .unwrap(),
            user_auth_key: user_auth_key.verifying_key().clone(),
            encrypted_signature_ear_key,
        }
    }

    /// Join a group with the provided welcome message. Returns the group name.
    pub(crate) fn join_group(
        backend: &impl OpenMlsCryptoProvider<KeyStoreProvider = MemoryKeyStore>,
        welcome_bundle: WelcomeBundle,
        // This is our own key that the sender uses to encrypt to us. We should
        // be able to retrieve it from the client's key store.
        welcome_attribution_info_ear_key: &WelcomeAttributionInfoEarKey,
        leaf_signers: &mut HashMap<
            SignaturePublicKey,
            (InfraCredentialSigningKey, SignatureEarKey),
        >,
        as_intermediate_credentials: &[AsIntermediateCredential],
        contacts: &HashMap<UserName, Contact>,
    ) -> Result<Self, GroupOperationError> {
        //log::debug!("{} joining group ...", self_user.username);
        let serialized_welcome = welcome_bundle.welcome.tls_serialize_detached().unwrap();

        let mls_group_config = Self::default_mls_group_config();

        // Decrypt encrypted credentials s.t. we can afterwards consume the welcome.
        let (key_package, _) = welcome_bundle
            .welcome
            .welcome
            .secrets()
            .iter()
            .find_map(|egs| {
                let hash_ref = egs.new_member().as_slice().to_vec();
                backend
                    .key_store()
                    .read(&hash_ref)
                    .map(|kp: KeyPackage| (kp, hash_ref))
            })
            .unwrap();

        let private_key = backend
            .key_store()
            .read::<HpkePrivateKey>(key_package.hpke_init_key().as_slice())
            .unwrap();
        let info = &[];
        let aad = &[];
        let decryption_key =
            JoinerInfoDecryptionKey::from((private_key, key_package.hpke_init_key().clone()));
        let joiner_info = DsJoinerInformationIn::decrypt(
            welcome_bundle.encrypted_joiner_info,
            &decryption_key,
            info,
            aad,
        )
        .unwrap();

        let mls_group = match MlsGroup::new_from_welcome(
            backend,
            &mls_group_config,
            welcome_bundle.welcome.welcome,
            None, /* no public tree here, has to be in the extension */
        ) {
            Ok(g) => g,
            Err(e) => {
                let s = format!("Error creating group from Welcome: {:?}", e);
                log::info!("{}", s);
                return Err(GroupOperationError::WelcomeError(e));
            }
        };

        // Decrypt WelcomeAttributionInfo
        let welcome_attribution_info = WelcomeAttributionInfo::decrypt(
            welcome_attribution_info_ear_key,
            &welcome_bundle.encrypted_attribution_info,
        )
        .unwrap();

        let verifiable_attribution_info = welcome_attribution_info
            .into_verifiable(mls_group.group_id().clone(), serialized_welcome);

        let sender_client_credential = contacts
            .get(&verifiable_attribution_info.sender().user_name())
            .and_then(|c| c.client_credential(&verifiable_attribution_info.sender()))
            .unwrap();

        let welcome_attribution_info: WelcomeAttributionInfoPayload = verifiable_attribution_info
            .verify(sender_client_credential.verifying_key())
            .unwrap();

        let client_information: BTreeMap<usize, (ClientCredential, SignatureEarKey)> = joiner_info
            .encrypted_client_information
            .into_iter()
            .enumerate()
            .filter_map(|(index, ciphertext_option)| {
                if let Some(client_information) = ciphertext_option.map(|(ecc, esek)| {
                    let verifiable_credential = VerifiableClientCredential::decrypt(
                        welcome_attribution_info.client_credential_encryption_key(),
                        &ecc,
                    )
                    .unwrap();
                    let as_credential = as_intermediate_credentials
                        .iter()
                        .find(|as_cred| {
                            &as_cred.fingerprint().unwrap()
                                == verifiable_credential.signer_fingerprint()
                        })
                        .unwrap();
                    let credential = verifiable_credential
                        .verify(as_credential.verifying_key())
                        .unwrap();
                    let sek = SignatureEarKey::decrypt(
                        welcome_attribution_info.signature_ear_key_wrapper_key(),
                        &esek,
                    )
                    .unwrap();
                    (credential, sek)
                }) {
                    Some((index, client_information))
                } else {
                    None
                }
            })
            .collect();

        let verifying_key = mls_group.own_leaf_node().unwrap().signature_key();
        let (leaf_signer, _signature_ear_key) = leaf_signers.remove(verifying_key).unwrap();

        // Decrypt and verify the infra credentials.
        // TODO: Right now, this just panics if the verification fails.
        mls_group
            .members()
            .for_each(|m| match m.credential.mls_credential_type() {
                MlsCredentialType::Infra(credential) => {
                    let (client_credential, signature_ear_key) =
                        client_information.get(&m.index.usize()).unwrap().clone();
                    let _verified_credential: InfraCredentialTbs =
                        InfraCredentialPlaintext::decrypt(credential, &signature_ear_key)
                            .unwrap()
                            .verify(client_credential.verifying_key())
                            .unwrap();
                }
                _ => panic_any("We should only use infra credentials."),
            });

        let group = Group {
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
            client_information,
            pending_diff: None,
        };

        Ok(group)
    }

    /// Join a group using an external commit.
    pub(crate) fn join_group_externally(
        backend: &impl OpenMlsCryptoProvider<KeyStoreProvider = MemoryKeyStore>,
        external_commit_info: ExternalCommitInfoIn,
        leaf_signer: InfraCredentialSigningKey,
        signature_ear_key: SignatureEarKey,
        group_state_ear_key: GroupStateEarKey,
        signature_ear_key_wrapper_key: SignatureEarKeyWrapperKey,
        credential_ear_key: ClientCredentialEarKey,
        as_intermediate_credentials: &[AsIntermediateCredential],
        aad: InfraAadMessage,
        own_client_credential: &ClientCredential,
    ) -> Result<(Self, MlsMessageOut, MlsMessageOut), GroupOperationError> {
        // TODO: We set the ratchet tree extension for now, as it is the only
        // way to make OpenMLS return a GroupInfo. This should change in the
        // future.
        let mls_group_config = Self::default_mls_group_config();
        let credential_with_key = CredentialWithKey {
            credential: leaf_signer.credential().clone().into(),
            signature_key: leaf_signer.credential().verifying_key().clone(),
        };
        let ExternalCommitInfoIn {
            verifiable_group_info,
            ratchet_tree_in,
            encrypted_client_info,
        } = external_commit_info;

        // Let's create the group first so that we can access the GroupId.
        let (mut mls_group, commit, group_info_option) = MlsGroup::join_by_external_commit(
            backend,
            &leaf_signer,
            Some(ratchet_tree_in),
            verifiable_group_info,
            &mls_group_config,
            &aad.tls_serialize_detached().unwrap(),
            credential_with_key,
        )
        .unwrap();
        mls_group.set_aad(&[]);
        mls_group.merge_pending_commit(backend).unwrap();

        let group_info = group_info_option.unwrap();

        let mut client_information: BTreeMap<usize, (ClientCredential, SignatureEarKey)> =
            encrypted_client_info
                .into_iter()
                .enumerate()
                .filter_map(|(index, ciphertext_option)| {
                    if let Some(client_info) = ciphertext_option.map(|(ecc, esek)| {
                        let verifiable_credential =
                            VerifiableClientCredential::decrypt(&credential_ear_key, &ecc).unwrap();
                        let as_credential = as_intermediate_credentials
                            .iter()
                            .find(|as_cred| {
                                &as_cred.fingerprint().unwrap()
                                    == verifiable_credential.signer_fingerprint()
                            })
                            .unwrap();
                        let client_credential = verifiable_credential
                            .verify(as_credential.verifying_key())
                            .unwrap();
                        let sek = SignatureEarKey::decrypt(&signature_ear_key_wrapper_key, &esek)
                            .unwrap();
                        (client_credential, sek)
                    }) {
                        Some((index, client_info))
                    } else {
                        None
                    }
                })
                .collect();

        // We still have to add ourselves to the encrypted client credentials.
        let own_client_credential = own_client_credential.clone();
        let own_signature_ear_key = signature_ear_key.clone();
        let own_index = mls_group.own_leaf_index().usize();
        debug_assert!(client_information.get(&own_index).is_none());
        client_information.insert(own_index, (own_client_credential, own_signature_ear_key));

        // Decrypt and verify the infra credentials.
        // TODO: Right now, this just panics if the verification fails.
        mls_group
            .members()
            .for_each(|m| match m.credential.mls_credential_type() {
                MlsCredentialType::Infra(credential) => {
                    let (client_credential, signature_ear_key) =
                        client_information.get(&m.index.usize()).unwrap();
                    let _verified_credential: InfraCredentialTbs =
                        InfraCredentialPlaintext::decrypt(credential, &signature_ear_key)
                            .unwrap()
                            .verify(client_credential.verifying_key())
                            .unwrap();
                }
                _ => panic_any("We should only use infra credentials."),
            });

        // TODO: Once we support multiple clients, this should be synchronized
        // across clients.
        let user_auth_key = UserAuthSigningKey::generate().unwrap();

        let group = Group {
            group_id: mls_group.group_id().clone(),
            mls_group,
            leaf_signer,
            signature_ear_key_wrapper_key,
            credential_ear_key,
            group_state_ear_key,
            user_auth_signing_key_option: Some(user_auth_key),
            client_information,
            pending_diff: None,
        };

        Ok((group, commit, group_info.into()))
    }

    /// Process inbound message
    ///
    /// Returns the processed message and whether the group was deleted.
    pub(crate) fn process_message(
        &mut self,
        backend: &impl OpenMlsCryptoProvider<KeyStoreProvider = MemoryKeyStore>,
        message: impl Into<ProtocolMessage>,
        // Required in case there are new joiners.
        // TODO: In the federated case, we might have to fetch them first.
        as_intermediate_credentials: &[AsIntermediateCredential],
    ) -> Result<(ProcessedMessage, bool, ClientCredential), GroupOperationError> {
        let processed_message = self.mls_group.process_message(backend, message)?;

        // Will be set to true if we were removed (or the group was deleted).
        let mut we_were_removed = false;
        let mut diff = GroupDiff::new(self);
        let sender_index = match processed_message.content() {
            // For now, we only care about commits.
            ProcessedMessageContent::ExternalJoinProposalMessage(_) => {
                panic!("Unsupported message type")
            }
            ProcessedMessageContent::ApplicationMessage(_) => {
                let (sender_credential, _) =
                    if let Sender::Member(index) = processed_message.sender() {
                        self.client_information.get(&index.usize()).unwrap()
                    } else {
                        panic!("Invalid sender type.")
                    };
                return Ok((processed_message, false, sender_credential.clone()));
            }
            ProcessedMessageContent::ProposalMessage(_proposal) => {
                // Proposals are just returned and can then be added to the
                // proposal store after the caller has inspected them.
                let sender_index = if let Sender::Member(index) = processed_message.sender() {
                    index.usize()
                } else {
                    panic!("Invalid sender type.")
                };
                sender_index
            }
            ProcessedMessageContent::StagedCommitMessage(staged_commit) => {
                // Before we process the AAD payload, we first process the
                // proposals by value. Currently only removes are allowed.
                for remove_proposal in staged_commit.remove_proposals() {
                    let removed_member = remove_proposal.remove_proposal().removed();
                    diff.remove_client_credential(removed_member);
                    if removed_member == self.mls_group().own_leaf_index() {
                        we_were_removed = true;
                    }
                }
                // Let's figure out which operation this is meant to be.
                let aad_payload =
                    InfraAadMessage::tls_deserialize_exact(processed_message.authenticated_data())
                        .unwrap()
                        .into_payload();
                let sender_index = match processed_message.sender() {
                    Sender::Member(index) => index.to_owned(),
                    Sender::NewMemberCommit => self
                        .mls_group
                        .ext_commit_sender_index(staged_commit)
                        .unwrap(),
                    Sender::External(_) | Sender::NewMemberProposal => {
                        panic!("Invalid sender type.")
                    }
                }
                .usize();
                match aad_payload {
                    InfraAadPayload::AddUsers(add_users_payload) => {
                        let client_credentials: Vec<(ClientCredential, SignatureEarKey)> =
                            add_users_payload
                                .encrypted_credential_information
                                .into_iter()
                                .map(|(ciphertext, esek)| {
                                    let client_credential = ClientCredential::decrypt_and_verify(
                                        &self.credential_ear_key,
                                        &ciphertext,
                                        as_intermediate_credentials,
                                    )
                                    .unwrap();
                                    let sek = SignatureEarKey::decrypt(
                                        &self.signature_ear_key_wrapper_key,
                                        &esek,
                                    )
                                    .unwrap();
                                    (client_credential, sek)
                                })
                                .collect();

                        // TODO: Validation:
                        // * Check that this commit only contains (inline) add proposals
                        // * Check that the leaf credential is not changed in the path
                        //   (or maybe if it is, check that it's valid).
                        // * User names MUST be unique within the group (check both new
                        //   and existing credentials for duplicates).
                        // * Client IDs MUST be unique within the group (only need to
                        //   check new credentials, as client IDs are scoped to user
                        //   names).
                        // * Once we do RBAC, check that the adder has sufficient
                        //   permissions.
                        // * Maybe check sender type (only Members can add users).

                        // Verify the leaf credentials in all add proposals. We assume
                        // that leaf credentials are in the same order as client
                        // credentials.
                        for (index, proposal) in staged_commit.add_proposals().enumerate() {
                            let (client_credential, signature_ear_key) =
                                client_credentials.get(index).unwrap();
                            match proposal
                                .add_proposal()
                                .key_package()
                                .leaf_node()
                                .credential()
                                .mls_credential_type()
                            {
                                MlsCredentialType::Basic(_) | MlsCredentialType::X509(_) => {
                                    panic!("Unsupported credential type.")
                                }
                                MlsCredentialType::Infra(infra_credential) => {
                                    // Verify the leaf credential
                                    let credential_plaintext = InfraCredentialPlaintext::decrypt(
                                        infra_credential,
                                        &signature_ear_key,
                                    )
                                    .unwrap();
                                    credential_plaintext
                                        .verify::<InfraCredentialTbs>(
                                            client_credential.verifying_key(),
                                        )
                                        .unwrap();
                                }
                            }
                        }

                        // Add the client credentials to the group.
                        for client_credential in client_credentials {
                            diff.add_client_information(&self.client_information, client_credential)
                        }
                    }
                    InfraAadPayload::UpdateClient(update_client_payload) => {
                        let sender_index = if let Sender::Member(index) = processed_message.sender()
                        {
                            index.usize()
                        } else {
                            panic!("Unsupported sender type.")
                        };
                        // Check if the client has updated its leaf credential.
                        let (client_credential, signature_ear_key) =
                            if processed_message.new_credential_option().is_some() {
                                // If so, then there has to be a new signature ear key.
                                let Some(encrypted_signature_ear_key) = update_client_payload
                                .option_encrypted_signature_ear_key else {
                                    panic!("Invalid update client payload.")
                                };
                                let signature_ear_key = SignatureEarKey::decrypt(
                                    &self.signature_ear_key_wrapper_key,
                                    &encrypted_signature_ear_key,
                                )
                                .unwrap();
                                // Optionally, the client could have updated its
                                // client credential.
                                let client_credential = if let Some(ecc) =
                                    update_client_payload.option_encrypted_client_credential
                                {
                                    let client_credential = ClientCredential::decrypt_and_verify(
                                        &self.credential_ear_key,
                                        &ecc,
                                        as_intermediate_credentials,
                                    )
                                    .unwrap();
                                    client_credential
                                } else {
                                    self.client_information
                                        .get(&sender_index)
                                        .unwrap()
                                        .0
                                        .clone()
                                };
                                diff.add_client_information(
                                    &self.client_information,
                                    (client_credential.clone(), signature_ear_key.clone()),
                                );
                                (client_credential, signature_ear_key)
                            } else {
                                // Otherwise, we just use the existing client credential.
                                self.client_information.get(&sender_index).unwrap().clone()
                            };
                        // TODO: Validation:
                        // * Check that the sender type fits.
                        // * Check that the client id is the same as before.
                        // * Check that the proposals fit the operation (i.e. in this
                        //   case that there are no proposals at all).

                        // Verify a potential new leaf credential.
                        if let Some(MlsCredentialType::Infra(infra_credential)) = processed_message
                            .new_credential_option()
                            .map(|cred| cred.mls_credential_type())
                        {
                            // Verify the leaf credential
                            let credential_plaintext = InfraCredentialPlaintext::decrypt(
                                infra_credential,
                                &signature_ear_key,
                            )
                            .unwrap();
                            credential_plaintext
                                .verify::<InfraCredentialTbs>(client_credential.verifying_key())
                                .unwrap();
                        }
                    }
                    InfraAadPayload::JoinGroup(join_group_payload) => {
                        // Decrypt and verify the client credential.
                        let (ecc, esek) = join_group_payload.encrypted_client_information;
                        let client_credential = ClientCredential::decrypt_and_verify(
                            &self.credential_ear_key,
                            &ecc,
                            as_intermediate_credentials,
                        )
                        .unwrap();
                        let sek =
                            SignatureEarKey::decrypt(&self.signature_ear_key_wrapper_key, &esek)
                                .unwrap();
                        // Validate the leaf credential.
                        if let MlsCredentialType::Infra(infra_credential) =
                            processed_message.credential().mls_credential_type()
                        {
                            // Verify the leaf credential
                            let credential_plaintext =
                                InfraCredentialPlaintext::decrypt(infra_credential, &sek).unwrap();
                            credential_plaintext
                                .verify::<InfraCredentialTbs>(client_credential.verifying_key())
                                .unwrap();
                        }
                        // Check that the existing user clients match up.
                        if self.user_client_indices(client_credential.identity().user_name())
                            != join_group_payload
                                .existing_user_clients
                                .into_iter()
                                .map(|index| index.usize())
                                .collect::<Vec<_>>()
                        {
                            panic!("User clients don't match up.")
                        };
                        // TODO: (More) validation:
                        // * Check that the client id is unique.
                        // * Check that the proposals fit the operation.
                        // Insert the client credential into the diff.
                        diff.add_client_information(
                            &self.client_information,
                            (client_credential, sek),
                        );
                    }
                    InfraAadPayload::JoinConnectionGroup(join_connection_group_payload) => {
                        let (ecc, esek) =
                            join_connection_group_payload.encrypted_client_information;
                        // Decrypt and verify the client credential.
                        let client_credential = ClientCredential::decrypt_and_verify(
                            &self.credential_ear_key,
                            &ecc,
                            as_intermediate_credentials,
                        )
                        .unwrap();
                        let sek =
                            SignatureEarKey::decrypt(&self.signature_ear_key_wrapper_key, &esek)
                                .unwrap();
                        // Validate the leaf credential.
                        if let MlsCredentialType::Infra(infra_credential) =
                            processed_message.credential().mls_credential_type()
                        {
                            // Verify the leaf credential
                            let credential_plaintext =
                                InfraCredentialPlaintext::decrypt(infra_credential, &sek).unwrap();
                            credential_plaintext
                                .verify::<InfraCredentialTbs>(client_credential.verifying_key())
                                .unwrap();
                        }
                        // TODO: (More) validation:
                        // * Check that the user name is unique.
                        // * Check that the proposals fit the operation.
                        // * Check that the sender type fits the operation.
                        // * Check that this group is indeed a connection group.

                        // Insert the client credential into the diff.
                        diff.add_client_information(
                            &self.client_information,
                            (client_credential, sek),
                        );
                    }
                    InfraAadPayload::AddClients(add_clients_payload) => {
                        let client_credentials: Vec<(ClientCredential, SignatureEarKey)> =
                            add_clients_payload
                                .encrypted_client_information
                                .into_iter()
                                .map(|(ecc, esek)| {
                                    let client_credential = ClientCredential::decrypt_and_verify(
                                        &self.credential_ear_key,
                                        &ecc,
                                        as_intermediate_credentials,
                                    )
                                    .unwrap();
                                    let sek = SignatureEarKey::decrypt(
                                        &self.signature_ear_key_wrapper_key,
                                        &esek,
                                    )
                                    .unwrap();
                                    (client_credential, sek)
                                })
                                .collect();

                        // TODO: Validation:
                        // * Check that this commit only contains (inline) add proposals
                        // * Check that the leaf credential is not changed in the path
                        //   (or maybe if it is, check that it's valid).
                        // * Client IDs MUST be unique within the group.
                        // * Maybe check sender type (only Members can add users).

                        // Verify the leaf credentials in all add proposals. We assume
                        // that leaf credentials are in the same order as client
                        // credentials.
                        for (index, proposal) in staged_commit.add_proposals().enumerate() {
                            let (client_credential, sek) = client_credentials.get(index).unwrap();
                            match proposal
                                .add_proposal()
                                .key_package()
                                .leaf_node()
                                .credential()
                                .mls_credential_type()
                            {
                                MlsCredentialType::Basic(_) | MlsCredentialType::X509(_) => {
                                    panic!("Unsupported credential type.")
                                }
                                MlsCredentialType::Infra(infra_credential) => {
                                    // Verify the leaf credential
                                    let credential_plaintext =
                                        InfraCredentialPlaintext::decrypt(infra_credential, &sek)
                                            .unwrap();
                                    credential_plaintext
                                        .verify::<InfraCredentialTbs>(
                                            client_credential.verifying_key(),
                                        )
                                        .unwrap();
                                }
                            }
                        }

                        // Add the client credentials to the group.
                        for client_credential in client_credentials {
                            diff.add_client_information(&self.client_information, client_credential)
                        }
                    }
                    InfraAadPayload::RemoveUsers | InfraAadPayload::RemoveClients => {
                        // We already processed remove proposals above, so there is nothing to do here.
                        // TODO: Validation:
                        // * Check that this commit only contains (inline) remove proposals
                        // * Check that the sender type is correct.
                        // * Check that the leaf credential is not changed in the path
                        // * Check that the remover has sufficient privileges.
                    }
                    InfraAadPayload::ResyncClient => {
                        // TODO: Validation:
                        // * Check that this commit contains exactly one remove proposal
                        // * Check that the sender type is correct (external commit).

                        let removed_index = staged_commit
                            .remove_proposals()
                            .next()
                            .unwrap()
                            .remove_proposal()
                            .removed();
                        let (client_credential, sek) =
                            self.client_information.get(&removed_index.usize()).unwrap();
                        // Let's verify the new leaf credential.
                        match processed_message.credential().mls_credential_type() {
                            MlsCredentialType::Basic(_) | MlsCredentialType::X509(_) => {
                                panic!("Unsupported credential type.")
                            }
                            MlsCredentialType::Infra(infra_credential) => {
                                // Verify the leaf credential
                                let credential_plaintext =
                                    InfraCredentialPlaintext::decrypt(infra_credential, &sek)
                                        .unwrap();
                                credential_plaintext
                                    .verify::<InfraCredentialTbs>(client_credential.verifying_key())
                                    .unwrap();
                            }
                        }

                        // Move the client credential to the new index.
                        diff.remove_client_credential(removed_index);
                        diff.add_client_information(
                            &self.client_information,
                            (client_credential.clone(), sek.clone()),
                        );
                    }
                    InfraAadPayload::DeleteGroup => {
                        we_were_removed = true;
                        // There is nothing else to do at this point.
                    }
                };
                sender_index
            }
        };
        // Get the sender's credential
        let (sender_credential, _sek) = diff
            .client_information(sender_index, &self.client_information)
            .unwrap()
            .clone();
        self.pending_diff = Some(diff);

        Ok((processed_message, we_were_removed, sender_credential))
    }

    /// Invite the given list of contacts to join the group.
    ///
    /// Returns the [`AddUserParamsOut`] as input for the API client.
    pub(crate) fn invite(
        &mut self,
        backend: &impl OpenMlsCryptoProvider<KeyStoreProvider = MemoryKeyStore>,
        signer: &ClientSigningKey,
        // The following three vectors have to be in sync, i.e. of the same length
        // and refer to the same contacts in order.
        add_infos: Vec<ContactAddInfos>,
        wai_keys: Vec<WelcomeAttributionInfoEarKey>,
        client_credentials: Vec<Vec<ClientCredential>>,
    ) -> Result<AddUsersParamsOut, GroupOperationError> {
        let Some(user_auth_key) = &self.user_auth_signing_key_option else {
            return Err(GroupOperationError::NoUserAuthKey);
        };
        let client_credentials = client_credentials.into_iter().flatten().collect::<Vec<_>>();
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
                let ecc = client_credential.encrypt(&self.credential_ear_key).unwrap();
                let esek = sek.encrypt(&self.signature_ear_key_wrapper_key).unwrap();
                (ecc, esek)
            })
            .collect::<Vec<_>>();
        let aad_message: InfraAadMessage = InfraAadPayload::AddUsers(AddUsersParamsAad {
            encrypted_credential_information: ecc,
        })
        .into();
        // Set Aad to contain the encrypted client credentials.
        self.mls_group
            .set_aad(&aad_message.tls_serialize_detached().unwrap());
        let (mls_commit, welcome, group_info_option) =
            self.mls_group
                .add_members(backend, &self.leaf_signer, key_packages.as_slice())?;
        // Reset Aad to empty.
        self.mls_group.set_aad(&[]);

        // Groups should always have the flag set that makes them return groupinfos with every Commit.
        // Or at least with Add commits for now.
        let group_info = group_info_option.unwrap();
        // TODO: For now, we use the full group info, as OpenMLS does not yet allow splitting up a group info.
        let assisted_group_info = AssistedGroupInfo::Full(group_info.into());
        let commit = AssistedMessageOut {
            mls_message: mls_commit,
            group_info_option: Some(assisted_group_info),
        };

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
                    group_id: self.group_id.clone(),
                    welcome: welcome.tls_serialize_detached().unwrap(),
                }
                .sign(signer)
                .unwrap();
                wai.encrypt(wai_key).unwrap()
            })
            .collect();

        // Create the GroupDiff
        let mut diff = GroupDiff::new(&self);
        diff.apply_pending_removes(self.mls_group().pending_commit().unwrap());
        for client_information in client_credentials
            .into_iter()
            .zip(signature_ear_keys.into_iter())
        {
            diff.add_client_information(&self.client_information, client_information)
        }

        self.pending_diff = Some(diff);

        let params = AddUsersParamsOut {
            commit,
            sender: user_auth_key.verifying_key().hash(),
            welcome,
            encrypted_welcome_attribution_infos,
            key_package_batches,
        };
        Ok(params)
    }

    pub(crate) fn remove(
        &mut self,
        backend: &impl OpenMlsCryptoProvider<KeyStoreProvider = MemoryKeyStore>,
        members: Vec<AsClientId>,
    ) -> Result<RemoveUsersParamsOut, GroupOperationError> {
        let Some(user_auth_key) = &self.user_auth_signing_key_option else {
            return Err(GroupOperationError::NoUserAuthKey);
        };
        let remove_indices = self
            .client_information
            .iter()
            .filter_map(|(index, (cred, _sek))| {
                if members.contains(&cred.identity()) {
                    Some(LeafNodeIndex::new(*index as u32))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        // There shouldn't be a welcome
        let aad_payload = InfraAadPayload::RemoveUsers;
        let aad = InfraAadMessage::from(aad_payload)
            .tls_serialize_detached()
            .unwrap();
        self.mls_group.set_aad(aad.as_slice());
        let (mls_message, _welcome_option, group_info_option) = self
            .mls_group
            .remove_members(backend, &self.leaf_signer, remove_indices.as_slice())
            .unwrap();
        self.mls_group.set_aad(&[]);
        debug_assert!(_welcome_option.is_none());
        let group_info = group_info_option.unwrap();
        let assisted_group_info = AssistedGroupInfo::Full(group_info.into());
        let commit = AssistedMessageOut {
            mls_message,
            group_info_option: Some(assisted_group_info),
        };

        let mut diff = GroupDiff::new(&self);
        diff.apply_pending_removes(self.mls_group().pending_commit().unwrap());
        for index in remove_indices {
            diff.remove_client_credential(index);
        }
        self.pending_diff = Some(diff);

        let params = RemoveUsersParamsOut {
            commit,
            sender: user_auth_key.verifying_key().hash(),
        };
        Ok(params)
    }

    pub(crate) fn delete(
        &mut self,
        backend: &impl OpenMlsCryptoProvider<KeyStoreProvider = MemoryKeyStore>,
    ) -> Result<DeleteGroupParamsOut, GroupOperationError> {
        let Some(user_auth_key) = &self.user_auth_signing_key_option else {
            return Err(GroupOperationError::NoUserAuthKey);
        };
        let remove_indices = self
            .client_information
            .keys()
            .filter_map(|&index| {
                if index != self.mls_group.own_leaf_index().usize() {
                    Some(LeafNodeIndex::new(index as u32))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        // There shouldn't be a welcome
        let aad_payload = InfraAadPayload::DeleteGroup;
        let aad = InfraAadMessage::from(aad_payload)
            .tls_serialize_detached()
            .unwrap();
        self.mls_group.set_aad(aad.as_slice());
        let (mls_message, _welcome_option, group_info_option) = self
            .mls_group
            .remove_members(backend, &self.leaf_signer, remove_indices.as_slice())
            .unwrap();
        self.mls_group.set_aad(&[]);
        debug_assert!(_welcome_option.is_none());
        let group_info = group_info_option.unwrap();
        let assisted_group_info = AssistedGroupInfo::Full(group_info.into());
        let commit = AssistedMessageOut {
            mls_message,
            group_info_option: Some(assisted_group_info),
        };

        let mut diff = GroupDiff::new(&self);
        diff.apply_pending_removes(self.mls_group().pending_commit().unwrap());
        for index in remove_indices {
            diff.remove_client_credential(index);
        }
        self.pending_diff = Some(diff);

        let params = DeleteGroupParamsOut {
            commit,
            sender: user_auth_key.verifying_key().hash(),
        };
        Ok(params)
    }

    /// If a [`StagedCommit`] is given, merge it and apply the pending group
    /// diff. If no [`StagedCommit`] is given, merge any pending commit and
    /// apply the pending group diff.
    pub(crate) fn merge_pending_commit(
        &mut self,
        backend: &impl OpenMlsCryptoProvider<KeyStoreProvider = MemoryKeyStore>,
        staged_commit_option: impl Into<Option<StagedCommit>>,
    ) -> Result<Vec<ConversationMessage>, GroupOperationError> {
        // Collect free indices s.t. we know where the added members will land
        // and we can look up their identifies later.
        let highest_index = self
            .client_information
            .last_key_value()
            .map(|(index, _)| *index)
            .unwrap();
        let free_indices: Vec<usize> = (0..2 * highest_index)
            .filter(|index| self.client_information.get(index).is_none())
            .collect();
        let staged_commit_option: Option<StagedCommit> = staged_commit_option.into();
        // Now we figure out who was removed. We do that before the diff is
        // applied s.t. we still have access to the user identities of the
        // removed members.
        let remover_removed: Vec<_> = if let Some(staged_commit) = self
            .mls_group
            .pending_commit()
            .or_else(|| staged_commit_option.as_ref())
        {
            staged_commit
                .remove_proposals()
                .map(|remove_proposal| {
                    if let Sender::Member(sender_index) = remove_proposal.sender() {
                        let remover = get_user_name(&self.client_information, sender_index.usize());
                        let removed_index = remove_proposal.remove_proposal().removed();
                        let removed =
                            get_user_name(&self.client_information, removed_index.usize());
                        (remover, removed)
                    } else {
                        panic!("Only member proposals are supported for now")
                    }
                })
                .collect()
        } else {
            vec![]
        };
        let mut messages: Vec<ConversationMessage> = remover_removed
            .into_iter()
            .map(|(remover, removed)| {
                let event_message = if remover == removed {
                    format!("{} left the conversation", remover.to_string(),)
                } else {
                    format!(
                        "{} removed {} from the conversation",
                        remover.to_string(),
                        removed.to_string()
                    )
                };
                event_message_from_string(event_message)
            })
            .collect();
        // We now apply the diff
        let Some(diff) = self.pending_diff.take() else {
            return Err(GroupOperationError::NoPendingGroupDiff);
        };
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
        for (index, client_information_option) in diff.client_information {
            if let Some(client_information) = client_information_option {
                let collision_entry = self.client_information.insert(index, client_information);
                debug_assert!(collision_entry.is_none());
            } else {
                let collision_entry = self.client_information.remove(&index);
                debug_assert!(collision_entry.is_some());
            }
        }
        self.pending_diff = None;
        let mut staged_commit_messages = if let Some(staged_commit) = staged_commit_option {
            let staged_commit_messages = staged_commit_to_conversation_messages(
                free_indices.iter().copied(),
                &self.client_information,
                &staged_commit,
            );
            self.mls_group.merge_staged_commit(backend, staged_commit)?;
            staged_commit_messages
        } else {
            // If we're merging a pending commit, we need to check if we have
            // committed a remove proposal by reference. If we have, we need to
            // create a notification message.
            let staged_commit_messages =
                if let Some(staged_commit) = self.mls_group.pending_commit() {
                    staged_commit_to_conversation_messages(
                        free_indices.iter().copied(),
                        &self.client_information,
                        staged_commit,
                    )
                } else {
                    vec![]
                };
            self.mls_group.merge_pending_commit(backend)?;
            staged_commit_messages
        };
        // Debug sanity checks after merging.
        #[cfg(debug_assertions)]
        {
            let mls_group_members = self.mls_group.members().count();
            let infra_group_members = self.client_information.len();
            debug_assert_eq!(mls_group_members, infra_group_members);
            self.mls_group.members().for_each(|m| {
                let index = m.index.usize();
                let client_information = self.client_information.get(&index);
                debug_assert!(client_information.is_some())
            });
        }
        messages.append(&mut staged_commit_messages);
        Ok(messages)
    }

    /// Send an application message to the group.
    pub fn create_message(
        &mut self,
        backend: &impl OpenMlsCryptoProvider<KeyStoreProvider = MemoryKeyStore>,
        msg: MessageContentType,
    ) -> Result<SendMessageParamsOut, GroupOperationError> {
        let mls_message = self.mls_group.create_message(
            backend,
            &self.leaf_signer,
            &msg.tls_serialize_detached()?,
        )?;

        let message = AssistedMessageOut {
            mls_message,
            group_info_option: None,
        };

        let send_message_params = SendMessageParamsOut {
            sender: self.mls_group.own_leaf_index(),
            message,
        };
        Ok(send_message_params)
    }

    /// Get a reference to the group's group id.
    pub(crate) fn group_id(&self) -> GroupId {
        self.group_id.clone()
    }

    pub(crate) fn user_auth_key(&self) -> Option<&UserAuthSigningKey> {
        self.user_auth_signing_key_option.as_ref()
    }

    pub(crate) fn epoch(&self) -> GroupEpoch {
        self.mls_group.epoch()
    }

    pub(crate) fn group_state_ear_key(&self) -> &GroupStateEarKey {
        &self.group_state_ear_key
    }

    pub(crate) fn pending_commit(&self) -> Option<&StagedCommit> {
        self.mls_group.pending_commit()
    }

    /// Returns the leaf indices of the clients owned by the given user.
    pub(crate) fn user_client_indices(&self, user_name: UserName) -> Vec<usize> {
        let mut user_clients = vec![];
        for (index, (cred, _sek)) in self.client_information.iter() {
            if cred.identity().user_name() == user_name {
                user_clients.push(*index)
            }
        }
        user_clients
    }

    /// Returns the [`AsClientId`] of the clients owned by the given user.
    pub(crate) fn user_client_ids(&self, user_name: &UserName) -> Vec<AsClientId> {
        let mut user_clients = vec![];
        for (_index, (cred, _sek)) in self.client_information.iter() {
            if &cred.identity().user_name() == user_name {
                user_clients.push(cred.identity())
            }
        }
        user_clients
    }

    pub fn client_by_index(&self, index: usize) -> Option<AsClientId> {
        self.client_information
            .get(&index)
            .map(|(cred, _sek)| cred.identity())
    }

    pub(crate) fn credential_ear_key(&self) -> &ClientCredentialEarKey {
        &self.credential_ear_key
    }

    pub(crate) fn signature_ear_key_wrapper_key(&self) -> &SignatureEarKeyWrapperKey {
        &self.signature_ear_key_wrapper_key
    }

    pub(crate) fn members(&self) -> Vec<UserName> {
        self.client_information
            .iter()
            .map(|(_index, (cred, _sek))| cred.identity().user_name())
            .collect::<HashSet<UserName>>()
            .into_iter()
            .collect()
    }

    pub(crate) fn update(&mut self, backend: &impl OpenMlsCryptoProvider) -> UpdateClientParamsOut {
        // We don't expect there to be a welcome.
        let aad_payload = UpdateClientParamsAad {
            option_encrypted_signature_ear_key: None,
            option_encrypted_client_credential: None,
        };
        let aad = InfraAadMessage::from(InfraAadPayload::UpdateClient(aad_payload))
            .tls_serialize_detached()
            .unwrap();
        self.mls_group.set_aad(&aad);
        let (mls_message, _welcome_option, group_info_option) = self
            .mls_group
            .self_update(backend, &self.leaf_signer)
            .unwrap();
        self.mls_group.set_aad(&[]);
        let group_info = group_info_option.unwrap();
        // Set an empty diff.
        let mut diff = GroupDiff::new(&self);
        diff.apply_pending_removes(self.mls_group().pending_commit().unwrap());
        self.pending_diff = Some(diff);
        let commit = AssistedMessageOut {
            mls_message,
            group_info_option: Some(AssistedGroupInfo::Full(group_info.into())),
        };
        UpdateClientParamsOut {
            commit,
            sender: self.mls_group.own_leaf_index(),
            new_user_auth_key_option: None,
        }
    }

    /// Update or set the user's auth key in this group.
    pub(crate) fn update_user_key(
        &mut self,
        backend: &impl OpenMlsCryptoProvider,
    ) -> UpdateClientParamsOut {
        let aad_payload = UpdateClientParamsAad {
            option_encrypted_signature_ear_key: None,
            option_encrypted_client_credential: None,
        };
        let aad = InfraAadMessage::from(InfraAadPayload::UpdateClient(aad_payload))
            .tls_serialize_detached()
            .unwrap();
        self.mls_group.set_aad(&aad);
        let (commit, _welcome_option, group_info_option) = self
            .mls_group
            .self_update(backend, &self.leaf_signer)
            .unwrap();
        self.mls_group.set_aad(&[]);
        let group_info = group_info_option.unwrap();
        let mut diff = GroupDiff::new(&self);
        diff.apply_pending_removes(self.mls_group().pending_commit().unwrap());
        let user_auth_signing_key = UserAuthSigningKey::generate().unwrap();
        let verifying_key = user_auth_signing_key.verifying_key().clone();
        diff.user_auth_key = Some(user_auth_signing_key);
        let params = UpdateClientParamsOut {
            commit: AssistedMessageOut {
                mls_message: commit,
                group_info_option: Some(AssistedGroupInfo::Full(group_info.into())),
            },
            sender: self.mls_group.own_leaf_index(),
            new_user_auth_key_option: Some(verifying_key),
        };
        self.pending_diff = Some(diff);
        params
    }

    pub(crate) fn leave_group(
        &mut self,
        backend: &impl OpenMlsCryptoProvider,
    ) -> SelfRemoveClientParamsOut {
        let Some(user_auth_key) = &self.user_auth_signing_key_option else {
            panic!("User auth key not set")
        };
        let proposal = self
            .mls_group
            .leave_group(backend, &self.leaf_signer)
            .unwrap();
        let assisted_message = AssistedMessageOut {
            mls_message: proposal,
            group_info_option: None,
        };
        let params = SelfRemoveClientParamsOut {
            remove_proposal: assisted_message,
            sender: user_auth_key.verifying_key().hash(),
        };
        params
    }

    pub(crate) fn leaf_signer(&self) -> &InfraCredentialSigningKey {
        &self.leaf_signer
    }

    pub(crate) fn store_proposal(&mut self, proposal: QueuedProposal) {
        self.mls_group.store_pending_proposal(proposal)
    }
}

pub(crate) fn application_message_to_conversation_messages(
    sender: &ClientCredential,
    application_message: ApplicationMessage,
) -> Vec<ConversationMessage> {
    vec![new_conversation_message(Message::Content(ContentMessage {
        sender: sender.identity().user_name().to_bytes(),
        content: MessageContentType::tls_deserialize(
            &mut application_message.into_bytes().as_slice(),
        )
        .unwrap(),
    }))]
}

fn get_user_name(
    client_information: &BTreeMap<usize, (ClientCredential, SignatureEarKey)>,
    index: usize,
) -> UserName {
    client_information
        .get(&index)
        .unwrap()
        .0
        .identity()
        .user_name()
}

/// For now, this doesn't cover removes.
pub(crate) fn staged_commit_to_conversation_messages(
    free_indices: impl Iterator<Item = usize>,
    client_information: &BTreeMap<usize, (ClientCredential, SignatureEarKey)>,
    staged_commit: &StagedCommit,
) -> Vec<ConversationMessage> {
    let adds: Vec<ConversationMessage> = staged_commit
        .add_proposals()
        .zip(free_indices)
        .map(|(staged_add_proposal, free_index)| {
            let sender = if let Sender::Member(sender_index) = staged_add_proposal.sender() {
                sender_index.usize()
            } else {
                // We don't support non-member adds.
                panic!("Non-member add proposal")
            };
            let event_message = format!(
                "{} added {} to the conversation",
                get_user_name(client_information, sender),
                get_user_name(client_information, free_index)
            );
            event_message_from_string(event_message)
        })
        .collect();
    let mut updates: Vec<ConversationMessage> = staged_commit
        .update_proposals()
        .map(|staged_update_proposal| {
            let updated_member = staged_update_proposal
                .update_proposal()
                .leaf_node()
                .credential()
                .identity();
            let event_message = format!("{} updated", String::from_utf8_lossy(updated_member),);
            event_message_from_string(event_message)
        })
        .collect();
    let mut events = adds;
    events.append(&mut updates);

    events
}

fn event_message_from_string(event_message: String) -> ConversationMessage {
    new_conversation_message(Message::Display(DisplayMessage {
        message: DisplayMessageType::System(SystemMessage {
            message: event_message,
        }),
    }))
}

/*
impl From<GroupEvent> for ConversationMessage {
    fn from(event: GroupEvent) -> ConversationMessage {
        let event_message = match &event {
            GroupEvent::MemberAdded(e) => Some(format!(
                "{} added {} to the conversation",
                String::from_utf8_lossy(e.sender().identity()),
                String::from_utf8_lossy(e.added_member().identity())
            )),
            GroupEvent::MemberRemoved(e) => match e.removal() {
                Removal::WeLeft => Some("We left the conversation".to_string()),
                Removal::TheyLeft(leaver) => Some(format!(
                    "{} left the conversation",
                    String::from_utf8_lossy(leaver.identity()),
                )),
                Removal::WeWereRemovedBy(remover) => Some(format!(
                    "{} removed us from the conversation",
                    String::from_utf8_lossy(remover.identity()),
                )),
                Removal::TheyWereRemovedBy(leaver, remover) => Some(format!(
                    "{} removed {} from the conversation",
                    String::from_utf8_lossy(remover.identity()),
                    String::from_utf8_lossy(leaver.identity())
                )),
            },
            GroupEvent::MemberUpdated(e) => Some(format!(
                "{} updated",
                String::from_utf8_lossy(e.updated_member().identity()),
            )),
            GroupEvent::PskReceived(_) => Some("PSK received".to_string()),
            GroupEvent::ReInit(_) => Some("ReInit received".to_string()),
            GroupEvent::ApplicationMessage(_) => None,
            openmls::group::GroupEvent::Error(e) => {
                Some(format!("Error occured in group: {:?}", e))
            }
        };

        let app_message = match &event {
            GroupEvent::ApplicationMessage(message) => {
                let content_message = ContentMessage {
                    sender: String::from_utf8_lossy(message.sender().identity()).into(),
                    content_type: Some(ContentType::TextMessage(TextMessage {
                        message: String::from_utf8_lossy(message.message()).into(),
                    })),
                };
                Some(content_message)
            }
            _ => None,
        };

        if let Some(event_message) = event_message {
            new_conversation_message(conversation_message::Message::DisplayMessage(
                DisplayMessage {
                    message: Some(display_message::Message::SystemMessage(SystemMessage {
                        content: event_message,
                    })),
                },
            ))
        } else {
            new_conversation_message(conversation_message::Message::ContentMessage(
                app_message.unwrap(),
            ))
        }
    }
}
*/
