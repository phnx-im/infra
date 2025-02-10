-- SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
--
-- SPDX-License-Identifier: AGPL-3.0-or-later
CREATE INDEX IF NOT EXISTS conversation_messages_timestamp_asc_idx ON conversation_messages (timestamp ASC);

CREATE INDEX IF NOT EXISTS conversation_messages_timestamp_desc_idx ON conversation_messages (timestamp DESC);
