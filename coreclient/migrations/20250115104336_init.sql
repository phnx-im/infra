-- SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
--
-- SPDX-License-Identifier: AGPL-3.0-or-later
CREATE TABLE IF NOT EXISTS client_record (
    client_id BLOB NOT NULL PRIMARY KEY,
    record_state TEXT NOT NULL CHECK (record_state IN ('in_progress', 'finished')),
    created_at DATETIME NOT NULL,
    is_default BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE TABLE IF NOT EXISTS user_creation_state (client_id BLOB PRIMARY KEY, state BLOB NOT NULL);

CREATE TABLE IF NOT EXISTS own_client_info (
    server_url TEXT NOT NULL,
    qs_user_id BLOB NOT NULL,
    qs_client_id BLOB NOT NULL,
    as_user_name TEXT NOT NULL,
    as_client_uuid BLOB NOT NULL
);

CREATE TABLE IF NOT EXISTS users (
    user_name TEXT NOT NULL PRIMARY KEY,
    display_name TEXT,
    profile_picture BLOB
);

CREATE TABLE IF NOT EXISTS "groups" (
    group_id BLOB NOT NULL PRIMARY KEY,
    leaf_signer BLOB NOT NULL,
    identity_link_wrapper_key BLOB NOT NULL,
    group_state_ear_key BLOB NOT NULL,
    pending_diff BLOB
);

CREATE TABLE IF NOT EXISTS client_credentials (
    fingerprint BLOB NOT NULL PRIMARY KEY,
    client_id TEXT NOT NULL,
    client_credential BLOB NOT NULL
);

CREATE TABLE IF NOT EXISTS group_membership (
    client_credential_fingerprint BLOB NOT NULL,
    group_id BLOB NOT NULL,
    client_uuid BLOB NOT NULL,
    user_name TEXT NOT NULL,
    leaf_index INTEGER NOT NULL,
    identity_link_key BLOB NOT NULL,
    status TEXT DEFAULT 'staged_update' NOT NULL CHECK (
        status IN (
            'staged_update',
            'staged_removal',
            'staged_add',
            'merged'
        )
    ),
    FOREIGN KEY (client_credential_fingerprint) REFERENCES client_credentials (fingerprint),
    PRIMARY KEY (group_id, leaf_index, status)
);

CREATE TABLE IF NOT EXISTS contacts (
    user_name TEXT NOT NULL PRIMARY KEY,
    conversation_id BLOB NOT NULL,
    clients TEXT NOT NULL,
    wai_ear_key BLOB NOT NULL,
    friendship_token BLOB NOT NULL,
    key_package_ear_key BLOB NOT NULL,
    connection_key BLOB NOT NULL,
    FOREIGN KEY (conversation_id) REFERENCES conversations (conversation_id)
);

CREATE TABLE IF NOT EXISTS partial_contacts (
    user_name TEXT NOT NULL PRIMARY KEY,
    conversation_id BLOB NOT NULL,
    friendship_package_ear_key BLOB NOT NULL,
    FOREIGN KEY (conversation_id) REFERENCES conversations (conversation_id)
);

CREATE TABLE IF NOT EXISTS conversations (
    conversation_id BLOB NOT NULL PRIMARY KEY,
    conversation_title TEXT NOT NULL,
    conversation_picture BLOB,
    group_id BLOB NOT NULL,
    last_read TEXT NOT NULL,
    conversation_status TEXT NOT NULL CHECK (
        conversation_status LIKE 'active'
        OR conversation_status LIKE 'inactive:%'
    ),
    conversation_type TEXT NOT NULL CHECK (
        conversation_type LIKE 'group'
        OR conversation_type LIKE 'unconfirmed_connection:%'
        OR conversation_type LIKE 'connection:%'
    )
);

CREATE TABLE IF NOT EXISTS conversation_messages (
    message_id BLOB NOT NULL PRIMARY KEY,
    conversation_id BLOB NOT NULL,
    timestamp TEXT NOT NULL,
    sender TEXT NOT NULL,
    content BLOB NOT NULL,
    sent BOOLEAN NOT NULL,
    CHECK (
        sender LIKE 'user:%'
        OR sender = 'system'
    ),
    FOREIGN KEY (conversation_id) REFERENCES conversations (conversation_id) DEFERRABLE INITIALLY DEFERRED
);

CREATE INDEX IF NOT EXISTS conversation_messages_conversation_id_idx ON conversation_messages (conversation_id);

CREATE INDEX IF NOT EXISTS conversation_messages_timetstamp_idx ON conversation_messages (timestamp);

CREATE TABLE IF NOT EXISTS own_leaf_nodes (
    group_id BLOB PRIMARY KEY,
    leaf_node BLOB NOT NULL
);

CREATE TABLE IF NOT EXISTS signature_keys (
    public_key BLOB PRIMARY KEY,
    signature_key BLOB NOT NULL
);

CREATE TABLE IF NOT EXISTS epoch_keys_pairs (
    group_id BLOB NOT NULL,
    epoch_id BLOB NOT NULL,
    leaf_index INTEGER NOT NULL,
    key_pairs BLOB NOT NULL,
    PRIMARY KEY (group_id, epoch_id, leaf_index)
);

CREATE TABLE IF NOT EXISTS encryption_keys (
    public_key BLOB PRIMARY KEY,
    key_pair BLOB NOT NULL
);

CREATE TABLE IF NOT EXISTS group_data (
    group_id BLOB NOT NULL,
    data_type TEXT NOT NULL CHECK (
        data_type IN (
            'join_group_config',
            'tree',
            'interim_transcript_hash',
            'context',
            'confirmation_tag',
            'group_state',
            'message_secrets',
            'resumption_psk_store',
            'own_leaf_index',
            'use_ratchet_tree_extension',
            'group_epoch_secrets'
        )
    ),
    group_data BLOB NOT NULL,
    PRIMARY KEY (group_id, data_type)
);

CREATE TABLE IF NOT EXISTS key_packages (
    key_package_ref BLOB PRIMARY KEY,
    key_package BLOB NOT NULL
);

CREATE TABLE IF NOT EXISTS proposals (
    group_id BLOB NOT NULL,
    proposal_ref BLOB NOT NULL,
    proposal BLOB NOT NULL,
    PRIMARY KEY (group_id, proposal_ref)
);

CREATE TABLE IF NOT EXISTS psks (psk_id BLOB PRIMARY KEY, psk_bundle BLOB NOT NULL);

CREATE TABLE IF NOT EXISTS qs_verifying_keys (
    domain TEXT PRIMARY KEY,
    verifying_key BLOB NOT NULL
);

CREATE TABLE IF NOT EXISTS queue_ratchets (
    queue_type TEXT PRIMARY KEY CHECK (queue_type IN ('as', 'qs')),
    queue_ratchet BLOB NOT NULL,
    sequence_number INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS as_credentials (
    fingerprint TEXT PRIMARY KEY,
    domain TEXT NOT NULL,
    credential_type TEXT NOT NULL CHECK (
        credential_type IN ('as_credential', 'as_intermediate_credential')
    ),
    credential BLOB NOT NULL
);

CREATE TABLE IF NOT EXISTS leaf_keys (
    verifying_key BLOB NOT NULL PRIMARY KEY,
    leaf_signing_key BLOB NOT NULL,
    identity_link_key BLOB NOT NULL
);

CREATE TABLE IF NOT EXISTS store_notifications (
    entity_id BLOB NOT NULL,
    kind INTEGER NOT NULL,
    added BOOLEAN NOT NULL,
    updated BOOLEAN NOT NULL,
    removed BOOLEAN NOT NULL,
    PRIMARY KEY (entity_id, kind)
);

CREATE TRIGGER IF NOT EXISTS delete_orphaned_data AFTER DELETE ON group_membership FOR EACH ROW BEGIN
-- Delete client credentials if they are not our own and not used in any group.
DELETE FROM client_credentials
WHERE
    fingerprint = OLD.client_credential_fingerprint
    AND NOT EXISTS (
        SELECT
            1
        FROM
            group_membership
        WHERE
            client_credential_fingerprint = OLD.client_credential_fingerprint
    )
    AND NOT EXISTS (
        SELECT
            1
        FROM
            own_client_info
        WHERE
            as_client_uuid = OLD.client_uuid
    );

-- Delete user profiles of users that are not in any group and that are not our own.
DELETE FROM users
WHERE
    user_name = OLD.user_name
    AND NOT EXISTS (
        SELECT
            1
        FROM
            group_membership
        WHERE
            user_name = OLD.user_name
    )
    AND NOT EXISTS (
        SELECT
            1
        FROM
            own_client_info
        WHERE
            as_user_name = OLD.user_name
    );

END;

CREATE TRIGGER IF NOT EXISTS no_partial_contact_overlap_on_insert BEFORE INSERT ON contacts FOR EACH ROW BEGIN
SELECT
    CASE
        WHEN EXISTS (
            SELECT
                1
            FROM
                partial_contacts
            WHERE
                user_name = NEW.user_name
        ) THEN RAISE (
            FAIL,
            'Can''t insert Contact: There already exists a partial contact with this user_name'
        )
    END;

END;

CREATE TRIGGER IF NOT EXISTS no_partial_contact_overlap_on_update BEFORE
UPDATE ON contacts FOR EACH ROW BEGIN
SELECT
    CASE
        WHEN EXISTS (
            SELECT
                1
            FROM
                partial_contacts
            WHERE
                user_name = NEW.user_name
        ) THEN RAISE (
            FAIL,
            'Can''t update Contact: There already exists a partial contact with this user_name'
        )
    END;

END;

CREATE TRIGGER IF NOT EXISTS no_contact_overlap_on_insert BEFORE INSERT ON partial_contacts FOR EACH ROW BEGIN
SELECT
    CASE
        WHEN EXISTS (
            SELECT
                1
            FROM
                contacts
            WHERE
                user_name = NEW.user_name
        ) THEN RAISE (
            FAIL,
            'Can''t insert PartialContact: There already exists a contact with this user_name'
        )
    END;

END;

CREATE TRIGGER IF NOT EXISTS no_contact_overlap_on_update BEFORE
UPDATE ON partial_contacts FOR EACH ROW BEGIN
SELECT
    CASE
        WHEN EXISTS (
            SELECT
                1
            FROM
                contacts
            WHERE
                user_name = NEW.user_name
        ) THEN RAISE (
            FAIL,
            'Can''t update PartialContact: There already exists a contact with this user_name'
        )
    END;

END;
