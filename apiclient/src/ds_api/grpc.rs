// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::{
    messages::AssistedMessageOut,
    openmls::{
        group::GroupEpoch,
        prelude::{GroupId, LeafNodeIndex, SignaturePublicKey},
    },
};
use phnxprotos::{
    convert::{RefInto, TryRefInto},
    delivery_service::v1::{
        AddUsersInfo, ConnectionGroupInfoRequest, CreateGroupPayload, DeleteGroupPayload,
        ExternalCommitInfoRequest, GroupOperationPayload, JoinConnectionGroupRequest,
        RequestGroupIdRequest, ResyncPayload, SelfRemovePayload, SendMessagePayload, UpdatePayload,
        UpdateProfileKeyPayload, WelcomeInfoPayload,
        delivery_service_client::DeliveryServiceClient,
    },
    validation::MissingFieldExt,
};
use phnxtypes::{
    credentials::keys::ClientSigningKey,
    crypto::{ear::keys::GroupStateEarKey, signatures::signable::Signable},
    identifiers::{QsReference, QualifiedGroupId},
    messages::{
        client_ds::UserProfileKeyUpdateParams,
        client_ds_out::{
            CreateGroupParamsOut, DeleteGroupParamsOut, ExternalCommitInfoIn,
            GroupOperationParamsOut, SendMessageParamsOut, WelcomeInfoIn,
        },
    },
    time::TimeStamp,
};
use tonic::transport::Channel;
use tracing::error;

use super::DsRequestError;

#[derive(Debug, Clone)]
pub(crate) struct DsGrpcClient {
    client: DeliveryServiceClient<Channel>,
}

impl DsGrpcClient {
    pub(crate) fn new(client: DeliveryServiceClient<Channel>) -> Self {
        Self { client }
    }

    pub(crate) async fn request_group_id(&self) -> Result<GroupId, DsRequestError> {
        let response = self
            .client
            .clone()
            .request_group_id(RequestGroupIdRequest {})
            .await?
            .into_inner();
        let qgid: QualifiedGroupId = response
            .group_id
            .ok_or_missing_field("group_id")
            .map_err(|error| {
                error!(%error, "unexpected response");
                DsRequestError::UnexpectedResponse
            })?
            .try_ref_into()
            .map_err(|error| {
                error!(%error, "unexpected response");
                DsRequestError::UnexpectedResponse
            })?;
        Ok(qgid.into())
    }

    pub(crate) async fn send_message(
        &self,
        params: SendMessageParamsOut,
        signing_key: &ClientSigningKey,
        group_state_ear_key: &GroupStateEarKey,
    ) -> Result<TimeStamp, DsRequestError> {
        let payload = SendMessagePayload {
            group_state_ear_key: Some(group_state_ear_key.ref_into()),
            message: Some(params.message.try_ref_into()?),
            sender: Some(params.sender.into()),
        };

        let request = payload.sign(signing_key)?;
        let response = self
            .client
            .clone()
            .send_message(request)
            .await?
            .into_inner();

        Ok(response
            .fanout_timestamp
            .ok_or(DsRequestError::UnexpectedResponse)?
            .into())
    }

    pub(crate) async fn create_group(
        &self,
        payload: CreateGroupParamsOut,
        signing_key: &ClientSigningKey,
        group_state_ear_key: &GroupStateEarKey,
    ) -> Result<(), DsRequestError> {
        let qgid: QualifiedGroupId = payload.group_id.try_into()?;
        let payload = CreateGroupPayload {
            qgid: Some(qgid.ref_into()),
            group_state_ear_key: Some(group_state_ear_key.ref_into()),
            ratchet_tree: Some(payload.ratchet_tree.try_ref_into()?),
            encrypted_identity_link_key: Some(payload.encrypted_identity_link_key.into()),
            encrypted_user_profile_key: Some(payload.encrypted_user_profile_key.into()),
            creator_client_reference: Some(payload.creator_client_reference.into()),
            group_info: Some(payload.group_info.try_ref_into()?),
            room_state: payload.room_state,
        };
        let request = payload.sign(signing_key)?;
        self.client.clone().create_group(request).await?;
        Ok(())
    }

    pub(crate) async fn delete_group(
        &self,
        params: DeleteGroupParamsOut,
        signing_key: &ClientSigningKey,
        group_state_ear_key: &GroupStateEarKey,
    ) -> Result<TimeStamp, DsRequestError> {
        let payload = DeleteGroupPayload {
            group_state_ear_key: Some(group_state_ear_key.ref_into()),
            commit: Some(params.commit.try_ref_into()?),
        };
        let request = payload.sign(signing_key)?;
        let response = self
            .client
            .clone()
            .delete_group(request)
            .await?
            .into_inner();
        Ok(response
            .fanout_timestamp
            .ok_or(DsRequestError::UnexpectedResponse)?
            .into())
    }

    pub(crate) async fn group_operation(
        &self,
        payload: GroupOperationParamsOut,
        signing_key: &ClientSigningKey,
        group_state_ear_key: &GroupStateEarKey,
    ) -> Result<TimeStamp, DsRequestError> {
        let add_users_info = payload
            .add_users_info_option
            .map(|add_user_infos| {
                Ok::<_, DsRequestError>(AddUsersInfo {
                    welcome: Some(add_user_infos.welcome.try_ref_into()?),
                    encrypted_welcome_attribution_info: add_user_infos
                        .encrypted_welcome_attribution_infos
                        .into_iter()
                        .map(From::from)
                        .collect(),
                })
            })
            .transpose()?;
        let payload = GroupOperationPayload {
            group_state_ear_key: Some(group_state_ear_key.ref_into()),
            commit: Some(payload.commit.try_ref_into()?),
            add_users_info,
        };
        let request = payload.sign(signing_key)?;
        let response = self
            .client
            .clone()
            .group_operation(request)
            .await?
            .into_inner();
        Ok(response
            .fanout_timestamp
            .ok_or(DsRequestError::UnexpectedResponse)?
            .into())
    }

    pub(crate) async fn connection_group_info(
        &self,
        group_id: GroupId,
        group_state_ear_key: &GroupStateEarKey,
    ) -> Result<ExternalCommitInfoIn, DsRequestError> {
        let qgid: QualifiedGroupId = group_id.try_into()?;
        let request = ConnectionGroupInfoRequest {
            group_id: Some(qgid.ref_into()),
            group_state_ear_key: Some(group_state_ear_key.ref_into()),
        };
        let response = self
            .client
            .clone()
            .connection_group_info(request)
            .await?
            .into_inner();
        Ok(ExternalCommitInfoIn {
            verifiable_group_info: response
                .group_info
                .ok_or(DsRequestError::UnexpectedResponse)?
                .try_ref_into()?,
            ratchet_tree_in: response
                .ratchet_tree
                .ok_or(DsRequestError::UnexpectedResponse)?
                .try_ref_into()?,
            encrypted_identity_link_keys: response
                .encrypted_identity_link_keys
                .into_iter()
                .map(TryFrom::try_from)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| DsRequestError::UnexpectedResponse)?,
            encrypted_user_profile_keys: response
                .encrypted_user_profile_keys
                .into_iter()
                .map(TryFrom::try_from)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| DsRequestError::UnexpectedResponse)?,
            room_state: response.room_state,
        })
    }

    pub(crate) async fn update(
        &self,
        commit: AssistedMessageOut,
        signing_key: &ClientSigningKey,
        group_state_ear_key: &GroupStateEarKey,
    ) -> Result<TimeStamp, DsRequestError> {
        let payload = UpdatePayload {
            group_state_ear_key: Some(group_state_ear_key.ref_into()),
            commit: Some(commit.try_ref_into()?),
        };
        let request = payload.sign(signing_key)?;
        let response = self.client.clone().update(request).await?.into_inner();
        Ok(response
            .fanout_timestamp
            .ok_or(DsRequestError::UnexpectedResponse)?
            .into())
    }

    pub(crate) async fn join_connection_group(
        &self,
        external_commit: AssistedMessageOut,
        qs_client_reference: QsReference,
        group_state_ear_key: &GroupStateEarKey,
    ) -> Result<TimeStamp, DsRequestError> {
        let request = JoinConnectionGroupRequest {
            group_state_ear_key: Some(group_state_ear_key.ref_into()),
            external_commit: Some(external_commit.try_ref_into()?),
            qs_client_reference: Some(qs_client_reference.into()),
        };
        let response = self
            .client
            .clone()
            .join_connection_group(request)
            .await?
            .into_inner();
        Ok(response
            .fanout_timestamp
            .ok_or(DsRequestError::UnexpectedResponse)?
            .into())
    }

    pub(crate) async fn self_remove(
        &self,
        remove_proposal: AssistedMessageOut,
        signing_key: &ClientSigningKey,
        group_state_ear_key: &GroupStateEarKey,
    ) -> Result<TimeStamp, DsRequestError> {
        let payload = SelfRemovePayload {
            group_state_ear_key: Some(group_state_ear_key.ref_into()),
            remove_proposal: Some(remove_proposal.try_ref_into()?),
        };
        let request = payload.sign(signing_key)?;
        let response = self.client.clone().self_remove(request).await?.into_inner();
        Ok(response
            .fanout_timestamp
            .ok_or(DsRequestError::UnexpectedResponse)?
            .into())
    }

    pub(crate) async fn welcome_info(
        &self,
        group_id: GroupId,
        epoch: GroupEpoch,
        group_state_ear_key: &GroupStateEarKey,
        signing_key: &ClientSigningKey,
    ) -> Result<WelcomeInfoIn, DsRequestError> {
        let qgid: QualifiedGroupId = group_id.try_into()?;

        let signature_public_key =
            SignaturePublicKey::from(signing_key.credential().verifying_key().clone());

        let payload = WelcomeInfoPayload {
            qgid: Some(qgid.ref_into()),
            group_state_ear_key: Some(group_state_ear_key.ref_into()),
            sender: Some(signature_public_key.ref_into()),
            epoch: Some(epoch.into()),
        };

        let request = payload.sign(signing_key)?;
        let response = self
            .client
            .clone()
            .welcome_info(request)
            .await?
            .into_inner();
        Ok(WelcomeInfoIn {
            ratchet_tree: response
                .ratchet_tree
                .ok_or(DsRequestError::UnexpectedResponse)?
                .try_ref_into()?,
            encrypted_identity_link_keys: response
                .encrypted_identity_link_keys
                .into_iter()
                .map(TryFrom::try_from)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| DsRequestError::UnexpectedResponse)?,
            encrypted_user_profile_keys: response
                .encrypted_user_profile_keys
                .into_iter()
                .map(TryFrom::try_from)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| DsRequestError::UnexpectedResponse)?,
        })
    }

    pub(crate) async fn resync(
        &self,
        external_commit: AssistedMessageOut,
        signing_key: &ClientSigningKey,
        own_leaf_index: LeafNodeIndex,
        group_state_ear_key: &GroupStateEarKey,
    ) -> Result<TimeStamp, DsRequestError> {
        let payload = ResyncPayload {
            group_state_ear_key: Some(group_state_ear_key.ref_into()),
            external_commit: Some(external_commit.try_ref_into()?),
            sender: Some(own_leaf_index.into()),
        };
        let request = payload.sign(signing_key)?;
        let response = self.client.clone().resync(request).await?.into_inner();
        Ok(response
            .fanout_timestamp
            .ok_or(DsRequestError::UnexpectedResponse)?
            .into())
    }

    pub(crate) async fn external_commit_info(
        &self,
        group_id: GroupId,
        group_state_ear_key: &GroupStateEarKey,
    ) -> Result<ExternalCommitInfoIn, DsRequestError> {
        let qgid: QualifiedGroupId = group_id.try_into()?;
        let request = ExternalCommitInfoRequest {
            qgid: Some(qgid.ref_into()),
            group_state_ear_key: Some(group_state_ear_key.ref_into()),
        };
        let response = self
            .client
            .clone()
            .external_commit_info(request)
            .await?
            .into_inner();
        Ok(ExternalCommitInfoIn {
            verifiable_group_info: response
                .group_info
                .ok_or(DsRequestError::UnexpectedResponse)?
                .try_ref_into()?,
            ratchet_tree_in: response
                .ratchet_tree
                .ok_or(DsRequestError::UnexpectedResponse)?
                .try_ref_into()?,
            encrypted_identity_link_keys: response
                .encrypted_identity_link_keys
                .into_iter()
                .map(TryFrom::try_from)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| DsRequestError::UnexpectedResponse)?,
            encrypted_user_profile_keys: response
                .encrypted_user_profile_keys
                .into_iter()
                .map(TryFrom::try_from)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| DsRequestError::UnexpectedResponse)?,
            room_state: response.room_state,
        })
    }

    pub(crate) async fn user_profile_key_update(
        &self,
        params: UserProfileKeyUpdateParams,
        signing_key: &ClientSigningKey,
        group_state_ear_key: &GroupStateEarKey,
    ) -> Result<(), DsRequestError> {
        let qgid: QualifiedGroupId = params.group_id.try_into()?;
        let payload = UpdateProfileKeyPayload {
            group_state_ear_key: Some(group_state_ear_key.ref_into()),
            group_id: Some(qgid.ref_into()),
            sender: Some(params.sender_index.into()),
            encrypted_user_profile_key: Some(params.user_profile_key.into()),
        };
        let request = payload.sign(signing_key)?;
        self.client.clone().update_profile_key(request).await?;
        Ok(())
    }
}
