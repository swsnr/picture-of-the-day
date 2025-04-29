// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

glib::wrapper! {
    pub struct AppUpdatedMonitor(ObjectSubclass<imp::AppUpdatedMonitor>);
}

impl Default for AppUpdatedMonitor {
    fn default() -> Self {
        glib::Object::builder().build()
    }
}

mod imp {
    use std::cell::{Cell, RefCell};

    use gio::prelude::*;
    use gio::subclass::prelude::*;
    use gtk::gio::{self, Cancellable, FileMonitorEvent};

    use crate::config::G_LOG_DOMAIN;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::AppUpdatedMonitor)]
    pub struct AppUpdatedMonitor {
        /// Whether the app was updated.
        #[property(get)]
        updated: Cell<bool>,
        monitor: RefCell<Option<gio::FileMonitor>>,
    }

    impl AppUpdatedMonitor {
        fn set_updated(&self, updated: bool) {
            if updated != self.updated.get() {
                self.updated.replace(updated);
                self.obj().notify_updated();
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AppUpdatedMonitor {
        const NAME: &'static str = "PotDAppUpdatedMonitor";

        type Type = super::AppUpdatedMonitor;
    }

    #[glib::derived_properties]
    impl ObjectImpl for AppUpdatedMonitor {
        fn constructed(&self) {
            self.parent_constructed();

            if crate::config::running_in_flatpak() {
                let updated = gio::File::for_path("/app/.updated");
                match updated.monitor(gio::FileMonitorFlags::NONE, Cancellable::NONE) {
                    Ok(monitor) => {
                        self.monitor.replace(Some(monitor.clone()));
                        monitor.connect_changed(glib::clone!(
                            #[weak(rename_to = monitor)]
                            self.obj(),
                            move |_, _, _, event| {
                                match event {
                                    FileMonitorEvent::Deleted => {
                                        monitor.imp().set_updated(false);
                                    }
                                    FileMonitorEvent::Created => {
                                        monitor.imp().set_updated(true);
                                    }
                                    _ => {}
                                }
                            }
                        ));
                    }
                    Err(error) => {
                        glib::warn!("Failed to monitor /app/.updated: {error}");
                    }
                }
            }
        }

        fn dispose(&self) {
            self.monitor.take();
        }
    }
}
