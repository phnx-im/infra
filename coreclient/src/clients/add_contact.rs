// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::ensure;
use openmls::group::GroupId;
use phnxtypes::{
    codec::PhnxCodec,
    crypto::{
        ear::keys::FriendshipPackageEarKey, hpke::HpkeEncryptable,
        indexed_aead::keys::UserProfileKey, signatures::signable::Signable,
    },
    identifiers::{Fqdn, QsReference, UserId},
    messages::{
        client_as::{ConnectionPackage, UserConnectionPackagesParams},
        client_as_out::UserConnectionPackagesResponseIn,
        client_ds_out::CreateGroupParamsOut,
    },
};
use sqlx::SqliteConnection;
use tracing::info;

use crate::{
    Conversation, ConversationAttributes, ConversationId, PartialContact,
    clients::connection_establishment::{ConnectionEstablishmentPackageTbs, FriendshipPackage},
    groups::{Group, PartialCreateGroupParams, openmls_provider::PhnxOpenMlsProvider},
    key_stores::{
        MemoryUserKeyStore, as_credentials::AsCredentials, indexed_keys::StorableIndexedKey,
    },
    store::StoreNotifier,
    utils::connection_ext::ConnectionExt as _,
};

use super::{
    CoreUser, api_clients::ApiClients, connection_establishment::ConnectionEstablishmentPackage,
};

impl CoreUser {
    /// Create a connection with a new user.
    ///
    /// Returns the [`ConversationId`] of the newly created connection
    /// conversation.
    pub(crate) async fn add_contact(&self, client_id: UserId) -> anyhow::Result<ConversationId> {
        let mut connection = self.pool().acquire().await?;

        let connection_packages =
            fetch_user_connection_packages(&self.inner.api_clients, client_id.clone())
                .await? // Phase 1: Fetch connection key packages from the AS
                .verify(&mut connection, &self.inner.api_clients, client_id.domain())
                .await? // Phase 2: Verify the connection key packages
                .request_group_id(&self.inner.api_clients)
                .await?; // Phase 3: Request a group id from the DS

        let mut notifier = self.store_notifier();

        // Phase 4: Prepare the connection locally
        let local_group = connection
            .with_transaction(async |txn| {
                connection_packages
                    .create_local_connection_group(
                        txn,
                        &mut notifier,
                        &self.inner.key_store,
                        self.as_client_id(),
                        &client_id,
                    )
                    .await
            })
            .await?;

        let client_reference = self.create_own_client_reference();

        let local_partial_contact = local_group
            .create_partial_contact(
                &mut connection,
                &mut notifier,
                &self.inner.key_store,
                client_reference,
                self.as_client_id(),
                client_id.clone(),
            )
            .await?;

        // Phase 5: Create the connection group on the DS and send off the
        // connection establishment packages
        let conversation_id = local_partial_contact
            .create_connection_group(&self.inner.api_clients, client_id.domain())
            .await?;

        notifier.notify();

        Ok(conversation_id)
    }
}

async fn fetch_user_connection_packages(
    api_clients: &ApiClients,
    client_id: UserId,
) -> anyhow::Result<FetchedUseConnectionPackage> {
    // Phase 1: Fetch connection key packages from the AS
    let domain = client_id.domain();
    info!(?client_id, "Adding contact");

    let client = api_clients.get(domain)?;
    let params = UserConnectionPackagesParams {
        client_id: client_id.clone(),
    };
    let user_key_packages = client.as_user_connection_packages(params).await?;

    // The AS should return an error if the user does not exist, but we
    // check here locally just to be sure.
    ensure!(
        !user_key_packages.connection_packages.is_empty(),
        "User {client_id:?} does not exist"
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
        key_store: &MemoryUserKeyStore,
        self_client_id: &UserId,
        connection_client_id: &UserId,
    ) -> anyhow::Result<LocalGroup> {
        let Self {
            verified_connection_packages,
            group_id,
        } = self;

        info!("Creating local connection group");
        // TODO: Use display names here
        let title = format!("Connection group: {self_client_id:?} - {connection_client_id:?}");
        let conversation_attributes = ConversationAttributes::new(title, None);
        let group_data = PhnxCodec::to_vec(&conversation_attributes)?.into();

        let provider = PhnxOpenMlsProvider::new(txn);
        let (group, group_membership, partial_params) = Group::create_group(
            &provider,
            &key_store.signing_key,
            &key_store.connection_key,
            group_id.clone(),
            group_data,
        )?;
        group_membership.store(txn.as_mut()).await?;
        group.store(txn).await?;

        // TODO: Once we allow multi-client, invite all our other clients to the
        // connection group.

        // Create the connection conversation
        let conversation = Conversation::new_connection_conversation(
            group_id.clone(),
            connection_client_id.clone(),
            conversation_attributes,
        )?;
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
        connection: &mut SqliteConnection,
        notifier: &mut StoreNotifier,
        key_store: &MemoryUserKeyStore,
        own_client_reference: QsReference,
        own_client_id: &UserId,
        contact_client_id: UserId,
    ) -> anyhow::Result<LocalPartialContact> {
        let Self {
            group,
            partial_params,
            conversation_id,
            verified_connection_packages,
        } = self;

        let own_user_profile_key = UserProfileKey::load_own(&mut *connection).await?;

        let friendship_package = FriendshipPackage {
            friendship_token: key_store.friendship_token.clone(),
            connection_key: key_store.connection_key.clone(),
            wai_ear_key: key_store.wai_ear_key.clone(),
            user_profile_base_secret: own_user_profile_key.base_secret().clone(),
        };

        let friendship_package_ear_key = FriendshipPackageEarKey::random()?;

        // Create and persist a new partial contact
        PartialContact::new(
            contact_client_id.clone(),
            conversation_id,
            friendship_package_ear_key.clone(),
        )
        .store(&mut *connection, notifier)
        .await?;

        // Create a connection establishment package
        let connection_establishment_package = ConnectionEstablishmentPackageTbs {
            sender_client_credential: key_store.signing_key.credential().clone(),
            connection_group_id: group.group_id().clone(),
            connection_group_ear_key: group.group_state_ear_key().clone(),
            connection_group_identity_link_wrapper_key: group.identity_link_wrapper_key().clone(),
            friendship_package_ear_key,
            friendship_package,
        }
        .sign(&key_store.signing_key)?;

        let encrypted_user_profile_key =
            own_user_profile_key.encrypt(group.identity_link_wrapper_key(), own_client_id)?;
        let params = partial_params.into_params(own_client_reference, encrypted_user_profile_key);

        Ok(LocalPartialContact {
            group,
            connection_establishment_package,
            params,
            conversation_id,
            verified_connection_packages,
        })
    }
}

struct LocalPartialContact {
    group: Group,
    connection_establishment_package: ConnectionEstablishmentPackage,
    params: CreateGroupParamsOut,
    conversation_id: ConversationId,
    verified_connection_packages: Vec<ConnectionPackage>,
}

impl LocalPartialContact {
    async fn create_connection_group(
        self,
        api_clients: &ApiClients,
        user_domain: &Fqdn,
    ) -> anyhow::Result<ConversationId> {
        let Self {
            group,
            connection_establishment_package,
            params,
            conversation_id,
            verified_connection_packages,
        } = self;

        info!("Creating connection group on DS");
        api_clients
            .default_client()?
            .ds_create_group(params, group.leaf_signer(), group.group_state_ear_key())
            .await?;

        // Encrypt the connection establishment package for each connection and send it off.
        for connection_package in verified_connection_packages {
            let ciphertext = connection_establishment_package.encrypt(
                connection_package.encryption_key(),
                &[],
                &[],
            );
            let client_id = connection_package.client_credential().identity();

            api_clients
                .get(user_domain)?
                .as_enqueue_message(client_id.clone(), ciphertext)
                .await?;
        }

        Ok(conversation_id)
    }
}
