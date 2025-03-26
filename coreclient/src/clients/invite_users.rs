use anyhow::Context;
use phnxtypes::{credentials::ClientCredential, identifiers::QualifiedUserName};

use crate::{
    Contact, Conversation, ConversationId, ConversationMessage,
    contacts::ContactAddInfos,
    groups::{Group, client_auth_info::StorableClientCredential},
};

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
        let conversation = Conversation::load(self.pool(), &conversation_id)
            .await?
            .with_context(|| format!("Can't find conversation with id {conversation_id}"))?;
        let group_id = conversation.group_id().clone();
        let owner_domain = conversation.owner_domain();

        let mut contact_wai_keys = vec![];
        let mut client_credentials = vec![];
        let mut contacts = vec![];
        for invited_user in invited_users {
            // Get the WAI keys and client credentials for the invited users.
            let contact = Contact::load(self.pool(), invited_user)
                .await?
                .with_context(|| format!("Can't find contact with user name {invited_user}"))?;
            contact_wai_keys.push(contact.wai_ear_key().clone());

            for client_id in contact.clients() {
                if let Some(client_credential) =
                    StorableClientCredential::load_by_client_id(self.pool(), client_id).await?
                {
                    client_credentials.push(ClientCredential::from(client_credential));
                }
            }

            contacts.push(contact);
        }

        // Phase 2: Load add infos for each contact
        // This needs the connection load (and potentially fetch and store).
        let mut contact_add_infos: Vec<ContactAddInfos> = vec![];
        for contact in contacts {
            let add_info = contact
                .fetch_add_infos(self.pool(), self.inner.api_clients.clone())
                .await?;
            contact_add_infos.push(add_info);
        }

        debug_assert!(contact_add_infos.len() == invited_users.len());

        // Phase 3: Load the group and create the commit to add the new members
        let mut group = Group::load(self.pool().acquire().await?.as_mut(), &group_id)
            .await?
            .with_context(|| format!("Can't find group with id {group_id:?}"))?;
        // Adds new member and staged commit
        let params = group
            .invite(
                self.pool(),
                &self.inner.key_store.signing_key,
                contact_add_infos,
                contact_wai_keys,
                client_credentials,
            )
            .await?;

        // Phase 4: Send the commit to the DS
        // The DS responds with the timestamp of the commit.
        let ds_timestamp = self
            .inner
            .api_clients
            .get(&owner_domain)?
            .ds_group_operation(params, group.leaf_signer(), group.group_state_ear_key())
            .await?;

        // Phase 5: Merge the commit into the group
        self.with_transaction_and_notifier(async |connection, notifier| {
            // Now that we know the commit went through, we can merge the commit
            let group_messages = group
                .merge_pending_commit(&mut *connection, None, ds_timestamp)
                .await?;
            group.store_update(&mut *connection).await?;
            self.store_messages(&mut *connection, notifier, conversation_id, group_messages)
                .await
        })
        .await
    }
}
