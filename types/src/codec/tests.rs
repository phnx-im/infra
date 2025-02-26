// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::memory_provider::Codec;
use serde::{Serialize, de::DeserializeOwned};

use crate::codec::PhnxCodec;

pub(super) struct Json;

impl super::Codec for Json {
    type Error = serde_json::Error;

    fn to_vec<T>(value: &T) -> Result<Vec<u8>, Self::Error>
    where
        T: Sized + Serialize,
    {
        serde_json::to_vec(value)
    }

    fn from_slice<T>(bytes: &[u8]) -> Result<T, Self::Error>
    where
        T: DeserializeOwned,
    {
        serde_json::from_slice(bytes)
    }
}

fn run_for_all_versions<F>(f: F)
where
    F: Fn(super::PhnxCodec),
{
    for version in 0..u8::MAX {
        let Ok(codec) = super::PhnxCodec::try_from(version) else {
            return;
        };
        f(codec)
    }
}

#[test]
fn serde_json() {
    let value = 42;
    let serialized = Json::to_vec(&value).unwrap();
    let deserialized: i32 = Json::from_slice(&serialized).unwrap();
    assert_eq!(value, deserialized);
}

/// Test that all codecs can serialize and deserialize a simple value
#[test]
fn functional_correctness() {
    let value = 42;
    let functional_correctness_inner = |codec: super::PhnxCodec| {
        println!("Testing codec: {:?}", codec);
        let serialized = codec.serialize(&value).unwrap();
        // The first byte should be the codec version
        assert!(serialized.first().copied() == Some(codec as u8));
        // The rest of the bytes should be the serialized value
        let deserialized: i32 = codec.deserialize(&serialized[1..]).unwrap();
        assert_eq!(value, deserialized);
    };
    run_for_all_versions(functional_correctness_inner);

    // For good measure, check that the public API works as well
    let serialized = PhnxCodec::to_vec(&value).unwrap();
    let deserialized: i32 = PhnxCodec::from_slice(&serialized).unwrap();
    assert_eq!(value, deserialized);
}

/// Test that the default codec can deserialize serialized values from all codec
/// versions
#[test]
fn default_codec_deserialization() {
    let value = 42;
    let default_codec_deserialization_inner = |codec: super::PhnxCodec| {
        let serialized = codec.serialize(&value).unwrap();
        let deserialized: i32 = super::PhnxCodec::from_slice(&serialized).unwrap();
        assert_eq!(value, deserialized);
    };

    run_for_all_versions(default_codec_deserialization_inner);
}
