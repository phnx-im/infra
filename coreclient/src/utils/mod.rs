// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{
    collections::{BTreeMap, HashMap},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

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

pub(crate) fn serialize_nested_hashmap<T, U, V, S>(
    v: &HashMap<T, HashMap<U, V>>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    T: Serialize,
    U: Serialize,
    V: Serialize,
    S: Serializer,
{
    let vec = v
        .iter()
        .map(|(index, inner_map)| {
            let inner_vec = inner_map.iter().collect::<Vec<_>>();
            (index, inner_vec)
        })
        .collect::<Vec<_>>();
    vec.serialize(serializer)
}

pub(crate) fn deserialize_hashmap<'de, T, U, D>(deserializer: D) -> Result<HashMap<T, U>, D::Error>
where
    T: Eq + std::hash::Hash + Deserialize<'de>,
    U: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Ok(Vec::<(T, U)>::deserialize(deserializer)?
        .into_iter()
        .collect::<HashMap<T, U>>())
}

pub(crate) fn deserialize_nested_hashmap<'de, T, U, V, D>(
    deserializer: D,
) -> Result<HashMap<T, HashMap<U, V>>, D::Error>
where
    T: Eq + std::hash::Hash + Deserialize<'de>,
    U: Eq + std::hash::Hash + Deserialize<'de>,
    V: Deserialize<'de>,
    D: Deserializer<'de>,
{
    let map = Vec::<(T, Vec<(U, V)>)>::deserialize(deserializer)?
        .into_iter()
        .map(|(index, inner_vec)| {
            let inner_map = inner_vec.into_iter().collect::<HashMap<U, V>>();
            (index, inner_map)
        })
        .collect();
    Ok(map)
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
