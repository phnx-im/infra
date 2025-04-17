// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! API endpoints of the DS

use crate::version::{extract_api_version_negotiation, negotiate_api_version};

use super::*;
use mls_assist::{
    messages::AssistedMessageOut,
    openmls::prelude::{GroupEpoch, GroupId, LeafNodeIndex, MlsMessageOut, tls_codec::Serialize},
};
use phnxtypes::{
    LibraryError,
    credentials::keys::PseudonymousCredentialSigningKey,
    crypto::{
        ear::keys::GroupStateEarKey,
        signatures::{signable::Signable, traits::SigningKeyBehaviour},
    },
    endpoint_paths::ENDPOINT_DS_GROUPS,
    errors::version::VersionError,
    identifiers::QsReference,
    messages::{
        client_ds::{
            ConnectionGroupInfoParams, ExternalCommitInfoParams, SUPPORTED_DS_API_VERSIONS,
            WelcomeInfoParams,
        },
        client_ds_out::{
            ClientToDsMessageOut, ClientToDsMessageTbsOut, CreateGroupParamsOut,
            DeleteGroupParamsOut, DsGroupRequestParamsOut, DsProcessResponseIn, DsRequestParamsOut,
            DsVersionedProcessResponseIn, DsVersionedRequestParamsOut, ExternalCommitInfoIn,
            GroupOperationParamsOut, JoinConnectionGroupParamsOut, ResyncParamsOut,
            SelfRemoveParamsOut, SendMessageParamsOut, UpdateParamsOut, WelcomeInfoIn,
        },
    },
    time::TimeStamp,
};
use tls_codec::DeserializeBytes;

pub mod grpc;

#[derive(Error, Debug)]
pub enum DsRequestError {
    #[error("Library Error")]
    LibraryError,
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error(transparent)]
    Tonic(#[from] tonic::Status),
    #[error(transparent)]
    Tls(#[from] tls_codec::Error),
    #[error("We received an unexpected response type.")]
    UnexpectedResponse,
    #[error("DS Error: {0}")]
    DsError(String),
    #[error("API Error: {0}")]
    Api(#[from] VersionError),
    #[error("Unsuccessful response: status = {status}, error = {error}")]
    RequestFailed { status: StatusCode, error: String },
}

impl From<LibraryError> for DsRequestError {
    fn from(_: LibraryError) -> Self {
        Self::LibraryError
    }
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
    async fn prepare_and_send_ds_message<'a, T: SigningKeyBehaviour + 'a>(
        &self,
        request_params: DsRequestParamsOut,
        auth_method: impl Into<AuthenticationMethod<'a, T>>,
    ) -> Result<DsProcessResponseIn, DsRequestError> {
        let api_version = self.negotiated_versions().ds_api_version();

        let auth_method = auth_method.into();
        let request_params =
            DsVersionedRequestParamsOut::with_version(request_params, api_version)?;
        let message = sign_ds_params(request_params, &auth_method)?;

        let response = self.send_ds_http_request(&message).await?;

        // check if we need to negotiate a new API version
        let Some(accepted_versions) = extract_api_version_negotiation(&response) else {
            return handle_ds_response(response).await;
        };

        let supported_versions = SUPPORTED_DS_API_VERSIONS;
        let accepted_version = negotiate_api_version(accepted_versions, supported_versions)
            .ok_or_else(|| VersionError::new(api_version, supported_versions))?;
        self.negotiated_versions()
            .set_ds_api_version(accepted_version);

        let (request_params, _) = message
            .into_payload()
            .into_body()
            .change_version(accepted_version)?;
        let message = sign_ds_params(request_params, &auth_method)?;

        let response = self.send_ds_http_request(&message).await?;
        handle_ds_response(response).await
    }

    async fn prepare_and_send_ds_group_message<'a, T: SigningKeyBehaviour + 'a>(
        &self,
        request_params: DsGroupRequestParamsOut,
        auth_method: impl Into<AuthenticationMethod<'a, T>>,
        group_state_ear_key: &GroupStateEarKey,
    ) -> Result<DsProcessResponseIn, DsRequestError> {
        self.prepare_and_send_ds_message(
            DsRequestParamsOut::Group {
                group_state_ear_key: group_state_ear_key.clone(),
                request_params,
            },
            auth_method,
        )
        .await
    }

    /// Creates a new group on the DS.
    pub async fn ds_create_group(
        &self,
        payload: CreateGroupParamsOut,
        signing_key: &PseudonymousCredentialSigningKey,
        group_state_ear_key: &GroupStateEarKey,
    ) -> Result<(), DsRequestError> {
        self.ds_grpc_client
            .create_group(payload, signing_key, group_state_ear_key)
            .await

        // self.prepare_and_send_ds_group_message(
        //     DsGroupRequestParamsOut::CreateGroupParams(payload),
        //     signing_key,
        //     group_state_ear_key,
        // )
        // .await
        // // Check if the response is what we expected it to be.
        // .and_then(|response| {
        //     if matches!(response, DsProcessResponseIn::Ok) {
        //         Ok(())
        //     } else {
        //         Err(DsRequestError::UnexpectedResponse)
        //     }
        // })
    }

    /// Performs a group operation.
    pub async fn ds_group_operation(
        &self,
        payload: GroupOperationParamsOut,
        signing_key: &PseudonymousCredentialSigningKey,
        group_state_ear_key: &GroupStateEarKey,
    ) -> Result<TimeStamp, DsRequestError> {
        self.prepare_and_send_ds_group_message(
            DsGroupRequestParamsOut::GroupOperation(payload),
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
    ) -> Result<WelcomeInfoIn, DsRequestError> {
        let payload = WelcomeInfoParams {
            sender: signing_key.credential().verifying_key().clone(),
            group_id,
            epoch,
        };
        self.prepare_and_send_ds_group_message(
            DsGroupRequestParamsOut::WelcomeInfo(payload),
            signing_key,
            group_state_ear_key,
        )
        .await
        // Check if the response is what we expected it to be.
        .and_then(|response| {
            if let DsProcessResponseIn::WelcomeInfo(welcome_info) = response {
                Ok(welcome_info)
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
            DsGroupRequestParamsOut::ExternalCommitInfo(payload),
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
            DsGroupRequestParamsOut::ConnectionGroupInfo(payload),
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

    /// Update your client in this group.
    pub async fn ds_update(
        &self,
        params: UpdateParamsOut,
        signing_key: &PseudonymousCredentialSigningKey,
        group_state_ear_key: &GroupStateEarKey,
    ) -> Result<TimeStamp, DsRequestError> {
        self.prepare_and_send_ds_group_message(
            DsGroupRequestParamsOut::Update(params),
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
            DsGroupRequestParamsOut::JoinConnectionGroup(payload),
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
            sender: own_leaf_index,
        };
        self.prepare_and_send_ds_group_message(
            DsGroupRequestParamsOut::Resync(payload),
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
            DsGroupRequestParamsOut::SelfRemove(params),
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
        self.ds_grpc_client
            .send_message(params, signing_key, group_state_ear_key)
            .await
    }

    /// Delete the given group.
    pub async fn ds_delete_group(
        &self,
        params: DeleteGroupParamsOut,
        signing_key: &PseudonymousCredentialSigningKey,
        group_state_ear_key: &GroupStateEarKey,
    ) -> Result<TimeStamp, DsRequestError> {
        self.ds_grpc_client
            .delete_group(params, signing_key, group_state_ear_key)
            .await
    }

    /// Delete the given group.
    pub async fn ds_request_group_id(&self) -> Result<GroupId, DsRequestError> {
        self.ds_grpc_client.request_group_id().await
    }

    async fn send_ds_http_request(
        &self,
        message: &ClientToDsMessageOut,
    ) -> Result<reqwest::Response, DsRequestError> {
        let message_bytes = message.tls_serialize_detached()?;
        let endpoint = self.build_url(Protocol::Http, ENDPOINT_DS_GROUPS);
        let response = self
            .client
            .post(endpoint)
            .body(message_bytes)
            .send()
            .await?;
        Ok(response)
    }
}

async fn handle_ds_response(res: reqwest::Response) -> Result<DsProcessResponseIn, DsRequestError> {
    let status = res.status();
    match status.as_u16() {
        // Success!
        _ if res.status().is_success() => {
            let ds_proc_res_bytes = res.bytes().await?;
            let ds_proc_res =
                DsVersionedProcessResponseIn::tls_deserialize_exact_bytes(&ds_proc_res_bytes)?
                    .into_unversioned()?;
            Ok(ds_proc_res)
        }
        // DS Specific Error
        418 => {
            let ds_proc_err_bytes = res.bytes().await?;
            let ds_proc_err = String::from_utf8_lossy(&ds_proc_err_bytes);
            Err(DsRequestError::DsError(ds_proc_err.into_owned()))
        }
        // All other errors
        _ => {
            let error = res
                .text()
                .await
                .unwrap_or_else(|error| format!("unprocessable response body due to: {error}"));
            Err(DsRequestError::RequestFailed { status, error })
        }
    }
}

fn sign_ds_params<'a, T: SigningKeyBehaviour + 'a>(
    request_params: DsVersionedRequestParamsOut,
    auth_method: &AuthenticationMethod<'a, T>,
) -> Result<ClientToDsMessageOut, DsRequestError> {
    let tbs = ClientToDsMessageTbsOut::new(request_params);
    let message = match auth_method {
        AuthenticationMethod::Signature(signer) => tbs.sign(*signer)?,
        AuthenticationMethod::None => ClientToDsMessageOut::without_signature(tbs),
    };
    Ok(message)
}
