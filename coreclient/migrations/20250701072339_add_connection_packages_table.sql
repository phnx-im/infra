-- Add migration script here

-- SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
--
-- SPDX-License-Identifier: AGPL-3.0-or-later

-- Connection packages uploaded to the AS.
CREATE TABLE IF NOT EXISTS connection_packages (
    connection_package_hash BLOB NOT NULL PRIMARY KEY,
    handle BLOB NOT NULL,
    decryption_key BLOB NOT NULL,
    expires_at TEXT NOT NULL,
    FOREIGN KEY (handle) REFERENCES user_handles (handle) ON DELETE CASCADE
);
