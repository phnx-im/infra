//! OpenAPI definitions.
//!
//! Use `cargo test generate_api_docs --features=api_docs` to generate the JSON
//! file.

use super::endpoints::{ds::*, qs::*, *};
use actix_web::http::header::ContentType;
use utoipa::OpenApi;

use phnxbackend::{
    crypto::{
        ear::{
            keys::{
                DeleteAuthKeyEarKey, DeleteAuthKeyEarKeySecret, EnqueueAuthKeyEarKey,
                EnqueueAuthKeyEarKeySecret, GroupStateEarKey, GroupStateEarKeySecret,
                PushTokenEarKey, PushTokenEarKeySecret,
            },
            Ciphertext,
        },
        kdf::keys::{InitialClientKdfKey, InitialClientKdfKeySecret},
        mac::{
            keys::{EnqueueAuthKeyCtxt, EnqueueAuthenticationKey, EnqueueAuthenticationKeySecret},
            MacTag,
        },
        HpkePublicKey, Signature, SignaturePublicKey,
    },
    ds::group_state::*,
    ds::*,
    messages::{client_backend::*, *},
    qs::*,
};

#[derive(OpenApi)]
#[openapi(
    // Endpoints
    paths(
        health_check,
        // DS
        ds_create_group,
        ds_update_queue_info,
        ds_welcome_info,
        ds_external_commit_info,
        ds_add_users,
        ds_remove_users,
        ds_join_group,
        ds_join_connection_group,
        ds_add_clients,
        ds_remove_clients,
        ds_resync_client,
        ds_self_remove_client,
        ds_self_remove_user,
        ds_send_message,
        ds_delete_group,
        // QS
        qs_qc_encryption_key,
        qs_create_user_record,
        qs_update_user_record,
        qs_user_record,
        qs_delete_user_record,
        qs_create_client_record,
        qs_update_client_record,
        qs_client_record,
        qs_delete_client_record,
        qs_publish_key_packages,
        qs_client_key_package,
        qs_key_package_batch,
        qs_dequeue_messages,
    ),
    components(
        // Schema definitions
        schemas(
            // DS
            CreateGroupParams,
            UpdateQueueInfoParams,
            WelcomeInfoParams,
            ExternalCommitInfoParams,
            AddUsersParams,
            RemoveUsersParams,
            JoinGroupParams,
            JoinConnectionGroupParams,
            AddClientsParams,
            RemoveClientsParams,
            ResyncClientParams,
            SelfRemoveClientParams,
            SelfRemoveUserParams,
            SendMessageParams,
            DeleteGroupParams,
            // DS group state
            UserAuthKey,
            Fqdn,
            SealedQueueConfig,
            GroupInfoUpdate,
            WelcomeAttributionInfo,
            WelcomeAttributionInfoPayload,
            ClientId,
            GroupId,
            ClientQueueConfig,
            // QS
            DequeueMessagesParams,
            KeyPackageBatchParams,
            ClientKeyPackageParams,
            PublishKeyPackagesParams,
            DeleteClientRecordParams,
            ClientRecordParams,
            UpdateClientRecordParams,
            CreateClientRecordParams,
            DeleteUserRecordParams,
            UserRecordParams,
            UpdateUserRecordParams,
            CreateUserRecordParams,
            // QS user and client records
            QsUid,
            QsCid,
            FriendshipToken,
            // KeyPackage publishing
            AddPackage,
            KeyPackageBatch,
            // Legacy QS
            QsFetchMessagesParams,
            QsFetchMessageParamsTBS,
            QsUpdateQueueInfoParams,
            QsUpdateQueueInfoParamsTBS,
            QsQueueInfoUpdate,
            QsFanOutQueueUpdate,
            QsQueueUpdate,
            EncryptedPushToken,
            QsDeleteQueueRequest,
            QsDeleteQueueParams,
            // Crypto (partially legacy)
            GroupStateEarKey,
            GroupStateEarKeySecret,
            HpkePublicKey,
            Signature,
            SignaturePublicKey,
            Ciphertext,
            EnqueueAuthKeyCtxt,
            DeleteAuthKeyEarKey,
            DeleteAuthKeyEarKeySecret,
            MacTag,
            InitialClientKdfKey,
            InitialClientKdfKeySecret,
            PushTokenEarKey,
            PushTokenEarKeySecret,
            EnqueueAuthKeyEarKey,
            EnqueueAuthKeyEarKeySecret,
            EnqueueAuthenticationKey,
            EnqueueAuthenticationKeySecret,
            // MLS
            KeyPackage,
            MlsRatchetTree,
            MlsMessage,
            LeafIndex,
            Welcome,
        )
    )
)]
pub(crate) struct ApiDoc;

pub(crate) async fn serve_api_docs(_req: actix_web::HttpRequest) -> impl actix_web::Responder {
    actix_web::HttpResponse::Ok()
        .content_type(ContentType::json())
        .body(ApiDoc::openapi().to_pretty_json().unwrap())
}
