-- SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
--
-- SPDX-License-Identifier: AGPL-3.0-or-later
CREATE TABLE IF NOT EXISTS client_record (
    user_uuid BLOB NOT NULL,
    user_domain TEXT NOT NULL,
    record_state TEXT NOT NULL CHECK (record_state IN ('in_progress', 'finished')),
    created_at DATETIME NOT NULL,
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    PRIMARY KEY (user_uuid, user_domain)
);

CREATE TABLE IF NOT EXISTS user_creation_state (
    user_uuid BLOB NOT NULL,
    user_domain TEXT NOT NULL,
    state BLOB NOT NULL,
    PRIMARY KEY (user_uuid, user_domain)
);

CREATE TABLE IF NOT EXISTS own_client_info (
    server_url TEXT NOT NULL,
    qs_user_id BLOB NOT NULL,
    qs_client_id BLOB NOT NULL,
    user_uuid BLOB NOT NULL,
    user_domain TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS users (
    user_uuid BLOB NOT NULL,
    user_domain TEXT NOT NULL,
    epoch INTEGER NOT NULL,
    decryption_key_index BLOB NOT NULL,
    display_name TEXT NOT NULL,
    profile_picture BLOB,
    PRIMARY KEY (user_uuid, user_domain)
);

CREATE TABLE IF NOT EXISTS "groups" (
    group_id BLOB NOT NULL PRIMARY KEY,
    identity_link_wrapper_key BLOB NOT NULL,
    group_state_ear_key BLOB NOT NULL,
    pending_diff BLOB,
    room_state BLOB NOT NULL
);

CREATE TABLE IF NOT EXISTS client_credentials (
    fingerprint BLOB NOT NULL PRIMARY KEY,
    user_uuid BLOB NOT NULL,
    user_domain TEXT NOT NULL,
    client_credential BLOB NOT NULL
);

CREATE INDEX IF NOT EXISTS client_credentials_user_id ON client_credentials (user_uuid, user_domain);

CREATE TABLE IF NOT EXISTS group_membership (
    client_credential_fingerprint BLOB NOT NULL,
    group_id BLOB NOT NULL,
    user_uuid BLOB NOT NULL,
    user_domain TEXT NOT NULL,
    leaf_index INTEGER NOT NULL,
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

CREATE TABLE IF NOT EXISTS indexed_keys (
    key_index BLOB NOT NULL PRIMARY KEY,
    key_value BLOB NOT NULL,
    base_secret BLOB NOT NULL
);

CREATE TABLE IF NOT EXISTS own_key_indices (
    key_type TEXT CHECK (key_type IN ('user_profile_key')) PRIMARY KEY,
    key_index BLOB NOT NULL,
    FOREIGN KEY (key_index) REFERENCES indexed_keys (key_index) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS contacts (
    user_uuid BLOB NOT NULL,
    user_domain TEXT NOT NULL,
    conversation_id BLOB NOT NULL,
    wai_ear_key BLOB NOT NULL,
    friendship_token BLOB NOT NULL,
    connection_key BLOB NOT NULL,
    user_profile_key_index BLOB NOT NULL,
    PRIMARY KEY (user_uuid, user_domain),
    FOREIGN KEY (conversation_id) REFERENCES conversations (conversation_id),
    FOREIGN KEY (user_profile_key_index) REFERENCES indexed_keys (key_index)
);

CREATE TABLE IF NOT EXISTS partial_contacts (
    user_uuid BLOB NOT NULL,
    user_domain TEXT NOT NULL,
    conversation_id BLOB NOT NULL,
    friendship_package_ear_key BLOB NOT NULL,
    PRIMARY KEY (user_uuid, user_domain),
    FOREIGN KEY (conversation_id) REFERENCES conversations (conversation_id)
);

CREATE TABLE IF NOT EXISTS conversations (
    conversation_id BLOB NOT NULL PRIMARY KEY,
    conversation_title TEXT NOT NULL,
    conversation_picture BLOB,
    group_id BLOB NOT NULL,
    last_read TEXT NOT NULL,
    -- missing `connection_as_{client_uuid,domain}` fields means it is a group conversation
    connection_user_uuid BLOB,
    connection_user_domain TEXT,
    is_confirmed_connection BOOLEAN NOT NULL DEFAULT FALSE,
    is_active BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE TABLE IF NOT EXISTS conversation_past_members (
    conversation_id BLOB NOT NULL,
    member_user_uuid BLOB NOT NULL,
    member_user_domain TEXT NOT NULL,
    PRIMARY KEY (
        conversation_id,
        member_user_uuid,
        member_user_domain
    ),
    FOREIGN KEY (conversation_id) REFERENCES conversations (conversation_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS conversation_past_members_conversation_id_idx ON conversation_past_members (conversation_id);

CREATE TABLE IF NOT EXISTS conversation_messages (
    message_id BLOB NOT NULL PRIMARY KEY,
    conversation_id BLOB NOT NULL,
    timestamp TEXT NOT NULL,
    -- missing `sender_as_{client_uuid,domain}` fields means it is a system message
    sender_user_uuid BLOB,
    sender_user_domain TEXT,
    content BLOB NOT NULL,
    sent BOOLEAN NOT NULL,
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
    user_domain TEXT PRIMARY KEY,
    verifying_key BLOB NOT NULL
);

CREATE TABLE IF NOT EXISTS queue_ratchets (
    queue_type TEXT PRIMARY KEY CHECK (queue_type IN ('as', 'qs')),
    queue_ratchet BLOB NOT NULL,
    sequence_number INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS as_credentials (
    fingerprint TEXT PRIMARY KEY,
    user_domain TEXT NOT NULL,
    credential_type TEXT NOT NULL CHECK (
        credential_type IN ('as_credential', 'as_intermediate_credential')
    ),
    credential BLOB NOT NULL
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
            user_uuid = OLD.user_uuid
            AND user_domain = OLD.user_domain
    );

-- Delete user profiles of users that are not in any group and that are not our own.
DELETE FROM users
WHERE
    user_uuid = OLD.user_uuid
    AND user_domain = OLD.user_domain
    AND NOT EXISTS (
        SELECT
            1
        FROM
            group_membership
        WHERE
            user_uuid = OLD.user_uuid
            AND user_domain = OLD.user_domain
    )
    AND NOT EXISTS (
        SELECT
            1
        FROM
            own_client_info
        WHERE
            user_uuid = OLD.user_uuid
            AND user_domain = OLD.user_domain
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
                user_uuid = NEW.user_uuid
                AND user_domain = NEW.user_domain
        ) THEN RAISE (
            FAIL,
            'Can''t insert Contact: There already exists a partial contact with this client_id and domain'
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
                user_uuid = NEW.user_uuid
                AND user_domain = NEW.user_domain
        ) THEN RAISE (
            FAIL,
            'Can''t update Contact: There already exists a partial contact with this client_id and domain'
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
                user_uuid = NEW.user_uuid
                AND user_domain = NEW.user_domain
        ) THEN RAISE (
            FAIL,
            'Can''t insert PartialContact: There already exists a contact with this client_id and domain'
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
                user_uuid = NEW.user_uuid
                AND user_domain = NEW.user_domain
        ) THEN RAISE (
            FAIL,
            'Can''t update PartialContact: There already exists a contact with this client_id and domain'
        )
    END;

END;

CREATE TRIGGER IF NOT EXISTS delete_keys AFTER DELETE ON contacts FOR EACH ROW BEGIN
-- Delete user profile keys if the corresponding contact is deleted. Since key
-- indexes include the user id in their derivation, they are unique per user
-- and we don't need to check if they are used by another user (or ourselves).
DELETE FROM indexed_keys
WHERE
    fingerprint = OLD.user_profile_key_index;

END;
