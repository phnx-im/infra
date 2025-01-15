use std::borrow::Cow;

use sqlx::{encode::IsNull, error::BoxDynError, Database, Encode, Sqlite, Type};
use uuid::Uuid;

use super::{ConversationId, ConversationStatus, ConversationType};

impl<DB> Type<DB> for ConversationId
where
    DB: Database,
    Uuid: Type<DB>,
{
    fn type_info() -> DB::TypeInfo {
        <Uuid as Type<DB>>::type_info()
    }
}

impl<'q, DB> Encode<'q, DB> for ConversationId
where
    DB: Database,
    Uuid: Encode<'q, DB>,
{
    fn encode_by_ref(
        &self,
        buf: &mut <DB as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, BoxDynError> {
        <Uuid as Encode<DB>>::encode_by_ref(&self.uuid, buf)
    }
}

impl ConversationStatus {
    pub(super) fn db_value(&self) -> Cow<'static, str> {
        match self {
            Self::Active => "active".into(),
            Self::Inactive(inactive_conversation) => {
                // TODO: use itertools to avoid allocation of Vec
                let user_names = inactive_conversation
                    .past_members()
                    .iter()
                    .map(|user_name| user_name.to_string())
                    .collect::<Vec<_>>()
                    .join(",");
                format!("inactive:{user_names}").into()
            }
        }
    }
}

impl Type<Sqlite> for ConversationStatus {
    fn type_info() -> <Sqlite as Database>::TypeInfo {
        <Cow<str> as Type<Sqlite>>::type_info()
    }
}

impl<'q> Encode<'q, Sqlite> for ConversationStatus {
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, BoxDynError> {
        <Cow<str> as Encode<Sqlite>>::encode(self.db_value(), buf)
    }

    fn encode(
        self,
        buf: &mut <Sqlite as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, BoxDynError> {
        <Cow<str> as Encode<Sqlite>>::encode(self.db_value(), buf)
    }
}

impl ConversationType {
    pub(super) fn db_value(&self) -> Cow<'static, str> {
        match self {
            Self::UnconfirmedConnection(user_name) => {
                format!("unconfirmed_connection:{user_name}").into()
            }
            Self::Connection(user_name) => format!("connection:{user_name}").into(),
            Self::Group => "group".into(),
        }
    }
}

impl Type<Sqlite> for ConversationType {
    fn type_info() -> <Sqlite as Database>::TypeInfo {
        <Cow<str> as Type<Sqlite>>::type_info()
    }
}

impl<'q> Encode<'q, Sqlite> for ConversationType {
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, BoxDynError> {
        <Cow<str> as Encode<Sqlite>>::encode(self.db_value(), buf)
    }

    fn encode(
        self,
        buf: &mut <Sqlite as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, BoxDynError> {
        <Cow<str> as Encode<Sqlite>>::encode(self.db_value(), buf)
    }
}
