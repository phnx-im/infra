-- SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
--
-- SPDX-License-Identifier: AGPL-3.0-or-later

-- migrations/{timestamp}_create_qs_signing_key_table.sql
-- Create signing key Table

CREATE TYPE signing_key_data AS (
    signing_key BYTEA,
    verifying_key BYTEA
);

CREATE TABLE qs_signing_key(
id INTEGER GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
signing_key signing_key_data NOT NULL
);
