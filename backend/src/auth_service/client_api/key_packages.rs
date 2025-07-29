// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxcommon::{
    identifiers::UserHandleHash,
    messages::connection_package::{ConnectionPackage, ConnectionPackageIn},
};

use crate::{
    auth_service::{AuthService, connection_package::StorableConnectionPackage},
    errors::auth_service::PublishConnectionPackageError,
};

impl AuthService {
    pub(crate) async fn as_publish_connection_packages_for_handle(
        &self,
        hash: &UserHandleHash,
        connection_packages: Vec<ConnectionPackageIn>,
    ) -> Result<(), PublishConnectionPackageError> {
        // TODO(#496): Last resort connection package
        let connection_packages = connection_packages
            .into_iter()
            .map(|cp| {
                cp.verify()
                    .map_err(|_| PublishConnectionPackageError::InvalidKeyPackage)
            })
            .collect::<Result<Vec<ConnectionPackage>, PublishConnectionPackageError>>()?;

        StorableConnectionPackage::store_multiple_for_handle(
            &self.db_pool,
            &connection_packages,
            hash,
        )
        .await
        .map_err(|_| PublishConnectionPackageError::StorageError)?;
        Ok(())
    }
}
