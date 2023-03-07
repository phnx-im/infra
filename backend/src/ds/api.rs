//! This module contains the public API of the delivery service (DS) module.
//! While the DS also contains some rate-limiting functionality, it should only
//! be used behind a rate-limiting module.
//!
//! NOTE: This document and the API stubs in this module represent a work in
//! progress and will likely change in their details. However, barring the
//! discovery of a major flaw in the current design, the general design of the
//! DS should remain the same.
//!
//! TODO: Write this with a low-metadata flag in mind that changes if missing
//! links and EID information are stored encrypted and separately.
//!
//! # Overview
//!
//! The main task of the delivery service is to distribute messages sent by
//! clients in the context of an MLS group to the members of said group
//! represented by each member's queueing service.
//!
//! To do this robustly, prevent the accumulation of metadata associated with
//! individual users and their devices and to prevent group-level
//! denial-of-service attacks, the delivery service has to keep some additional
//! state and perform a few checks with each message received.
//!
//! The DS also performs the secondary task of keeping track of the state of a
//! given group well enough to provide joiners with enough information to join
//! the group via an external commit.
//!
//! # Encryption at rest (EAR)
//!
//! The metadata that the DS has to store to fulfill its functionality is
//! encrypted at rest using keys which the clients provide when querying an API
//! endpoint of the DS.
//!
//! The EAR key is ratcheted forward and injected with a fresh secret with every
//! commit. The fresh secret is provided by the committer in the AAD of its
//! query such that the server and all other group members can compute the new
//! key.
//!
//! TODO: Add note about key-committing encryption scheme.
//!
//! # State expiration
//!
//! TODO: Explain how each state has a time-stamp that is updated whenever the
//! state is accessed. Also clean-up of pieces of state that have not been used
//! for a certain period of time.
//!
//! # Group State
//!
//! The delivery service requires clients to communicate using MLS plaintexts
//! for a number of reasons.
//!
//! * To perform validity checks on incoming messages (especially commits)
//! * To be able to provide a GroupInfo object to joiners that want to join via
//!   an external commit without requiring committers to send a full GroupInfo
//!   every time
//! * To authenticate messages by the individual group members via the signature
//!   key in the key package of the respective member
//!
//! ## Pseudonymous LeafNodes
//!
//! To avoid having to store the identity of individual group members, group
//! members can use pseudonymous LeafNodes. A Pseudonymous LeafNodes does not
//! contain a Credential with the client's real Client- and UserID, but instead
//! contains a pseudonymous (random) Client- and UserID. Since the DS should be
//! able to enforce group policies and thus needs to know which clients belong
//! to which user (at least in the context of an individual group), the
//! pseudonymous UserID needs to be the same for all clients of a given user in
//! a given group.
//!
//! ## Missing link certificate chains
//!
//! Group members (and especially newly joining group members) still need to be
//! able to authenticate all other members of a given group. This is achieved by
//! the DS keeping an encrypted "missing link certificate chains" for each group
//! member, which contains the key with which the pseudonymous credential is
//! signed and which is in turn signed by the client's (intermediate) client
//! key.
//!
//! The encrypted missing link certificate chains need to be stored by the DS
//! twice, each time encrypted under a different symmetric key with overlapping
//! validity periods. This is to allow key rotations in the asynchronous
//! setting.
//!
//! When an old key expires, the next committer uploads new ciphertexts
//! encrypted under a group key derived from the key schedule of the old epoch.
//!
//! Whenever a new member joins the group, the adding group member needs to
//! include the decryption key in the Welcome, so that new group members can
//! authenticate existing ones.
//!
//! ## Evolving identity state
//!
//! To fully authenticate existing group members, new group members need the
//! Evolving Identity state of the existing group members.
//!
//! Thus, in addition to the encrypted missing link credential, the DS stores
//! the evolving identity state of each group member encrypted using the same
//! key rotation scheme, such that new group members can fully authenticate
//!
//! ## Queue information
//!
//! When receiving a message from a client, the DS main functionality is the
//! delivery of the message to its recipients. It thus needs to store a QueueID
//! for each member of each group.
//!
//! To this end, the DS keeps an additional record for each LeafNode, which
//! contains the QueueID that the corresponding member wishes to use for this
//! group, along with other information such as an authentication key that the
//! DS can use to prove to the queuing service (QS) that it is authorized to
//! enqueue messages in this particular queue.
//!
//! For new members that are added via a Welcome, the QueueID and corresponding
//! information needs to be present in the KeyPackage encrypted asymmetrically
//! under the private key the DS uses for this purpose.
//!
//! TODO: This is problematic, as the QueueID and other information is not
//! encrypted at rest here. Since it's (intended to be) temporary, maybe this is
//! not a problem?
//!
//! TODO: We should explain the generation of QueueIDs and temporary QueueIDs in
//! another place that we can link to here.
//!
//! # Welcome message delivery
//!
//! The DS does not provide an API endpoint for Welcome message delivery.
//! Instead, clients that invite new group members should send the messages via
//! their connection group.
//!
//! TODO: Add link to an explanation of a connection group.
//!
//! # Pseudonym-based rate-limiting
//!
//! TODO: Explain pseudonym-based rate limiting
//!
//! # Metadata on the DS
//!
//! TODO: Discuss here what the data in the group state actually reveals about
//! the individual members.
//!
//! # Message format
//!
//! TODO: Discuss message format here or point to a discussion of what the
//! message format looks like.
//!

use mls_assist::{group::Group, Node, Sender};

use crate::{
    crypto::{
        ear::EarEncryptable,
        signatures::{keys::LeafSignatureKeyRef, signable::Verifiable},
    },
    messages::client_ds::{
        CreateGroupParams, DsSender, RequestParams, VerifiableClientToDsMessage,
    },
    qs::QsEnqueueProvider,
};

use super::{errors::DsProcessingError, group_state::DsGroupState, DsStorageProvider, LoadState};

pub const USER_EXPIRATION_DAYS: i64 = 90;

pub struct DsApi {}

impl DsApi {
    pub async fn process<Dsp: DsStorageProvider, Q: QsEnqueueProvider>(
        ds_storage_provider: &Dsp,
        qs_enqueue_provider: &Q,
        message: VerifiableClientToDsMessage,
    ) -> Result<Option<DsProcessResponse>, DsProcessingError> {
        let group_id = message.group_id().clone();
        let ear_key = message.ear_key().clone();
        // Load encrypted group state.
        let encrypted_group_state = if let LoadState::Success(group_state) =
            ds_storage_provider.load_group_state(&group_id).await
        {
            group_state
        } else {
            return Err(DsProcessingError::GroupNotFound);
        };

        // Depending on the message, either decrypt an encrypted group state or
        // create a new one.
        let mut group_state = if let Some(create_group_params) = message.create_group_params() {
            let CreateGroupParams {
                group_id: _,
                leaf_node,
                encrypted_credential_chain,
                creator_client_reference: creator_queue_config,
                creator_user_auth_key,
                group_info,
            } = create_group_params;
            let group_state = Group::new(group_info.clone(), leaf_node.clone())
                .map_err(|_| DsProcessingError::InvalidMessage)?;
            DsGroupState::new(
                group_state,
                creator_user_auth_key.clone(),
                encrypted_credential_chain.clone(),
                creator_queue_config.clone(),
            )
        } else {
            DsGroupState::decrypt(&ear_key, &encrypted_group_state)
                .map_err(|_| DsProcessingError::CouldNotDecrypt)?
        };

        // Verify the message.
        let verified_message: RequestParams = match message.sender() {
            DsSender::LeafIndex(leaf_index) => {
                let verifying_key: LeafSignatureKeyRef = group_state
                    .group()
                    .leaf(leaf_index)
                    .ok_or(DsProcessingError::UnknownSender)?
                    .signature_key()
                    .into();
                message
                    .verify(&verifying_key)
                    .map_err(|_| DsProcessingError::InvalidSignature)?
            }
            DsSender::LeafSignatureKey(verifying_key) => message
                .verify(&verifying_key)
                .map_err(|_| DsProcessingError::InvalidSignature)?,
            DsSender::UserKeyHash(user_key_hash) => {
                let verifying_key = group_state
                    .get_user_key(&user_key_hash)
                    .ok_or(DsProcessingError::UnknownSender)?;
                message
                    .verify(verifying_key)
                    .map_err(|_| DsProcessingError::InvalidSignature)?
            }
        };

        let sender = verified_message.mls_sender().cloned();

        // For now, we just process directly.
        // TODO: We might want to realize this via a trait.
        let (c2c_message_option, response_option) = match verified_message {
            RequestParams::AddUsers(add_user_params) => {
                let result = group_state.add_users(add_user_params)?;
                (Some(result), None)
            }
            RequestParams::WelcomeInfo(welcome_info_params) => {
                let ratchet_tree = group_state
                    .welcome_info(welcome_info_params)
                    .ok_or(DsProcessingError::NoWelcomeInfoFound)?;
                (
                    None,
                    Some(DsProcessResponse::RatchetTree(ratchet_tree.to_vec())),
                )
            }
            RequestParams::CreateGroupParams(_) => (None, None),
        };

        // TODO: We could optimize here by only re-encrypting and persisting the
        // group state if it has actually changed.

        // ... before we distribute the message, we encrypt ...
        let encrypted_group_state = group_state
            .encrypt(&ear_key)
            .map_err(|_| DsProcessingError::CouldNotEncrypt)?;

        // ... and store the modified group state.
        ds_storage_provider
            .save_group_state(
                group_state.group().group_info().group_context().group_id(),
                encrypted_group_state,
            )
            .await
            .map_err(|_| DsProcessingError::StorageError)?;

        if let Some(c2c_message) = c2c_message_option {
            let sender_index = if let Some(Sender::Member(leaf_index)) = sender {
                leaf_index
            } else {
                return Err(DsProcessingError::InvalidSenderType);
            };

            group_state
                .distribute_message(qs_enqueue_provider, c2c_message, sender_index)
                .await
                .map_err(|_| DsProcessingError::DistributionError)?;
        }

        Ok(response_option)
    }
}

pub enum DsProcessResponse {
    RatchetTree(Vec<Option<Node>>),
}
