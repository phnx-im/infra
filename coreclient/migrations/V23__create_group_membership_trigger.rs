// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later


use crate::groups::openmls_provider::key_packages::StorableKeyPackage;
use crate::utils::persistence::Storable;

/// OpenMLS provider data
pub fn migration() -> String {
    "CREATE TRIGGER IF NOT EXISTS delete_orphaned_data 
        AFTER DELETE ON group_membership
        FOR EACH ROW
        BEGIN
            -- Delete client credentials if they are not our own and not used in any group.
            DELETE FROM client_credentials
            WHERE fingerprint = OLD.client_credential_fingerprint AND NOT EXISTS (
                SELECT 1 FROM group_membership WHERE client_credential_fingerprint = OLD.client_credential_fingerprint
            ) AND NOT EXISTS (
                SELECT 1 FROM own_client_info WHERE as_client_uuid = OLD.client_uuid
            );

            -- Delete user profiles of users that are not in any group and that are not our own.
            DELETE FROM users
            WHERE user_name = OLD.user_name AND NOT EXISTS (
                SELECT 1 FROM group_membership WHERE user_name = OLD.user_name
            ) AND NOT EXISTS (
                SELECT 1 FROM own_client_info WHERE as_user_name = OLD.user_name
            );
        END".to_string()
}
