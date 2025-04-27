// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use futures::StreamExt;
use glib::variant::Handle;
use glib::{Priority, VariantTy, WeakRef};
use glib::{Variant, VariantDict, object::IsA};
use gtk::gio::{self, DBusSignalFlags, FileDescriptorBased, SignalSubscriptionId, UnixFDList};
use gtk::gio::{DBusConnection, IOErrorEnum};
use gtk::prelude::*;
use strum::EnumIter;

use crate::config::G_LOG_DOMAIN;

use super::background::{RequestBackground, RequestBackgroundFlags, RequestBackgroundResult};
use super::wallpaper::{Preview, SetOn, SetWallpaperFile};
use super::window::PortalWindowHandle;

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
    /// Get the result of the request.
    pub fn result(&self) -> RequestResult {
        self.0
    }

    /// Get the options returned along with the response.
    pub fn options(&self) -> &VariantDict {
        &self.1
    }
}

pub trait PortalCall: ToVariant {
    /// The interface to make this portal call on.
    const INTERFACE: &'static str;

    /// The method name of this portal call.
    const METHOD_NAME: &'static str;

    /// Get a mutable reference to the options of this request.
    ///
    /// The portal client inserts the request token into options using this reference.
    fn options_mut(&mut self) -> &mut VariantDict;
}

fn request_object_path(connection: &DBusConnection, handle_token: &str) -> String {
    let sender = connection
        .unique_name()
        .unwrap()
        .trim_start_matches(':')
        .replace('.', "_");
    format!("/org/freedesktop/portal/desktop/request/{sender}/{handle_token}")
}

/// A subscription to a D-Bus signal.
///
/// When dropped unsubscribe from the signal.
#[derive(Debug)]
struct SignalSubscription(WeakRef<gio::DBusConnection>, Option<SignalSubscriptionId>);

impl Drop for SignalSubscription {
    fn drop(&mut self) {
        if let Some(connection) = self.0.upgrade() {
            if let Some(id) = self.1.take() {
                glib::debug!("Dropping signal connection {id:?}");
                connection.signal_unsubscribe(id);
            }
        }
    }
}

#[derive(Debug)]
pub struct PortalRequest {
    /// The handle token of this request.
    handle_token: String,
    /// The signal connection for the response signal.
    /// The connection the request runs on.
    connection: DBusConnection,
    /// A receiver for the response to this request.
    rx: futures::channel::mpsc::Receiver<Result<PortalResponse, glib::Error>>,
    /// The signal connection to receive the response from.
    response_signal_subscription: SignalSubscription,
}

impl PortalRequest {
    /// Subscribe to a fresh portal request on the given `connection`.
    ///
    /// Create a fresh [handle token][1] and connect to the [`Response` signal][2] of the
    /// request.
    ///
    /// [1]: https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.Request.html#description
    /// [2]: https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.Request.html#org-freedesktop-portal-request-response
    fn subscribe(connection: &DBusConnection) -> Self {
        let handle_token = format!("potd_{}", glib::random_int());
        // We buffer only a single element, since the portal must only trigger
        // the response signal once.
        let (tx, rx) = futures::channel::mpsc::channel(1);
        let signal_subscription_id = connection.signal_subscribe(
            Some("org.freedesktop.portal.Desktop"),
            Some("org.freedesktop.portal.Request"),
            Some("Response"),
            Some(&request_object_path(connection, &handle_token)),
            None,
            DBusSignalFlags::NO_MATCH_RULE,
            move |_connection, _sender, _path, _interface, _signal, parameters| {
                let mut tx = tx.clone();
                let result = parameters.get::<PortalResponse>().ok_or_else(|| {
                    glib::Error::new(
                        IOErrorEnum::InvalidData,
                        &format!("Unexpected parameters received: {parameters:?}"),
                    )
                });
                if let Err(error) = tx.try_send(result) {
                    glib::warn!("Channel for response already closed? {error}");
                }
            },
        );
        Self {
            handle_token,
            connection: connection.clone(),
            rx,
            response_signal_subscription: SignalSubscription(
                connection.downgrade(),
                Some(signal_subscription_id),
            ),
        }
    }

    /// Get the object path for this request.
    ///
    /// The object path is derived from the sender identity of the connection and
    /// the handle token of this request.
    ///
    /// See [Shared Request Interface][1] for details.
    ///
    /// [1]: https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.Request.html#description
    pub fn object_path(&self) -> String {
        request_object_path(&self.connection, &self.handle_token)
    }

    pub async fn receive_response(mut self) -> Result<PortalResponse, glib::Error> {
        let path = self.object_path();
        let result = self.rx.select_next_some().await;
        // Explicitly keep the signal subscription until after we've received the response
        drop(self.response_signal_subscription);
        result
            .inspect(|response| {
                glib::debug!(
                    "Received response result {:?} for request {path}",
                    response.result(),
                );
            })
            .inspect_err(|error| {
                glib::warn!("Failed to receive for request {path}: {error}",);
            })
    }
}

#[derive(Debug, Clone, Variant)]
struct PortalReturnValue(String);

impl PortalReturnValue {
    fn request_object_path(&self) -> &str {
        &self.0
    }
}

/// A client for the portal API on top of a Gio D-Bus connection.
///
/// Cloning this client simply increments the ref-count of the inner Gio D-Bus
/// connection.
#[derive(Debug, Clone, glib::Boxed)]
#[boxed_type(name = "PotDPortalClient", nullable)]
pub struct PortalClient {
    connection: DBusConnection,
}

impl PortalClient {
    /// Create a portal client on an existing D-Bus `connection`./
    pub fn new(connection: &DBusConnection) -> Self {
        Self {
            connection: connection.clone(),
        }
    }

    async fn invoke_with_unix_fd_list<C: PortalCall>(
        &self,
        mut call: C,
        fd_list: Option<&(impl IsA<UnixFDList> + Clone + 'static)>,
    ) -> Result<PortalRequest, glib::Error> {
        // Subscribe to the request first
        let request = PortalRequest::subscribe(&self.connection);

        // Add the request handle token to the portal call.
        call.options_mut()
            .insert("handle_token", &request.handle_token);

        glib::debug!(
            "Calling {}.{}, with request {}",
            C::INTERFACE,
            C::METHOD_NAME,
            request.object_path()
        );
        let (return_value, _) = self
            .connection
            .call_with_unix_fd_list_future(
                Some("org.freedesktop.portal.Desktop"),
                "/org/freedesktop/portal/desktop",
                C::INTERFACE,
                C::METHOD_NAME,
                Some(&call.to_variant()),
                Some(VariantTy::new("(o)").unwrap()),
                gio::DBusCallFlags::NONE,
                -1,
                fd_list,
            )
            .await?;

        let return_value = return_value.get::<PortalReturnValue>().ok_or_else(|| {
            glib::Error::new(
                IOErrorEnum::InvalidData,
                &format!("Unexpected return value: {return_value:?}"),
            )
        })?;
        glib::debug!(
            "Received return value {return_value:?} from request {}",
            request.object_path()
        );

        // Assert that we're listening on the correct response path!
        assert_eq!(return_value.request_object_path(), request.object_path());

        Ok(request)
    }

    pub async fn set_wallpaper(
        &self,
        file: &gio::File,
        window: &PortalWindowHandle,
        show_preview: Preview,
        set_on: SetOn,
    ) -> Result<RequestResult, glib::Error> {
        let fd = file
            .read_future(Priority::DEFAULT)
            .await?
            .dynamic_cast::<FileDescriptorBased>()
            .map_err(|_| {
                glib::Error::new(
                    IOErrorEnum::Failed,
                    &format!(
                        "Failed to obtain file descriptor for {}",
                        file.path().unwrap().display()
                    ),
                )
            })?;
        let fdlist = UnixFDList::new();
        let call = SetWallpaperFile::new(
            window.identifier(),
            Handle(fdlist.append(fd)?),
            show_preview,
            set_on,
        );
        let result = self
            .invoke_with_unix_fd_list(call, Some(&fdlist))
            .await?
            .receive_response()
            .await?
            .result();
        Ok(result)
    }

    pub async fn request_background(
        &self,
        window: &PortalWindowHandle,
        reason: &str,
        command_line: Option<&[&str]>,
        flags: RequestBackgroundFlags,
    ) -> Result<RequestBackgroundResult, glib::Error> {
        let call = RequestBackground::new(window.identifier(), reason, command_line, flags);
        let response = self
            .invoke_with_unix_fd_list(call, UnixFDList::NONE)
            .await?
            .receive_response()
            .await?;
        Ok(response.into())
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
