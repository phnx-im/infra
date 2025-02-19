// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! API endpoints of the DS

use super::*;
use mls_assist::{
    messages::AssistedMessageOut,
    openmls::prelude::{
        tls_codec::Serialize, GroupEpoch, GroupId, LeafNodeIndex, MlsMessageOut, RatchetTreeIn,
    },
};
use phnxtypes::{
    credentials::keys::PseudonymousCredentialSigningKey,
    crypto::{
        ear::keys::GroupStateEarKey,
        signatures::{signable::Signable, traits::SigningKeyBehaviour},
    },
    endpoint_paths::ENDPOINT_DS_GROUPS,
    identifiers::QsReference,
    messages::{
        client_ds::{
            ConnectionGroupInfoParams, ExternalCommitInfoParams, UpdateQsClientReferenceParams,
            WelcomeInfoParams,
        },
        client_ds_out::{
            ClientToDsMessageOut, ClientToDsMessageTbsOut, CreateGroupParamsOut,
            DeleteGroupParamsOut, DsMessageTypeOut, DsProcessResponseIn, DsRequestParamsOut,
            ExternalCommitInfoIn, GroupOperationParamsOut, JoinConnectionGroupParamsOut,
            ResyncParamsOut, SelfRemoveParamsOut, SendMessageParamsOut, UpdateParamsOut,
        },
    },
    time::TimeStamp,
};

use tls_codec::DeserializeBytes;
use tracing::warn;

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
    #[error("DS Error: {0}")]
    DsError(String),
}

pub enum AuthenticationMethod<'a, T: SigningKeyBehaviour> {
    Signature(&'a T),
    None,
}

impl<'a, T: SigningKeyBehaviour + 'a> From<&'a T> for AuthenticationMethod<'a, T> {
    fn from(key: &'a T) -> Self {
        AuthenticationMethod::Signature(key)
    }
}

impl ApiClient {
    // Single purpose function since this is the only endpoint that doesn't require authentication.
    pub async fn send_ds_message(
        &self,
        message: DsMessageTypeOut,
    ) -> Result<DsProcessResponseIn, DsRequestError> {
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
                    _ if res.status().is_success() => {
                        let ds_proc_res_bytes =
                            res.bytes().await.map_err(|_| DsRequestError::BadResponse)?;
                        let ds_proc_res =
                            DsProcessResponseIn::tls_deserialize_exact_bytes(&ds_proc_res_bytes)
                                .map_err(|error| {
                                    warn!(%error, "Couldn't deserialize OK response body");
                                    DsRequestError::BadResponse
                                })?;
                        Ok(ds_proc_res)
                    }
                    // DS Specific Error
                    418 => {
                        let ds_proc_err_bytes = res.bytes().await.map_err(|_| {
                            warn!("No body in DS-error response");
                            DsRequestError::BadResponse
                        })?;
                        let ds_proc_err =
                            String::from_utf8(ds_proc_err_bytes.to_vec()).map_err(|_| {
                                warn!("Couldn't deserialize DS-error response body");
                                DsRequestError::BadResponse
                            })?;
                        Err(DsRequestError::DsError(ds_proc_err))
                    }
                    // All other errors
                    _ => {
                        let error_text = res.text().await.map_err(|_| {
                            warn!("Other network error without body");
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

    async fn prepare_and_send_ds_group_message<'a, T: SigningKeyBehaviour + 'a>(
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
        let message_type = DsMessageTypeOut::Group(message);
        self.send_ds_message(message_type).await
    }

    /// Creates a new group on the DS.
    pub async fn ds_create_group(
        &self,
        payload: CreateGroupParamsOut,
        signing_key: &PseudonymousCredentialSigningKey,
        group_state_ear_key: &GroupStateEarKey,
    ) -> Result<(), DsRequestError> {
        self.prepare_and_send_ds_group_message(
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

    /// Performs a group operation.
    pub async fn ds_group_operation(
        &self,
        payload: GroupOperationParamsOut,
        signing_key: &PseudonymousCredentialSigningKey,
        group_state_ear_key: &GroupStateEarKey,
    ) -> Result<TimeStamp, DsRequestError> {
        self.prepare_and_send_ds_group_message(
            DsRequestParamsOut::GroupOperation(payload),
            signing_key,
            group_state_ear_key,
        )
        .await
        // Check if the response is what we expected it to be.
        .and_then(|response| {
            if let DsProcessResponseIn::FanoutTimestamp(ts) = response {
                Ok(ts)
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
        signing_key: &PseudonymousCredentialSigningKey,
    ) -> Result<RatchetTreeIn, DsRequestError> {
        let payload = WelcomeInfoParams {
            sender: signing_key.credential().verifying_key().clone(),
            group_id,
            epoch,
        };
        self.prepare_and_send_ds_group_message(
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
    ) -> Result<ExternalCommitInfoIn, DsRequestError> {
        let payload = ExternalCommitInfoParams { group_id };
        self.prepare_and_send_ds_group_message(
            DsRequestParamsOut::ExternalCommitInfo(payload),
            AuthenticationMethod::<PseudonymousCredentialSigningKey>::None,
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
        self.prepare_and_send_ds_group_message(
            DsRequestParamsOut::ConnectionGroupInfo(payload),
            AuthenticationMethod::<PseudonymousCredentialSigningKey>::None,
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
    /// have [`phnxtypes::messages::client_ds::UpdateClientParamsAad`] in its AAD.
    pub async fn ds_update(
        &self,
        params: UpdateParamsOut,
        signing_key: &PseudonymousCredentialSigningKey,
        group_state_ear_key: &GroupStateEarKey,
    ) -> Result<TimeStamp, DsRequestError> {
        self.prepare_and_send_ds_group_message(
            DsRequestParamsOut::Update(params),
            signing_key,
            group_state_ear_key,
        )
        .await
        // Check if the response is what we expected it to be.
        .and_then(|response| {
            if let DsProcessResponseIn::FanoutTimestamp(ts) = response {
                Ok(ts)
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
        qs_client_reference: QsReference,
        group_state_ear_key: &GroupStateEarKey,
    ) -> Result<TimeStamp, DsRequestError> {
        // We unwrap here, because we know that the group_info is present.
        let external_commit = AssistedMessageOut::new(commit, Some(group_info)).unwrap();
        let payload = JoinConnectionGroupParamsOut {
            external_commit,
            qs_client_reference,
        };
        self.prepare_and_send_ds_group_message(
            DsRequestParamsOut::JoinConnectionGroup(payload),
            AuthenticationMethod::<PseudonymousCredentialSigningKey>::None,
            group_state_ear_key,
        )
        .await
        // Check if the response is what we expected it to be.
        .and_then(|response| {
            if let DsProcessResponseIn::FanoutTimestamp(ts) = response {
                Ok(ts)
            } else {
                Err(DsRequestError::UnexpectedResponse)
            }
        })
    }

    /// Resync a client to rejoin a group.
    pub async fn ds_resync(
        &self,
        external_commit: AssistedMessageOut,
        signing_key: &PseudonymousCredentialSigningKey,
        group_state_ear_key: &GroupStateEarKey,
        own_leaf_index: LeafNodeIndex,
    ) -> Result<TimeStamp, DsRequestError> {
        let payload = ResyncParamsOut {
            external_commit,
            sender_index: own_leaf_index,
        };
        self.prepare_and_send_ds_group_message(
            DsRequestParamsOut::Resync(payload),
            signing_key,
            group_state_ear_key,
        )
        .await
        // Check if the response is what we expected it to be.
        .and_then(|response| {
            if let DsProcessResponseIn::FanoutTimestamp(ts) = response {
                Ok(ts)
            } else {
                Err(DsRequestError::UnexpectedResponse)
            }
        })
    }

    /// Leave the given group with this client.
    pub async fn ds_self_remove(
        &self,
        params: SelfRemoveParamsOut,
        signing_key: &PseudonymousCredentialSigningKey,
        group_state_ear_key: &GroupStateEarKey,
    ) -> Result<TimeStamp, DsRequestError> {
        self.prepare_and_send_ds_group_message(
            DsRequestParamsOut::SelfRemove(params),
            signing_key,
            group_state_ear_key,
        )
        .await
        // Check if the response is what we expected it to be.
        .and_then(|response| {
            if let DsProcessResponseIn::FanoutTimestamp(ts) = response {
                Ok(ts)
            } else {
                Err(DsRequestError::UnexpectedResponse)
            }
        })
    }

    /// Send a message to the given group.
    pub async fn ds_send_message(
        &self,
        params: SendMessageParamsOut,
        signing_key: &PseudonymousCredentialSigningKey,
        group_state_ear_key: &GroupStateEarKey,
    ) -> Result<TimeStamp, DsRequestError> {
        self.prepare_and_send_ds_group_message(
            DsRequestParamsOut::SendMessage(params),
            signing_key,
            group_state_ear_key,
        )
        .await
        // Check if the response is what we expected it to be.
        .and_then(|response| {
            if let DsProcessResponseIn::FanoutTimestamp(ts) = response {
                Ok(ts)
            } else {
                Err(DsRequestError::UnexpectedResponse)
            }
        })
    }

    /// Delete the given group.
    pub async fn ds_delete_group(
        &self,
        params: DeleteGroupParamsOut,
        signing_key: &PseudonymousCredentialSigningKey,
        group_state_ear_key: &GroupStateEarKey,
    ) -> Result<TimeStamp, DsRequestError> {
        self.prepare_and_send_ds_group_message(
            DsRequestParamsOut::DeleteGroup(params),
            signing_key,
            group_state_ear_key,
        )
        .await
        // Check if the response is what we expected it to be.
        .and_then(|response| {
            if let DsProcessResponseIn::FanoutTimestamp(ts) = response {
                Ok(ts)
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
        new_queue_config: QsReference,
        signing_key: &PseudonymousCredentialSigningKey,
        group_state_ear_key: &GroupStateEarKey,
    ) -> Result<(), DsRequestError> {
        let payload = UpdateQsClientReferenceParams {
            group_id,
            sender: own_index,
            new_qs_reference: new_queue_config,
        };
        self.prepare_and_send_ds_group_message(
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

    /// Delete the given group.
    pub async fn ds_request_group_id(&self) -> Result<GroupId, DsRequestError> {
        let message_type = DsMessageTypeOut::NonGroup;
        self.send_ds_message(message_type)
            .await
            // Check if the response is what we expected it to be.
            .and_then(|response| {
                if let DsProcessResponseIn::GroupId(group_id) = response {
                    Ok(group_id)
                } else {
                    Err(DsRequestError::UnexpectedResponse)
                }
            })
    }
}
