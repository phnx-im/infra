// SPDX-FileCopyrightText: Copyright (c) The Rust Project Contributors.
//
// SPDX-License-Identifier: MIT OR Apache-2.0
use std::fmt;

/// Backport of the unstable [`std::assert_matches::assert_matches!`] macro.
///
/// For sources see the above macro definition and
/// <https://doc.rust-lang.org/beta/src/core/panicking.rs.html>.
///
/// License: MIT OR Apache-2.0
#[macro_export]
macro_rules! assert_matches {
    ($left:expr, $(|)? $( $pattern:pat_param )|+ $( if $guard: expr )? $(,)?) => {
        match $left {
            $( $pattern )|+ $( if $guard )? => {}
            ref left_val => {
                $crate::assert_matches::assert_matches_failed(
                    left_val,
                    std::stringify!($($pattern)|+ $(if $guard)?),
                    std::option::Option::None
                );
            }
        }
    };

    ($left:expr, $(|)? $( $pattern:pat_param )|+ $( if $guard: expr )?, $($arg:tt)+) => {
        match $left {
            $( $pattern )|+ $( if $guard )? => {}
            ref left_val => {
                $crate::assert_matches::assert_matches_failed(
                    left_val,
                    std::stringify!($($pattern)|+ $(if $guard)?),
                    std::option::Option::Some(std::format_args!($($arg)+))
                );
            }
        }
    };
}

#[derive(Debug)]
#[doc(hidden)]
pub enum AssertKind {
    Eq,
    Ne,
    Match,
}

/// Internal function for `assert_match!`
#[track_caller]
#[doc(hidden)]
pub fn assert_matches_failed<T: fmt::Debug + ?Sized>(
    left: &T,
    right: &str,
    args: Option<fmt::Arguments<'_>>,
) -> ! {
    // The pattern is a string so it can be displayed directly.
    struct Pattern<'a>(&'a str);
    impl fmt::Debug for Pattern<'_> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(self.0)
        }
    }
    assert_failed_inner(AssertKind::Match, &left, &Pattern(right), args);
}

/// Non-generic version of the above functions, to avoid code bloat.
#[track_caller]
fn assert_failed_inner(
    kind: AssertKind,
    left: &dyn fmt::Debug,
    right: &dyn fmt::Debug,
    args: Option<fmt::Arguments<'_>>,
) -> ! {
    let op = match kind {
        AssertKind::Eq => "==",
        AssertKind::Ne => "!=",
        AssertKind::Match => "matches",
    };

    match args {
        Some(args) => panic!(
            r#"assertion `left {op} right` failed: {args}
  left: {left:?}
 right: {right:?}"#
        ),
        None => panic!(
            r#"assertion `left {op} right` failed
  left: {left:?}
 right: {right:?}"#
        ),
    }
}
