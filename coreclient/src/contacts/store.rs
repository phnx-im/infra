// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::ops::Deref;

use openmls::versions::ProtocolVersion;
use openmls_traits::crypto::OpenMlsCrypto;
use phnxbackend::crypto::{ear::EarDecryptable, signatures::signable::Verifiable};
use rusqlite::Connection;

use crate::{
    key_stores::qs_verifying_keys::QsVerifyingKeyStore,
    users::api_clients::ApiClients,
    utils::persistence::{DataType, Persistable, PersistenceError},
};

use super::*;

pub(crate) struct ContactStore<'a> {
    db_connection: &'a Connection,
    api_clients: ApiClients,
    qs_verifying_key_store: QsVerifyingKeyStore<'a>,
}

impl<'a> ContactStore<'a> {
    pub(crate) fn new(
        db_connection: &'a Connection,
        qs_verifying_key_store: QsVerifyingKeyStore<'a>,
        api_clients: ApiClients,
    ) -> Self {
        Self {
            db_connection,
            api_clients,
            qs_verifying_key_store,
        }
    }

    pub(crate) fn get(
        &self,
        user_name: &UserName,
    ) -> Result<Option<PersistableContact<'a>>, PersistenceError> {
        PersistableContact::load_one(&self.db_connection, Some(user_name), None)
    }

    async fn fetch_add_infos(
        &self,
        crypto_backend: &impl OpenMlsCrypto,
        contact: &mut PersistableContact<'_>,
    ) -> Result<()> {
        let contact_domain = &contact.user_name.domain();
        let qs_verifying_key = self.qs_verifying_key_store.get(contact_domain).await?;
        let mut add_infos = Vec::new();
        for _ in 0..5 {
            let response = self
                .api_clients
                .get(contact_domain)?
                .qs_key_package_batch(
                    contact.friendship_token.clone(),
                    contact.add_package_ear_key.clone(),
                )
                .await?;
            let key_packages: Vec<(KeyPackage, SignatureEarKey)> = response
                .add_packages
                .into_iter()
                .map(|add_package| {
                    let validated_add_package =
                        add_package.validate(crypto_backend, ProtocolVersion::default())?;
                    let key_package = validated_add_package.key_package().clone();
                    let sek = SignatureEarKey::decrypt(
                        &contact.signature_ear_key_wrapper_key,
                        validated_add_package.encrypted_signature_ear_key(),
                    )?;
                    Ok((key_package, sek))
                })
                .collect::<Result<Vec<_>>>()?;
            let add_info = ContactAddInfos {
                key_packages,
                key_package_batch: response
                    .key_package_batch
                    .verify(qs_verifying_key.deref())?,
            };
            add_infos.push(add_info);
        }
        contact.payload.add_infos.append(&mut add_infos);
        contact.persist()?;
        Ok(())
    }

    pub(crate) async fn add_infos(
        &self,
        crypto_backend: &impl OpenMlsCrypto,
        contact: &mut PersistableContact<'_>,
    ) -> Result<ContactAddInfos> {
        let add_infos = if let Some(add_infos) = contact.payload.add_infos() {
            add_infos
        } else {
            self.fetch_add_infos(crypto_backend, contact).await?;
            // We unwrap here because we just fetched the add_infos.
            contact.payload.add_infos().unwrap()
        };
        contact.persist()?;
        Ok(add_infos)
    }

    pub(crate) fn new_partial_contact(
        &self,
        user_name: &UserName,
        conversation_id: &Uuid,
        friendship_package_ear_key: FriendshipPackageEarKey,
    ) -> Result<PersistablePartialContact> {
        let payload = PartialContact::new(
            user_name.clone(),
            conversation_id.clone(),
            friendship_package_ear_key,
        )?;
        let partial_contact =
            PersistablePartialContact::from_connection_and_payload(self.db_connection, payload);
        partial_contact.persist()?;
        Ok(partial_contact)
    }

    pub(crate) fn get_partial_contact(
        &self,
        user_name: &UserName,
    ) -> Result<Option<PersistablePartialContact>, PersistenceError> {
        PersistablePartialContact::load_one(&self.db_connection, Some(user_name), None)
    }

    pub(crate) fn get_all_contacts(&self) -> Result<Vec<PersistableContact>, PersistenceError> {
        PersistableContact::load_all(self.db_connection)
    }

    pub(crate) fn get_all_partial_contacts(
        &self,
    ) -> Result<Vec<PersistablePartialContact>, PersistenceError> {
        PersistablePartialContact::load_all(self.db_connection)
    }
}

pub(crate) struct PersistableContact<'a> {
    connection: &'a Connection,
    payload: Contact,
}

impl std::ops::Deref for PersistableContact<'_> {
    type Target = Contact;

    fn deref(&self) -> &Self::Target {
        &self.payload
    }
}

impl PersistableContact<'_> {
    pub(crate) fn convert_for_export(self) -> Contact {
        self.payload
    }
}

impl<'a> Persistable<'a> for PersistableContact<'a> {
    type Key = UserName;
    type SecondaryKey = UserName;
    const DATA_TYPE: DataType = DataType::Contact;

    fn key(&self) -> &Self::Key {
        &self.payload.user_name
    }

    fn secondary_key(&self) -> &Self::SecondaryKey {
        &self.payload.user_name
    }

    type Payload = Contact;

    fn connection(&self) -> &Connection {
        self.connection
    }

    fn payload(&self) -> &Self::Payload {
        &self.payload
    }

    fn from_connection_and_payload(conn: &'a Connection, payload: Self::Payload) -> Self {
        Self {
            connection: conn,
            payload,
        }
    }
}

pub(crate) struct PersistablePartialContact<'a> {
    connection: &'a Connection,
    payload: PartialContact,
}

impl Deref for PersistablePartialContact<'_> {
    type Target = PartialContact;

    fn deref(&self) -> &Self::Target {
        &self.payload
    }
}

impl PersistablePartialContact<'_> {
    /// Creates a Contact from this PartialContact and the additional data. Then
    /// persists the resulting contact.
    pub(crate) fn into_contact_and_persist(
        self,
        friendship_package: FriendshipPackage,
        add_infos: Vec<ContactAddInfos>,
        client_credential: ClientCredential,
    ) -> Result<()> {
        self.purge()?;
        let payload = Contact {
            user_name: self.payload.user_name,
            add_infos,
            client_credentials: vec![client_credential],
            wai_ear_key: friendship_package.wai_ear_key,
            friendship_token: friendship_package.friendship_token,
            add_package_ear_key: friendship_package.add_package_ear_key,
            client_credential_ear_key: friendship_package.client_credential_ear_key,
            signature_ear_key_wrapper_key: friendship_package.signature_ear_key_wrapper_key,
            conversation_id: self.payload.conversation_id,
        };
        let persistable_contact =
            PersistableContact::from_connection_and_payload(self.connection, payload);
        persistable_contact.persist()?;
        Ok(())
    }

    pub(crate) fn convert_for_export(self) -> PartialContact {
        self.payload
    }
}

impl<'a> Persistable<'a> for PersistablePartialContact<'a> {
    type Key = UserName;

    type SecondaryKey = UserName;

    const DATA_TYPE: DataType = DataType::PartialContact;

    fn key(&self) -> &Self::Key {
        &self.payload.user_name
    }

    fn secondary_key(&self) -> &Self::SecondaryKey {
        &self.payload.user_name
    }

    type Payload = PartialContact;

    fn connection(&self) -> &'a Connection {
        self.connection
    }

    fn payload(&self) -> &Self::Payload {
        &self.payload
    }

    fn from_connection_and_payload(conn: &'a Connection, payload: Self::Payload) -> Self {
        Self {
            connection: conn,
            payload,
        }
    }
}
