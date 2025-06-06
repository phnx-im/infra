-- SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
--
-- SPDX-License-Identifier: AGPL-3.0-or-later
--
-- Adds support for user handles in the client database.
--
CREATE TABLE IF NOT EXISTS user_handles (
    handle TEXT NOT NULL PRIMARY KEY,
    hash BLOB NOT NULL,
    signing_key BLOB NOT NULL,
    created_at TEXT NOT NULL,
    refreshed_at TEXT NOT NULL
);
