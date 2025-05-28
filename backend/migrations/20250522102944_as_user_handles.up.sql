-- SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
--
-- SPDX-License-Identifier: AGPL-3.0-or-later
CREATE TABLE IF NOT EXISTS as_user_handles (
    hash BYTEA PRIMARY KEY,
    verifying_key BYTEA NOT NULL,
    expiration_data expiration NOT NULL
);
