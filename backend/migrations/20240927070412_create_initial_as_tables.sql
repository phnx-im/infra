-- SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
--
-- SPDX-License-Identifier: AGPL-3.0-or-later

CREATE TABLE as_batched_keys(
    token_key_id smallint PRIMARY KEY,
    voprf_server BYTEA NOT NULL
);

CREATE TYPE aead_ciphertext AS (
    ciphertext BYTEA,
    nonce BYTEA
);

CREATE TABLE as_user_records(
    user_name TEXT PRIMARY KEY,
    password_file BYTEA NOT NULL,
    encrypted_user_profile aead_ciphertext NOT NULL
);

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
    client_id uuid PRIMARY KEY,
    user_name TEXT NOT NULL,
    queue_encryption_key BYTEA NOT NULL,
    ratchet BYTEA NOT NULL,
    activity_time timestamptz NOT NULL,
    credential client_credential NOT NULL,
    remaining_tokens integer NOT NULL,
    FOREIGN KEY (user_name) REFERENCES as_user_records(user_name) ON DELETE CASCADE
);

CREATE TABLE connection_packages (
    id INTEGER GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    client_id uuid NOT NULL,
    connection_package BYTEA NOT NULL,
    FOREIGN KEY (client_id) REFERENCES as_client_records(client_id) ON DELETE CASCADE
);

CREATE INDEX idx_connection_package_client_id ON connection_packages(client_id);

CREATE TYPE credential_type AS ENUM ('as', 'intermediate');

CREATE TABLE as_signing_keys (
    id INTEGER GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    cred_type credential_type NOT NULL,
    credential_fingerprint BYTEA NOT NULL,
    signing_key BYTEA NOT NULL,
    currently_active boolean NOT NULL
);

CREATE TABLE as_queue_data (
    queue_id uuid PRIMARY KEY,
    sequence_number BIGINT NOT NULL,
    FOREIGN KEY (queue_id) REFERENCES as_client_records(client_id) ON DELETE CASCADE
);

CREATE TABLE as_queues (
    message_id uuid NOT NULL,
    queue_id uuid NOT NULL,
    sequence_number BIGINT NOT NULL,
    message_bytes BYTEA NOT NULL,
    PRIMARY KEY (queue_id, sequence_number),
    FOREIGN KEY (queue_id) REFERENCES as_queue_data(queue_id) ON DELETE CASCADE
);

CREATE TABLE opaque_setup(
    id INTEGER GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    opaque_setup BYTEA NOT NULL
);