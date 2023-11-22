// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

pub fn main() {
    {
        std::process::Command::new("make")
            .current_dir("dart-bridge")
            .arg("dart-bridge")
            .output()
            .unwrap();
    }
}
