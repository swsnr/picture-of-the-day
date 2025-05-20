// Copyright Sebastian Wiesner <sebastian@swsnr.de>
//
// Licensed under the EUPL
//
// See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

use std::collections::HashMap;

use glib::{Variant, VariantTy, variant::FromVariant};
use gtk::gio::{self, DBusCallFlags, DBusError};

/// Timeout for D-Bus calls to logind, in milliseconds.
///
/// In a GNOME GUI app we can safely assume that logind runs and doesn't have to
/// be activated, and we do not call expensive interfaces, so we can use a very
/// short timeout for logind calls.
const CALL_TIMEOUT_MS: i32 = 500;

/// Object path for the auto-session convenience object.
pub const AUTO_SESSION: &str = "/org/freedesktop/login1/session/auto";

/// Get a property of a logind session.
///
/// Get the value of `property` as type `T` from the session at
/// `session_object_path` from logind on the given `bus`.
pub async fn get_session_property<T: FromVariant>(
    bus: &gio::DBusConnection,
    session_object_path: &str,
    property: &str,
) -> Result<T, glib::Error> {
    let reply = bus
        .call_future(
            Some("org.freedesktop.login1"),
            session_object_path,
            "org.freedesktop.DBus.Properties",
            "Get",
            Some(&("org.freedesktop.login1.Session", property).into()),
            Some(VariantTy::new("(v)").unwrap()),
            DBusCallFlags::NONE,
            CALL_TIMEOUT_MS,
        )
        .await?;
    reply
        .try_get::<(glib::Variant,)>()
        // Type is validated by the `reply_type` arg above, so we can safely unwrap here.
        .unwrap()
        .0
        .try_get::<T>()
        .map_err(|e| {
            glib::Error::new(
                DBusError::InvalidArgs,
                &format!("Unexpected value for session property {property}: {e}"),
            )
        })
}

/// Get the object path of a session by its ID.
///
/// Get the object path of the session with ID `id` from logind on the given
/// `bus`.
pub async fn get_session_by_id(bus: &gio::DBusConnection, id: &str) -> Result<String, glib::Error> {
    let reply = bus
        .call_future(
            Some("org.freedesktop.login1"),
            "/org/freedesktop/login1",
            "org.freedesktop.login1.Manager",
            "GetSession",
            Some(&(id,).into()),
            Some(VariantTy::new("(o)").unwrap()),
            DBusCallFlags::NONE,
            CALL_TIMEOUT_MS,
        )
        .await?;
    // Type is validated by the `reply_type` arg above, so we can safely unwrap here.
    Ok(reply.try_get::<(String,)>().unwrap().0)
}

/// Parameters for the `PropertiesChanged` signal.
///
/// See <https://dbus.freedesktop.org/doc/dbus-specification.html#standard-interfaces-properties>
#[derive(Variant, Debug, Clone)]
pub struct PropertiesChangedParameters {
    /// The name of the interface whose properties changed.
    pub interface_name: String,
    /// Changed properties and their values.
    pub changed_properties: HashMap<String, Variant>,
    /// Changed properties whose value wasn't known at the time of change.
    ///
    /// Clients need to get the value of these properties explicitly.
    pub invalidated_properties: Vec<String>,
}
