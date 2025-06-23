// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use aws_config::Region;
use aws_sdk_s3::{
    Client, Config,
    config::{Credentials, http},
    error::{BuildError, SdkError},
    operation::{get_object, put_object},
    presigning::{PresigningConfig, PresigningConfigError},
};
use chrono::{DateTime, Duration, Utc};
use displaydoc::Display;
use phnxcommon::{identifiers::AttachmentId, time::ExpirationData};
use phnxprotos::delivery_service::v1::{
    GetAttachmentUrlResponse, HeaderEntry, ProvisionAttachmentPayload, ProvisionAttachmentResponse,
};
use tonic::{Response, Status};
use tracing::error;
use uuid::Uuid;

use super::Ds;

impl Ds {
    pub(super) async fn provision_attachment(
        &self,
        _payload: ProvisionAttachmentPayload,
    ) -> Result<Response<ProvisionAttachmentResponse>, ProvisionAttachmentError> {
        let minio_endpoint = "http://localhost:9000";
        let minio_access_key_id = "minioaccesskey";
        let minio_secret_access_key = "miniosecretkey";
        let minio_region = "eu-west-1";

        let credentials = Credentials::new(
            minio_access_key_id,
            minio_secret_access_key,
            None,
            None,
            "minio",
        );

        let config = Config::builder()
            .endpoint_url(minio_endpoint)
            .region(Region::new(minio_region))
            .credentials_provider(credentials.clone())
            .force_path_style(true)
            .behavior_version_latest()
            .build();

        let attachment_id = Uuid::new_v4();

        let client = Client::from_conf(config);

        let expiration = ExpirationData::now(Duration::minutes(5));
        let not_before: DateTime<Utc> = expiration.not_before().into();
        let not_after: DateTime<Utc> = expiration.not_after().into();
        let duration = not_after - not_before;

        let mut presigning_config = PresigningConfig::builder();
        presigning_config.set_start_time(Some(not_before.into()));
        presigning_config.set_expires_in(Some(duration.to_std()?));
        let presigning_config = presigning_config.build()?;

        let request = client
            .put_object()
            .bucket("data")
            .key(attachment_id.as_simple().to_string())
            .presigned(presigning_config)
            .await
            .map_err(Box::new)?;

        let url = request.uri().to_owned();
        let header: Vec<HeaderEntry> = request
            .headers()
            .map(|(k, v)| HeaderEntry {
                key: k.to_owned(),
                value: v.to_owned(),
            })
            .collect();

        Ok(Response::new(ProvisionAttachmentResponse {
            attachment_id: Some(attachment_id.into()),
            upload_url_expiration: Some(expiration.into()),
            upload_url: url,
            upload_headers: header,
        }))
    }

    pub(super) async fn get_attachment_url(
        &self,
        attachment_id: AttachmentId,
    ) -> Result<Response<GetAttachmentUrlResponse>, GetAttachmentUrlError> {
        let minio_endpoint = "http://localhost:9000";
        let minio_access_key_id = "minioaccesskey";
        let minio_secret_access_key = "miniosecretkey";
        let minio_region = "eu-west-1";

        let credentials = Credentials::new(
            minio_access_key_id,
            minio_secret_access_key,
            None,
            None,
            "minio",
        );

        let config = Config::builder()
            .endpoint_url(minio_endpoint)
            .region(Region::new(minio_region))
            .credentials_provider(credentials.clone())
            .force_path_style(true)
            .behavior_version_latest()
            .build();

        let client = Client::from_conf(config);

        let expiration = ExpirationData::now(Duration::minutes(5));
        let not_before: DateTime<Utc> = expiration.not_before().into();
        let not_after: DateTime<Utc> = expiration.not_after().into();
        let duration = not_after - not_before;

        let mut presigning_config = PresigningConfig::builder();
        presigning_config.set_start_time(Some(not_before.into()));
        presigning_config.set_expires_in(Some(duration.to_std()?));
        let presigning_config = presigning_config.build()?;

        let request = client
            .get_object()
            .bucket("data")
            .key(attachment_id.uuid().as_simple().to_string())
            .presigned(presigning_config)
            .await
            .map_err(Box::new)?;

        let url = request.uri().to_owned();
        let headers: Vec<HeaderEntry> = request
            .headers()
            .map(|(k, v)| HeaderEntry {
                key: k.to_owned(),
                value: v.to_owned(),
            })
            .collect();

        Ok(Response::new(GetAttachmentUrlResponse {
            download_url_expiration: Some(expiration.into()),
            download_url: url,
            download_headers: headers,
        }))
    }
}

#[derive(Debug, thiserror::Error, Display)]
pub(super) enum ProvisionAttachmentError {
    /// Internal error
    Build(#[from] BuildError),
    /// Internal error
    Duration(#[from] chrono::OutOfRangeError),
    /// Internal error
    Presigning(#[from] PresigningConfigError),
    /// Internal error
    Sdk(#[from] Box<SdkError<put_object::PutObjectError, http::HttpResponse>>),
}

impl From<ProvisionAttachmentError> for Status {
    fn from(error: ProvisionAttachmentError) -> Self {
        let msg = error.to_string();
        match error {
            ProvisionAttachmentError::Build(error) => {
                error!(%error, "Failed to build S3 config");
                Status::internal(msg)
            }
            ProvisionAttachmentError::Duration(error) => {
                error!(%error, "Failed to convert chrono to std duration");
                Status::internal(msg)
            }
            ProvisionAttachmentError::Presigning(error) => {
                error!(%error, "Failed to create presigning config");
                Status::internal(msg)
            }
            ProvisionAttachmentError::Sdk(error) => {
                error!(%error, "Failed to build S3 request");
                Status::internal(msg)
            }
        }
    }
}

#[derive(Debug, thiserror::Error, Display)]
pub(super) enum GetAttachmentUrlError {
    /// Internal error
    Build(#[from] BuildError),
    /// Internal error
    Duration(#[from] chrono::OutOfRangeError),
    /// Internal error
    Presigning(#[from] PresigningConfigError),
    /// Internal error
    Sdk(#[from] Box<SdkError<get_object::GetObjectError, http::HttpResponse>>),
}

impl From<GetAttachmentUrlError> for Status {
    fn from(error: GetAttachmentUrlError) -> Self {
        let msg = error.to_string();
        match error {
            GetAttachmentUrlError::Build(error) => {
                error!(%error, "Failed to build S3 config");
                Status::internal(msg)
            }
            GetAttachmentUrlError::Duration(error) => {
                error!(%error, "Failed to convert chrono to std duration");
                Status::internal(msg)
            }
            GetAttachmentUrlError::Presigning(error) => {
                error!(%error, "Failed to create presigning config");
                Status::internal(msg)
            }
            GetAttachmentUrlError::Sdk(error) => {
                error!(%error, "Failed to build S3 request");
                Status::internal(msg)
            }
        }
    }
}
