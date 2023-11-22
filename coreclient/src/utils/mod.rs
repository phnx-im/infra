// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub(crate) mod persistence;

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
