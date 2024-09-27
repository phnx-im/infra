-- SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
--
-- SPDX-License-Identifier: AGPL-3.0-or-later

-- migrations/{timestamp}_create_as_client_records_table.sql
-- Create client records Table
CREATE TYPE qualified_user_name AS (
    user_name TEXT,
    domain TEXT
);

CREATE TYPE as_client_id AS (
    user_name qualified_user_name,
    client_id uuid
);

CREATE TYPE expiration AS (
    not_before timestamptz,
    not_after timestamptz
);

CREATE TYPE client_credential AS (
    version BYTEA,
    client_id as_client_id,
    signature_scheme BYTEA,
    verifying_key BYTEA,
    expiration_data expiration,
    signer_fingerprint BYTEA,
    signature BYTEA
);

CREATE TABLE as_client_records (
    client_id uuid NOT NULL,
    user_name TEXT NOT NULL,
    queue_encryption_key BYTEA NOT NULL,
    ratchet BYTEA NOT NULL,
    activity_time timestamptz NOT NULL,
    credential client_credential NOT NULL,
    remaining_tokens integer NOT NULL,
    PRIMARY KEY (client_id),
    FOREIGN KEY (user_name) REFERENCES as_user_records(user_name) ON DELETE CASCADE
);

