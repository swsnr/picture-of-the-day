// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::path::PathBuf;

use glib::{GStr, gstr};
use gtk::gio;

pub static APP_ID: &GStr = gstr!("de.swsnr.pictureoftheday");

/// The Cargo package verson.
///
/// This provides the full version from `Cargo.toml`.
pub static CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Get [`CARGO_PKG_VERSION`] parsed.
fn cargo_pkg_version() -> semver::Version {
    semver::Version::parse(CARGO_PKG_VERSION).unwrap()
}
/// The version to use for release notes.
///
/// Returns [`CARGO_PKG_VERSION`] but with patch set to 0, and all pre and
/// build parts emptied.
///
/// This follows our versioning policy which uses patch releases only for
/// translation updates.
pub fn release_notes_version() -> semver::Version {
    let mut version = cargo_pkg_version();
    version.patch = 0;
    version.pre = semver::Prerelease::EMPTY;
    version.build = semver::BuildMetadata::EMPTY;
    version
}

/// Log domain for the entire application.
pub const G_LOG_DOMAIN: &str = "PictureOfTheDay";

/// User Agent to use for all HTTP traffic.
pub static USER_AGENT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("CARGO_PKG_HOMEPAGE"),
    ")"
);

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
pub fn locale_directory() -> &'static GStr {
    if running_in_flatpak() {
        gstr!("/app/share/locale")
    } else {
        gstr!("/usr/share/locale")
    }
}

/// Get a schema source for this application.
///
/// In a debug build load compiled schemas from the manifest directory, to allow
/// running the application uninstalled.
///
/// In a release build only use the default schema source.
pub fn schema_source() -> gio::SettingsSchemaSource {
    let default = gio::SettingsSchemaSource::default().unwrap();
    if cfg!(debug_assertions) {
        let directory = concat!(env!("CARGO_MANIFEST_DIR"), "/schemas");
        if std::fs::exists(directory).unwrap_or_default() {
            gio::SettingsSchemaSource::from_directory(directory, Some(&default), false).unwrap()
        } else {
            default
        }
    } else {
        default
    }
}

/// Get settings for this application.
pub fn get_settings() -> gio::Settings {
    gio::Settings::new_full(
        &crate::config::schema_source()
            .lookup(crate::config::APP_ID, true)
            .unwrap(),
        gio::SettingsBackend::NONE,
        None,
    )
}

pub fn images_directory() -> PathBuf {
    glib::user_data_dir()
        .join(crate::config::APP_ID)
        .join("images")
}

#[cfg(test)]
mod tests {

    #[test]
    fn release_notes_version_only_has_major_and_minor() {
        let version = super::release_notes_version();
        assert_eq!(version.major, super::cargo_pkg_version().major);
        assert_eq!(version.minor, super::cargo_pkg_version().minor);
        assert_eq!(version.patch, 0);
        assert!(version.pre.is_empty());
        assert!(version.build.is_empty());
    }
    #[test]
    fn release_notes_for_release_notes_version() {
        let metadata = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/resources/de.swsnr.pictureoftheday.metainfo.xml.in"
        ))
        .unwrap();
        assert!(metadata.contains(&format!(
            "<release version=\"{}\"",
            super::release_notes_version()
        )));
    }

    #[test]
    fn no_release_notes_for_cargo_pkg_version() {
        let version = super::cargo_pkg_version();
        if version != super::release_notes_version() {
            let metadata = std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/resources/de.swsnr.pictureoftheday.metainfo.xml.in"
            ))
            .unwrap();
            assert!(!metadata.contains(&format!("version=\"{version}\"")));
        }
    }
}
