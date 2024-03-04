// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use rusqlite::Connection;

use crate::{
    users::connection_establishment::FriendshipPackage,
    utils::persistence::{DataType, Persistable, PersistableStruct, PersistenceError, SqlKey},
    ConversationId,
};

use super::*;

pub(crate) struct ContactStore<'a> {
    db_connection: &'a Connection,
}

impl<'a> ContactStore<'a> {
    pub(crate) fn new(db_connection: &'a Connection) -> Self {
        Self { db_connection }
    }

    pub(crate) fn get(
        &self,
        user_name: &UserName,
    ) -> Result<Option<PersistableStruct<'a, Contact>>, PersistenceError> {
        PersistableStruct::load_one(&self.db_connection, Some(user_name), None)
    }

    pub(crate) fn store_partial_contact(
        &self,
        user_name: &UserName,
        conversation_id: &ConversationId,
        friendship_package_ear_key: FriendshipPackageEarKey,
    ) -> Result<PersistableStruct<'_, PartialContact>> {
        let payload = PartialContact::new(
            user_name.clone(),
            conversation_id.clone(),
            friendship_package_ear_key,
        )?;
        let partial_contact = PersistableStruct::<'_, PartialContact>::from_connection_and_payload(
            self.db_connection,
            payload,
        );
        partial_contact.persist()?;
        Ok(partial_contact)
    }

    pub(crate) fn get_partial_contact(
        &self,
        user_name: &UserName,
    ) -> Result<Option<PersistableStruct<'_, PartialContact>>, PersistenceError> {
        PersistableStruct::load_one(&self.db_connection, Some(user_name), None)
    }

    pub(crate) fn get_all_contacts(
        &self,
    ) -> Result<Vec<PersistableStruct<'_, Contact>>, PersistenceError> {
        PersistableStruct::load_all_unfiltered(self.db_connection)
    }

    pub(crate) fn get_all_partial_contacts(
        &self,
    ) -> Result<Vec<PersistableStruct<'_, PartialContact>>, PersistenceError> {
        PersistableStruct::load_all_unfiltered(self.db_connection)
    }
}

impl PersistableStruct<'_, Contact> {
    pub(crate) fn convert_for_export(self) -> Contact {
        self.payload
    }
}

impl SqlKey for UserName {
    fn to_sql_key(&self) -> String {
        self.to_string()
    }
}

impl Persistable for Contact {
    type Key = UserName;
    type SecondaryKey = UserName;
    const DATA_TYPE: DataType = DataType::Contact;

    fn key(&self) -> &Self::Key {
        &self.user_name
    }

    fn secondary_key(&self) -> &Self::SecondaryKey {
        &self.user_name
    }
}

impl PersistableStruct<'_, PartialContact> {
    /// Creates a Contact from this PartialContact and the additional data. Then
    /// persists the resulting contact.
    pub(crate) fn mark_as_complete(
        self,
        friendship_package: FriendshipPackage,
        client_credential: ClientCredential,
    ) -> Result<()> {
        // TODO: This should be a transaction
        self.purge()?;
        let payload = Contact {
            user_name: self.payload.user_name,
            client_credentials: vec![client_credential],
            wai_ear_key: friendship_package.wai_ear_key,
            friendship_token: friendship_package.friendship_token,
            add_package_ear_key: friendship_package.add_package_ear_key,
            client_credential_ear_key: friendship_package.client_credential_ear_key,
            signature_ear_key_wrapper_key: friendship_package.signature_ear_key_wrapper_key,
            conversation_id: self.payload.conversation_id,
            user_profile: friendship_package.user_profile,
        };
        let persistable_contact =
            PersistableStruct::from_connection_and_payload(self.connection, payload);
        persistable_contact.persist()?;
        Ok(())
    }

    pub(crate) fn convert_for_export(self) -> PartialContact {
        self.payload
    }
}

impl Persistable for PartialContact {
    type Key = UserName;

    type SecondaryKey = UserName;

    const DATA_TYPE: DataType = DataType::PartialContact;

    fn key(&self) -> &Self::Key {
        &self.user_name
    }

    fn secondary_key(&self) -> &Self::SecondaryKey {
        &self.user_name
    }
}
