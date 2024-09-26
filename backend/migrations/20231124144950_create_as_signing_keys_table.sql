-- SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
--
-- SPDX-License-Identifier: AGPL-3.0-or-later

-- migrations/{timestamp}_create_as_signing_keys_table.sql
-- Create signing key Table
-- The signing keys include the corresponding as credentials
CREATE TYPE credential_type AS ENUM ('as', 'intermediate');

CREATE TABLE as_signing_keys(
id INTEGER GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
cred_type credential_type NOT NULL,
credential_fingerprint BYTEA NOT NULL,
signing_key BYTEA NOT NULL,
currently_active boolean NOT NULL
);

