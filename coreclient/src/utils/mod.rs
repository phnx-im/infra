use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub(crate) struct Timestamp(u64);

impl Timestamp {
    pub(crate) fn now() -> Self {
        let now = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(st) => st.as_millis() as u64,
            _ => 0,
        };
        Self(now)
    }

    pub(crate) fn _from_u64(t: u64) -> Self {
        Self(t)
    }

    pub(crate) fn as_u64(&self) -> u64 {
        self.0
    }
}
