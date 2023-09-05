// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{
    collections::BTreeMap,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub(crate) mod persistance;

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

// Generic serialization function for HashMaps and BTreeMaps (and anything else
// that yields iterators over tuples).
pub(crate) fn serialize_hashmap<'a, T, U, V, S>(v: &'a V, serializer: S) -> Result<S::Ok, S::Error>
where
    T: Serialize,
    U: Serialize,
    &'a V: IntoIterator<Item = (T, U)> + 'a,
    S: Serializer,
{
    let vec = v.into_iter().collect::<Vec<_>>();
    vec.serialize(serializer)
}

pub(crate) fn deserialize_btreemap<'de, T, U, D>(
    deserializer: D,
) -> Result<BTreeMap<T, U>, D::Error>
where
    T: Ord + Eq + std::hash::Hash + Deserialize<'de>,
    U: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Ok(Vec::<(T, U)>::deserialize(deserializer)?
        .into_iter()
        .collect::<BTreeMap<T, U>>())
}
