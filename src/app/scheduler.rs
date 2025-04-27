// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Schedule automatic wallpaper updates.

use std::fmt::Display;

use futures::channel::oneshot;
use glib::subclass::types::ObjectSubclassIsExt;
use gtk::gio::{self, NetworkConnectivity};

use crate::config::G_LOG_DOMAIN;
use crate::images::{Source, SourceError};

#[glib::flags(name = "PotDAutomaticWallpaperUpdateInhibitor")]
pub enum AutomaticWallpaperUpdateInhibitor {
    /// The user explicitly disabled automatic wallpaper updates in configuration.
    DisabledByUser = 0b0000_0001,
    /// The main window is shown.
    ///
    /// While a main window is active we do not schedule automatic updates,
    /// assuming that the user wishes to preview different sources before making
    /// their final decision on the preferred wallpaper.
    MainWindowActive = 0b0000_0010,
    /// The system is in low power mode.
    LowPower = 0b0000_0100,
    /// The system has no network connectivity.
    NoNetwork = 0b0000_1000,
    /// The desktop session is locked.
    SessionLocked = 0b0001_0000,
}

impl Display for AutomaticWallpaperUpdateInhibitor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        glib::bitflags::parser::to_writer(self, f)
    }
}

/// A message indicating that a scheduled wallpaper update is due.
#[derive(Debug)]
pub struct ScheduledWallpaperUpdate {
    /// The source to update the wallpaper from.
    pub source: Source,
    /// A cancellable indicating when automatic updates are inhibited.
    pub cancellable: gio::Cancellable,
    /// A channel to notify the scheduler about the result of the update.
    pub response: oneshot::Sender<Result<(), SourceError>>,
}

impl ScheduledWallpaperUpdate {
    fn for_source(
        source: Source,
        cancellable: gio::Cancellable,
    ) -> (Self, oneshot::Receiver<Result<(), SourceError>>) {
        let (response, rx) = oneshot::channel();
        let update = Self {
            source,
            cancellable,
            response,
        };
        (update, rx)
    }
}

glib::wrapper! {
    pub struct AutomaticWallpaperUpdateScheduler(ObjectSubclass<imp::AutomaticWallpaperUpdateScheduler>);
}

impl Default for AutomaticWallpaperUpdateScheduler {
    fn default() -> Self {
        glib::Object::builder().build()
    }
}

impl AutomaticWallpaperUpdateScheduler {
    /// Get a channel receiver to be notified scheduled updates.
    pub fn update_receiver(&self) -> async_channel::Receiver<ScheduledWallpaperUpdate> {
        self.imp().update_rx.clone()
    }

    /// Set or clear an inhibitor.
    ///
    /// If `set` is `true` add `inhibitor`, otherwise clear it.
    pub fn set_inhibitor(&self, inhibitor: AutomaticWallpaperUpdateInhibitor, set: bool) {
        if set {
            self.add_inhibitor(inhibitor);
        } else {
            self.clear_inhibitor(inhibitor);
        }
    }

    /// Add an inhibitor to this scheduler.
    ///
    /// This disables scheduled updates until all inhibitors are cleared again.
    pub fn add_inhibitor(&self, inhibitor: AutomaticWallpaperUpdateInhibitor) {
        self.imp().add_inhibitor(inhibitor);
    }

    /// Clear the given `inhibitor` on this scheduler.
    ///
    /// If it was the last inhibitor scheduled updates will commence again.
    pub fn clear_inhibitor(&self, inhibitor: AutomaticWallpaperUpdateInhibitor) {
        self.imp().clear_inhibitor(inhibitor);
    }

    /// Inhibit automatic wallpaper updates depending on network connectivity.
    ///
    /// If `connectivity` is limited or full, clear the [`AutomaticWallpaperUpdateInhibitor::NoNetwork`]
    /// inhibitor, otherwise add it.
    pub fn inhibit_according_to_network_connectivity(&self, connectivity: NetworkConnectivity) {
        use gio::NetworkConnectivity::*;
        self.set_inhibitor(
            AutomaticWallpaperUpdateInhibitor::NoNetwork,
            match connectivity {
                // We do not inhibit on "limited" connectivity, because
                // that just might be a badly configured proxy or
                // captive portal, where we still might have success
                // in updating the wallpaper.
                Limited | Full => false,
                other => {
                    glib::info!(
                        "Inibiting automatic wallpaper updates \
due to network connectivity {other:?}"
                    );
                    true
                }
            },
        );
    }
}

mod imp {
    use std::{
        cell::{Cell, RefCell},
        time::Duration,
    };

    use async_channel::{Receiver, Sender};
    use futures::{StreamExt, stream};
    use glib::prelude::*;
    use glib::subclass::prelude::*;
    use gtk::gio::{self, Cancellable, prelude::CancellableExt};

    use crate::{config::G_LOG_DOMAIN, images::Source};

    use super::{AutomaticWallpaperUpdateInhibitor, ScheduledWallpaperUpdate};

    #[derive(glib::Properties)]
    #[properties(wrapper_type = super::AutomaticWallpaperUpdateScheduler)]
    pub struct AutomaticWallpaperUpdateScheduler {
        #[property(get)]
        inhibitors: Cell<AutomaticWallpaperUpdateInhibitor>,
        #[property(get, set = Self::set_source, builder(Source::default()))]
        source: Cell<Source>,
        #[property(get = Self::get_is_scheduled, type = bool)]
        is_scheduled: RefCell<Option<Cancellable>>,
        update_tx: Sender<ScheduledWallpaperUpdate>,
        pub update_rx: Receiver<ScheduledWallpaperUpdate>,
    }

    async fn schedule_automatic_updates(
        initial_delay: Duration,
        source: Source,
        cancellable: Cancellable,
        tx: Sender<ScheduledWallpaperUpdate>,
    ) {
        // Delay the initial wallpaper update a bit, this behaves nicer when the
        // user changes the corresponding setting.
        stream::once(glib::timeout_future_seconds(
            initial_delay.as_secs().try_into().unwrap(),
        ))
        .chain(glib::interval_stream_seconds(30 * 60))
        .map(|()| glib::DateTime::now_utc().unwrap())
        .fold(
            // This is definitely more than 12 hours ago ;)
            glib::DateTime::from_unix_utc(0).unwrap(),
            move |last_update, now| {
                let tx = tx.clone();
                let cancellable = cancellable.clone();
                async move {
                    let hours_since_last_update = now.difference(&last_update).as_hours();
                    if hours_since_last_update < 12 {
                        // If the last update's less than twelve hours ago
                        // we don't do anything
                        glib::info!(
                            "Not updating wallpaper, \
                            last update was {hours_since_last_update:?} \
                            hours (< 12) ago"
                        );
                        last_update
                    } else {
                        // If the last update's more than twelve hours ago
                        // pass true downstream to indicate that another
                        // update is needed, and remember now as the time
                        // of the last update
                        glib::info!(
                            "Signalling wallpaper update, \
                        last update was more than {hours_since_last_update:?}\
                         hours (>= 12) ago"
                        );
                        let (update, receive_response) =
                            ScheduledWallpaperUpdate::for_source(source, cancellable.clone());
                        // We can safely unwrap because we'll never drop this channel
                        // while before stopping the updates, as the scheduler itself
                        // retains a reference to one receiver
                        tx.force_send(update).unwrap();
                        match receive_response.await {
                            Ok(Ok(())) => {
                                // Update successful, remember so
                                now
                            }
                            Err(_) | Ok(Err(_)) => {
                                // If the update failed, or if the sender dropped
                                // before it tell us how the update went, try
                                // again next time
                                last_update
                            }
                        }
                    }
                }
            },
        )
        .await;
    }

    impl AutomaticWallpaperUpdateScheduler {
        fn get_is_scheduled(&self) -> bool {
            self.is_scheduled.borrow().is_some()
        }

        fn set_source(&self, source: Source) {
            self.source.set(source);
            if let Some(cancellable) = self.is_scheduled.take() {
                // If updates are already scheduled, cancel scheduled updates,
                // in order to restart with the updated source.
                cancellable.cancel();
            }
            self.schedule_updates_unless_inhibited(Duration::from_secs(10));
        }

        pub fn add_inhibitor(&self, inhibitor: AutomaticWallpaperUpdateInhibitor) {
            glib::info!("Adding inhibitor {inhibitor}");
            self.inhibitors.set(self.inhibitors.get() | inhibitor);
            self.obj().notify_inhibitors();
            self.cancel_scheduled_updates_if_inhibited();
        }

        pub fn clear_inhibitor(&self, inhibitor: AutomaticWallpaperUpdateInhibitor) {
            glib::info!("Clearing inhibitor {inhibitor}");
            self.inhibitors.set(self.inhibitors.get() - inhibitor);
            self.obj().notify_inhibitors();
            self.schedule_updates_unless_inhibited(Duration::from_secs(10));
        }

        fn cancel_scheduled_updates_if_inhibited(&self) {
            let inhibitors = self.inhibitors.get();
            if !inhibitors.is_empty() {
                if let Some(cancellable) = self.is_scheduled.take() {
                    glib::info!(
                        "Cancelling scheduled wallpaper updates, inhibited by {inhibitors}"
                    );
                    cancellable.cancel();
                    self.obj().notify_is_scheduled();
                }
            }
        }

        /// Schedule automatic wallpaper updates unless inhibited.
        fn schedule_updates_unless_inhibited(&self, initial_delay: Duration) {
            let inhibitors = self.inhibitors.get();
            if inhibitors.is_empty() {
                if self.is_scheduled.borrow().is_some() {
                    glib::info!("Automatic wallpaper updates already scheduled.");
                } else {
                    let cancellable = gio::Cancellable::new();
                    self.is_scheduled.replace(Some(cancellable.clone()));
                    self.obj().notify_is_scheduled();

                    let source = self.source.get();
                    glib::info!("Scheduling automatic wallpaper updates from {source:?}");
                    let tx = self.update_tx.clone();
                    glib::spawn_future_local(gio::CancellableFuture::new(
                        schedule_automatic_updates(initial_delay, source, cancellable.clone(), tx),
                        cancellable,
                    ));
                }
            } else {
                glib::info!(
                    "Not scheduling automatic wallpaper updates, still inhibited by {inhibitors}",
                );
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AutomaticWallpaperUpdateScheduler {
        const NAME: &'static str = "PotDAutomaticWallpaperUpdateScheduler";

        type Type = super::AutomaticWallpaperUpdateScheduler;

        fn new() -> Self {
            let (tx, rx) = async_channel::bounded(1);
            Self {
                inhibitors: Cell::new(AutomaticWallpaperUpdateInhibitor::empty()),
                is_scheduled: RefCell::new(None),
                update_tx: tx,
                update_rx: rx,
                source: Cell::new(Source::default()),
            }
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for AutomaticWallpaperUpdateScheduler {
        fn dispose(&self) {
            glib::info!(
                "Automatic wallpaper update scheduler disposed, cancelling scheduled updates"
            );
            if let Some(cancellable) = self.is_scheduled.take() {
                cancellable.cancel();
            }
        }
    }
}
