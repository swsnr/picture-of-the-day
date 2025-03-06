// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use adw::prelude::*;
use glib::{Object, dgettext, dpgettext2, subclass::types::ObjectSubclassIsExt};
use gtk::gio::{ActionEntry, ApplicationFlags};

use crate::config::G_LOG_DOMAIN;

mod model;
mod widgets;

use widgets::{ApplicationWindow, PreferencesDialog};

glib::wrapper! {
    pub struct Application(ObjectSubclass<imp::Application>)
        @extends adw::Application, gtk::Application, gtk::gio::Application,
        @implements gtk::gio::ActionGroup, gtk::gio::ActionMap;
}

impl Application {
    /// Setup actions of the application.
    ///
    /// - `app.quit` quits the application.
    fn setup_actions(&self) {
        let actions = [
            ActionEntry::builder("quit")
                .activate(|app: &Self, _, _| app.quit())
                .build(),
            ActionEntry::builder("new-window")
                .activate(|app: &Self, _, _| app.new_window())
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
        ];
        self.add_action_entries(actions);
        self.set_accels_for_action("app.quit", &["<Control>q"]);
        self.set_accels_for_action("app.preferences", &["<Control>comma"]);
        self.set_accels_for_action("app.new-window", &["<Control><Shift>n"]);
    }

    fn show_about_dialog(&self) {
        let dialog = adw::AboutDialog::from_appdata(
            "/de/swsnr/picture-of-the-day/de.swsnr.picture-of-the-day.metainfo.xml",
            Some(&crate::config::release_notes_version().to_string()),
        );
        dialog.set_version(crate::config::CARGO_PKG_VERSION);

        // TODO translations link to codeberg translate
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

        dialog.present(self.active_window().as_ref());
    }

    fn show_preferences(&self) -> PreferencesDialog {
        let prefs = PreferencesDialog::default();
        prefs.bind(&self.imp().settings());
        prefs.present(self.active_window().as_ref());
        prefs
    }

    fn new_window(&self) {
        glib::debug!("Creating new window");
        let source = self
            .active_window()
            .and_downcast::<ApplicationWindow>()
            .map(|w| w.selected_source())
            .unwrap_or_default();
        let window = ApplicationWindow::new(self, self.http_session(), source);
        if crate::config::is_development() {
            window.add_css_class("devel");
        }
        let settings = self.imp().settings();
        settings
            .bind("last-source", &window, "selected-source")
            .build();
        window.present();
    }
}

impl Default for Application {
    fn default() -> Self {
        Object::builder()
            .property("application-id", crate::config::APP_ID)
            .property("resource-base-path", "/de/swsnr/picture-of-the-day")
            .property("flags", ApplicationFlags::HANDLES_COMMAND_LINE)
            .build()
    }
}

mod imp {
    use adw::gio::ApplicationCommandLine;
    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use futures::StreamExt;
    use glib::{ExitCode, OptionArg, OptionFlags, Properties, dpgettext2};
    use gtk::gio;
    use soup::prelude::SessionExt;
    use std::cell::RefCell;

    use crate::config::G_LOG_DOMAIN;

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::Application)]
    pub struct Application {
        #[property(get)]
        http_session: soup::Session,
        settings: RefCell<Option<gio::Settings>>,
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
                &dpgettext2(
                    None,
                    "command-line.option.description",
                    "Show preferences dialog",
                ),
                None,
            );
        }
    }

    impl ApplicationImpl for Application {
        fn startup(&self) {
            self.parent_startup();

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
            self.settings.replace(Some(crate::config::get_settings()));

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
            } else {
                self.obj().activate();
                ExitCode::SUCCESS
            }
        }

        fn activate(&self) {
            glib::debug!("Activating application");
            self.parent_activate();
            self.obj().new_window();
        }
    }

    impl GtkApplicationImpl for Application {}

    impl AdwApplicationImpl for Application {}
}
