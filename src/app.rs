// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::time::Duration;

use adw::prelude::*;
use futures::{StreamExt, stream};
use glib::{Object, dgettext, dpgettext2, subclass::types::ObjectSubclassIsExt};
use gtk::{
    UriLauncher,
    gio::{self, ActionEntry, ApplicationFlags},
};
use model::{ErrorNotification, ErrorNotificationActions};

use crate::{
    config::G_LOG_DOMAIN,
    io::ensure_directory,
    portal::{
        client::RequestResult,
        wallpaper::{Preview, SetOn},
        window::PortalWindowHandle,
    },
    source::{Source, SourceError},
};

mod model;
mod widgets;

use widgets::PreferencesDialog;

glib::wrapper! {
    pub struct Application(ObjectSubclass<imp::Application>)
        @extends adw::Application, gtk::Application, gtk::gio::Application,
        @implements gtk::gio::ActionGroup, gtk::gio::ActionMap;
}

const ERROR_NOTIFICATION_ID: &str = "automatic-wallpaper-error";

impl Application {
    /// Setup actions of the application.
    ///
    /// - `app.quit` quits the application.
    /// - `app.about` shows the about dialog over the active window if any.
    /// - `app.about` shows the preferences dialog over the active window if any.
    /// - `app.open-source-url` opens the main URL of the selected source.
    fn setup_actions(&self) {
        let actions = [
            ActionEntry::builder("quit")
                .activate(|app: &Self, _, _| {
                    glib::debug!("Quitting");
                    // Close the active window if any, and stop automatic wallpaper
                    // updates; this effectively drops all app guards and thus
                    // makes the app quit.
                    //
                    // We explicitly don't use app.quit() here because it'd
                    // immediately shut down the event loop, so any ongoing IO
                    // operations don't have any change to clean up.
                    if let Some(window) = app.active_window() {
                        window.close();
                    }
                    app.imp().stop_automatic_wallpaper_update();
                })
                .build(),
            ActionEntry::builder("about")
                .activate(|app: &Self, _, _| {
                    app.show_about_dialog();
                })
                .build(),
            ActionEntry::builder("preferences")
                .activate(|app: &Self, _, _| {
                    app.show_preferences();
                })
                .build(),
            ActionEntry::builder("open-source-url")
                .activate(|app: &Application, _, parameter| {
                    if let Some(source) = parameter.and_then(glib::Variant::get::<Source>) {
                        glib::spawn_future_local(glib::clone!(
                            #[weak]
                            app,
                            async move {
                                if let Err(error) = UriLauncher::new(source.url())
                                    .launch_future(app.active_window().as_ref())
                                    .await
                                {
                                    // TODO: Perhaps show this as another error notification
                                    glib::warn!(
                                        "Failed to launch URI of \
                                        {source:?}: {error}"
                                    );
                                }
                            }
                        ));
                    }
                })
                .build(),
        ];
        self.add_action_entries(actions);
        self.set_accels_for_action("app.quit", &["<Control>q"]);
        self.set_accels_for_action("app.preferences", &["<Control>comma"]);
    }

    fn show_about_dialog(&self) {
        let dialog = adw::AboutDialog::from_appdata(
            "/de/swsnr/pictureoftheday/de.swsnr.pictureoftheday.metainfo.xml",
            Some(&crate::config::release_notes_version().to_string()),
        );
        dialog.set_version(crate::config::CARGO_PKG_VERSION);

        dialog.add_link(
            &dpgettext2(None, "about-dialog.link.label", "Translations"),
            "https://translate.codeberg.org/engage/de-swsnr-pictureoftheday/",
        );

        dialog.set_developers(&["Sebastian Wiesner https://swsnr.de"]);
        dialog.set_designers(&["Sebastian Wiesner https://swsnr.de"]);
        // Credits for the translator to the current language.
        // Translators: Add your name here, as "Jane Doe <jdoe@example.com>" or "Jane Doe https://jdoe.example.com"
        // Mail address or URL are optional.  Separate multiple translators with a newline, i.e. \n
        dialog.set_translator_credits(&dgettext(None, "translator-credits"));
        dialog.add_acknowledgement_section(
            Some(&dpgettext2(
                None,
                "about-dialog.acknowledgment-section",
                "Help and inspiration",
            )),
            &[
                "Sebastian Dröge https://github.com/sdroege",
                "Bilal Elmoussaoui https://github.com/bilelmoussaoui",
                "Authenticator https://gitlab.gnome.org/World/Authenticator",
                "Decoder https://gitlab.gnome.org/World/decoder/",
            ],
        );
        dialog.add_acknowledgement_section(
            Some(&dpgettext2(
                None,
                "about-dialog.acknowledgment-section",
                "Helpful services",
            )),
            &[
                "Flathub https://flathub.org/",
                "Open Build Service https://build.opensuse.org/",
                "GitHub actions https://github.com/features/actions",
            ],
        );

        dialog.add_other_app(
            "de.swsnr.turnon",
            "Turn On",
            "Turn on devices in your network",
        );

        dialog.present(self.active_window().as_ref());
    }

    fn show_preferences(&self) -> PreferencesDialog {
        let prefs = PreferencesDialog::default();
        prefs.bind(&self.imp().settings());
        prefs.present(self.active_window().as_ref());
        prefs
    }

    fn show_error_from_automatic_wallpaper(&self, source: Source, error: &SourceError) {
        let error = ErrorNotification::from_error(source, error);
        if error.needs_attention() {
            let notification = gio::Notification::new(&error.title());
            notification.set_body(Some(&error.description()));
            notification.set_priority(gio::NotificationPriority::Normal);
            for action in error.actions().iter() {
                let (label, action, target) = match action {
                    ErrorNotificationActions::OPEN_ABOUT_DIALOG => {
                        let label =
                            dpgettext2(None, "notification.button.label", "Contact information");
                        (label, "app.about", None)
                    }
                    ErrorNotificationActions::OPEN_PREFERENCES => {
                        let label =
                            dpgettext2(None, "notification.button.label", "Open preferences");
                        (label, "app.preferences", None)
                    }
                    ErrorNotificationActions::OPEN_SOURCE_URL => {
                        let label = dpgettext2(None, "notification.button.label", "Open URL");
                        (label, "app.open-source-url", Some(source.to_variant()))
                    }
                    _ => unreachable!(),
                };
                notification.add_button_with_target_value(&label, action, target.as_ref());
            }

            self.send_notification(Some(ERROR_NOTIFICATION_ID), &notification);
        }
    }

    /// Update wallpaper periodically.
    ///
    /// Periodically check whether `source` has a new wallpaper, and udpate it.
    /// Do the first check after `initial_delay`.
    ///
    /// Stop updating when the given `cancellable` is triggered.
    async fn update_wallpaper_periodically(&self, source: Source, initial_delay: Duration) {
        // Delay the initial wallpaper update a bit, this behaves nicer when the
        // user changes the corresponding setting.
        stream::once(glib::timeout_future_seconds(
            initial_delay.as_secs().try_into().unwrap(),
        ))
        .chain(glib::interval_stream_seconds(30 * 60))
        .map(|()| glib::DateTime::now_utc().unwrap())
        .fold(
            // This is definitetly more than 12 hours ago ;)
            glib::DateTime::from_unix_utc(0).unwrap(),
            move |last_update, now| {
                // We keep the application alive through a hold handle,
                // so we can just as well keep a strong reference here.
                // We'll break the cycle explicitly when shutting down.
                let app = self.clone();
                async move {
                    let hours_since_last_update = now.difference(&last_update).as_hours();
                    if 12 < hours_since_last_update {
                        // If the last update's more than twelve hours ago
                        // pass true downstream to indicate that another
                        // update is needed, and remember now as the time
                        // of the last update
                        glib::info!(
                            "Updating wallpaper, \
                        last update was more than {hours_since_last_update:?}\
                         hours (>= 12) ago"
                        );
                        match app.fetch_and_set_wallpaper(source).await {
                            Ok(()) => {
                                // If we successfully updated the wallpaper,
                                // automatically hide any previous error notification.
                                app.withdraw_notification(ERROR_NOTIFICATION_ID);
                                now
                            }
                            Err(error) => {
                                glib::warn!(
                                    "Failed to fetch and set \
                                        wallpaper from {source:?}: \
                                        {error}"
                                );
                                app.show_error_from_automatic_wallpaper(source, &error);
                                last_update
                            }
                        }
                    } else {
                        // If the last update's less than twelve hours ago
                        // pass false downstream to indicate that we should
                        // skip this trigger.
                        glib::info!(
                            "Not updating wallpaper, \
                            last update was {hours_since_last_update:?} \
                            hours (< 12) ago"
                        );
                        last_update
                    }
                }
            },
        )
        .await;
    }

    async fn fetch_and_set_wallpaper(&self, source: Source) -> Result<(), SourceError> {
        let session = self.http_session();
        glib::info!("Setting wallpaper from {source:?}");
        let images = source.get_images(&session, None).await?;

        let image = if images.len() == 1 {
            // This won't panic because  we just checked that we have one element
            #[allow(clippy::indexing_slicing)]
            &images[0]
        } else {
            // This won't panic because `get_images` never returns an empty list,
            // never returns more images than i32::max, and we take care to
            // generate a random index within bounds.
            let index = glib::random_int_range(0, i32::try_from(images.len()).unwrap());
            #[allow(clippy::indexing_slicing)]
            &images[usize::try_from(index).unwrap()]
        };

        let target_directory = source.images_directory();
        ensure_directory(&target_directory).await?;
        let target = image
            .download_to_directory(&target_directory, &session)
            .await?;

        glib::info!("Setting wallpaper to {}", target.display());
        let window = PortalWindowHandle::new_for_app(self).await;
        let response = self
            .portal_client()
            .unwrap()
            .set_wallpaper(
                &gio::File::for_path(&target),
                &window,
                Preview::NoPreview,
                SetOn::Both,
            )
            .await?;
        if !matches!(response, RequestResult::Success) {
            glib::warn!(
                "Request to set wallpaper to {} denied, got {response:?}",
                target.display()
            );
        }
        Ok(())
    }
}

impl Default for Application {
    fn default() -> Self {
        Object::builder()
            .property("application-id", crate::config::APP_ID)
            .property("resource-base-path", "/de/swsnr/pictureoftheday")
            .property("flags", ApplicationFlags::HANDLES_COMMAND_LINE)
            .build()
    }
}

mod imp {
    use crate::{
        app::widgets::ApplicationWindow,
        config::G_LOG_DOMAIN,
        portal::{
            background::RequestBackgroundFlags,
            client::{PortalClient, RequestResult},
            window::PortalWindowHandle,
        },
        source::Source,
    };
    use adw::gio::ApplicationCommandLine;
    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use chrono::NaiveDate;
    use futures::StreamExt;
    use glib::{ExitCode, OptionArg, OptionFlags, Properties, dpgettext2};
    use gtk::gio::{self, ApplicationHoldGuard, Cancellable};
    use soup::prelude::*;
    use std::cell::Cell;
    use std::str::FromStr;
    use std::{cell::RefCell, time::Duration};

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::Application)]
    pub struct Application {
        #[property(get)]
        http_session: soup::Session,
        #[property(get)]
        portal_client: RefCell<Option<PortalClient>>,
        #[property(get)]
        settings: RefCell<Option<gio::Settings>>,
        /// The overridden date, if any.
        ///
        /// Set if the user specified --date on the command line.
        date: Cell<Option<NaiveDate>>,
        /// State of automatic wallpaper update.
        ///
        /// If `None` automatic wallpaper update is off.  If set contains a
        /// cancellable to stop wallpaper updates, and a guard keeping the
        /// application alive and preventing it from quitting on idle.
        automatic_wallpaper_update: RefCell<Option<(Cancellable, ApplicationHoldGuard)>>,
    }

    impl Application {
        pub fn settings(&self) -> gio::Settings {
            self.settings.borrow().as_ref().unwrap().clone()
        }

        /// Hold onto this app until `dialog` is closed.
        async fn hold_until_dialog_closed(&self, dialog: &impl IsA<adw::Dialog>) {
            let guard = self.obj().hold();
            let (tx, mut rx) = futures::channel::mpsc::unbounded();
            dialog.connect_closed(glib::clone!(
                #[strong]
                tx,
                move |_| {
                    #[allow(clippy::let_underscore_must_use)]
                    let _: Result<_, _> = tx.unbounded_send(());
                }
            ));
            let _: Option<()> = rx.next().await;
            drop(guard);
        }

        pub fn stop_automatic_wallpaper_update(&self) {
            if let Some((cancellable, guard)) = self.automatic_wallpaper_update.take() {
                glib::debug!("Canceling automatic wallpaper update");
                cancellable.cancel();
                // Redundant, but I like to see this explicitly.
                drop(guard);
            }
        }

        fn start_automatic_wallpaper_update(&self, source: Source, initial_delay: Duration) {
            self.stop_automatic_wallpaper_update();

            let guard = self.obj().hold();
            let cancellable = gio::Cancellable::new();
            self.automatic_wallpaper_update
                .replace(Some((cancellable.clone(), guard)));

            let app = self.obj().clone();
            glib::spawn_future_local(gio::CancellableFuture::new(
                async move {
                    app.update_wallpaper_periodically(source, initial_delay)
                        .await;
                },
                cancellable,
            ));
        }

        fn start_stop_wallpaper_update(&self, initial_delay: Duration) {
            let settings = self.settings();
            if settings.boolean("set-wallpaper-automatically") {
                self.start_automatic_wallpaper_update(
                    settings.get::<Source>("selected-source"),
                    initial_delay,
                );
            } else {
                self.stop_automatic_wallpaper_update();
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Application {
        const NAME: &'static str = "PotDApplication";

        type Type = super::Application;

        type ParentType = adw::Application;
    }

    #[glib::derived_properties]
    impl ObjectImpl for Application {
        fn constructed(&self) {
            self.parent_constructed();

            let app = self.obj();
            app.add_main_option(
                "preferences",
                0.into(),
                OptionFlags::NONE,
                OptionArg::None,
                &dpgettext2(None, "command-line.option.description", "Open preferences"),
                None,
            );
            app.add_main_option(
                "date",
                0.into(),
                OptionFlags::NONE,
                OptionArg::String,
                &dpgettext2(
                    None,
                    "command-line.option.description",
                    "Show image for the given date",
                ),
                Some(&dpgettext2(
                    None,
                    "command-line.option.arg.description",
                    "YYYY-MM-DD",
                )),
            );
        }
    }

    impl ApplicationImpl for Application {
        fn startup(&self) {
            self.parent_startup();

            // Set default icon for all Gtk windows; we do this here instead of
            // in main because by this time Gtk is already initialized.
            gtk::Window::set_default_icon_name(crate::config::APP_ID);

            if crate::config::is_development() {
                glib::warn!(
                    "Starting application version {} (DEVELOPMENT BUILD)",
                    crate::config::CARGO_PKG_VERSION
                );
            } else {
                glib::debug!(
                    "Starting application version {}",
                    crate::config::CARGO_PKG_VERSION
                );
            }

            self.obj().setup_actions();

            glib::info!("Loading settings");
            let settings = crate::config::get_settings();
            self.settings.replace(Some(settings.clone()));

            glib::info!(
                "Initializing soup session with user agent {}",
                crate::config::USER_AGENT
            );

            // If the default glib logger logs debug logs of our log domain
            // enable debug logging for soup
            self.http_session.set_user_agent(crate::config::USER_AGENT);
            if !glib::log_writer_default_would_drop(glib::LogLevel::Debug, Some(G_LOG_DOMAIN)) {
                glib::info!("Enabling HTTP logging");
                let log = soup::Logger::builder()
                    .level(soup::LoggerLogLevel::Body)
                    // Omit bodies larger than 100KiB
                    .max_body_size(102_400)
                    .build();
                self.http_session.add_feature(&log);
            }

            glib::info!("Starting automatic updates");
            // On startup check for a new wallpaper almost immediately (we wait
            // 10 seconds for things to settle down in case we were auto-started)
            self.start_stop_wallpaper_update(Duration::from_secs(10));
            for (key, initial_delay) in [
                // When the user enabled automatic wallpaper, update the wallpaper immediately
                ("set-wallpaper-automatically", Duration::from_secs(1)),
                // When the user changed the source we wait considerably longer before we
                // schedule a new update, because the user may just have switched the source
                // to see a preview of todays image.
                ("selected-source", Duration::from_secs(30)),
            ] {
                settings.connect_changed(
                    Some(key),
                    glib::clone!(
                        #[weak(rename_to = app)]
                        self.obj(),
                        move |_, _| {
                            app.imp().start_stop_wallpaper_update(initial_delay);
                        }
                    ),
                );
            }
        }

        fn command_line(&self, command_line: &ApplicationCommandLine) -> ExitCode {
            // Hold on to the app while we're processing the command line and
            // spawn futures to handle it.
            let guard = self.obj().hold();
            glib::debug!(
                "Handling command line. Remote? {}",
                command_line.is_remote()
            );
            let options = command_line.options_dict();
            if let Ok(Some(true)) = options.lookup("preferences") {
                glib::debug!("Showing preferences");
                let prefs = self.obj().show_preferences();
                glib::spawn_future_local(glib::clone!(
                    #[strong(rename_to = app)]
                    self.obj(),
                    #[strong]
                    command_line,
                    async move {
                        // Hold onto the app until the prefs dialog is closed,
                        // the end command line processing, and drop our outer
                        // hold on the application.
                        app.imp().hold_until_dialog_closed(&prefs).await;
                        command_line.set_exit_status(ExitCode::SUCCESS.value());
                        command_line.done();
                        drop(guard);
                    }
                ));
                ExitCode::SUCCESS
            } else if let Ok(Some(date)) = options.lookup::<String>("date") {
                match chrono::NaiveDate::from_str(&date) {
                    Ok(date) => {
                        glib::warn!("Overriding date to {date}");
                        self.date.replace(Some(date));
                        self.obj().activate();
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        command_line.printerr_literal(&format!("Invalid date '{date}': {error}"));
                        command_line.set_exit_status(ExitCode::FAILURE.into());
                        ExitCode::FAILURE
                    }
                }
            } else {
                self.obj().activate();
                ExitCode::SUCCESS
            }
        }

        fn dbus_register(
            &self,
            connection: &gio::DBusConnection,
            object_path: &str,
        ) -> Result<(), glib::Error> {
            self.parent_dbus_register(connection, object_path)?;
            self.portal_client
                .replace(Some(PortalClient::new(connection)));
            Ok(())
        }

        fn activate(&self) {
            glib::info!("Activating application");
            self.parent_activate();

            if let Some(window) = self.obj().active_window() {
                window.present();
            } else {
                glib::debug!("Creating new window");
                let window = ApplicationWindow::new(
                    &*self.obj(),
                    self.obj().http_session(),
                    self.obj().portal_client().unwrap(),
                    self.date.get(),
                );
                if crate::config::is_development() {
                    window.add_css_class("devel");
                }

                let settings = self.settings();
                for setting in ["selected-source", "set-wallpaper-automatically"] {
                    settings.bind(setting, &window, setting).build();
                }
                settings
                    .bind("main-window-width", &window, "default-width")
                    .build();
                settings
                    .bind("main-window-height", &window, "default-height")
                    .build();
                settings
                    .bind("main-window-maximized", &window, "maximized")
                    .build();
                settings
                    .bind("main-window-fullscreen", &window, "fullscreened")
                    .build();
                window.present();

                // Request background if the app gets activated the first time.
                let portal_client = self.obj().portal_client().unwrap();
                glib::spawn_future_local(async move {
                    let reason = dpgettext2(
                        None,
                        "portal.request-background.reason",
                        "Automatically fetch and set wallpaper in background",
                    );
                    let window_handle = PortalWindowHandle::new_for_native(&window).await;
                    glib::info!("Requesting permission to run in background and autostart");
                    match portal_client
                        .request_background(
                            &window_handle,
                            &reason,
                            Some(&[crate::config::APP_ID, "--gapplication-service"]),
                            RequestBackgroundFlags::AUTOSTART,
                        )
                        .await
                    {
                        Ok(response) => {
                            if response.request_result == RequestResult::Success {
                                if !response.background {
                                    glib::warn!(
                                        "Background request successful, but background not granted?"
                                    );
                                }
                                if !response.autostart {
                                    glib::warn!(
                                        "Background request successful, but autostart not granted?"
                                    );
                                }
                            } else {
                                glib::warn!(
                                    "Background request no successfully: {:?}",
                                    response.request_result
                                );
                            }
                        }
                        Err(error) => {
                            glib::error!("Failed to request background with autostart: {error}");
                        }
                    }
                });
            }
        }
    }

    impl GtkApplicationImpl for Application {}

    impl AdwApplicationImpl for Application {}
}
