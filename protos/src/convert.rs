// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

/// Same as [`std::convert::From`] but takes `self` by reference.
pub trait FromRef<'a, T: ?Sized>: 'a {
    fn from_ref(value: &'a T) -> Self;
}

/// Same as [`std::convert::Into`] but takes `self` by reference.
pub trait RefInto<'a, T: ?Sized + 'a> {
    fn ref_into(&'a self) -> T;
}

impl<'a, U: ?Sized, T: Sized + FromRef<'a, U>> RefInto<'a, T> for U {
    fn ref_into(&'a self) -> T {
        T::from_ref(self)
    }
}

/// Same as [`std::convert::TryFrom`] but takes `self` by reference.
pub trait TryFromRef<'a, T: ?Sized>: Sized + 'a {
    type Error;

    fn try_from_ref(value_ref: &'a T) -> Result<Self, Self::Error>;
}

/// Same as [`std::convert::TryInto`] but takes `self` by reference.
pub trait TryRefInto<'a, T: 'a> {
    type Error;

    fn try_ref_into(&'a self) -> Result<T, Self::Error>;
}

impl<'a, U, T> TryRefInto<'a, T> for U
where
    U: ?Sized,
    T: Sized + TryFromRef<'a, U>,
{
    type Error = T::Error;

    fn try_ref_into(&'a self) -> Result<T, Self::Error> {
        T::try_from_ref(self)
    }
}
