// Copyright Sebastian Wiesner <sebastian@swsnr.de>
//
// Licensed under the EUPL
//
// See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

//! D-Bus helpers.

use glib::{WeakRef, clone::Downgrade};
use gtk::gio::{self, SignalSubscriptionId};

/// A subscription to a D-Bus signal.
///
/// When dropped unsubscribe from the signal.
#[derive(Debug)]
pub struct SignalSubscription {
    bus: WeakRef<gio::DBusConnection>,
    id: Option<SignalSubscriptionId>,
}

impl SignalSubscription {
    fn new(bus: &gio::DBusConnection, id: SignalSubscriptionId) -> Self {
        Self {
            bus: bus.downgrade(),
            id: Some(id),
        }
    }
}

impl Drop for SignalSubscription {
    fn drop(&mut self) {
        if let Some(connection) = self.bus.upgrade() {
            if let Some(id) = self.id.take() {
                connection.signal_unsubscribe(id);
            }
        }
    }
}

pub trait SignalSubscriptionIdExt {
    /// Track this signal subscription ID on the given `bus`.
    fn track_on(self, bus: &gio::DBusConnection) -> SignalSubscription;
}

impl SignalSubscriptionIdExt for SignalSubscriptionId {
    fn track_on(self, bus: &gio::DBusConnection) -> SignalSubscription {
        SignalSubscription::new(bus, self)
    }
}
