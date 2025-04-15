// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use invite_users_flow::InviteUsersData;
use phnxtypes::identifiers::QualifiedUserName;

use crate::{ConversationId, ConversationMessage};

use super::CoreUser;

impl CoreUser {
    /// Invite users to an existing conversation.
    ///
    /// Since this function causes the creation of an MLS commit, it can cause
    /// more than one effect on the group. As a result this function returns a
    /// vector of [`ConversationMessage`]s that represents the changes to the
    /// group. Note that these returned message have already been persisted.
    pub(crate) async fn invite_users(
        &self,
        conversation_id: ConversationId,
        invited_users: &[QualifiedUserName],
    ) -> anyhow::Result<Vec<ConversationMessage>> {
        // Phase 1: Load all the relevant conversation and all the contacts we
        // want to add.
        let invited = InviteUsersData::load(self.pool(), conversation_id, invited_users)
            .await?
            // Phase 2: Load add infos for each contact
            // This needs the connection load (and potentially fetch and store).
            .load_add_infos(self.pool(), &self.inner.api_clients)
            .await?
            // Phase 3: Load the group and create the commit to add the new members
            .create_commit(self.pool(), &self.inner.key_store)
            .await?
            // Phase 4: Send the commit to the DS
            // The DS responds with the timestamp of the commit.
            .ds_group_operation(&self.inner.api_clients)
            .await?;
        // Phase 5: Merge the commit into the group
        self.with_transaction_and_notifier(async |connection, notifier| {
            // Now that we know the commit went through, we can merge the commit
            invited
                .merge_pending_commit(&mut *connection, notifier, conversation_id)
                .await
        })
        .await
    }
}

mod invite_users_flow {
    use anyhow::Context;
    use openmls::group::GroupId;
    use phnxtypes::{
        credentials::ClientCredential,
        crypto::ear::keys::WelcomeAttributionInfoEarKey,
        identifiers::{Fqdn, QualifiedUserName},
        messages::client_ds_out::GroupOperationParamsOut,
        time::TimeStamp,
    };
    use sqlx::SqlitePool;

    use crate::{
        Contact, Conversation, ConversationId, ConversationMessage,
        clients::{CoreUser, api_clients::ApiClients},
        contacts::ContactAddInfos,
        groups::{Group, client_auth_info::StorableClientCredential},
        key_stores::MemoryUserKeyStore,
        store::StoreNotifier,
    };

    pub(super) struct InviteUsersData<S> {
        group_id: GroupId,
        owner_domain: Fqdn,
        contact_wai_keys: Vec<WelcomeAttributionInfoEarKey>,
        client_credentials: Vec<ClientCredential>,
        state: S,
    }

    impl InviteUsersData<()> {
        pub(super) async fn load(
            pool: &SqlitePool,
            conversation_id: ConversationId,
            invited_users: &[QualifiedUserName],
        ) -> anyhow::Result<InviteUsersData<Vec<Contact>>> {
            let conversation = Conversation::load(pool, &conversation_id)
                .await?
                .with_context(|| format!("Can't find conversation with id {conversation_id}"))?;

            let mut contact_wai_keys = Vec::with_capacity(invited_users.len());
            let mut contacts = Vec::with_capacity(invited_users.len());
            let mut client_credentials = Vec::with_capacity(invited_users.len());

            for invited_user in invited_users {
                // Get the WAI keys and client credentials for the invited users.
                let contact = Contact::load(pool.acquire().await?.as_mut(), invited_user)
                    .await?
                    .with_context(|| format!("Can't find contact with user name {invited_user}"))?;
                contact_wai_keys.push(contact.wai_ear_key().clone());

                for client_id in contact.clients() {
                    if let Some(client_credential) =
                        StorableClientCredential::load_by_client_id(pool, client_id).await?
                    {
                        client_credentials.push(ClientCredential::from(client_credential));
                    }
                }

                contacts.push(contact);
            }

            Ok(InviteUsersData {
                group_id: conversation.group_id().clone(),
                owner_domain: conversation.owner_domain(),
                contact_wai_keys,
                client_credentials,
                state: contacts,
            })
        }
    }

    impl InviteUsersData<Vec<Contact>> {
        pub(super) async fn load_add_infos(
            self,
            pool: &SqlitePool,
            api_clients: &ApiClients,
        ) -> anyhow::Result<InviteUsersData<Vec<ContactAddInfos>>> {
            let Self {
                group_id,
                owner_domain,
                contact_wai_keys,
                client_credentials,
                state: contacts,
            } = self;

            let mut contact_add_infos: Vec<ContactAddInfos> = Vec::with_capacity(contacts.len());
            for contact in contacts {
                let add_info = contact.fetch_add_infos(pool, api_clients).await?;
                contact_add_infos.push(add_info);
            }

            Ok(InviteUsersData {
                group_id,
                owner_domain,
                contact_wai_keys,
                client_credentials,
                state: contact_add_infos,
            })
        }
    }

    impl InviteUsersData<Vec<ContactAddInfos>> {
        pub(super) async fn create_commit(
            self,
            pool: &SqlitePool,
            key_store: &MemoryUserKeyStore,
        ) -> anyhow::Result<InviteUsersParams> {
            let Self {
                group_id,
                owner_domain,
                contact_wai_keys,
                client_credentials,
                state: contact_add_infos,
            } = self;

            let mut group = Group::load(pool.acquire().await?.as_mut(), &group_id)
                .await?
                .with_context(|| format!("Can't find group with id {group_id:?}"))?;
            // Adds new member and staged commit
            let params = group
                .invite(
                    pool,
                    &key_store.signing_key,
                    contact_add_infos,
                    contact_wai_keys,
                    client_credentials,
                )
                .await?;

            Ok(InviteUsersParams {
                group,
                params,
                owner_domain,
            })
        }
    }

    pub(super) struct InviteUsersParams {
        group: Group,
        params: GroupOperationParamsOut,
        owner_domain: Fqdn,
    }

    impl InviteUsersParams {
        pub(super) async fn ds_group_operation(
            self,
            api_clients: &ApiClients,
        ) -> anyhow::Result<InvitedUsers> {
            let Self {
                group,
                params,
                owner_domain,
            } = self;

            let ds_timestamp = api_clients
                .get(&owner_domain)?
                .ds_group_operation(params, group.leaf_signer(), group.group_state_ear_key())
                .await?;

            Ok(InvitedUsers {
                group,
                ds_timestamp,
            })
        }
    }

    pub(super) struct InvitedUsers {
        group: Group,
        ds_timestamp: TimeStamp,
    }

    impl InvitedUsers {
        pub(super) async fn merge_pending_commit(
            self,
            connection: &mut sqlx::SqliteConnection,
            notifier: &mut StoreNotifier,
            conversation_id: ConversationId,
        ) -> anyhow::Result<Vec<ConversationMessage>> {
            let Self {
                mut group,
                ds_timestamp,
            } = self;

            let group_messages = group
                .merge_pending_commit(&mut *connection, None, ds_timestamp)
                .await?;
            group.store_update(&mut *connection).await?;
            CoreUser::store_messages(&mut *connection, notifier, conversation_id, group_messages)
                .await
        }
    }
}
