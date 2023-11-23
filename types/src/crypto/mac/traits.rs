// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! This module contains traits that facilitate the computation and verification
//! of MACs over other serializable types.
//! TODO: Provide more documentation on the nature and the relationship of the
//! individual traits.

use digest::Mac as DigestMac;
use serde::Serialize;
use thiserror::Error;
use tracing::instrument;

use crate::crypto::{errors::RandomnessError, secrets::Secret};

use super::{Mac, MacTag, MAC_KEY_SIZE};

/// Error computing MAC.
#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum MacComputationError {
    /// Failed to serialize the given payload.
    #[error("Failed to serialize the given payload.")]
    SerializationError,
    /// An unrecoverable error has occurred.
    #[error("An unrecoverable error has occurred.")]
    LibraryError,
}

/// Error verifying MAC.
#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum MacVerificationError {
    /// Could not verify this mac with the given payload.
    #[error("Could not verify this mac with the given payload.")]
    VerificationFailure,
    /// An unrecoverable error has occurred.
    #[error("An unrecoverable error has occurred.")]
    LibraryError,
}

/// A trait that allows the use of secrets of size [`MAC_KEY_SIZE`] for the
/// computation and verification of MAC tags.
pub trait MacKey:
    From<Secret<MAC_KEY_SIZE>> + AsRef<Secret<MAC_KEY_SIZE>> + std::fmt::Debug
{
    /// Generate a random new MAC key.
    #[instrument(level = "trace")]
    fn random() -> Result<Self, RandomnessError> {
        Ok(Secret::random()?.into())
    }

    /// Compute a MAC tag over the given payload and return the tag.
    #[instrument(level = "trace", skip_all, fields(
        mac_key_type = std::any::type_name::<Self>(),
    ))]
    fn mac(&self, payload: &[u8]) -> MacTag {
        let mut mac = match Mac::new_from_slice(self.as_ref().secret()) {
            Ok(mac) => mac,
            // TODO: Have a test that checks that MAC_KEY_SIZE is actually correct.
            Err(_) => return MacTag { tag: vec![] },
        };
        mac.update(payload);
        let tag = mac.finalize().into_bytes().to_vec();
        MacTag { tag }
    }

    /// Verify the given MAC tag with the given payload. Returns an error if the
    /// verification fails or if the tag does not have the right length.
    #[instrument(level = "trace", skip_all, fields(
        mac_key_type = std::any::type_name::<Self>(),
    ))]
    fn verify(&self, payload: &[u8], tag: &MacTag) -> Result<(), MacVerificationError> {
        let mut mac = match Mac::new_from_slice(self.as_ref().secret()) {
            Ok(mac) => mac,
            // TODO: Have a test that checks that MAC_KEY_SIZE is actually correct.
            Err(_) => return Err(MacVerificationError::LibraryError),
        };
        mac.update(payload);
        mac.verify_slice(&tag.tag)
            .map_err(|_| MacVerificationError::VerificationFailure)
    }
}

pub trait TaggedStruct<T> {
    fn from_untagged_payload(payload: T, mac: MacTag) -> Self;
}

/// This trait should be implemented by structs that can be tagged with a mac
/// key.
pub trait Taggable: Sized + Serialize {
    type TaggedOutput: TaggedStruct<Self>;
    type Key: MacKey;

    //fn serialized_payload<S: Serializer>(&self) -> Result<Vec<u8>, S::Error>;

    fn tag(self, key: &Self::Key) -> Result<Self::TaggedOutput, serde_json::Error> {
        // TODO: Not sure how to make serialization generic.
        let serialized_payload: Vec<u8> = serde_json::to_vec(&self)?;
        let tag = key.mac(&serialized_payload);
        Ok(<Self::TaggedOutput as TaggedStruct<Self>>::from_untagged_payload(self, tag))
    }
}

pub trait TagVerified<T> {
    type SealingType: Default;

    #[doc(hidden)]
    fn from_payload(_seal: Self::SealingType, payload: T) -> Self;
}

pub trait TagVerifiable: Sized {
    type VerifiedOutput: TagVerified<Self>;
    type Key: MacKey;

    /// Return the payload over which the tag should be verified.
    fn payload(&self) -> &[u8];

    /// Return the mac tag to be verified.
    fn tag(&self) -> &MacTag;

    fn verify(self, key: &Self::Key) -> Result<Self::VerifiedOutput, MacVerificationError> {
        key.verify(self.payload(), self.tag())?;
        Ok(<Self::VerifiedOutput as TagVerified<Self>>::from_payload(
            <Self::VerifiedOutput as TagVerified<Self>>::SealingType::default(),
            self,
        ))
    }
}
