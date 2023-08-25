// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::users::ApiClients;

use super::*;

#[derive(Default)]
pub(crate) struct GroupStore {
    groups: HashMap<GroupId, Group>,
}

impl GroupStore {
    pub(crate) fn store_group(&mut self, group: Group) -> Result<(), GroupStoreError> {
        match self.groups.insert(group.group_id.clone(), group) {
            Some(_) => Err(GroupStoreError::DuplicateGroup),
            None => Ok(()),
        }
    }

    pub(crate) async fn join_group(
        &mut self,
        provider: &impl OpenMlsProvider<KeyStoreProvider = MemoryKeyStore>,
        welcome_bundle: WelcomeBundle,
        // This is our own key that the sender uses to encrypt to us. We should
        // be able to retrieve it from the client's key store.
        welcome_attribution_info_ear_key: &WelcomeAttributionInfoEarKey,
        leaf_signers: &mut HashMap<
            SignaturePublicKey,
            (InfraCredentialSigningKey, SignatureEarKey),
        >,
        api_clients: &mut ApiClients,
        as_credentials: &mut AsCredentials,
        contacts: &HashMap<UserName, Contact>,
    ) -> Result<GroupId> {
        let group = Group::join_group(
            provider,
            welcome_bundle,
            welcome_attribution_info_ear_key,
            leaf_signers,
            api_clients,
            as_credentials,
            contacts,
        )
        .await?;
        let group_id = group.group_id().clone();
        self.groups.insert(group_id.clone(), group);

        Ok(group_id)
    }

    //pub(crate) fn invite_user(&mut self, self_user: &mut SelfUser, group_id: Uuid, user: String) {}

    pub(crate) fn get_group_mut(&mut self, group_id: &GroupId) -> Option<&mut Group> {
        self.groups.get_mut(group_id)
    }
    pub(crate) fn get_group(&self, group_id: &GroupId) -> Option<&Group> {
        self.groups.get(group_id)
    }

    pub(crate) fn create_message(
        &mut self,
        provider: &impl OpenMlsProvider<KeyStoreProvider = MemoryKeyStore>,
        group_id: &GroupId,
        message: MessageContentType,
    ) -> Result<SendMessageParamsOut> {
        let group = self
            .groups
            .get_mut(group_id)
            .ok_or(anyhow!("Unknown group"))?;
        let message = group.create_message(provider, message)?;
        Ok(message)
    }

    /// Returns the leaf signing key for the given group.
    /// TODO: We're returning a copy here, which is not ideal.
    pub(crate) fn leaf_signing_key(
        &self,
        group_id: &GroupId,
    ) -> Option<&InfraCredentialSigningKey> {
        self.groups.get(group_id).map(|g| &g.leaf_signer)
    }

    /// Returns the group state EAR key for the given group.
    /// TODO: We're returning a copy here, which is not ideal.
    pub(crate) fn group_state_ear_key(&self, group_id: &GroupId) -> Option<&GroupStateEarKey> {
        self.groups.get(group_id).map(|g| &g.group_state_ear_key)
    }

    pub fn group(&self, group_id: &GroupId) -> Option<&Group> {
        self.groups.get(group_id)
    }
}
