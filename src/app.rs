// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use adw::prelude::*;
use glib::Object;
use gtk::gio::ActionEntry;

use crate::config::G_LOG_DOMAIN;

mod widgets;

use widgets::ApplicationWindow;

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
        ];
        self.add_action_entries(actions);
        self.set_accels_for_action("app.quit", &["<Control>q"]);
        self.set_accels_for_action("app.new-window", &["<Control><Shift>n"]);
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
        window.present();
    }
}

impl Default for Application {
    fn default() -> Self {
        Object::builder()
            .property("application-id", crate::config::APP_ID)
            .property("resource-base-path", "/de/swsnr/picture-of-the-day")
            .build()
    }
}

mod imp {
    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use glib::Properties;
    use soup::prelude::SessionExt;

    use crate::config::G_LOG_DOMAIN;

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::Application)]
    pub struct Application {
        #[property(get)]
        http_session: soup::Session,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Application {
        const NAME: &'static str = "PotDApplication";

        type Type = super::Application;

        type ParentType = adw::Application;
    }

    #[glib::derived_properties]
    impl ObjectImpl for Application {}

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

        fn activate(&self) {
            glib::debug!("Activating application");
            self.parent_activate();
            self.obj().new_window();
        }
    }

    impl GtkApplicationImpl for Application {}

    impl AdwApplicationImpl for Application {}
}
