-- SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
--
-- SPDX-License-Identifier: AGPL-3.0-or-later
CREATE TABLE handle_connection_packages (
    id INTEGER GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    hash BYTEA NOT NULL,
    connection_package BYTEA NOT NULL,
    FOREIGN KEY (hash) REFERENCES as_user_handles (hash) ON DELETE CASCADE
);
