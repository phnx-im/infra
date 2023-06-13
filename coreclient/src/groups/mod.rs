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
                ClientCredentialEarKey, GroupStateEarKey, SignatureEarKey,
                WelcomeAttributionInfoEarKey,
            },
            EarDecryptable, EarEncryptable,
        },
        signatures::{
            keys::UserAuthSigningKey,
            signable::{Signable, Verifiable},
        },
        DecryptionPrivateKey,
    },
    ds::{
        group_state::EncryptedClientCredential, WelcomeAttributionInfo,
        WelcomeAttributionInfoPayload, WelcomeAttributionInfoTbs,
    },
    messages::{
        client_ds::{
            AddUsersParamsAad, DsJoinerInformationIn, InfraAadMessage, InfraAadPayload,
            JoinGroupParams, JoinGroupParamsAad, WelcomeBundle,
        },
        client_ds_out::{AddUsersParamsOut, RemoveUsersParamsOut, SendMessageParamsOut},
    },
    qs::{KeyPackageBatch, VERIFIED},
    AssistedGroupInfo,
};
pub(crate) use store::*;
use tls_codec::DeserializeBytes;

use crate::{
    contacts::{Contact, ContactAddInfos},
    conversations::*,
    types::MessageContentType,
    types::*,
    users::process,
};
use std::{
    collections::{HashMap, HashSet},
    panic::panic_any,
};

use openmls::prelude::*;

use self::diff::GroupDiff;

#[derive(Debug)]
pub(crate) struct Group {
    group_id: GroupId,
    leaf_signer: InfraCredentialSigningKey,
    signature_ear_key: SignatureEarKey,
    credential_ear_key: ClientCredentialEarKey,
    group_state_ear_key: GroupStateEarKey,
    user_auth_key: UserAuthSigningKey,
    mls_group: MlsGroup,
    client_credentials: Vec<Option<ClientCredential>>,
    pending_diff: Option<GroupDiff>,
}

impl Group {
    /// Create a group.
    pub fn create_group(
        backend: &impl OpenMlsCryptoProvider,
        signer: &ClientSigningKey,
        group_id: GroupId,
    ) -> Self {
        let credential_ear_key = ClientCredentialEarKey::random().unwrap();
        let user_auth_key = UserAuthSigningKey::generate().unwrap();
        let group_state_ear_key = GroupStateEarKey::random().unwrap();
        let signature_ear_key = SignatureEarKey::random().unwrap();

        let leaf_signer = InfraCredentialSigningKey::generate(signer, &signature_ear_key);

        let mls_group_config = MlsGroupConfig::builder()
            .use_ratchet_tree_extension(true)
            .build();

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
            signature_ear_key,
            mls_group,
            credential_ear_key,
            group_state_ear_key,
            user_auth_key,
            client_credentials: vec![Some(signer.credential().clone())],
            pending_diff: None,
        }
    }

    /// Join a group with the provided welcome message. Returns the group name.
    pub(crate) fn join_group(
        backend: &impl OpenMlsCryptoProvider<KeyStoreProvider = MemoryKeyStore>,
        welcome_bundle: WelcomeBundle,
        // This is our own key that the sender uses to encrypt to us. We should
        // be able to retrieve it from the client's key store.
        welcome_attribution_info_ear_key: &WelcomeAttributionInfoEarKey,
        leaf_signers: &mut HashMap<SignaturePublicKey, InfraCredentialSigningKey>,
        as_intermediate_credentials: &[AsIntermediateCredential],
        contacts: &HashMap<UserName, Contact>,
    ) -> Result<Self, GroupOperationError> {
        //log::debug!("{} joining group ...", self_user.username);
        let serialized_welcome = welcome_bundle.welcome.tls_serialize_detached().unwrap();

        let mls_group_config = MlsGroupConfig::default();

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

        // Let's create the group first so that we can access the GroupId.
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

        // TODO: This should probably all be moved into a member function "decrypt" of JoinerInfo.
        let public_key: Vec<u8> = key_package.hpke_init_key().clone().into();
        let private_key = backend
            .key_store()
            .read::<HpkePrivateKey>(key_package.hpke_init_key().as_slice())
            .unwrap();
        let info = [
            "GroupStateEarKey ".as_bytes(),
            mls_group.group_id().as_slice(),
        ]
        .concat();
        let decryption_key =
            DecryptionPrivateKey::new(private_key.as_ref().to_vec().into(), public_key.into());
        let joiner_info_bytes = decryption_key
            .decrypt(
                &info,
                &[],
                &<HpkeCiphertext as DeserializeBytes>::tls_deserialize_exact(
                    welcome_bundle.encrypted_joiner_info.as_slice(),
                )
                .unwrap(),
            )
            .unwrap();
        let joiner_info = DsJoinerInformationIn::tls_deserialize_exact(&joiner_info_bytes).unwrap();

        // Decrypt WelcomeAttributionInfo
        let welcome_attribution_info = WelcomeAttributionInfo::decrypt(
            welcome_attribution_info_ear_key,
            &welcome_bundle.encrypted_attribution_info,
        )
        .unwrap();

        let verifiable_attribution_info = welcome_attribution_info
            .into_verifiable(mls_group.group_id().clone(), serialized_welcome);

        let sender_client_credential = contacts
            .get(&verifiable_attribution_info.sender().username())
            .and_then(|c| c.client_credential(&verifiable_attribution_info.sender()))
            .unwrap();

        let welcome_attribution_info: WelcomeAttributionInfoPayload = verifiable_attribution_info
            .verify(sender_client_credential.verifying_key())
            .unwrap();

        let client_credentials: Vec<Option<ClientCredential>> = joiner_info
            .encrypted_client_credentials
            .into_iter()
            .map(|ciphertext_option| {
                ciphertext_option.map(|ciphertext| {
                    let verifiable_credential = VerifiableClientCredential::decrypt(
                        welcome_attribution_info.client_credential_encryption_key(),
                        &ciphertext,
                    )
                    .unwrap();
                    let as_credential = as_intermediate_credentials
                        .iter()
                        .find(|as_cred| {
                            &as_cred.fingerprint().unwrap()
                                == verifiable_credential.signer_fingerprint()
                        })
                        .unwrap();
                    verifiable_credential
                        .verify(as_credential.verifying_key())
                        .unwrap()
                })
            })
            .collect();

        // Decrypt and verify the infra credentials.
        // TODO: Right now, this just panics if the verification fails.
        mls_group
            .members()
            .for_each(|m| match m.credential.mls_credential_type() {
                MlsCredentialType::Infra(credential) => {
                    let _verified_credential: InfraCredentialTbs =
                        InfraCredentialPlaintext::decrypt(
                            credential,
                            welcome_attribution_info.signature_encryption_key(),
                        )
                        .unwrap()
                        .verify(
                            client_credentials
                                .get(m.index.usize())
                                .unwrap()
                                .as_ref()
                                .unwrap()
                                .verifying_key(),
                        )
                        .unwrap();
                }
                _ => panic_any("We should only use infra credentials."),
            });

        let verifying_key = mls_group.own_leaf_node().unwrap().signature_key();
        let leaf_signer = leaf_signers.remove(verifying_key).unwrap();

        // TODO: Once we support multiple clients, this should be synchronized
        // across clients.
        let user_auth_key = UserAuthSigningKey::generate().unwrap();

        let group = Group {
            group_id: mls_group.group_id().clone(),
            mls_group,
            leaf_signer,
            signature_ear_key: welcome_attribution_info.signature_encryption_key().clone(),
            credential_ear_key: welcome_attribution_info
                .client_credential_encryption_key()
                .clone(),
            group_state_ear_key: joiner_info.group_state_ear_key,
            // This one needs to be rolled fresh.
            user_auth_key,
            client_credentials,
            pending_diff: None,
        };

        Ok(group)
    }

    /// Process inbound message
    pub(crate) fn process_message(
        &mut self,
        backend: &impl OpenMlsCryptoProvider,
        message: impl Into<ProtocolMessage>,
        // Required in case there are new joiners.
        // TODO: In the federated case, we might have to fetch them first.
        as_intermediate_credentials: &[AsIntermediateCredential],
    ) -> Result<ProcessedMessage, GroupOperationError> {
        let processed_message = self.mls_group.process_message(backend, message)?;
        let staged_commit = match processed_message.content() {
            // For now, we only care about commits.
            ProcessedMessageContent::ApplicationMessage(_)
            | ProcessedMessageContent::ProposalMessage(_)
            | ProcessedMessageContent::ExternalJoinProposalMessage(_) => {
                return Ok(processed_message)
            }
            ProcessedMessageContent::StagedCommitMessage(staged_commit) => staged_commit,
        };
        // Process removes
        let mut diff = GroupDiff::new(self);
        for proposal in staged_commit.remove_proposals() {
            let removed_member = proposal.remove_proposal().removed();
            diff.remove_client_credential(removed_member);
        }

        // Let's figure out which operation this is meant to be.
        let aad_payload =
            InfraAadMessage::tls_deserialize_exact(processed_message.authenticated_data())
                .unwrap()
                .into_payload();
        match aad_payload {
            InfraAadPayload::AddUsers(add_users_payload) => {
                let client_credentials: Vec<ClientCredential> = add_users_payload
                    .encrypted_credential_information
                    .into_iter()
                    .map(|ciphertext| {
                        ClientCredential::decrypt_and_verify(
                            &self.credential_ear_key,
                            &ciphertext,
                            as_intermediate_credentials,
                        )
                        .unwrap()
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

                // Verify the leaf credentials in all add proposals. We assume
                // that leaf credentials are in the same order as client
                // credentials.
                for (index, proposal) in staged_commit.add_proposals().enumerate() {
                    let client_credential = client_credentials.get(index).unwrap();
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
                                &self.signature_ear_key,
                            )
                            .unwrap();
                            credential_plaintext
                                .verify::<InfraCredentialTbs>(client_credential.verifying_key())
                                .unwrap();
                        }
                    }
                }

                // Add the client credentials to the group.
                for client_credential in client_credentials {
                    diff.add_client_credentials(&self.client_credentials, client_credential)
                }
            }
            InfraAadPayload::UpdateClient(_) => todo!(),
            InfraAadPayload::JoinGroup(_) => todo!(),
            InfraAadPayload::JoinConnectionGroup(_) => todo!(),
            InfraAadPayload::AddClients(_) => todo!(),
        };

        // Process adds
        // To process adds, we have to check the Aad. Depending on the
        // operation, the AAD looks slightly different. We recognize the
        // operation by checking if it's an external commit.
        match processed_message.sender() {
            Sender::External(_) | Sender::NewMemberProposal => panic!("Unsupported sender type."),
            // If it's a member, it can be a number of operations.
            Sender::Member(_) => {
                // Check which operation it is:
                // * AddUser: Sender is not the owner of the added clients. We
                // can only check this after we have parsed and processed the
                // EncryptedClientCredentials, which we can only do after we
                // know which operation we have.
                // TODO: Add Enum to AAD to identify operation.

                // TODO: Branch here to see if it's an AddUsers, an UpdateClient
                // or a RemoveUsers operation, etc. Before we do that, we should
                // probably add a notion of User on the group level. Then verify
                // that the operation is clean (no adds in a RemoveClient
                // operation, ets.).
                // Then create and populate a diff s.t. we can perform
                // verification based on potentially changed client credentials.
                // Then perform further verification:
                // * new leaf credentials in proposals and path
                // * new client credentials (of new members or as part of an update)
                // Don't forget to deal with connection groups in the TODO below.

                // Decrypt and verify client credentials for added users.
                let client_credentials: Vec<ClientCredential> =
                    Vec::<EncryptedClientCredential>::tls_deserialize_exact(
                        processed_message.authenticated_data(),
                    )
                    .unwrap()
                    .into_iter()
                    .map(|ciphertext| {
                        ClientCredential::decrypt_and_verify(
                            &self.credential_ear_key,
                            &ciphertext,
                            as_intermediate_credentials,
                        )
                        .unwrap()
                    })
                    .collect();

                // Add the client credentials to the group.
                for client_credential in client_credentials {
                    diff.add_client_credentials(&self.client_credentials, client_credential)
                }

                // Verify the leaf credentials in all add proposals.
                for proposal in staged_commit.add_proposals() {
                    // TODO
                }

                // TODO: Verify a potential new leaf credential from the leaf in
                // the path. How to get it from the ProcessedMessage?
                if let Some(new_credential) = processed_message.new_credential_option() {}
            }
            // If it's a NewMemberCommit, it's either a new client of an
            // existing member, or a new member in a connection group.
            // TODO: Deal with connection groups.
            Sender::NewMemberCommit => {
                // deserialize aad.
                let join_group_params = JoinGroupParamsAad::tls_deserialize_exact(
                    processed_message.authenticated_data(),
                )
                .unwrap();
                // TODO: Check if the clients in the params match. We can do
                // this once we have a notion of user in Group.
                // Verify the client credential.
                let client_credential = ClientCredential::decrypt_and_verify(
                    &self.credential_ear_key,
                    &join_group_params.encrypted_credential_information,
                    as_intermediate_credentials,
                )
                .unwrap();
                match processed_message.credential().mls_credential_type() {
                    MlsCredentialType::Basic(_) | MlsCredentialType::X509(_) => {
                        panic!("Unsupported credential type.")
                    }
                    MlsCredentialType::Infra(infra_credential) => {
                        // Verify the leaf credential
                        let credential_plaintext = InfraCredentialPlaintext::decrypt(
                            infra_credential,
                            &self.signature_ear_key,
                        )
                        .unwrap();
                        credential_plaintext
                            .verify::<InfraCredentialTbs>(client_credential.verifying_key())
                            .unwrap();
                        // Insert the client credential into the diff. This covers resync operations, since we've already dealt with removes.
                        diff.add_client_credentials(&self.client_credentials, client_credential);
                    }
                }
            }
        }

        Ok(processed_message)
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
        let client_credentials = client_credentials.into_iter().flatten().collect::<Vec<_>>();
        // Prepare KeyPackageBatches and KeyPackages
        let (key_package_vecs, key_package_batches): (
            Vec<Vec<KeyPackage>>,
            Vec<KeyPackageBatch<VERIFIED>>,
        ) = add_infos
            .into_iter()
            .map(|add_info| (add_info.key_packages, add_info.key_package_batch))
            .unzip();

        let key_packages = key_package_vecs.into_iter().flatten().collect::<Vec<_>>();

        let ecc = client_credentials
            .iter()
            .map(|client_credential| client_credential.encrypt(&self.credential_ear_key).unwrap())
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
        let commit = (mls_commit, assisted_group_info);

        let encrypted_welcome_attribution_infos = wai_keys
            .iter()
            .map(|wai_key| {
                // WAI = WelcomeAttributionInfo
                let wai_payload = WelcomeAttributionInfoPayload::new(
                    signer.credential().identity(),
                    self.credential_ear_key.clone(),
                    self.signature_ear_key.clone(),
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
        let mut diff = GroupDiff::new(self);
        for client_credential in client_credentials.into_iter() {
            diff.add_client_credentials(&self.client_credentials, client_credential)
        }

        self.pending_diff = Some(diff);

        let params = AddUsersParamsOut {
            commit,
            sender: self.user_auth_key.verifying_key().hash(),
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
        let remove_indices = self
            .client_credentials
            .iter()
            .enumerate()
            .filter_map(|(index, cred_option)| {
                if let Some(cred) = cred_option {
                    if members.contains(&cred.identity()) {
                        Some(LeafNodeIndex::new(index as u32))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        // There shouldn't be a welcome
        let (mls_message, _welcome_option, group_info_option) = self
            .mls_group
            .remove_members(backend, &self.leaf_signer, remove_indices.as_slice())
            .unwrap();
        debug_assert!(_welcome_option.is_none());
        let group_info = group_info_option.unwrap();
        let assisted_group_info = AssistedGroupInfo::Full(group_info.into());
        let commit = (mls_message, assisted_group_info);

        let mut diff = GroupDiff::new(self);
        for index in remove_indices {
            diff.remove_client_credential(index);
        }
        self.pending_diff = Some(diff);

        let params = RemoveUsersParamsOut {
            commit,
            sender: self.user_auth_key().verifying_key().hash(),
        };
        Ok(params)
    }

    /// Merge the pending commit and apply the pending group diff.
    pub(crate) fn merge_pending_commit(
        &mut self,
        backend: &impl OpenMlsCryptoProvider<KeyStoreProvider = MemoryKeyStore>,
    ) -> Result<(), GroupOperationError> {
        if let Some(diff) = self.pending_diff.take() {
            if let Some(leaf_signer) = diff.leaf_signer {
                self.leaf_signer = leaf_signer;
            }
            if let Some(signature_ear_key) = diff.signature_ear_key {
                self.signature_ear_key = signature_ear_key;
            }
            if let Some(credential_ear_key) = diff.credential_ear_key {
                self.credential_ear_key = credential_ear_key;
            }
            if let Some(group_state_ear_key) = diff.group_state_ear_key {
                self.group_state_ear_key = group_state_ear_key;
            }
            if let Some(user_auth_key) = diff.user_auth_key {
                self.user_auth_key = user_auth_key;
            }
            for (index, credential) in diff.client_credentials {
                if index >= self.client_credentials.len() {
                    debug_assert!(index == self.client_credentials.len() + 1);
                    self.client_credentials.push(credential)
                } else {
                    *self.client_credentials.get_mut(index).unwrap() = credential;
                }
            }
            // We might have inadvertendly extended the Credential vector with
            // `None`s. Let's trim it down again.
            // It shouldn't be empty, but we don't want to loop forever.
            while let Some(last_entry) = self.client_credentials.last() {
                if last_entry.is_none() {
                    self.client_credentials.pop();
                } else {
                    break;
                }
            }
        } else {
            return Err(GroupOperationError::NoPendingGroupDiff);
        }
        self.pending_diff = None;
        Ok(self.mls_group.merge_pending_commit(backend)?)
    }

    /// Get a list of clients in the group to send messages to.
    fn recipients(&self, own_credential: &ClientCredential) -> Vec<AsClientId> {
        let recipients: Vec<AsClientId> = self
            .client_credentials
            .iter()
            .filter_map(|client_credential_option| {
                if let Some(client_credential) = client_credential_option {
                    if own_credential.identity() != client_credential.identity() {
                        Some(client_credential.identity())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();
        recipients
    }

    /// Send an application message to the group.
    pub fn create_message(
        &mut self,
        backend: &impl OpenMlsCryptoProvider<KeyStoreProvider = MemoryKeyStore>,
        msg: &str,
    ) -> Result<SendMessageParamsOut, GroupOperationError> {
        let message = self
            .mls_group
            .create_message(backend, &self.leaf_signer, msg.as_bytes())?;

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

    pub(crate) fn user_auth_key(&self) -> &UserAuthSigningKey {
        &self.user_auth_key
    }

    pub(crate) fn group_state_ear_key(&self) -> &GroupStateEarKey {
        &self.group_state_ear_key
    }

    pub(crate) fn pending_commit(&self) -> Option<&StagedCommit> {
        self.mls_group.pending_commit()
    }

    /// Returns the leaf indices of the clients owned by the given user.
    pub(crate) fn user_clients(&self, user_name: UserName) -> Vec<usize> {
        let mut user_clients = vec![];
        for (index, cred_option) in self.client_credentials.iter().enumerate() {
            if let Some(cred_user_name) =
                cred_option.as_ref().map(|cred| cred.identity().username())
            {
                if cred_user_name == user_name {
                    user_clients.push(index)
                }
            }
        }
        user_clients
    }
}

pub(crate) fn application_message_to_conversation_messages(
    sender: &Credential,
    application_message: ApplicationMessage,
) -> Vec<ConversationMessage> {
    vec![new_conversation_message(Message::Content(ContentMessage {
        sender: sender.identity().to_vec().into(),
        content: MessageContentType::Text(TextMessage {
            message: String::from_utf8_lossy(&application_message.into_bytes()).into(),
        }),
    }))]
}

pub(crate) fn staged_commit_to_conversation_messages(
    own_identity: &UserName,
    staged_commit: &StagedCommit,
) -> Vec<ConversationMessage> {
    let adds: Vec<ConversationMessage> = staged_commit
        .add_proposals()
        .map(|staged_add_proposal| {
            let added_member = staged_add_proposal
                .add_proposal()
                .key_package()
                .leaf_node()
                .credential()
                .identity();
            let event_message = format!(
                "{} added {} to the conversation",
                String::from_utf8_lossy(own_identity.as_bytes()),
                String::from_utf8_lossy(added_member)
            );
            event_message_from_string(event_message)
        })
        .collect();
    let mut removes: Vec<ConversationMessage> = staged_commit
        .remove_proposals()
        .map(|staged_remove_proposal| {
            let removed_member = staged_remove_proposal.remove_proposal().removed();
            let event_message = format!(
                "{} removed {:?} from the conversation",
                String::from_utf8_lossy(own_identity.as_bytes()),
                removed_member
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
    events.append(&mut removes);
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
