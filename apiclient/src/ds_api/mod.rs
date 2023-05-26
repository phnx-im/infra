// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! API endpoints of the DS

use super::*;
use mls_assist::{
    messages::AssistedWelcome,
    openmls::{
        prelude::{
            group_info::{GroupInfo, VerifiableGroupInfo},
            GroupEpoch, GroupId, LeafNodeIndex, RatchetTreeIn, TlsDeserializeTrait,
            TlsSerializeTrait,
        },
        treesync::RatchetTree,
    },
};
use phnxbackend::{
    crypto::{
        ear::keys::GroupStateEarKey,
        signatures::{
            keys::{LeafSigningKey, UserAuthKey, UserAuthSigningKey},
            signable::Signable,
            traits::SigningKey,
        },
    },
    ds::{errors::DsProcessingError, group_state::EncryptedCredentialChain},
    messages::{
        client_ds::{ExternalCommitInfoParams, UpdateQsClientReferenceParams, WelcomeInfoParams},
        client_ds_out::{
            AddClientsParamsOut, AddUsersParamsOut, AssistedMessagePlusOut,
            ClientToDsMessageTbsOut, CreateGroupParamsOut, DeleteGroupParamsOut,
            DsProcessResponseIn, DsRequestParamsOut, JoinConnectionGroupParamsOut,
            JoinGroupParamsOut, RemoveClientsParamsOut, RemoveUsersParamsOut,
            ResyncClientParamsOut, SelfRemoveClientParamsOut, SendMessageParamsOut,
            UpdateClientParamsOut,
        },
    },
    qs::{KeyPackageBatch, QsClientReference, VERIFIED},
};
use phnxserver::endpoints::ENDPOINT_DS;

#[cfg(test)]
mod tests;

#[derive(Error, Debug)]
pub enum DsRequestError {
    #[error("Library Error")]
    LibraryError,
    #[error("Couldn't deserialize response body.")]
    BadResponse,
    #[error("We received an unexpected response type.")]
    UnexpectedResponse,
    #[error("Network error")]
    NetworkError(String),
    #[error(transparent)]
    DsError(#[from] DsProcessingError),
}

impl ApiClient {
    async fn prepare_and_send_ds_message(
        &self,
        request_params: DsRequestParamsOut,
        signing_key: &impl SigningKey,
        group_state_ear_key: &GroupStateEarKey,
    ) -> Result<DsProcessResponseIn, DsRequestError> {
        let tbs = ClientToDsMessageTbsOut::new(group_state_ear_key.clone(), request_params);
        let message = tbs
            .sign(signing_key)
            .map_err(|_| DsRequestError::LibraryError)?;
        let message_bytes = message
            .tls_serialize_detached()
            .map_err(|_| DsRequestError::LibraryError)?;
        match self
            .client
            .post(self.build_url(Protocol::Http, ENDPOINT_DS))
            .body(message_bytes)
            .send()
            .await
        {
            Ok(res) => {
                match res.status().as_u16() {
                    // Success!
                    x if (200..=299).contains(&x) => {
                        let ds_proc_res_bytes =
                            res.bytes().await.map_err(|_| DsRequestError::BadResponse)?;
                        let ds_proc_res =
                            DsProcessResponseIn::tls_deserialize_bytes(ds_proc_res_bytes)
                                .map_err(|_| DsRequestError::BadResponse)?;
                        Ok(ds_proc_res)
                    }
                    // DS Specific Error
                    418 => {
                        let ds_proc_err_bytes =
                            res.bytes().await.map_err(|_| DsRequestError::BadResponse)?;
                        let ds_proc_err =
                            DsProcessingError::tls_deserialize_bytes(ds_proc_err_bytes)
                                .map_err(|_| DsRequestError::BadResponse)?;
                        Err(DsRequestError::DsError(ds_proc_err))
                    }
                    // All other errors
                    _ => {
                        let error_text =
                            res.text().await.map_err(|_| DsRequestError::BadResponse)?;
                        Err(DsRequestError::NetworkError(error_text))
                    }
                }
            }
            // A network error occurred.
            Err(err) => Err(DsRequestError::NetworkError(err.to_string())),
        }
    }

    /// Creates a new group on the DS.
    pub async fn ds_create_group(
        &self,
        leaf_node: RatchetTree,
        encrypted_credential_chain: EncryptedCredentialChain,
        creator_client_reference: QsClientReference,
        group_info: GroupInfo,
        group_state_ear_key: &GroupStateEarKey,
        signing_key: &UserAuthSigningKey,
    ) -> Result<(), DsRequestError> {
        let payload = CreateGroupParamsOut {
            group_id: group_info.group_context().group_id().clone(),
            leaf_node,
            encrypted_credential_chain,
            creator_client_reference,
            group_info,
            creator_user_auth_key: signing_key.verifying_key().clone(),
        };
        self.prepare_and_send_ds_message(
            DsRequestParamsOut::CreateGroupParams(payload),
            signing_key,
            group_state_ear_key,
        )
        .await
        // Check if the response is what we expected it to be.
        .and_then(|response| {
            if matches!(response, DsProcessResponseIn::Ok) {
                Ok(())
            } else {
                Err(DsRequestError::UnexpectedResponse)
            }
        })
    }

    /// Add one or more users to a group.
    pub async fn ds_add_users(
        &self,
        commit: AssistedMessagePlusOut,
        welcome: AssistedWelcome,
        encrypted_welcome_attribution_infos: Vec<Vec<u8>>,
        key_package_batches: Vec<KeyPackageBatch<VERIFIED>>,
        group_state_ear_key: &GroupStateEarKey,
        signing_key: &UserAuthSigningKey,
    ) -> Result<(), DsRequestError> {
        let payload = AddUsersParamsOut {
            commit,
            sender: signing_key.verifying_key().hash(),
            welcome,
            encrypted_welcome_attribution_infos,
            key_package_batches,
        };
        self.prepare_and_send_ds_message(
            DsRequestParamsOut::AddUsers(payload),
            signing_key,
            group_state_ear_key,
        )
        .await
        // Check if the response is what we expected it to be.
        .and_then(|response| {
            if matches!(response, DsProcessResponseIn::Ok) {
                Ok(())
            } else {
                Err(DsRequestError::UnexpectedResponse)
            }
        })
    }

    /// Remove one or more users from a group.
    pub async fn ds_remove_users(
        &self,
        commit: AssistedMessagePlusOut,
        group_state_ear_key: &GroupStateEarKey,
        signing_key: &UserAuthSigningKey,
    ) -> Result<(), DsRequestError> {
        let payload = RemoveUsersParamsOut {
            commit,
            sender: signing_key.verifying_key().hash(),
        };
        self.prepare_and_send_ds_message(
            DsRequestParamsOut::RemoveUsers(payload),
            signing_key,
            group_state_ear_key,
        )
        .await
        // Check if the response is what we expected it to be.
        .and_then(|response| {
            if matches!(response, DsProcessResponseIn::Ok) {
                Ok(())
            } else {
                Err(DsRequestError::UnexpectedResponse)
            }
        })
    }

    /// Get welcome information for a group.
    pub async fn ds_welcome_info(
        &self,
        group_id: GroupId,
        epoch: GroupEpoch,
        group_state_ear_key: &GroupStateEarKey,
        signing_key: &LeafSigningKey,
    ) -> Result<RatchetTreeIn, DsRequestError> {
        let payload = WelcomeInfoParams {
            sender: signing_key.verifying_key().clone(),
            group_id,
            epoch,
        };
        self.prepare_and_send_ds_message(
            DsRequestParamsOut::WelcomeInfo(payload),
            signing_key,
            group_state_ear_key,
        )
        .await
        // Check if the response is what we expected it to be.
        .and_then(|response| {
            if let DsProcessResponseIn::WelcomeInfo(ratchet_tree) = response {
                Ok(ratchet_tree)
            } else {
                Err(DsRequestError::UnexpectedResponse)
            }
        })
    }

    /// Get external commit information for a group.
    pub async fn ds_external_commit_info(
        &self,
        group_id: GroupId,
        group_state_ear_key: &GroupStateEarKey,
        signing_key: &UserAuthSigningKey,
    ) -> Result<(VerifiableGroupInfo, RatchetTreeIn), DsRequestError> {
        let payload = ExternalCommitInfoParams {
            sender: signing_key.verifying_key().hash(),
            group_id,
        };
        self.prepare_and_send_ds_message(
            DsRequestParamsOut::ExternalCommitInfo(payload),
            signing_key,
            group_state_ear_key,
        )
        .await
        // Check if the response is what we expected it to be.
        .and_then(|response| {
            if let DsProcessResponseIn::ExternalCommitInfo(info) = response {
                Ok(info)
            } else {
                Err(DsRequestError::UnexpectedResponse)
            }
        })
    }

    /// Update your client in this group. Note that the given commit needs to
    /// have [`UpdateClientParamsAad`] in its AAD.
    pub async fn ds_update_client(
        &self,
        commit: AssistedMessagePlusOut,
        group_state_ear_key: &GroupStateEarKey,
        own_index: LeafNodeIndex,
        signing_key: &LeafSigningKey,
        new_user_auth_key_option: Option<UserAuthKey>,
    ) -> Result<(), DsRequestError> {
        let payload = UpdateClientParamsOut {
            commit,
            sender: own_index,
            new_user_auth_key_option,
        };
        self.prepare_and_send_ds_message(
            DsRequestParamsOut::UpdateClient(payload),
            signing_key,
            group_state_ear_key,
        )
        .await
        // Check if the response is what we expected it to be.
        .and_then(|response| {
            if matches!(response, DsProcessResponseIn::Ok) {
                Ok(())
            } else {
                Err(DsRequestError::UnexpectedResponse)
            }
        })
    }

    /// Join the group with a new client.
    pub async fn ds_join_group(
        &self,
        external_commit: AssistedMessagePlusOut,
        qs_client_reference: QsClientReference,
        signing_key: &UserAuthSigningKey,
        group_state_ear_key: &GroupStateEarKey,
    ) -> Result<(), DsRequestError> {
        let payload = JoinGroupParamsOut {
            sender: signing_key.verifying_key().hash(),
            external_commit,
            qs_client_reference,
        };
        self.prepare_and_send_ds_message(
            DsRequestParamsOut::JoinGroup(payload),
            signing_key,
            group_state_ear_key,
        )
        .await
        // Check if the response is what we expected it to be.
        .and_then(|response| {
            if matches!(response, DsProcessResponseIn::Ok) {
                Ok(())
            } else {
                Err(DsRequestError::UnexpectedResponse)
            }
        })
    }

    /// Join the connection group with a new client.
    pub async fn ds_join_connection_group(
        &self,
        external_commit: AssistedMessagePlusOut,
        qs_client_reference: QsClientReference,
        signing_key: &UserAuthSigningKey,
        group_state_ear_key: &GroupStateEarKey,
    ) -> Result<(), DsRequestError> {
        let payload = JoinConnectionGroupParamsOut {
            sender: signing_key.verifying_key().clone(),
            external_commit,
            qs_client_reference,
        };
        self.prepare_and_send_ds_message(
            DsRequestParamsOut::JoinConnectionGroup(payload),
            signing_key,
            group_state_ear_key,
        )
        .await
        // Check if the response is what we expected it to be.
        .and_then(|response| {
            if matches!(response, DsProcessResponseIn::Ok) {
                Ok(())
            } else {
                Err(DsRequestError::UnexpectedResponse)
            }
        })
    }

    /// Add clients to a group.
    pub async fn ds_add_clients(
        &self,
        commit: AssistedMessagePlusOut,
        welcome: AssistedWelcome,
        encrypted_welcome_attribution_infos: Vec<u8>,
        signing_key: &UserAuthSigningKey,
        group_state_ear_key: &GroupStateEarKey,
    ) -> Result<(), DsRequestError> {
        let payload = AddClientsParamsOut {
            sender: signing_key.verifying_key().hash(),
            commit,
            welcome,
            encrypted_welcome_attribution_infos,
        };
        self.prepare_and_send_ds_message(
            DsRequestParamsOut::AddClients(payload),
            signing_key,
            group_state_ear_key,
        )
        .await
        // Check if the response is what we expected it to be.
        .and_then(|response| {
            if matches!(response, DsProcessResponseIn::Ok) {
                Ok(())
            } else {
                Err(DsRequestError::UnexpectedResponse)
            }
        })
    }

    /// Remove clients from a group.
    pub async fn ds_remove_clients(
        &self,
        commit: AssistedMessagePlusOut,
        new_auth_key: UserAuthKey,
        signing_key: &UserAuthSigningKey,
        group_state_ear_key: &GroupStateEarKey,
    ) -> Result<(), DsRequestError> {
        let payload = RemoveClientsParamsOut {
            commit,
            sender: signing_key.verifying_key().hash(),
            new_auth_key,
        };
        self.prepare_and_send_ds_message(
            DsRequestParamsOut::RemoveClients(payload),
            signing_key,
            group_state_ear_key,
        )
        .await
        // Check if the response is what we expected it to be.
        .and_then(|response| {
            if matches!(response, DsProcessResponseIn::Ok) {
                Ok(())
            } else {
                Err(DsRequestError::UnexpectedResponse)
            }
        })
    }

    /// Resync a client to rejoin a group.
    pub async fn ds_resync_client(
        &self,
        external_commit: AssistedMessagePlusOut,
        signing_key: &UserAuthSigningKey,
        group_state_ear_key: &GroupStateEarKey,
    ) -> Result<(), DsRequestError> {
        let payload = ResyncClientParamsOut {
            external_commit,
            sender: signing_key.verifying_key().hash(),
        };
        self.prepare_and_send_ds_message(
            DsRequestParamsOut::ResyncClient(payload),
            signing_key,
            group_state_ear_key,
        )
        .await
        // Check if the response is what we expected it to be.
        .and_then(|response| {
            if matches!(response, DsProcessResponseIn::Ok) {
                Ok(())
            } else {
                Err(DsRequestError::UnexpectedResponse)
            }
        })
    }

    /// Leave the given group with this client.
    pub async fn ds_self_remove_client(
        &self,
        remove_proposal: AssistedMessagePlusOut,
        signing_key: &UserAuthSigningKey,
        group_state_ear_key: &GroupStateEarKey,
    ) -> Result<(), DsRequestError> {
        let payload = SelfRemoveClientParamsOut {
            remove_proposal,
            sender: signing_key.verifying_key().hash(),
        };
        self.prepare_and_send_ds_message(
            DsRequestParamsOut::SelfRemoveClient(payload),
            signing_key,
            group_state_ear_key,
        )
        .await
        // Check if the response is what we expected it to be.
        .and_then(|response| {
            if matches!(response, DsProcessResponseIn::Ok) {
                Ok(())
            } else {
                Err(DsRequestError::UnexpectedResponse)
            }
        })
    }

    /// Send a message to the given group.
    pub async fn ds_send_message(
        &self,
        message: AssistedMessagePlusOut,
        own_index: LeafNodeIndex,
        signing_key: &LeafSigningKey,
        group_state_ear_key: &GroupStateEarKey,
    ) -> Result<(), DsRequestError> {
        let payload = SendMessageParamsOut {
            message,
            sender: own_index,
        };
        self.prepare_and_send_ds_message(
            DsRequestParamsOut::SendMessage(payload),
            signing_key,
            group_state_ear_key,
        )
        .await
        // Check if the response is what we expected it to be.
        .and_then(|response| {
            if matches!(response, DsProcessResponseIn::Ok) {
                Ok(())
            } else {
                Err(DsRequestError::UnexpectedResponse)
            }
        })
    }

    /// Delete the given group.
    pub async fn ds_delete_group(
        &self,
        commit: AssistedMessagePlusOut,
        signing_key: &UserAuthSigningKey,
        group_state_ear_key: &GroupStateEarKey,
    ) -> Result<(), DsRequestError> {
        let payload = DeleteGroupParamsOut {
            commit,
            sender: signing_key.verifying_key().hash(),
        };
        self.prepare_and_send_ds_message(
            DsRequestParamsOut::DeleteGroup(payload),
            signing_key,
            group_state_ear_key,
        )
        .await
        // Check if the response is what we expected it to be.
        .and_then(|response| {
            if matches!(response, DsProcessResponseIn::Ok) {
                Ok(())
            } else {
                Err(DsRequestError::UnexpectedResponse)
            }
        })
    }

    /// Update the client's queue info.
    pub async fn ds_update_queue_info(
        &self,
        own_index: LeafNodeIndex,
        group_id: GroupId,
        new_queue_config: QsClientReference,
        signing_key: &LeafSigningKey,
        group_state_ear_key: &GroupStateEarKey,
    ) -> Result<(), DsRequestError> {
        let payload = UpdateQsClientReferenceParams {
            group_id,
            sender: own_index,
            new_queue_config,
        };
        self.prepare_and_send_ds_message(
            DsRequestParamsOut::UpdateQsClientReference(payload),
            signing_key,
            group_state_ear_key,
        )
        .await
        // Check if the response is what we expected it to be.
        .and_then(|response| {
            if matches!(response, DsProcessResponseIn::Ok) {
                Ok(())
            } else {
                Err(DsRequestError::UnexpectedResponse)
            }
        })
    }
}
