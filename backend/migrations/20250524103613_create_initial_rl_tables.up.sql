-- SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
--
-- SPDX-License-Identifier: AGPL-3.0-or-later
-- Allowance records
CREATE UNLOGGED TABLE allowance_records(
    key_value bytea PRIMARY KEY,
    remaining bigint NOT NULL,
    valid_until timestamptz NOT NULL
);

