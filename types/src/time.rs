// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use chrono::{DateTime, SubsecRound, TimeZone, Utc};
use rusqlite::{types::FromSql, ToSql};

use super::*;

pub use chrono::Duration;

/// A time stamp that can be used to represent a point in time.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Eq, Hash, Copy)]
pub struct TimeStamp {
    time: DateTime<Utc>,
}

impl From<DateTime<Utc>> for TimeStamp {
    fn from(time: DateTime<Utc>) -> Self {
        let time = time.round_subsecs(3);
        Self { time }
    }
}

impl From<TimeStamp> for i64 {
    fn from(time: TimeStamp) -> Self {
        time.time.timestamp_millis()
    }
}

impl TryFrom<i64> for TimeStamp {
    type Error = TimeStampError;

    fn try_from(time: i64) -> Result<Self, Self::Error> {
        let time_result = Utc.timestamp_millis_opt(time);
        match time_result {
            chrono::LocalResult::Single(time) => Ok(time.into()),
            chrono::LocalResult::None | chrono::LocalResult::Ambiguous(_, _) => {
                Err(TimeStampError::InvalidInput)
            }
        }
    }
}

#[derive(Error, Debug)]
pub enum TimeStampError {
    #[error("Invalid input")]
    InvalidInput,
}

// We need this conversion, because Dart will only be able to send us u64.
impl TryFrom<u64> for TimeStamp {
    type Error = TimeStampError;

    fn try_from(time: u64) -> Result<Self, Self::Error> {
        Self::try_from(time as i64)
    }
}

impl Size for TimeStamp {
    fn tls_serialized_len(&self) -> usize {
        8
    }
}

impl TlsSerializeTrait for TimeStamp {
    fn tls_serialize<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, tls_codec::Error> {
        let time_i64: i64 = (*self).into();
        time_i64.to_be_bytes().tls_serialize(writer)
    }
}

impl TlsDeserializeBytesTrait for TimeStamp {
    fn tls_deserialize_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), tls_codec::Error>
    where
        Self: Sized,
    {
        const I64_SIZE: usize = i64::BITS as usize / 8;
        let time_i64_bytes: [u8; I64_SIZE] = bytes
            .get(..I64_SIZE)
            .ok_or(tls_codec::Error::EndOfStream)?
            .try_into()
            .map_err(|_| tls_codec::Error::EndOfStream)?;
        let time_i64 = i64::from_be_bytes(time_i64_bytes);
        let time = TimeStamp::try_from(time_i64)
            .map_err(|_| tls_codec::Error::DecodingError("Invalid timestamp".to_string()))?;
        Ok((time, &bytes[I64_SIZE..]))
    }
}

impl ToSql for TimeStamp {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        self.time.to_sql()
    }
}

#[cfg(feature = "sqlite")]
impl FromSql for TimeStamp {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let time = DateTime::<Utc>::column_result(value)?;
        Ok(time.into())
    }
}

#[cfg(feature = "sqlite")]
impl TimeStamp {
    pub fn now() -> Self {
        // We round the subseconds to 3 digits, because we don't need more
        // precision.
        Utc::now().into()
    }

    pub fn in_days(days_in_the_future: i64) -> Self {
        let time = Utc::now() + Duration::days(days_in_the_future);
        time.into()
    }

    /// Checks if this time stamp is more than `expiration_days` in the past.
    pub fn has_expired(&self, expiration_days: i64) -> bool {
        let time_left = Utc::now() - Duration::days(expiration_days);
        Self::from(time_left).time >= self.time
    }

    pub fn is_between(&self, start: &Self, end: &Self) -> bool {
        self.time >= start.time && self.time <= end.time
    }

    pub fn is_more_recent_than(&self, other: &Self) -> bool {
        self.time > other.time
    }

    pub fn time(&self) -> DateTime<Utc> {
        self.time
    }

    pub fn as_u64(&self) -> u64 {
        let time_i64: i64 = (*self).into();
        time_i64 as u64
    }
}

#[cfg(test)]
mod timestamp_conversion {
    use super::*;

    #[test]
    fn timestamp_conversion() {
        let time = TimeStamp::now();
        let time_u64 = time.as_u64();
        let time_converted = TimeStamp::try_from(time_u64).unwrap();
        assert_eq!(time, time_converted);

        let time = TimeStamp::now();
        let time_serialized = time.tls_serialize_detached().unwrap();
        let time_deserialized = TimeStamp::tls_deserialize_exact_bytes(&time_serialized).unwrap();
        assert_eq!(time, time_deserialized);
    }
}

#[derive(Clone, Debug, TlsDeserializeBytes, TlsSerialize, TlsSize, Serialize, Deserialize)]
pub struct ExpirationData {
    not_before: TimeStamp,
    not_after: TimeStamp,
}

impl ExpirationData {
    /// Create a new instance of [`ExpirationData`] that expires in `lifetime`
    /// days and the validity of which starts now.
    pub fn new(lifetime: i64) -> Self {
        let not_before = Utc::now() - Duration::minutes(15);
        Self {
            not_before: TimeStamp::from(not_before),
            not_after: TimeStamp::in_days(lifetime),
        }
    }

    /// Return false either if the `not_after` date has passed, or if the
    /// `not_before` date has not passed yet.
    pub fn validate(&self) -> bool {
        let now = TimeStamp::now();
        now.is_between(&self.not_before, &self.not_after)
    }

    pub fn not_before(&self) -> TimeStamp {
        self.not_before
    }

    pub fn not_after(&self) -> TimeStamp {
        self.not_after
    }
}
