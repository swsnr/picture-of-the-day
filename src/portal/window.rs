// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::fmt::Display;

use futures::StreamExt;
use gdk4_wayland::WaylandToplevel;
use gdk4_x11::{X11Surface, XID};
use glib::Variant;
use glib::object::{Cast, IsA};
use gtk::gio::IOErrorEnum;
use gtk::prelude::*;

use crate::config::G_LOG_DOMAIN;

/// A window handle for use with the portal API.
#[derive(Debug)]
pub enum PortalWindowHandle {
    None,
    Wayland(WaylandToplevel, String),
    X11(XID),
}

impl PortalWindowHandle {
    async fn wayland_identifier(toplevel: &WaylandToplevel) -> Option<Self> {
        let (tx, mut rx) = futures::channel::mpsc::channel(1);
        toplevel.export_handle(move |toplevel, handle| {
            let mut tx = tx.clone();
            match handle {
                Ok(handle) => {
                    glib::debug!("Obtained top-level window wayland handle {handle}");
                    tx.try_send(Ok((toplevel.clone(), handle.to_owned())))
                        .unwrap();
                }
                Err(error) => {
                    glib::warn!("Failed to obtain handle for top-level window.");
                    tx.try_send(Err(glib::Error::new(
                        IOErrorEnum::Failed,
                        &format!("Failed to get top-level handle: {error}"),
                    )))
                    .unwrap();
                }
            }
        });
        let (toplevel, handle) = rx.next().await.unwrap().unwrap();
        Some(Self::Wayland(toplevel, handle))
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

    /// Get the identifier for this handle.
    pub fn identifier(&self) -> PortalWindowIdentifier<'_> {
        PortalWindowIdentifier::Handle(self)
    }
}

impl Drop for PortalWindowHandle {
    /// Drop the window identifier.
    ///
    /// On wayland drop the exported window handle, on other windowing systems
    /// do nothing.
    fn drop(&mut self) {
        match self {
            PortalWindowHandle::None | PortalWindowHandle::X11(_) => {}
            PortalWindowHandle::Wayland(wayland_toplevel, handle) => {
                glib::debug!("Dropping top-level window wayland handle {handle}");
                wayland_toplevel.drop_exported_handle(handle);
            }
        }
    }
}

/// A window identifier for the portal API.
///
/// Window identifiers convert to variants according to the format documented in
/// [Window Identifiers](https://flatpak.github.io/xdg-desktop-portal/docs/window-identifiers.html)
/// and can be used with the portal API.
#[derive(Debug)]
pub enum PortalWindowIdentifier<'a> {
    Handle(&'a PortalWindowHandle),
    Static(String),
}

impl Display for PortalWindowIdentifier<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Static(id) => write!(f, "{id}"),
            Self::Handle(PortalWindowHandle::None) => write!(f, ""),
            Self::Handle(PortalWindowHandle::X11(xid)) => write!(f, "x11:{xid:x}"),
            Self::Handle(PortalWindowHandle::Wayland(_, handle)) => write!(f, "wayland:{handle}"),
        }
    }
}

impl StaticVariantType for PortalWindowIdentifier<'_> {
    fn static_variant_type() -> std::borrow::Cow<'static, glib::VariantTy> {
        String::static_variant_type()
    }
}

impl FromVariant for PortalWindowIdentifier<'_> {
    fn from_variant(variant: &glib::Variant) -> Option<Self> {
        variant.get::<String>().map(Self::Static)
    }
}

impl ToVariant for PortalWindowIdentifier<'_> {
    fn to_variant(&self) -> glib::Variant {
        self.to_string().into()
    }
}

impl From<PortalWindowIdentifier<'_>> for Variant {
    fn from(value: PortalWindowIdentifier<'_>) -> Self {
        value.to_variant()
    }
}
