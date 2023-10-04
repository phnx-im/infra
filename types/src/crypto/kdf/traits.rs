// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! This module contains traits that facilitate the derivation of key material
//! from other key material, as well as traits that allow the expansion of key
//! material and other values into new key material.

use tracing::instrument;

use crate::{crypto::secrets::Secret, LibraryError};

use super::{Kdf, KDF_KEY_SIZE};

/// A trait that allows the use of a symmetric secret of size [`KDF_KEY_SIZE`]
/// to derive additional key material.
pub(crate) trait KdfKey: AsRef<Secret<KDF_KEY_SIZE>> + From<Secret<KDF_KEY_SIZE>> {
    /// Label used as additional input in all derivations made with this KDF key.
    const ADDITIONAL_LABEL: &'static str;

    /// Derive a secret of the given length from the KdfKey using the given info
    /// as context. Returns [`InvalidLength`] if the given length is an invalid
    /// output length for the KDF.
    fn derive<const LENGTH: usize>(&self, info: &[u8]) -> Result<Secret<LENGTH>, LibraryError> {
        let kdf = Kdf::from_prk(self.as_ref().secret()).map_err(|_| LibraryError)?;
        let kdf_info = [Self::ADDITIONAL_LABEL.as_bytes(), info].concat();
        let mut output = [0u8; LENGTH];
        let _ = kdf.expand(info, &mut output);
        Ok(Secret { secret: output })
    }
}

/// A trait meant for all keys that can be derived from KDF keys of type
/// `DerivingKey` and the given length `OUTPUT_LENGTH`. Upon derivation, the
/// structs of type `AdditionalInfo` can be provided as context. [`Self::LABEL`]
/// is used as label in the derivation.
pub(crate) trait KdfDerivable<
    DerivingKey: KdfKey + std::fmt::Debug,
    AdditionalInfo: tls_codec::Serialize + std::fmt::Debug,
    const OUTPUT_LENGTH: usize,
>: From<Secret<OUTPUT_LENGTH>> + std::fmt::Debug
{
    /// This label is appended to the info given in the derivation.
    const LABEL: &'static str;

    #[instrument(level = "trace", ret, skip_all, fields(
        kdf_key_type = std::any::type_name::<Self>(),
        label = %Self::LABEL,
        additional_info = ?additional_info,
    ))]
    fn derive(
        kdf_key: &DerivingKey,
        additional_info: AdditionalInfo,
    ) -> Result<Self, LibraryError> {
        let info = [
            &additional_info
                .tls_serialize_detached()
                .map_err(|_| LibraryError)?,
            Self::LABEL.as_bytes(),
        ]
        .concat();
        let secret = kdf_key.derive::<OUTPUT_LENGTH>(&info);
        secret.map(|res| res.into())
    }
}

/// A trait that allows the extraction of the struct from the two given input
/// key types. The output length is fixed to [`KDF_KEY_SIZE`].
pub(crate) trait KdfExtractable<
    FirstInput: AsRef<Secret<KDF_KEY_SIZE>> + std::fmt::Debug,
    SecondInput: AsRef<Secret<KDF_KEY_SIZE>> + std::fmt::Debug,
>: From<Secret<KDF_KEY_SIZE>> + std::fmt::Debug
{
    #[instrument(level = "trace", ret, skip_all, fields(
        first_input_type = std::any::type_name::<FirstInput>(),
        second_input_type = std::any::type_name::<SecondInput>(),
    ))]
    fn extract(input_1: &FirstInput, input_2: &SecondInput) -> Self {
        let (output, _) = Kdf::extract(Some(input_1.as_ref().secret()), input_2.as_ref().secret());
        Secret {
            secret: output.into(),
        }
        .into()
    }
}
