// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use url::Url;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct AttachmentId {
    pub uuid: Uuid,
}

impl AttachmentId {
    pub fn new(uuid: Uuid) -> Self {
        Self { uuid }
    }

    pub fn url(&self) -> String {
        format!("phnx://attachment/{}", self.uuid)
    }

    pub fn from_url(url: &str) -> Option<Self> {
        let url = Url::parse(url).ok()?;
        let suffix = url.path().strip_prefix("phnx://attachment/")?;
        let uuid = suffix.parse().ok()?;
        Some(Self { uuid })
    }

    pub fn uuid(&self) -> Uuid {
        self.uuid
    }
}

mod sqlx_impls {
    use sqlx::{Database, Decode, Encode, Sqlite, Type, encode::IsNull, error::BoxDynError};

    use super::*;

    impl Type<Sqlite> for AttachmentId {
        fn type_info() -> <Sqlite as Database>::TypeInfo {
            <Uuid as Type<Sqlite>>::type_info()
        }
    }

    impl<'q> Encode<'q, Sqlite> for AttachmentId {
        fn encode_by_ref(
            &self,
            buf: &mut <Sqlite as Database>::ArgumentBuffer<'q>,
        ) -> Result<IsNull, BoxDynError> {
            Encode::<Sqlite>::encode_by_ref(&self.uuid, buf)
        }
    }

    impl<'r> Decode<'r, Sqlite> for AttachmentId {
        fn decode(value: <Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
            let id: Uuid = Decode::<Sqlite>::decode(value)?;
            Ok(Self::new(id))
        }
    }
}
