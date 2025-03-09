// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::os::fd::AsFd;
use std::rc::Rc;

use glib::VariantTy;
use glib::object::IsA;
use glib::variant::Handle;
use gtk::gio::prelude::*;
use gtk::gio::{self, IOErrorEnum, UnixFDList};

use crate::portal::request::{HandleToken, WindowIdentifier};

use super::request::RequestResult;

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
    Preview,
    #[allow(dead_code)]
    NoPreview,
}

/// Set the current wallaper to a file.
///
/// Talk to the wallpaper portal on `connection`, and set the given `file` as
/// wallpaper.
///
/// `window` is the parent window to use for dialogs the portal needs to show
/// in order to process the request, such as the preview window.
///
/// `preview` denotes whether or not to show a preview, and `set_on` determines
/// where to set the wallpaper.
pub async fn set_wallpaper_file<F: AsFd>(
    connection: &gio::DBusConnection,
    window: Option<&impl IsA<gtk::Window>>,
    file: F,
    show_preview: Preview,
    set_on: SetOn,
) -> Result<RequestResult, glib::Error> {
    let token = Rc::new(HandleToken::new());
    let set_on: &'static str = set_on.into();
    let options = token.create_options();
    options.insert("show-preview", matches!(show_preview, Preview::Preview));
    options.insert("set-on", set_on);

    let window_identifier = if let Some(window) = window {
        WindowIdentifier::new_for_window(window.as_ref()).await
    } else {
        WindowIdentifier::None
    };
    let fdlist = UnixFDList::new();
    let args = (
        &window_identifier,
        Handle(fdlist.append(file.as_fd())?),
        options,
    )
        .to_variant();

    let result = glib::spawn_future_local(glib::clone!(
        #[strong]
        token,
        #[strong]
        connection,
        async move { token.wait_for_response(&connection).await }
    ));
    let (return_value, _) = connection
        .call_with_unix_fd_list_future(
            Some("org.freedesktop.portal.Desktop"),
            "/org/freedesktop/portal/desktop",
            "org.freedesktop.portal.Wallpaper",
            "SetWallpaperFile",
            Some(&args),
            Some(VariantTy::new("(o)").unwrap()),
            gio::DBusCallFlags::NONE,
            -1,
            Some(&fdlist),
        )
        .await?;
    let path = return_value.get::<(String,)>().ok_or_else(|| {
        glib::Error::new(
            IOErrorEnum::InvalidData,
            &format!("Unexpected return type: {}", return_value.value_type()),
        )
    })?;
    // Assert that we're listening on the correct response path!
    assert_eq!(path.0, token.request_object_path(connection));

    let request_result = result.await.unwrap()?.result();

    // Make sure we keep the window identifier alive until the call's finished.
    drop(window_identifier);

    Ok(request_result)
}
