// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[derive(Default)]
pub(crate) struct GroupStore {
    groups: HashMap<Uuid, Group>,
}

impl GroupStore {
    pub(crate) fn create_group(
        &mut self,
        backend: &impl OpenMlsCryptoProvider,
        signer: &ClientSigningKey,
    ) -> Result<Uuid, GroupStoreError> {
        let mut try_counter = 0;
        while try_counter < 10 {
            let group = Group::create_group(backend, signer);
            let uuid = group.group_id();
            if self.groups.insert(uuid, group).is_some() {
                try_counter += 1;
            } else {
                return Ok(uuid);
            }
        }
        Err(GroupStoreError::InsertionError)
    }

    pub(crate) fn store_group(&mut self, group: Group) -> Result<(), GroupStoreError> {
        match self.groups.insert(group.group_id, group) {
            Some(_) => Err(GroupStoreError::DuplicateGroup),
            None => Ok(()),
        }
    }

    //pub(crate) fn invite_user(&mut self, self_user: &mut SelfUser, group_id: Uuid, user: String) {}

    pub(crate) fn get_group_mut(&mut self, group_id: &Uuid) -> Option<&mut Group> {
        self.groups.get_mut(group_id)
    }

    pub(crate) fn create_message(
        &mut self,
        backend: &impl OpenMlsCryptoProvider<KeyStoreProvider = MemoryKeyStore>,
        signer: &impl Signer,
        credential_with_key: &CredentialWithKey,
        group_id: &Uuid,
        message: &str,
    ) -> Result<GroupMessage, GroupOperationError> {
        let group = self.groups.get_mut(group_id).unwrap();
        group.create_message(backend, signer, credential_with_key, message)
    }
}
