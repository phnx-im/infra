-- SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
--
-- SPDX-License-Identifier: AGPL-3.0-or-later
DROP TABLE IF EXISTS key_packages;

DROP INDEX IF EXISTS idx_key_package_client_id;

DROP TABLE IF EXISTS qs_queues;

DROP TABLE IF EXISTS qs_queue_data;

DROP TABLE IF EXISTS qs_client_records;

DROP TABLE IF EXISTS qs_user_records;

DROP TABLE IF EXISTS qs_decryption_key;

DROP TABLE IF EXISTS qs_signing_key;

DROP TYPE IF EXISTS decryption_key_data;

DROP TYPE IF EXISTS signing_key_data;

