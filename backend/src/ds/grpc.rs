// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxprotos::delivery_service::v1::{
    ConnectionGroupInfoRequest, ConnectionGroupInfoResponse, CreateGroupRequest,
    CreateGroupResponse, DeleteGroupRequest, DeleteGroupResponse, ExternalCommitInfoRequest,
    ExternalCommitInfoResponse, GroupOperationRequest, GroupOperationResponse,
    JoinConnectionGroupRequest, JoinConnectionGroupResponse, RequestGroupIdRequest,
    RequestGroupIdResponse, ResyncRequest, ResyncResponse, SelfRemoveRequest, SelfRemoveResponse,
    SendMessageRequest, SendMessageResponse, UpdateQsClientReferenceRequest,
    UpdateQsClientReferenceResponse, UpdateRequest, UpdateResponse, WelcomeInfoRequest,
    WelcomeInfoResponse,
};
use tonic::{Request, Response, Status, async_trait};

use crate::qs::QsConnector;

use super::Ds;

#[expect(dead_code)]
pub struct GrpcDs<Qep: QsConnector> {
    ds: Ds,
    qs_connector: Qep,
}

impl<Qep: QsConnector> GrpcDs<Qep> {
    pub fn new(ds: Ds, qs_connector: Qep) -> Self {
        Self { ds, qs_connector }
    }
}

#[async_trait]
impl<Qep: QsConnector> phnxprotos::delivery_service::v1::delivery_service_server::DeliveryService
    for GrpcDs<Qep>
{
    async fn request_group_id(
        &self,
        _request: Request<RequestGroupIdRequest>,
    ) -> Result<Response<RequestGroupIdResponse>, Status> {
        todo!()
    }

    async fn create_group(
        &self,
        _request: Request<CreateGroupRequest>,
    ) -> Result<Response<CreateGroupResponse>, Status> {
        todo!()
    }

    async fn welcome_info(
        &self,
        _request: Request<WelcomeInfoRequest>,
    ) -> Result<Response<WelcomeInfoResponse>, Status> {
        todo!()
    }

    async fn external_commit_info(
        &self,
        _request: Request<ExternalCommitInfoRequest>,
    ) -> Result<Response<ExternalCommitInfoResponse>, Status> {
        todo!()
    }

    async fn connection_group_info(
        &self,
        _request: Request<ConnectionGroupInfoRequest>,
    ) -> Result<Response<ConnectionGroupInfoResponse>, Status> {
        todo!()
    }

    async fn update_qs_client_reference(
        &self,
        _request: Request<UpdateQsClientReferenceRequest>,
    ) -> Result<Response<UpdateQsClientReferenceResponse>, Status> {
        todo!()
    }

    async fn update(
        &self,
        _request: Request<UpdateRequest>,
    ) -> Result<Response<UpdateResponse>, Status> {
        todo!()
    }

    async fn join_connection_group(
        &self,
        _request: Request<JoinConnectionGroupRequest>,
    ) -> Result<Response<JoinConnectionGroupResponse>, Status> {
        todo!()
    }

    async fn resync(
        &self,
        _request: Request<ResyncRequest>,
    ) -> Result<Response<ResyncResponse>, Status> {
        todo!()
    }

    async fn self_remove(
        &self,
        _request: Request<SelfRemoveRequest>,
    ) -> Result<Response<SelfRemoveResponse>, Status> {
        todo!()
    }

    async fn send_message(
        &self,
        _request: Request<SendMessageRequest>,
    ) -> Result<Response<SendMessageResponse>, Status> {
        todo!()
    }

    async fn delete_group(
        &self,
        _request: Request<DeleteGroupRequest>,
    ) -> Result<Response<DeleteGroupResponse>, Status> {
        todo!()
    }

    async fn group_operation(
        &self,
        _request: Request<GroupOperationRequest>,
    ) -> Result<Response<GroupOperationResponse>, Status> {
        todo!()
    }
}
