// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

implement_error! {
    pub enum ConversationStoreError{
        Simple {
            UnknownConversation = "The conversation does not exist",
        }
        Complex {}
    }
}
