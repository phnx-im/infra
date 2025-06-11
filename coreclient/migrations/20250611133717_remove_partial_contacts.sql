-- SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
--
-- SPDX-License-Identifier: AGPL-3.0-or-later
--
-- This migration removes the `partial_contacts` table and the corresponding
-- triggers.
DROP TRIGGER IF EXISTS no_partial_contact_overlap_on_update;

DROP TRIGGER IF EXISTS no_partial_contact_overlap_on_insert;

DROP TRIGGER IF EXISTS no_contact_overlap_on_insert;

DROP TABLE IF EXISTS partial_contacts;
