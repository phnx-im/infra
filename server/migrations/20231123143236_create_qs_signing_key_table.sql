-- SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
--
-- SPDX-License-Identifier: AGPL-3.0-or-later

-- migrations/{timestamp}_create_qs_signing_key_table.sql
-- Create signing key Table
CREATE TABLE qs_signing_key(
id uuid NOT NULL,
PRIMARY KEY (id),
signing_key BYTEA NOT NULL
);
