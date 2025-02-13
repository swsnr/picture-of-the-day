// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::path::PathBuf;

use glib::{gstr, GStr};

pub static APP_ID: &GStr = gstr!("de.swsnr.picture-of-the-day");

/// The Cargo package verson.
///
/// This provides the full version from `Cargo.toml`.
pub static CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const G_LOG_DOMAIN: &str = "PictureOfTheDay";

/// Whether this is a development/nightly build.
pub fn is_development() -> bool {
    APP_ID.ends_with(".Devel")
}

/// Whether the app is running in flatpak.
pub fn running_in_flatpak() -> bool {
    std::fs::exists("/.flatpak-info").unwrap_or_default()
}

/// Get the locale directory.
///
/// Return the flatpak locale directory when in
pub fn locale_directory() -> PathBuf {
    if running_in_flatpak() {
        "/app/share/locale".into()
    } else {
        "/usr/share/locale".into()
    }
}
