// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt;

use anyhow::{Context, ensure};
use openmls::group::GroupId;
use phnxapiclient::{ApiClient, as_api::ConnectionOfferResponder};
use phnxcommon::{
    codec::PhnxCodec,
    credentials::keys::ClientSigningKey,
    crypto::{
        ear::keys::FriendshipPackageEarKey, hpke::HpkeEncryptable,
        indexed_aead::keys::UserProfileKey,
    },
    identifiers::{Fqdn, QsReference, UserHandle, UserId},
    messages::{
        client_as::{ConnectionPackage, UserConnectionPackagesParams},
        client_as_out::UserConnectionPackagesResponseIn,
        client_ds_out::CreateGroupParamsOut,
    },
};
use sqlx::{SqliteConnection, SqliteTransaction};
use tracing::info;

use crate::{
    Conversation, ConversationAttributes, ConversationId, PartialContact,
    clients::connection_offer::FriendshipPackage,
    contacts::HandleContact,
    groups::{Group, PartialCreateGroupParams, openmls_provider::PhnxOpenMlsProvider},
    key_stores::{
        MemoryUserKeyStore, as_credentials::AsCredentials, indexed_keys::StorableIndexedKey,
    },
    store::StoreNotifier,
    utils::connection_ext::ConnectionExt as _,
};

use super::{
    CoreUser,
    api_clients::ApiClients,
    connection_offer::{ConnectionOffer, payload::ConnectionOfferPayload},
};

impl CoreUser {
    /// Create a connection with via a user handle.
    pub(crate) async fn add_contact_via_handle(
        &self,
        handle: UserHandle,
    ) -> anyhow::Result<ConversationId> {
        let client = self.api_client()?;

        // Phase 1: Fetch a connection package from the AS
        let (connection_package, connection_offer_responder) =
            client.as_connect_handle(&handle).await?;

        // Phase 2: Verify the connection package
        let as_intermediate_credential = AsCredentials::get(
            self.pool().acquire().await?.as_mut(),
            &self.inner.api_clients,
            self.user_id().domain(),
            connection_package.client_credential_signer_fingerprint(),
        )
        .await?;
        let verifying_key = as_intermediate_credential.verifying_key();
        let connection_package = connection_package.verify(verifying_key)?;

        // Phase 3: Prepare the connection locally
        let group_id = client.ds_request_group_id().await?;
        let connection_package = VerifiedConnectionPackagesWithGroupId {
            verified_connection_packages: vec![connection_package],
            group_id,
        };

        let contact_id = ContactId::Handle(handle);
        let client_reference = self.create_own_client_reference();

        self.with_transaction_and_notifier(async |txn, notifier| {
            // Phase 4: Create a connection group
            let local_group = connection_package
                .create_local_connection_group(
                    txn,
                    notifier,
                    &self.inner.key_store.signing_key,
                    contact_id.clone(),
                )
                .await?;

            let local_partial_contact = local_group
                .create_partial_contact(
                    txn,
                    notifier,
                    &self.inner.key_store,
                    client_reference,
                    self.user_id(),
                    contact_id,
                )
                .await?;

            // Phase 5: Create the connection group on the DS and send off the connection offer
            let conversation_id = local_partial_contact
                .create_connection_group_via_handle(
                    &client,
                    self.signing_key(),
                    connection_offer_responder,
                )
                .await?;

            Ok(conversation_id)
        })
        .await
    }

    /// Create a connection with a new user.
    ///
    /// Returns the [`ConversationId`] of the newly created connection
    /// conversation.
    pub(crate) async fn add_contact(&self, user_id: UserId) -> anyhow::Result<ConversationId> {
        let mut connection = self.pool().acquire().await?;

        let connection_packages =
            fetch_user_connection_packages(&self.inner.api_clients, user_id.clone())
                .await? // Phase 1: Fetch connection key packages from the AS
                .verify(&mut connection, &self.inner.api_clients, user_id.domain())
                .await? // Phase 2: Verify the connection key packages
                .request_group_id(&self.inner.api_clients)
                .await?; // Phase 3: Request a group id from the DS

        let mut notifier = self.store_notifier();

        let contact_id = ContactId::UserId(user_id.clone());

        // Phase 4: Prepare the connection locally
        let local_group = connection
            .with_transaction(async |txn| {
                connection_packages
                    .create_local_connection_group(
                        txn,
                        &mut notifier,
                        &self.inner.key_store.signing_key,
                        contact_id.clone(),
                    )
                    .await
            })
            .await?;

        let client_reference = self.create_own_client_reference();

        let local_partial_contact = connection
            .with_transaction(async |txn| {
                local_group
                    .create_partial_contact(
                        txn,
                        &mut notifier,
                        &self.inner.key_store,
                        client_reference,
                        self.user_id(),
                        contact_id,
                    )
                    .await
            })
            .await?;

        // Phase 5: Create the connection group on the DS and send off the connection offer
        let conversation_id = local_partial_contact
            .create_connection_group(
                &self.inner.api_clients,
                user_id.domain(),
                self.signing_key(),
            )
            .await?;

        notifier.notify();

        Ok(conversation_id)
    }
}

async fn fetch_user_connection_packages(
    api_clients: &ApiClients,
    user_id: UserId,
) -> anyhow::Result<FetchedUseConnectionPackage> {
    // Phase 1: Fetch connection key packages from the AS
    info!(?user_id, "Adding contact");

    let client = api_clients.get(user_id.domain())?;
    let params = UserConnectionPackagesParams {
        user_id: user_id.clone(),
    };
    let user_key_packages = client.as_user_connection_packages(params).await?;

    // The AS should return an error if the user does not exist, but we
    // check here locally just to be sure.
    ensure!(
        !user_key_packages.connection_packages.is_empty(),
        "User {user_id:?} does not exist"
    );

    Ok(FetchedUseConnectionPackage { user_key_packages })
}

struct FetchedUseConnectionPackage {
    user_key_packages: UserConnectionPackagesResponseIn,
}

impl FetchedUseConnectionPackage {
    async fn verify(
        self,
        connection: &mut SqliteConnection,
        api_clients: &ApiClients,
        user_domain: &Fqdn,
    ) -> anyhow::Result<VerifiedConnectionPackages> {
        let Self { user_key_packages } = self;

        info!("Verifying connection packages");
        let mut verified_connection_packages = vec![];
        for connection_package in user_key_packages.connection_packages.into_iter() {
            let as_intermediate_credential = AsCredentials::get(
                &mut *connection,
                api_clients,
                user_domain,
                connection_package.client_credential_signer_fingerprint(),
            )
            .await?;
            let verifying_key = as_intermediate_credential.verifying_key();
            verified_connection_packages.push(connection_package.verify(verifying_key)?)
        }

        // TODO: Connection Package Validation
        // * Version
        // * Lifetime

        Ok(VerifiedConnectionPackages {
            verified_connection_packages,
        })
    }
}

struct VerifiedConnectionPackages {
    verified_connection_packages: Vec<ConnectionPackage>,
}

impl VerifiedConnectionPackages {
    async fn request_group_id(
        self,
        api_clients: &ApiClients,
    ) -> anyhow::Result<VerifiedConnectionPackagesWithGroupId> {
        info!("Requesting group id");
        let group_id = api_clients.default_client()?.ds_request_group_id().await?;
        let Self {
            verified_connection_packages,
        } = self;
        Ok(VerifiedConnectionPackagesWithGroupId {
            verified_connection_packages,
            group_id,
        })
    }
}

struct VerifiedConnectionPackagesWithGroupId {
    verified_connection_packages: Vec<ConnectionPackage>,
    group_id: GroupId,
}

impl VerifiedConnectionPackagesWithGroupId {
    async fn create_local_connection_group(
        self,
        txn: &mut sqlx::SqliteTransaction<'_>,
        notifier: &mut StoreNotifier,
        signing_key: &ClientSigningKey,
        contact_id: ContactId,
    ) -> anyhow::Result<LocalGroup> {
        let Self {
            verified_connection_packages,
            group_id,
        } = self;

        info!("Creating local connection group");
        let title = format!("Connection group: {contact_id}");
        let conversation_attributes = ConversationAttributes::new(title, None);
        let group_data = PhnxCodec::to_vec(&conversation_attributes)?.into();

        let provider = PhnxOpenMlsProvider::new(txn);
        let (group, group_membership, partial_params) =
            Group::create_group(&provider, signing_key, group_id.clone(), group_data)?;
        group_membership.store(txn.as_mut()).await?;
        group.store(txn.as_mut()).await?;

        // TODO: Once we allow multi-client, invite all our other clients to the
        // connection group.

        // Create the connection conversation
        let conversation = match contact_id {
            ContactId::UserId(user_id) => Conversation::new_connection_conversation(
                group_id.clone(),
                user_id.clone(),
                conversation_attributes,
            )?,
            ContactId::Handle(handle) => Conversation::new_handle_conversation(
                group_id.clone(),
                conversation_attributes,
                handle.clone(),
            ),
        };
        conversation.store(txn.as_mut(), notifier).await?;

        Ok(LocalGroup {
            group,
            partial_params,
            conversation_id: conversation.id(),
            verified_connection_packages,
        })
    }
}

struct LocalGroup {
    group: Group,
    partial_params: PartialCreateGroupParams,
    conversation_id: ConversationId,
    verified_connection_packages: Vec<ConnectionPackage>,
}

impl LocalGroup {
    async fn create_partial_contact(
        self,
        txn: &mut SqliteTransaction<'_>,
        notifier: &mut StoreNotifier,
        key_store: &MemoryUserKeyStore,
        own_client_reference: QsReference,
        own_user_id: &UserId,
        contact_id: ContactId,
    ) -> anyhow::Result<LocalPartialContact> {
        let Self {
            group,
            partial_params,
            conversation_id,
            verified_connection_packages,
        } = self;

        let own_user_profile_key = UserProfileKey::load_own(txn.as_mut()).await?;

        let friendship_package = FriendshipPackage {
            friendship_token: key_store.friendship_token.clone(),
            connection_key: key_store.connection_key.clone(),
            wai_ear_key: key_store.wai_ear_key.clone(),
            user_profile_base_secret: own_user_profile_key.base_secret().clone(),
        };

        let friendship_package_ear_key = FriendshipPackageEarKey::random()?;

        // Create and persist a new partial contact
        match contact_id.clone() {
            ContactId::UserId(user_id) => {
                PartialContact::new(
                    user_id.clone(),
                    conversation_id,
                    friendship_package_ear_key.clone(),
                )
                .store(txn.as_mut(), notifier)
                .await?;
            }
            ContactId::Handle(handle) => {
                HandleContact::new(handle, conversation_id, friendship_package_ear_key.clone())
                    .upsert(txn.as_mut(), notifier)
                    .await?;
            }
        };

        // Create a connection offer
        let connection_offer_payload = ConnectionOfferPayload {
            sender_client_credential: key_store.signing_key.credential().clone(),
            connection_group_id: group.group_id().clone(),
            connection_group_ear_key: group.group_state_ear_key().clone(),
            connection_group_identity_link_wrapper_key: group.identity_link_wrapper_key().clone(),
            friendship_package_ear_key,
            friendship_package,
        };
        let connection_offer = match contact_id {
            ContactId::UserId(_) => {
                unimplemented!()
            }
            ContactId::Handle(handle) => {
                connection_offer_payload.sign(&key_store.signing_key, handle)?
            }
        };

        let encrypted_user_profile_key =
            own_user_profile_key.encrypt(group.identity_link_wrapper_key(), own_user_id)?;
        let params = partial_params.into_params(own_client_reference, encrypted_user_profile_key);

        Ok(LocalPartialContact {
            group,
            connection_offer,
            params,
            conversation_id,
            verified_connection_packages,
        })
    }
}

struct LocalPartialContact {
    group: Group,
    connection_offer: ConnectionOffer,
    params: CreateGroupParamsOut,
    conversation_id: ConversationId,
    verified_connection_packages: Vec<ConnectionPackage>,
}

impl LocalPartialContact {
    async fn create_connection_group_via_handle(
        self,
        client: &ApiClient,
        signer: &ClientSigningKey,
        responder: ConnectionOfferResponder,
    ) -> anyhow::Result<ConversationId> {
        let Self {
            group,
            connection_offer,
            params,
            conversation_id,
            verified_connection_packages,
        } = self;

        info!("Creating connection group on DS");
        client
            .ds_create_group(params, signer, group.group_state_ear_key())
            .await?;

        // Encrypt the connection offer and send it off.
        debug_assert!(
            verified_connection_packages.len() == 1,
            "Only one connection package is supported for handle connections"
        );
        let connection_package = verified_connection_packages
            .into_iter()
            .next()
            .context("logic error: no connection package")?;
        let ciphertext = connection_offer.encrypt(connection_package.encryption_key(), &[], &[]);
        responder.send(ciphertext).await?;

        Ok(conversation_id)
    }

    async fn create_connection_group(
        self,
        api_clients: &ApiClients,
        user_domain: &Fqdn,
        signer: &ClientSigningKey,
    ) -> anyhow::Result<ConversationId> {
        let Self {
            group,
            connection_offer,
            params,
            conversation_id,
            verified_connection_packages,
        } = self;

        info!("Creating connection group on DS");
        api_clients
            .default_client()?
            .ds_create_group(params, signer, group.group_state_ear_key())
            .await?;

        // Encrypt the connection offer for each connection and send it off.
        for connection_package in verified_connection_packages {
            let ciphertext =
                connection_offer.encrypt(connection_package.encryption_key(), &[], &[]);
            let user_id = connection_package.client_credential().identity();

            api_clients
                .get(user_domain)?
                .as_enqueue_message(user_id.clone(), ciphertext)
                .await?;
        }

        Ok(conversation_id)
    }
}

#[derive(Debug, Clone)]
enum ContactId {
    UserId(UserId),
    Handle(UserHandle),
}

impl fmt::Display for ContactId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // TODO: Use display names here
            ContactId::UserId(user_id) => write!(f, "{user_id:?}"),
            ContactId::Handle(handle) => write!(f, "{}", handle.plaintext()),
        }
    }
}
