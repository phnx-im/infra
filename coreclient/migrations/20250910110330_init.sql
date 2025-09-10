-- SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
--
-- SPDX-License-Identifier: AGPL-3.0-or-later
--
--
-- MLS Storage Provider Schema
--
CREATE TABLE group_data (
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

CREATE TABLE proposal (
    group_id BLOB NOT NULL,
    proposal_ref BLOB NOT NULL,
    proposal BLOB NOT NULL,
    PRIMARY KEY (group_id, proposal_ref)
);

CREATE TABLE own_leaf_node (
    group_id BLOB PRIMARY KEY,
    leaf_node BLOB NOT NULL
);

CREATE TABLE signature_key (
    public_key BLOB PRIMARY KEY,
    signature_key BLOB NOT NULL
);

CREATE TABLE encryption_key (
    public_key BLOB PRIMARY KEY,
    key_pair BLOB NOT NULL
);

CREATE TABLE epoch_key_pairs (
    group_id BLOB NOT NULL,
    epoch_id BLOB NOT NULL,
    leaf_index INTEGER NOT NULL,
    key_pairs BLOB NOT NULL,
    PRIMARY KEY (group_id, epoch_id, leaf_index)
);

CREATE TABLE key_package (
    key_package_ref BLOB PRIMARY KEY,
    key_package BLOB NOT NULL
);

CREATE TABLE psk (psk_id BLOB PRIMARY KEY, psk_bundle BLOB NOT NULL);

--
-- Client Records Schema
--
CREATE TABLE client_record (
    user_uuid BLOB NOT NULL,
    user_domain TEXT NOT NULL,
    record_state TEXT NOT NULL CHECK (record_state IN ('in_progress', 'finished')),
    created_at DATETIME NOT NULL,
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    PRIMARY KEY (user_uuid, user_domain)
);

--
-- Client Storage Schema
--
CREATE TABLE user_creation_state (
    user_uuid BLOB NOT NULL,
    user_domain TEXT NOT NULL,
    state BLOB NOT NULL,
    PRIMARY KEY (user_uuid, user_domain)
);

CREATE TABLE own_client_info (
    server_url TEXT NOT NULL,
    qs_user_id BLOB NOT NULL,
    qs_client_id BLOB NOT NULL,
    user_uuid BLOB NOT NULL,
    user_domain TEXT NOT NULL
);

CREATE TABLE user (
    user_uuid BLOB NOT NULL,
    user_domain TEXT NOT NULL,
    epoch INTEGER NOT NULL,
    decryption_key_index BLOB NOT NULL,
    display_name TEXT NOT NULL,
    profile_picture BLOB,
    PRIMARY KEY (user_uuid, user_domain)
);

CREATE TABLE IF NOT EXISTS "group" (
    group_id BLOB NOT NULL PRIMARY KEY,
    identity_link_wrapper_key BLOB NOT NULL,
    group_state_ear_key BLOB NOT NULL,
    pending_diff BLOB,
    room_state BLOB NOT NULL
);

CREATE TABLE client_credential (
    fingerprint BLOB NOT NULL PRIMARY KEY,
    user_uuid BLOB NOT NULL,
    user_domain TEXT NOT NULL,
    client_credential BLOB NOT NULL
);

CREATE INDEX client_credential_user_id ON client_credential (user_uuid, user_domain);

CREATE TABLE group_membership (
    client_credential_fingerprint BLOB NOT NULL REFERENCES client_credential (fingerprint),
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
    PRIMARY KEY (group_id, leaf_index, status)
);

CREATE TABLE indexed_key (
    key_index BLOB NOT NULL PRIMARY KEY,
    key_value BLOB NOT NULL,
    base_secret BLOB NOT NULL
);

CREATE TABLE own_key_index (
    key_type TEXT CHECK (key_type IN ('user_profile_key')) PRIMARY KEY,
    key_index BLOB NOT NULL REFERENCES indexed_key (key_index) ON DELETE CASCADE
);

CREATE TABLE chat (
    chat_id BLOB NOT NULL PRIMARY KEY,
    chat_title TEXT NOT NULL,
    chat_picture BLOB,
    group_id BLOB NOT NULL,
    last_read TEXT NOT NULL,
    -- missing `connection_as_{client_uuid,domain}` fields means it is a group chat
    connection_user_uuid BLOB,
    connection_user_domain TEXT,
    is_confirmed_connection BOOLEAN NOT NULL DEFAULT FALSE,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    connection_user_handle TEXT
);

CREATE TABLE chat_past_member (
    chat_id BLOB NOT NULL REFERENCES chat (chat_id) ON DELETE CASCADE,
    member_user_uuid BLOB NOT NULL,
    member_user_domain TEXT NOT NULL,
    PRIMARY KEY (chat_id, member_user_uuid, member_user_domain)
);

CREATE INDEX chat_past_member_chat_id_idx ON chat_past_member (chat_id);

CREATE TABLE contact (
    user_uuid BLOB NOT NULL,
    user_domain TEXT NOT NULL,
    chat_id BLOB NOT NULL REFERENCES chat (chat_id) ON DELETE CASCADE,
    wai_ear_key BLOB NOT NULL,
    friendship_token BLOB NOT NULL,
    connection_key BLOB NOT NULL,
    user_profile_key_index BLOB NOT NULL REFERENCES indexed_key (key_index),
    PRIMARY KEY (user_uuid, user_domain)
);

CREATE TABLE message (
    message_id BLOB NOT NULL PRIMARY KEY,
    chat_id BLOB NOT NULL REFERENCES chat (chat_id) ON DELETE CASCADE DEFERRABLE INITIALLY DEFERRED,
    timestamp TEXT NOT NULL,
    -- missing `sender_as_{client_uuid,domain}` fields means it is a system message
    sender_user_uuid BLOB,
    sender_user_domain TEXT,
    content BLOB NOT NULL,
    sent BOOLEAN NOT NULL,
    mimi_id BLOB,
    status INT NOT NULL DEFAULT 0,
    edited_at TEXT
);

CREATE INDEX message_chat_id_idx ON message (chat_id);

CREATE INDEX message_timetstamp_idx ON message (timestamp);

CREATE INDEX message_mimi_id_idx ON message (mimi_id);

CREATE TABLE qs_verifying_key (
    user_domain TEXT PRIMARY KEY,
    verifying_key BLOB NOT NULL
);

CREATE TABLE queue_ratchet (
    queue_type TEXT PRIMARY KEY CHECK (queue_type IN ('qs')),
    queue_ratchet BLOB NOT NULL,
    sequence_number INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE as_credential (
    fingerprint TEXT PRIMARY KEY,
    user_domain TEXT NOT NULL,
    credential_type TEXT NOT NULL CHECK (
        credential_type IN ('as_credential', 'as_intermediate_credential')
    ),
    credential BLOB NOT NULL
);

CREATE TABLE store_notification (
    entity_id BLOB NOT NULL,
    kind INTEGER NOT NULL,
    added BOOLEAN NOT NULL,
    updated BOOLEAN NOT NULL,
    removed BOOLEAN NOT NULL,
    PRIMARY KEY (entity_id, kind)
);

CREATE TRIGGER delete_orphaned_data AFTER DELETE ON group_membership FOR EACH ROW BEGIN
-- Delete client credentials if they are not our own and not used in any group.
DELETE FROM client_credential
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
DELETE FROM user
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

CREATE TRIGGER delete_keys AFTER DELETE ON contact FOR EACH ROW BEGIN
-- Delete user profile keys if the corresponding contact is deleted. Since key
-- indexes include the user id in their derivation, they are unique per user
-- and we don't need to check if they are used by another user (or ourselves).
DELETE FROM indexed_key
WHERE
    key_index = OLD.user_profile_key_index
    AND (
        key_index NOT IN (
            SELECT
                decryption_key_index
            FROM
                user
        )
    );

END;

CREATE TABLE user_handle (
    handle TEXT NOT NULL PRIMARY KEY,
    hash BLOB NOT NULL,
    signing_key BLOB NOT NULL,
    created_at TEXT NOT NULL,
    refreshed_at TEXT NOT NULL
);

CREATE TABLE user_handle_contact (
    user_handle TEXT NOT NULL PRIMARY KEY,
    -- 1:1 relationship with chat
    chat_id BLOB NOT NULL UNIQUE REFERENCES chat (chat_id) ON DELETE CASCADE,
    friendship_package_ear_key BLOB NOT NULL,
    created_at TEXT NOT NULL,
    connection_offer_hash BLOB NOT NULL
);

CREATE TABLE attachment (
    attachment_id BLOB NOT NULL PRIMARY KEY,
    chat_id BLOB NOT NULL REFERENCES chat (chat_id) ON DELETE CASCADE,
    message_id BLOB NOT NULL REFERENCES message (message_id) ON DELETE CASCADE,
    content_type TEXT NOT NULL,
    content BLOB,
    status INTEGER NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX attachment_created_at_index ON attachment (created_at);

CREATE TABLE pending_attachment (
    attachment_id BLOB NOT NULL PRIMARY KEY REFERENCES attachment (attachment_id) ON DELETE CASCADE,
    size INTEGER NOT NULL,
    enc_alg INTEGER NOT NULL,
    enc_key BLOB NOT NULL,
    nonce BLOB NOT NULL,
    aad BLOB NOT NULL,
    hash_alg INTEGER NOT NULL,
    hash BLOB NOT NULL
);

CREATE TABLE user_setting (
    setting TEXT NOT NULL PRIMARY KEY,
    value BLOB NOT NULL
);

CREATE TABLE connection_package (
    connection_package_hash BLOB NOT NULL PRIMARY KEY,
    handle TEXT NOT NULL REFERENCES user_handle (handle) ON DELETE CASCADE,
    decryption_key BLOB NOT NULL,
    expires_at TEXT NOT NULL
);

CREATE TABLE message_draft (
    chat_id BLOB NOT NULL PRIMARY KEY REFERENCES chat (chat_id) ON DELETE CASCADE,
    message TEXT NOT NULL,
    editing_id BLOB REFERENCES message (message_id) ON DELETE CASCADE,
    updated_at TEXT NOT NULL
);

CREATE TABLE message_status (
    message_id BLOB NOT NULL REFERENCES message (message_id) ON DELETE CASCADE,
    sender_user_uuid BLOB NOT NULL,
    sender_user_domain TEXT NOT NULL,
    status INT NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY (message_id, sender_user_domain, sender_user_uuid)
);

CREATE TABLE message_edit (
    -- This is the Mimi ID of the `content` field.
    mimi_id BLOB NOT NULL PRIMARY KEY,
    -- the message that was edited
    --
    -- The content of the message is always the latest version of the message.
    -- That is, the latest edit contains the previous message content. The
    -- second latest edit contains the content before the latest edit, and so
    -- on.
    message_id BLOB NOT NULL REFERENCES message (message_id) ON DELETE CASCADE,
    created_at TEXT NOT NULL,
    -- content of the edited message
    content BLOB NOT NULL
);

CREATE INDEX message_edit_message_id_idx ON message_edit (message_id);
