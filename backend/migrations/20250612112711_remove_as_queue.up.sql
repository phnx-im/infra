-- SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
--
-- SPDX-License-Identifier: AGPL-3.0-or-later
--
-- Removes AS queues and related fields.
--
DROP TABLE IF EXISTS as_queues;

DROP TABLE IF EXISTS as_queue_data;

ALTER TABLE as_client_records
DROP COLUMN queue_encryption_key;

ALTER TABLE as_client_records
DROP COLUMN ratchet;

DROP INDEX IF EXISTS idx_connection_package_user_uuid;

DROP TABLE IF EXISTS connection_packages;
