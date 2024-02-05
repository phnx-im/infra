// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use chrono::{DateTime, NaiveDateTime, Utc};

use super::*;

pub use chrono::Duration;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Eq, Hash, Copy)]
pub struct TimeStamp {
    time: DateTime<Utc>,
}

impl From<DateTime<Utc>> for TimeStamp {
    fn from(time: DateTime<Utc>) -> Self {
        Self { time }
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
        let time = DateTime::<Utc>::from_naive_utc_and_offset(
            NaiveDateTime::from_timestamp_millis(time as i64)
                .ok_or(TimeStampError::InvalidInput)?,
            Utc,
        );
        Ok(Self { time })
    }
}

impl Size for TimeStamp {
    fn tls_serialized_len(&self) -> usize {
        8
    }
}

impl TlsSerializeTrait for TimeStamp {
    fn tls_serialize<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, tls_codec::Error> {
        self.time
            .timestamp_millis()
            .to_be_bytes()
            .tls_serialize(writer)
    }
}

impl TlsDeserializeBytesTrait for TimeStamp {
    fn tls_deserialize_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), tls_codec::Error>
    where
        Self: Sized,
    {
        let millis_bytes: [u8; 8] = bytes
            .get(..8)
            .ok_or(tls_codec::Error::EndOfStream)?
            .try_into()
            .map_err(|_| tls_codec::Error::EndOfStream)?;
        let millis = i64::from_be_bytes(millis_bytes);
        let time = DateTime::<Utc>::from_naive_utc_and_offset(
            NaiveDateTime::from_timestamp_millis(millis).ok_or(tls_codec::Error::InvalidInput)?,
            Utc,
        );
        Ok((Self { time }, &bytes[8..]))
    }
}

impl TimeStamp {
    pub fn now() -> Self {
        let time = Utc::now();
        Self { time }
    }

    pub fn in_days(days_in_the_future: i64) -> Self {
        let time = Utc::now() + Duration::days(days_in_the_future);
        Self { time }
    }

    /// Checks if this time stamp is more than `expiration_days` in the past.
    pub fn has_expired(&self, expiration_days: i64) -> bool {
        Utc::now() - Duration::days(expiration_days) >= self.time
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
        self.time.timestamp_millis() as u64
    }

    pub fn as_i64(&self) -> i64 {
        self.time.timestamp_millis() as i64
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
        Self {
            not_before: TimeStamp::now(),
            not_after: TimeStamp::in_days(lifetime),
        }
    }

    /// Return false either if the `not_after` date has passed, or if the
    /// `not_before` date has not passed yet.
    pub fn validate(&self) -> bool {
        let now = TimeStamp::now();
        now.is_between(&self.not_before, &self.not_after)
    }
}
