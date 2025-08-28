-- SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
--
-- SPDX-License-Identifier: AGPL-3.0-or-later
--
-- Add encrypted DEK column to client_record table
ALTER TABLE client_record 
ADD COLUMN encrypted_dek BLOB NOT NULL;