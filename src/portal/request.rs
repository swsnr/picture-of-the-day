// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use futures::StreamExt;
use gdk4_wayland::WaylandToplevel;
use gdk4_x11::{X11Surface, XID};
use glib::{Variant, VariantDict, object::IsA};
use gtk::gio::IOErrorEnum;
use gtk::gio::{self, DBusSignalFlags, SignalSubscriptionId};
use gtk::prelude::*;
use strum::EnumIter;

use crate::config::G_LOG_DOMAIN;

/// The result of a portal request.
///
/// See <https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.Request.html#org-freedesktop-portal-request-response>
#[derive(Debug, Copy, Clone, Eq, PartialEq, Variant, EnumIter)]
#[repr(u32)]
#[variant_enum(repr)]
pub enum RequestResult {
    ///Success, the request is carried out
    Success = 0,
    /// The user cancelled the interaction
    Cancelled = 1,
    /// The user interaction was ended in some other way
    Ended = 2,
}

/// A response to a portal request.
///
/// See <https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.Request.html#org-freedesktop-portal-request-response>
#[derive(Clone, glib::Variant)]
pub struct PortalResponse(RequestResult, VariantDict);

impl PortalResponse {
    /// Get the result of this request.
    pub fn result(&self) -> RequestResult {
        self.0
    }
}

/// A request handle token.
///
/// The portal API requires that clients pass a `handle_token` along with the
/// invocation; the portal API then exports the request object under a
/// predictable object name, to connect to the request before the portal exports
/// it.  This prevents race conditions, should the portal be faster than the
/// client.
///
/// See [Requests][1] for the overall request/response flow of portal APIs, and
/// [Request][1] for the actual request object and the use of `handle_token`s
/// to predict object paths of request objects.
///
/// [1]: https://flatpak.github.io/xdg-desktop-portal/docs/requests.html
/// [2]: https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.Request.html#org-freedesktop-portal-request
#[derive(Debug, Clone)]
pub struct HandleToken(String);

impl HandleToken {
    pub fn new() -> Self {
        Self(format!("potd_{}", glib::random_int()))
    }

    /// Create a request options dictionary containing this handle token.
    pub fn create_options(&self) -> glib::VariantDict {
        let options = VariantDict::new(None);
        options.insert("handle_token", &self.0);
        options
    }

    /// Get the object path for the request corresponding to this token on `connection`.
    ///
    /// The object path is derived from the sender identity of `connection`, and this token.
    /// See [Shared Request Interface][1] for details.
    ///
    /// [1]: https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.Request.html#description
    pub fn request_object_path(&self, connection: &gio::DBusConnection) -> String {
        let sender = connection
            .unique_name()
            .unwrap()
            .trim_start_matches(':')
            .replace('.', "_");
        format!(
            "/org/freedesktop/portal/desktop/request/{sender}/{}",
            self.0
        )
    }

    /// Wait for a response to the request corresponding to this token on `connection`.
    ///
    /// Connect to the [`Response`][1] signal of the request object corresponding to this token
    /// on `connection`.
    ///
    /// If the signal is received, return the result.
    ///
    /// When the future is dropped before, disconnect from the signal but do not
    /// explicitly close the request.
    ///
    /// [1]: https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.Request.html#org-freedesktop-portal-request-response
    pub async fn wait_for_response(
        &self,
        connection: &gio::DBusConnection,
    ) -> Result<PortalResponse, glib::Error> {
        let (tx, mut rx) = futures::channel::mpsc::unbounded();
        let id = connection.signal_subscribe(
            Some("org.freedesktop.portal.Desktop"),
            Some("org.freedesktop.portal.Request"),
            Some("Response"),
            Some(&self.request_object_path(connection)),
            None,
            DBusSignalFlags::NO_MATCH_RULE,
            move |_connection, _sender, _path, _interface, _signal, parameters| {
                let result = parameters.get::<PortalResponse>().ok_or_else(|| {
                    glib::Error::new(
                        IOErrorEnum::InvalidData,
                        &format!("Unexpected parameters received: {parameters:?}"),
                    )
                });
                if let Err(error) = tx.unbounded_send(result) {
                    glib::warn!("Channel for response already closed? {error}");
                }
            },
        );
        let signal = ConnectedSignal(connection.clone(), Some(id));
        let result = rx.next().await.unwrap()?;
        // Disconnect from the signal after we received the response; do this
        // explicitly to make the lifetime of the signal connection abundantly clear here.
        drop(signal);
        Ok(result)
    }
}

/// A connected DBUS signal.
///
/// When dropped unsubscribe from the signal.
struct ConnectedSignal(gio::DBusConnection, Option<SignalSubscriptionId>);

impl Drop for ConnectedSignal {
    fn drop(&mut self) {
        if let Some(id) = self.1.take() {
            self.0.signal_unsubscribe(id);
        }
    }
}

/// A window identifier for the portal API.
///
/// Window identifiers convert to variants according to the format documented in
/// [Window Identifiers](https://flatpak.github.io/xdg-desktop-portal/docs/window-identifiers.html)
/// and can be used with the portal API.
#[derive(Debug)]
pub enum WindowIdentifier {
    None,
    Wayland(WaylandToplevel, String),
    X11(XID),
}

impl ToVariant for WindowIdentifier {
    fn to_variant(&self) -> glib::Variant {
        match self {
            WindowIdentifier::None => "".into(),
            WindowIdentifier::X11(xid) => format!("x11:{xid:x}").into(),
            WindowIdentifier::Wayland(_, handle) => format!("wayland:{handle}").into(),
        }
    }
}

impl StaticVariantType for WindowIdentifier {
    fn static_variant_type() -> std::borrow::Cow<'static, glib::VariantTy> {
        str::static_variant_type()
    }
}

impl From<WindowIdentifier> for Variant {
    fn from(value: WindowIdentifier) -> Self {
        value.to_variant()
    }
}

impl Drop for WindowIdentifier {
    /// Drop the window identifier.
    ///
    /// On wayland drop the exported window handle, on other windowing systems
    /// do nothing.
    fn drop(&mut self) {
        match self {
            WindowIdentifier::None | WindowIdentifier::X11(_) => {}
            WindowIdentifier::Wayland(wayland_toplevel, handle) => {
                glib::debug!("Dropping top-level window wayland handle {handle}");
                wayland_toplevel.drop_exported_handle(handle);
            }
        }
    }
}

impl WindowIdentifier {
    async fn wayland_identifier(toplevel: &WaylandToplevel) -> Option<Self> {
        let (tx, mut rx) = futures::channel::mpsc::unbounded();
        toplevel.export_handle(move |toplevel, handle| match handle {
            Ok(handle) => {
                glib::debug!("Obtained top-level window wayland handle {handle}");
                tx.unbounded_send(Ok((toplevel.clone(), handle.to_owned())))
                    .unwrap();
            }
            Err(error) => {
                glib::warn!("Failed to obtain handle for top-level window.");
                tx.unbounded_send(Err(glib::Error::new(
                    IOErrorEnum::Failed,
                    &format!("Failed to get top-level handle: {error}"),
                )))
                .unwrap();
            }
        });
        let (toplevel, handle) = rx.next().await.unwrap().unwrap();
        Some(WindowIdentifier::Wayland(toplevel, handle))
    }

    /// Get a window identifier for a native window.
    ///
    /// On wayland export a window handle for the given `toplevel` and return
    /// the handle.  If the return value is dropped the handle is dropped from
    /// the window.
    ///
    /// On X11 return the XID.
    ///
    /// In case of error or on other windowing systems return [`Self::None`].
    pub async fn new_for_window(window: &impl IsA<gtk::Native>) -> Self {
        if let Some(surface) = window.as_ref().surface() {
            if let Some(toplevel) = surface.downcast_ref::<WaylandToplevel>() {
                Self::wayland_identifier(toplevel)
                    .await
                    .unwrap_or(Self::None)
            } else if let Some(toplevel) = surface.downcast_ref::<X11Surface>() {
                Self::X11(toplevel.xid())
            } else {
                Self::None
            }
        } else {
            Self::None
        }
    }
}

#[cfg(test)]
mod tests {
    use glib::VariantTy;
    use strum::IntoEnumIterator;

    use super::*;

    #[test]
    #[allow(clippy::as_conversions)]
    fn request_result_variant() {
        for result in RequestResult::iter() {
            let variant = result.to_variant();
            assert_eq!(variant.type_(), VariantTy::UINT32);
            assert_eq!(variant.get::<u32>().unwrap(), result as u32);
            assert_eq!(variant.get::<RequestResult>().unwrap(), result);
        }
    }
}
