// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use glib::Object;
use gtk::gio;

glib::wrapper! {
    pub struct ImagePage(ObjectSubclass<imp::ImagePage>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl ImagePage {
    pub fn reset(&self) {
        self.set_image_file(None::<gio::File>);
        self.set_error_message(None::<String>);
    }
}

impl Default for ImagePage {
    fn default() -> Self {
        Object::builder().build()
    }
}

mod imp {
    use std::cell::RefCell;

    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use glib::{subclass::InitializingObject, Properties};
    use gtk::{gio, CompositeTemplate};

    #[derive(Default, CompositeTemplate, Properties)]
    #[properties(wrapper_type = super::ImagePage)]
    #[template(resource = "/de/swsnr/picture-of-the-day/ui/image-page.ui")]
    pub struct ImagePage {
        #[property(get, set = Self::set_error_message, nullable)]
        error_message: RefCell<Option<String>>,
        #[property(get, set = Self::set_image_file, nullable)]
        image_file: RefCell<Option<gio::File>>,
        #[template_child]
        stack: TemplateChild<gtk::Stack>,
        #[template_child]
        loading: TemplateChild<gtk::Widget>,
        #[template_child]
        picture: TemplateChild<gtk::Widget>,
        #[template_child]
        error: TemplateChild<gtk::Widget>,
    }

    impl ImagePage {
        fn has_error_message(&self) -> bool {
            self.error_message.borrow().is_some()
        }

        fn has_image_file(&self) -> bool {
            self.image_file.borrow().is_some()
        }

        fn update_stack(&self) {
            let child = if self.has_error_message() {
                self.error.get()
            } else if self.has_image_file() {
                self.picture.get()
            } else {
                self.loading.get()
            };
            self.stack.set_visible_child(&child);
        }

        fn set_error_message(&self, error_message: Option<String>) {
            self.error_message.replace(error_message);
            if self.has_error_message() {
                self.obj().set_image_file(None::<gio::File>);
            } else {
                self.update_stack();
            }
        }

        fn set_image_file(&self, image_file: Option<gio::File>) {
            self.image_file.replace(image_file);
            if self.has_image_file() {
                self.obj().set_error_message(None::<String>);
            } else {
                self.update_stack();
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImagePage {
        const NAME: &'static str = "PotDImagePage";

        type Type = super::ImagePage;

        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for ImagePage {}

    impl WidgetImpl for ImagePage {}

    impl BinImpl for ImagePage {}
}
