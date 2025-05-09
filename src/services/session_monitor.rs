// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use glib::Object;

glib::wrapper! {
    /// Monitor whether the session is locked or not.
    pub struct SessionMonitor(ObjectSubclass<imp::SessionMonitor>);
}

impl Default for SessionMonitor {
    fn default() -> Self {
        Object::builder().build()
    }
}

mod imp {
    use std::cell::{Cell, RefCell};

    use gio::prelude::*;
    use glib::Variant;
    use glib::subclass::prelude::*;
    use gtk::gio::{self, DBusError, DBusSignalFlags};

    use crate::{
        config::G_LOG_DOMAIN,
        dbus::{SignalSubscription, SignalSubscriptionIdExt},
        services::logind,
    };

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::SessionMonitor)]
    pub struct SessionMonitor {
        #[property(get)]
        locked: Cell<bool>,
        connected_properties_changed: RefCell<Option<SignalSubscription>>,
    }

    impl SessionMonitor {
        fn set_locked(&self, locked: bool) {
            self.locked.set(locked);
            self.obj().notify_locked();
        }

        fn handle_session_properties_changed(&self, params: &Variant) -> Result<(), glib::Error> {
            let params = params
                .try_get::<logind::PropertiesChangedParameters>()
                .map_err(|e| {
                    glib::Error::new(
                        DBusError::InvalidArgs,
                        &format!("Invalid parameters signature for PropertiesChanged: {e}"),
                    )
                })?;

            // We ignore invalidated properties: LockedHint is a simple property
            // which can be directly changed; there's no reason why it would ever
            // just be invalidated.

            if let Some(locked) = params
                .changed_properties
                .get("LockedHint")
                .and_then(Variant::get::<bool>)
            {
                self.set_locked(locked);
            }
            Ok(())
        }

        async fn start(&self) -> Result<(), glib::Error> {
            if self.connected_properties_changed.borrow().is_some() {
                return Ok(());
            }

            let system_bus = gio::bus_get_future(gio::BusType::System).await?;
            let our_session_id =
                logind::get_session_property::<String>(&system_bus, logind::AUTO_SESSION, "Id")
                    .await?;
            glib::debug!("Got session ID {our_session_id}");
            let our_session = logind::get_session_by_id(&system_bus, &our_session_id).await?;
            glib::debug!("Got session {our_session} for session ID {our_session_id}");

            let monitor = self.obj();
            let properties_changed_signal = system_bus
                .signal_subscribe(
                    Some("org.freedesktop.login1"),
                    Some("org.freedesktop.DBus.Properties"),
                    Some("PropertiesChanged"),
                    Some(&our_session),
                    None,
                    DBusSignalFlags::NONE,
                    glib::clone!(
                        #[weak]
                        monitor,
                        move |_connection, _sender, _path, _iface, _signal, params| {
                            if let Err(error) =
                                monitor.imp().handle_session_properties_changed(params)
                            {
                                glib::warn!("Failed to handle changed session properties: {error}");
                            }
                        }
                    ),
                )
                .track_on(&system_bus);
            self.connected_properties_changed
                .replace(Some(properties_changed_signal));

            let is_locked_now =
                logind::get_session_property(&system_bus, &our_session, "LockedHint").await?;
            self.set_locked(is_locked_now);

            Ok(())
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SessionMonitor {
        const NAME: &'static str = "PotDSessionMonitor";

        type Type = super::SessionMonitor;
    }

    #[glib::derived_properties]
    impl ObjectImpl for SessionMonitor {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj().clone();
            glib::spawn_future_local(async move {
                if let Err(error) = obj.imp().start().await {
                    glib::warn!("Failed to subscribe to login session properties: {error}");
                }
            });
        }

        fn dispose(&self) {
            // Take and thus drop our signal subscription
            self.connected_properties_changed.take();
        }
    }
}
