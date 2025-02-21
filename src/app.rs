// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use adw::prelude::*;
use glib::Object;
use gtk::gio::ActionEntry;
use widgets::PictureOfTheDayWindow;

use crate::config::G_LOG_DOMAIN;

mod widgets;

glib::wrapper! {
    pub struct PictureOfTheDayApplication(ObjectSubclass<imp::PictureOfTheDayApplication>)
        @extends adw::Application, gtk::Application, gtk::gio::Application,
        @implements gtk::gio::ActionGroup, gtk::gio::ActionMap;
}

impl PictureOfTheDayApplication {
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
            .and_downcast::<PictureOfTheDayWindow>()
            .map(|w| w.selected_source())
            .unwrap_or_default();
        let window = PictureOfTheDayWindow::new(self, self.http_session(), source);
        if crate::config::is_development() {
            window.add_css_class("devel");
        }
        window.present();
    }
}

impl Default for PictureOfTheDayApplication {
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
    #[properties(wrapper_type = super::PictureOfTheDayApplication)]
    pub struct PictureOfTheDayApplication {
        #[property(get)]
        http_session: soup::Session,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PictureOfTheDayApplication {
        const NAME: &'static str = "PictureOfTheDayApplication";

        type Type = super::PictureOfTheDayApplication;

        type ParentType = adw::Application;
    }

    #[glib::derived_properties]
    impl ObjectImpl for PictureOfTheDayApplication {}

    impl ApplicationImpl for PictureOfTheDayApplication {
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
            self.http_session.set_user_agent(crate::config::USER_AGENT);
        }

        fn activate(&self) {
            glib::debug!("Activating application");
            self.parent_activate();
            self.obj().new_window();
        }
    }

    impl GtkApplicationImpl for PictureOfTheDayApplication {}

    impl AdwApplicationImpl for PictureOfTheDayApplication {}
}
