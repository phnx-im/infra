-- SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
--
-- SPDX-License-Identifier: AGPL-3.0-or-later

CREATE TABLE IF NOT EXISTS indexed_keys (
    key_index BLOB NOT NULL PRIMARY KEY,
    key_value BLOB NOT NULL,
    base_secret BLOB NOT NULL,
    key_type TEXT CHECK(key_type IN ('user_profile_key')) NOT NULL
);
