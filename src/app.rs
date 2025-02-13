// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use glib::Object;

glib::wrapper! {
    pub struct PictureOfTheDayApplication(ObjectSubclass<imp::PictureOfTheDayApplication>)
        @extends adw::Application, gtk::Application, gtk::gio::Application,
        @implements gtk::gio::ActionGroup, gtk::gio::ActionMap;
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
    use adw::subclass::prelude::*;

    use crate::config::G_LOG_DOMAIN;

    #[derive(Default)]
    pub struct PictureOfTheDayApplication {}

    #[glib::object_subclass]
    impl ObjectSubclass for PictureOfTheDayApplication {
        const NAME: &'static str = "PictureOfTheDayApplication";

        type Type = super::PictureOfTheDayApplication;

        type ParentType = adw::Application;
    }

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
        }
    }

    impl GtkApplicationImpl for PictureOfTheDayApplication {}

    impl AdwApplicationImpl for PictureOfTheDayApplication {}
}
