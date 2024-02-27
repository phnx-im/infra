// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::{identifiers::Fqdn, time::TimeStamp};

use super::{MessageId, MimiContent, NestablePart, ReplyToInfo, TopicId};

pub(super) struct MimiContentBuilder {
    content: MimiContent,
}

#[allow(dead_code)]
impl MimiContentBuilder {
    pub(super) fn new(sender_domain: Fqdn, nestable_part: NestablePart) -> Self {
        let content = MimiContent {
            id: MessageId::new(sender_domain),
            timestamp: TimeStamp::now(),
            replaces: None,
            topic_id: None,
            expires: None,
            in_reply_to: None,
            last_seen: Vec::new(),
            body: nestable_part,
        };
        Self { content }
    }

    pub(super) fn with_replaces(mut self, replaces: MessageId) -> Self {
        self.content.replaces = Some(replaces);
        self
    }

    pub(super) fn with_topic_id(mut self, id: Vec<u8>) -> Self {
        self.content.topic_id = Some(TopicId { id });
        self
    }

    pub(super) fn with_expires(mut self, expires: TimeStamp) -> Self {
        self.content.expires = Some(expires);
        self
    }

    pub(super) fn with_in_reply_to(mut self, in_reply_to: ReplyToInfo) -> Self {
        self.content.in_reply_to = Some(in_reply_to);
        self
    }

    pub(super) fn with_last_seen(mut self, last_seen: Vec<MessageId>) -> Self {
        self.content.last_seen = last_seen;
        self
    }

    pub(super) fn build(self) -> MimiContent {
        self.content
    }
}
