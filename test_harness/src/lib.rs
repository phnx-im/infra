// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use tracing::Level;
use tracing_subscriber::EnvFilter;

pub mod docker;
pub mod test_scenarios;
pub mod utils;

fn init_test_tracing() {
    let _ = tracing_subscriber::fmt::fmt()
        .with_test_writer()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(Level::INFO.into())
                .from_env_lossy(),
        )
        .try_init();
}
