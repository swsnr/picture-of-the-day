// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use glib::Object;

use crate::app::model::ErrorNotification;

glib::wrapper! {
    pub struct ErrorNotificationPage(ObjectSubclass<imp::ErrorNotificationPage>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl ErrorNotificationPage {
    pub fn new(error: &ErrorNotification) -> Self {
        Object::builder().property("error", error).build()
    }
}

impl Default for ErrorNotificationPage {
    fn default() -> Self {
        Object::builder().build()
    }
}

mod imp {
    use std::cell::RefCell;

    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::CompositeTemplate;

    use crate::app::model::{ErrorNotification, ErrorNotificationActions};

    #[derive(Default, glib::Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ErrorNotificationPage)]
    #[template(resource = "/de/swsnr/picture-of-the-day/ui/error-notification-page.ui")]
    pub struct ErrorNotificationPage {
        #[property(get, set = Self::set_error, nullable)]
        error: RefCell<Option<ErrorNotification>>,
        #[template_child]
        preferences: TemplateChild<gtk::Button>,
        #[template_child]
        about: TemplateChild<gtk::Button>,
        #[template_child]
        source: TemplateChild<gtk::Button>,
    }

    impl ErrorNotificationPage {
        fn set_error(&self, error: Option<ErrorNotification>) {
            for widget in [&self.preferences, &self.about, &self.source] {
                widget.set_visible(false);
            }
            for (_, action) in error
                .as_ref()
                .map_or(ErrorNotificationActions::NONE, ErrorNotification::actions)
                .iter_names()
            {
                let widget = match action {
                    ErrorNotificationActions::OPEN_ABOUT_DIALOG => &self.about,
                    ErrorNotificationActions::OPEN_PREFERENCES => &self.preferences,
                    ErrorNotificationActions::OPEN_SOURCE_URL => &self.source,
                    _ => unreachable!(),
                };
                widget.set_visible(true);
            }
            self.error.replace(error);
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ErrorNotificationPage {
        const NAME: &'static str = "PotDErrorNotificationPage";

        type Type = super::ErrorNotificationPage;

        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            ErrorNotification::ensure_type();

            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for ErrorNotificationPage {}

    impl WidgetImpl for ErrorNotificationPage {}

    impl BinImpl for ErrorNotificationPage {}
}
