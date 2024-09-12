// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later


use crate::key_stores::queue_ratchets::StorableAsQueueRatchet;
use crate::utils::persistence::Storable;

/// The table for queue ratchets contains both the AsQueueRatchet and the
/// QsQueueRatchet.
pub fn migration() -> String {
    <StorableAsQueueRatchet as Storable>::CREATE_TABLE_STATEMENT.to_string()
}
