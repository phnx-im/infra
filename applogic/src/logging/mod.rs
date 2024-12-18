// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

pub(crate) mod dart;

use std::sync::Once;

use tracing::level_filters::LevelFilter;
use tracing::warn;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::{SubscriberInitExt, TryInitError};
use tracing_subscriber::{registry, EnvFilter, Layer};

static INIT_LOGGER_ONCE: Once = Once::new();

pub fn init_logger() {
    INIT_LOGGER_ONCE.call_once(|| {
        do_init_logger().expect("failed to init logger");
    });
}

fn do_init_logger() -> Result<(), TryInitError> {
    let default_level = if cfg!(debug_assertions) {
        LevelFilter::INFO
    } else {
        LevelFilter::WARN
    };

    let env_filter = EnvFilter::builder()
        .with_default_directive(default_level.into())
        .from_env_lossy();

    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        registry()
            .with(dart::layer("phnx").with_filter(env_filter))
            .try_init()?;
    }

    #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
    {
        registry()
            .with(tracing_subscriber::fmt::layer().with_filter(env_filter))
            .try_init()?;
    }

    #[cfg(not(any(
        target_os = "android",
        target_os = "ios",
        target_os = "linux",
        target_os = "macos",
        target_os = "windows"
    )))]
    {
        unimplemented!("logging is not supported on this platform");
    }

    warn!(
        %default_level,
        "init_logger finished (deliberately output by warn level)"
    );

    Ok(())
}
