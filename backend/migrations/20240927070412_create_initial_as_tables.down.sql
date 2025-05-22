-- SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
--
-- SPDX-License-Identifier: AGPL-3.0-or-later
DROP TABLE IF EXISTS as_user_handles;

DROP TABLE IF EXISTS as_queues;

DROP TABLE IF EXISTS as_queue_data;

DROP TABLE IF EXISTS as_signing_keys;

DROP INDEX IF EXISTS idx_connection_package_user_uuid;

DROP TABLE IF EXISTS connection_packages;

DROP TABLE IF EXISTS as_client_records;

DROP TABLE IF EXISTS as_user_records;

DROP TABLE IF EXISTS as_batched_keys;

DROP TYPE IF EXISTS credential_type;

DROP TYPE IF EXISTS client_credential;

DROP TYPE IF EXISTS expiration;

DROP TYPE IF EXISTS indexed_ciphertext;

DROP TYPE IF EXISTS aead_ciphertext;
