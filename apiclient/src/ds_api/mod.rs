// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! API endpoints of the DS

use super::*;
use mls_assist::{
    messages::{AssistedMessageOut, AssistedWelcome},
    openmls::prelude::{
        GroupEpoch, GroupId, LeafNodeIndex, MlsMessageOut, RatchetTreeIn, TlsSerializeTrait,
    },
};
use phnxbackend::{
    auth_service::credentials::keys::InfraCredentialSigningKey,
    crypto::{
        ear::keys::GroupStateEarKey,
        signatures::{
            keys::{UserAuthSigningKey, UserAuthVerifyingKey},
            signable::Signable,
            traits::SigningKey,
        },
    },
    ds::{errors::DsProcessingError, EncryptedWelcomeAttributionInfo},
    messages::{
        client_ds::{
            ConnectionGroupInfoParams, ExternalCommitInfoParams, UpdateQsClientReferenceParams,
            WelcomeInfoParams,
        },
        client_ds_out::{
            AddClientsParamsOut, AddUsersParamsOut, ClientToDsMessageOut, ClientToDsMessageTbsOut,
            CreateGroupParamsOut, DeleteGroupParamsOut, DsProcessResponseIn, DsRequestParamsOut,
            ExternalCommitInfoIn, JoinConnectionGroupParamsOut, JoinGroupParamsOut,
            RemoveClientsParamsOut, RemoveUsersParamsOut, ResyncClientParamsOut,
            SelfRemoveClientParamsOut, SendMessageParamsOut, UpdateClientParamsOut,
        },
    },
    qs::QsClientReference,
    AssistedGroupInfo,
};
use phnxserver::endpoints::{ENDPOINT_DS_GROUPS, ENDPOINT_DS_GROUP_IDS};
use tls_codec::DeserializeBytes;

#[derive(Error, Debug)]
pub enum DsRequestError {
    #[error("Library Error")]
    LibraryError,
    #[error("Couldn't deserialize response body.")]
    BadResponse,
    #[error("We received an unexpected response type.")]
    UnexpectedResponse,
    #[error("Network error: {0}")]
    NetworkError(String),
    #[error(transparent)]
    DsError(#[from] DsProcessingError),
}

pub enum AuthenticationMethod<'a, T: SigningKey> {
    Signature(&'a T),
    None,
}

impl<'a, T: SigningKey + 'a> From<&'a T> for AuthenticationMethod<'a, T> {
    fn from(key: &'a T) -> Self {
        AuthenticationMethod::Signature(key)
    }
}

impl ApiClient {
    // Single purpose function since this is the only endpoint that doesn't require authentication.
    pub async fn ds_request_group_id(&self) -> Result<GroupId, DsRequestError> {
        let url = self.build_url(Protocol::Http, ENDPOINT_DS_GROUP_IDS);
        log::info!("Requesting group ID from DS at {}", url);

        match self.client.post(url).send().await {
            Ok(res) => {
                log::info!("Got response from DS: {:?}", res);
                match res.status().as_u16() {
                    // Success!
                    x if (200..=299).contains(&x) => {
                        let ds_proc_res_bytes =
                            res.bytes().await.map_err(|_| DsRequestError::BadResponse)?;
                        let ds_proc_res = GroupId::tls_deserialize_exact(&ds_proc_res_bytes)
                            .map_err(|_| DsRequestError::BadResponse)?;
                        Ok(ds_proc_res)
                    }
                    // An error occurred. (There are no DS specific errors for this endpoint.)
                    _ => {
                        let error_text =
                            res.text().await.map_err(|_| DsRequestError::BadResponse)?;
                        Err(DsRequestError::NetworkError(error_text))
                    }
                }
            }
            // A network error occurred.
            Err(err) => {
                log::error!("Network error: {:?}", err);
                Err(DsRequestError::NetworkError(err.to_string()))
            }
        }
    }

    async fn prepare_and_send_ds_message<'a, T: SigningKey + 'a>(
        &self,
        request_params: DsRequestParamsOut,
        auth_method: impl Into<AuthenticationMethod<'a, T>>,
        group_state_ear_key: &GroupStateEarKey,
    ) -> Result<DsProcessResponseIn, DsRequestError> {
        let tbs = ClientToDsMessageTbsOut::new(group_state_ear_key.clone(), request_params);
        let message = match auth_method.into() {
            AuthenticationMethod::Signature(signer) => {
                tbs.sign(signer).map_err(|_| DsRequestError::LibraryError)?
            }
            AuthenticationMethod::None => ClientToDsMessageOut::without_signature(tbs),
        };
        let message_bytes = message
            .tls_serialize_detached()
            .map_err(|_| DsRequestError::LibraryError)?;
        match self
            .client
            .post(self.build_url(Protocol::Http, ENDPOINT_DS_GROUPS))
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
                            DsProcessResponseIn::tls_deserialize_exact(&ds_proc_res_bytes)
                                .map_err(|e| {
                                    log::warn!("Couldn't deserialize OK response body: {:?}", e);
                                    DsRequestError::BadResponse
                                })?;
                        Ok(ds_proc_res)
                    }
                    // DS Specific Error
                    418 => {
                        let ds_proc_err_bytes = res.bytes().await.map_err(|_| {
                            log::warn!("No body in DS-error response.");
                            DsRequestError::BadResponse
                        })?;
                        let ds_proc_err = DsProcessingError::tls_deserialize_exact(
                            &ds_proc_err_bytes,
                        )
                        .map_err(|_| {
                            log::warn!("Couldn't deserialize DS-error response body.");
                            DsRequestError::BadResponse
                        })?;
                        Err(DsRequestError::DsError(ds_proc_err))
                    }
                    // All other errors
                    _ => {
                        let error_text = res.text().await.map_err(|_| {
                            log::warn!("Other network error without body");
                            DsRequestError::BadResponse
                        })?;
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
        payload: CreateGroupParamsOut,
        group_state_ear_key: &GroupStateEarKey,
        signing_key: &UserAuthSigningKey,
    ) -> Result<(), DsRequestError> {
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
        payload: AddUsersParamsOut,
        group_state_ear_key: &GroupStateEarKey,
        signing_key: &UserAuthSigningKey,
    ) -> Result<(), DsRequestError> {
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
        params: RemoveUsersParamsOut,
        group_state_ear_key: &GroupStateEarKey,
        signing_key: &UserAuthSigningKey,
    ) -> Result<(), DsRequestError> {
        self.prepare_and_send_ds_message(
            DsRequestParamsOut::RemoveUsers(params),
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
        signing_key: &InfraCredentialSigningKey,
    ) -> Result<RatchetTreeIn, DsRequestError> {
        let payload = WelcomeInfoParams {
            sender: signing_key.credential().verifying_key().clone(),
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
    ) -> Result<ExternalCommitInfoIn, DsRequestError> {
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

    /// Get external commit information for a connection group.
    pub async fn ds_connection_group_info(
        &self,
        group_id: GroupId,
        group_state_ear_key: &GroupStateEarKey,
    ) -> Result<ExternalCommitInfoIn, DsRequestError> {
        let payload = ConnectionGroupInfoParams { group_id };
        self.prepare_and_send_ds_message(
            DsRequestParamsOut::ConnectionGroupInfo(payload),
            AuthenticationMethod::<InfraCredentialSigningKey>::None,
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
        params: UpdateClientParamsOut,
        group_state_ear_key: &GroupStateEarKey,
        signing_key: &InfraCredentialSigningKey,
    ) -> Result<(), DsRequestError> {
        self.prepare_and_send_ds_message(
            DsRequestParamsOut::UpdateClient(params),
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
        external_commit: AssistedMessageOut,
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
        commit: MlsMessageOut,
        group_info: MlsMessageOut,
        qs_client_reference: QsClientReference,
        signing_key: &UserAuthSigningKey,
        group_state_ear_key: &GroupStateEarKey,
    ) -> Result<(), DsRequestError> {
        let external_commit = AssistedMessageOut {
            mls_message: commit,
            group_info_option: Some(AssistedGroupInfo::Full(group_info)),
        };
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
        commit: AssistedMessageOut,
        welcome: AssistedWelcome,
        encrypted_welcome_attribution_infos: Vec<EncryptedWelcomeAttributionInfo>,
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
        commit: AssistedMessageOut,
        new_auth_key: UserAuthVerifyingKey,
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
        external_commit: AssistedMessageOut,
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
        params: SelfRemoveClientParamsOut,
        signing_key: &UserAuthSigningKey,
        group_state_ear_key: &GroupStateEarKey,
    ) -> Result<(), DsRequestError> {
        self.prepare_and_send_ds_message(
            DsRequestParamsOut::SelfRemoveClient(params),
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
        params: SendMessageParamsOut,
        signing_key: &InfraCredentialSigningKey,
        group_state_ear_key: &GroupStateEarKey,
    ) -> Result<(), DsRequestError> {
        self.prepare_and_send_ds_message(
            DsRequestParamsOut::SendMessage(params),
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
        params: DeleteGroupParamsOut,
        signing_key: &UserAuthSigningKey,
        group_state_ear_key: &GroupStateEarKey,
    ) -> Result<(), DsRequestError> {
        self.prepare_and_send_ds_message(
            DsRequestParamsOut::DeleteGroup(params),
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
        signing_key: &InfraCredentialSigningKey,
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
