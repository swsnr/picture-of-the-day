// Copyright Sebastian Wiesner <sebastian@swsnr.de>
//
// Licensed under the EUPL
//
// See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

use adw::prelude::*;
use glib::{Object, dgettext, dpgettext2, subclass::types::ObjectSubclassIsExt};
use gtk::{
    UriLauncher,
    gio::{self, ActionEntry, ApplicationFlags},
};
use model::{ErrorNotification, ErrorNotificationActions};
use scheduler::ScheduledWallpaperUpdate;

use crate::{
    config::G_LOG_DOMAIN,
    images::{Source, SourceError},
    io::ensure_directory,
    services::portal::{
        RequestResult,
        wallpaper::{Preview, SetOn},
        window::PortalWindowHandle,
    },
};

mod model;
mod scheduler;
mod updated_monitor;
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
                    // We explicitly don't use app.quit() here because it'd
                    // immediately shut down the event loop, so any ongoing IO
                    // operations don't have any change to clean up.
                    //
                    // Instead, we close the active window, and explicitly take
                    // our hold for scheduled wallpaper updates; this drops all
                    // active holds, and thus the application will automatically
                    // exit after a short timeout.
                    if let Some(window) = app.active_window() {
                        window.close();
                    }
                    app.imp().scheduled_updates_hold.take();
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
            ActionEntry::builder("post-update")
                .activate(|app: &Application, _, _| {
                    // Activate the app to make sure we show the post-restart dialog on top of a visible window.
                    // Having the dialog appear all on its own is probably quite confusing because it's not easy
                    // to match it with a running app.
                    app.activate();
                    app.ask_post_update_restart();
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
                "Codeberg https://codeberg.org",
                "Flathub https://flathub.org/",
                "Open Build Service https://build.opensuse.org/",
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

    fn notify_about_update(&self) {
        let notification = gio::Notification::new(&dpgettext2(
            None,
            "notification.title",
            "Picture Of The Day updated",
        ));
        notification.set_body(Some(&dpgettext2(
            None,
            "notification.body",
            "Please restart the app to use the new version.",
        )));
        notification.set_priority(gio::NotificationPriority::High);
        notification.set_default_action("app.post-update");
        self.send_notification(None, &notification);
    }

    fn ask_post_update_restart(&self) {
        let dialog = adw::AlertDialog::new(
            Some(&dpgettext2(
                None,
                "alert-dialog.title",
                "Restart after update?",
            )),
            Some(&dpgettext2(
                None,
                "alert-dialog.body",
                "\
Picture Of The Day was updated, and needs to be restarted.\n\n\
Automatic restarting is not yet supported, but you can quit the app and start \
it again manually.",
            )),
        );
        dialog.add_responses(&[
            (
                "cancel",
                &dpgettext2(None, "alert-dialog.response", "Cancel"),
            ),
            ("quit", &dpgettext2(None, "alert-dialog.response", "Quit")),
        ]);
        dialog.set_body_use_markup(true);
        dialog.set_prefer_wide_layout(true);
        dialog.set_response_appearance("quit", adw::ResponseAppearance::Destructive);
        dialog.set_default_response(Some("quit"));
        dialog.set_close_response("cancel");
        dialog.connect_response(
            Some("quit"),
            glib::clone!(
                #[weak(rename_to = app)]
                self,
                move |_, _| {
                    app.activate_action("quit", None);
                }
            ),
        );
        dialog.present(self.active_window().as_ref());
    }

    async fn handle_scheduled_wallpaper_update(&self, scheduled_update: ScheduledWallpaperUpdate) {
        let source = scheduled_update.source;
        match gio::CancellableFuture::new(
            self.fetch_and_set_wallpaper(source),
            scheduled_update.cancellable,
        )
        .await
        {
            Ok(result) => {
                match &result {
                    Ok(()) => {
                        // If we successfully updated the wallpaper,
                        // automatically hide any previous error notification.
                        self.withdraw_notification(ERROR_NOTIFICATION_ID);
                    }
                    Err(error) => {
                        glib::warn!(
                            "Failed to fetch and set \
                             wallpaper from {source:?}: \
                             {error}"
                        );
                        self.show_error_from_automatic_wallpaper(source, error);
                    }
                }
                if scheduled_update.response.send(result).is_err() {
                    glib::warn!("Response channel for scheduled wallpaper updated closed");
                }
            }
            Err(_) => {
                // We just do nothing and drop the response channel, to tell the
                // receiver that we cancelled things
                glib::info!("Scheduled wallpaper update cancelled");
            }
        }
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
        app::{scheduler::AutomaticWallpaperUpdateInhibitor, widgets::ApplicationWindow},
        config::G_LOG_DOMAIN,
        services::{
            SessionMonitor,
            portal::{
                PortalClient, RequestResult, background::RequestBackgroundFlags,
                window::PortalWindowHandle,
            },
        },
    };
    use adw::gio::ApplicationCommandLine;
    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use futures::StreamExt;
    use glib::{ExitCode, OptionArg, OptionFlags, Properties, dpgettext2};
    use gtk::gio::{self, ApplicationHoldGuard, NetworkConnectivity};
    use jiff::civil::Date;
    use soup::prelude::*;
    use std::cell::RefCell;
    use std::{cell::Cell, str::FromStr};

    use super::{scheduler::AutomaticWallpaperUpdateScheduler, updated_monitor::AppUpdatedMonitor};

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
        date: Cell<Option<Date>>,
        /// Scheduler used for automatic updates.
        scheduler: AutomaticWallpaperUpdateScheduler,
        /// User session monitor.
        session_monitor: SessionMonitor,
        /// App updates monitor,
        updated_monitor: AppUpdatedMonitor,
        /// Hold on to ourselves while automatic wallpaper updates are scheduled
        pub scheduled_updates_hold: RefCell<Option<ApplicationHoldGuard>>,
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

        fn inhibitors_for_binding_target(
            binding: &glib::Binding,
            inhibitor: AutomaticWallpaperUpdateInhibitor,
            set: bool,
        ) -> Option<AutomaticWallpaperUpdateInhibitor> {
            let inhibitors = binding
                .target()?
                .downcast_ref::<AutomaticWallpaperUpdateScheduler>()?
                .inhibitors();
            if set {
                Some(inhibitors | inhibitor)
            } else {
                Some(inhibitors - inhibitor)
            }
        }

        fn setup_scheduled_wallpaper_updates(&self, settings: &gio::Settings) {
            // Inhibit automatic updates if the user disables them.
            // We do this first, to make sure all inhibitors are in place before
            // we start updates by setting the source.
            settings
                .bind("set-wallpaper-automatically", &self.scheduler, "inhibitors")
                .get_only()
                .mapping(glib::clone!(
                    #[weak(rename_to = scheduler)]
                    &self.scheduler,
                    #[upgrade_or_default]
                    move |value, _| {
                        let inhibitors = scheduler
                            .downcast_ref::<AutomaticWallpaperUpdateScheduler>()?
                            .inhibitors();
                        let set_automatically = value.get::<bool>()?;
                        let inhibitors = if set_automatically {
                            inhibitors - AutomaticWallpaperUpdateInhibitor::DisabledByUser
                        } else {
                            inhibitors | AutomaticWallpaperUpdateInhibitor::DisabledByUser
                        };
                        Some(inhibitors.into())
                    }
                ))
                .build();

            // Inhibit if the app has an active main window.
            self.obj()
                .bind_property("active-window", &self.scheduler, "inhibitors")
                .sync_create()
                .transform_to(|binding, window: Option<gtk::Window>| {
                    Self::inhibitors_for_binding_target(
                        binding,
                        AutomaticWallpaperUpdateInhibitor::MainWindowActive,
                        window.is_some(),
                    )
                })
                .build();

            // Inhibit if the system is on low power
            gio::PowerProfileMonitor::get_default()
                .bind_property("power-saver-enabled", &self.scheduler, "inhibitors")
                .sync_create()
                .transform_to(|binding, power_saver_enabled: bool| {
                    Self::inhibitors_for_binding_target(
                        binding,
                        AutomaticWallpaperUpdateInhibitor::LowPower,
                        power_saver_enabled,
                    )
                })
                .build();

            // Inhibit if the network is down
            gio::NetworkMonitor::default()
                .bind_property("connectivity", &self.scheduler, "inhibitors")
                .sync_create()
                .transform_to(|binding, connectivity: NetworkConnectivity| {
                    let no_network = match connectivity {
                        // We do not inhibit on "limited" connectivity, because
                        // that just might be a badly configured proxy or
                        // captive portal, where we still might have success
                        // in updating the wallpaper.
                        NetworkConnectivity::Limited | NetworkConnectivity::Full => false,
                        other => {
                            glib::info!(
                                "Inibiting automatic wallpaper updates \
    due to network connectivity {other:?}"
                            );
                            true
                        }
                    };
                    Self::inhibitors_for_binding_target(
                        binding,
                        AutomaticWallpaperUpdateInhibitor::NoNetwork,
                        no_network,
                    )
                })
                .build();

            // Inhibit while the session is locked
            self.session_monitor
                .bind_property("locked", &self.scheduler, "inhibitors")
                .sync_create()
                .transform_to(|binding, locked: bool| {
                    Self::inhibitors_for_binding_target(
                        binding,
                        AutomaticWallpaperUpdateInhibitor::SessionLocked,
                        locked,
                    )
                })
                .build();

            // Listen to scheduled updates, and set the wallpaper in response
            let rx = self.scheduler.update_receiver();
            // Explicitly pass a weak ref into the long running future, and
            // attempt to upgrade for each iteration; glib::clone! #[weak]
            // would upgrade when the future is polled for the first time: the
            // future would then just continue to have a strong reference to the
            // app
            let app = self.obj().downgrade();
            glib::spawn_future_local(async move {
                while let Ok(update) = rx.recv().await {
                    if let Some(app) = app.upgrade() {
                        app.handle_scheduled_wallpaper_update(update).await;
                    } else {
                        break;
                    }
                }
            });

            // Finally, update the source for scheduled wallpaper updates.
            // This implicit starts scheduled updates.
            settings
                .bind("selected-source", &self.scheduler, "source")
                .build();
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

            // Properly quit the app on applicable unix signals
            let app = self.obj().downgrade();
            glib::spawn_future_local(async move {
                futures::future::select(
                    glib::unix_signal_future(libc::SIGINT),
                    glib::unix_signal_future(libc::SIGTERM),
                )
                .await;
                if let Some(app) = app.upgrade() {
                    app.activate_action("quit", None);
                }
            });

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

            glib::info!("Monitoring app updates");
            self.updated_monitor.connect_updated_notify(glib::clone!(
                #[weak(rename_to = app)]
                self.obj(),
                move |monitor| {
                    if monitor.updated() {
                        glib::info!("App updated");
                        app.notify_about_update();
                    }
                }
            ));

            glib::info!("Configuring automatic updates");
            self.setup_scheduled_wallpaper_updates(&settings);

            // If the user enabled automatic udpates hold on to the application
            // to keep it running in background.
            if settings.boolean("set-wallpaper-automatically") {
                self.scheduled_updates_hold.replace(Some(self.obj().hold()));
            }

            settings.connect_changed(
                Some("set-wallpaper-automatically"),
                glib::clone!(
                    #[weak(rename_to = app)]
                    self.obj(),
                    move |settings, _| {
                        if settings.boolean("set-wallpaper-automatically") {
                            app.imp().scheduled_updates_hold.replace(Some(app.hold()));
                        } else {
                            app.imp().scheduled_updates_hold.take();
                        }
                    }
                ),
            );
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
                match jiff::civil::Date::from_str(&date) {
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

                self.updated_monitor
                    .bind_property("updated", &window, "show-update-indicator")
                    .sync_create()
                    .build();

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
