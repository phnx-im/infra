// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[derive(Default)]
pub(crate) struct GroupStore {
    groups: HashMap<GroupId, Group>,
}

impl GroupStore {
    pub(crate) fn create_group(
        &mut self,
        backend: &impl OpenMlsCryptoProvider,
        signer: &ClientSigningKey,
        group_id: GroupId,
    ) {
        let group = Group::create_group(backend, signer, group_id.clone());
        // TODO: For now we trust that the server won't serve us colliding group
        // ids.
        self.groups.insert(group_id, group);
    }

    pub(crate) fn store_group(&mut self, group: Group) -> Result<(), GroupStoreError> {
        match self.groups.insert(group.group_id, group) {
            Some(_) => Err(GroupStoreError::DuplicateGroup),
            None => Ok(()),
        }
    }

    //pub(crate) fn invite_user(&mut self, self_user: &mut SelfUser, group_id: Uuid, user: String) {}

    pub(crate) fn get_group_mut(&mut self, group_id: &GroupId) -> Option<&mut Group> {
        self.groups.get_mut(group_id)
    }

    pub(crate) fn create_message(
        &mut self,
        backend: &impl OpenMlsCryptoProvider<KeyStoreProvider = MemoryKeyStore>,
        group_id: &GroupId,
        message: &str,
    ) -> Result<SendMessageParamsOut, GroupOperationError> {
        let group = self.groups.get_mut(group_id).unwrap();
        group.create_message(backend, message)
    }

    /// Returns the leaf signing key for the given group.
    /// TODO: We're returning a copy here, which is not ideal.
    pub(crate) fn leaf_signing_key(&self, group_id: &GroupId) -> InfraCredentialSigningKey {
        self.groups.get(group_id).unwrap().leaf_signer.clone()
    }

    /// Returns the group state EAR key for the given group.
    /// TODO: We're returning a copy here, which is not ideal.
    pub(crate) fn group_state_ear_key(&self, group_id: &GroupId) -> GroupStateEarKey {
        self.groups.get(group_id).unwrap().group_state_ear_key
    }
}
