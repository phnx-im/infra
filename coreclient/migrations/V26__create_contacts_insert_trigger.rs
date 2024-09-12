// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later


use crate::groups::openmls_provider::key_packages::StorableKeyPackage;
use crate::utils::persistence::Storable;

/// OpenMLS provider data
pub fn migration() -> String {
    "DROP TRIGGER IF EXISTS no_contact_overlap_on_insert;

    CREATE TRIGGER no_contact_overlap_on_insert
    BEFORE INSERT ON partial_contacts
    FOR EACH ROW
    BEGIN
        SELECT CASE
            WHEN EXISTS (SELECT 1 FROM contacts WHERE user_name = NEW.user_name)
            THEN RAISE(FAIL, 'Can''t insert PartialContact: There already exists a contact with this user_name')
        END;
    END;".to_string()
}
