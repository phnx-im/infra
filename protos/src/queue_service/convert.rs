// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::identifiers;
use uuid::Uuid;

use crate::validation::{MissingFieldError, MissingFieldExt};

use super::v1::QsClientId;

impl TryFrom<QsClientId> for identifiers::QsClientId {
    type Error = MissingFieldError<&'static str>;

    fn try_from(proto: QsClientId) -> Result<Self, Self::Error> {
        Ok(Self::from(Uuid::from(
            proto.value.ok_or_missing_field("uuid")?,
        )))
    }
}
