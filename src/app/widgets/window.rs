// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use glib::object::IsA;
use gtk::gio;

glib::wrapper! {
    pub struct PictureOfTheDayWindow(ObjectSubclass<imp::PictureOfTheDayWindow>)
        @extends adw::ApplicationWindow, gtk::ApplicationWindow, gtk::Window, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap,
            gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget,
            gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl PictureOfTheDayWindow {
    pub fn new(application: &impl IsA<gtk::Application>) -> Self {
        glib::Object::builder()
            .property("application", application)
            .build()
    }
}

mod imp {
    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::CompositeTemplate;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/de/swsnr/picture-of-the-day/ui/picture-of-the-day-window.ui")]
    pub struct PictureOfTheDayWindow {}

    #[glib::object_subclass]
    impl ObjectSubclass for PictureOfTheDayWindow {
        const NAME: &'static str = "PictureOfTheDayWindow";

        type Type = super::PictureOfTheDayWindow;

        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PictureOfTheDayWindow {}

    impl AdwApplicationWindowImpl for PictureOfTheDayWindow {}

    impl ApplicationWindowImpl for PictureOfTheDayWindow {}

    impl WindowImpl for PictureOfTheDayWindow {}

    impl WidgetImpl for PictureOfTheDayWindow {}
}
