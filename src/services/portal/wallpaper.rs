// Copyright Sebastian Wiesner <sebastian@swsnr.de>
//
// Licensed under the EUPL
//
// See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

use std::collections::HashMap;

use glib::Variant;
use glib::variant::{Handle, ToVariant};

use super::client::PortalCall;
use super::window::PortalWindowIdentifier;

/// Where to set the wallpaper.
#[derive(Debug, Copy, Clone, strum::IntoStaticStr)]
#[strum(serialize_all = "kebab-case")]
pub enum SetOn {
    /// Set the wallpaper on the regular desktop background.
    Background,
    /// Set the wallpaper on the publicly visible lockscreen.
    Lockscreen,
    /// Set the wallpaper as background as well as on the lockscreen.
    Both,
}

/// Whether to show a preview for the wallpaper or not.
#[derive(Debug, Copy, Clone)]
pub enum Preview {
    /// Show a preview window.
    ///
    /// If a preview is shown the portal does not ask for permission separately.
    Preview,
    #[allow(dead_code)]
    /// Do not show a preview.
    ///
    /// If the app lacks permission to set the wallpaper the portal asks the
    /// user to grant the app this permission.
    NoPreview,
}

#[derive(Variant)]
pub struct SetWallpaperFile<'a>(PortalWindowIdentifier<'a>, Handle, HashMap<String, Variant>);

impl<'a> SetWallpaperFile<'a> {
    /// Create a request to set the wallpaper to a file.
    ///
    /// - `window` denotes the parent window to show portal dialogs on.
    /// - `file` points to the file to use as new wallpaper.
    /// - `show_preview` determines whether the portal should show a preview for
    ///   the wallpaper.
    /// - `set_on` tells the portal where to set the wallpaper.
    ///
    /// See [`org.freedesktop.portal.Wallpaper.SetWallpaperFile`][1].
    ///
    /// [1]: https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.Wallpaper.html#org-freedesktop-portal-wallpaper-setwallpaperfile
    pub fn new(
        window: PortalWindowIdentifier<'a>,
        file: Handle,
        show_preview: Preview,
        set_on: SetOn,
    ) -> Self {
        let set_on: &'static str = set_on.into();
        let options = HashMap::from([
            (
                "show-preview".to_string(),
                matches!(show_preview, Preview::Preview).to_variant(),
            ),
            ("set-on".to_string(), set_on.to_variant()),
        ]);
        Self(window, file, options)
    }
}

impl PortalCall for SetWallpaperFile<'_> {
    const INTERFACE: &'static str = "org.freedesktop.portal.Wallpaper";

    const METHOD_NAME: &'static str = "SetWallpaperFile";

    fn with_option(mut self, key: &str, value: Variant) -> Self {
        self.2.insert(key.to_string(), value);
        self
    }
}
