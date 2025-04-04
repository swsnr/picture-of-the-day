// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use glib::Object;

glib::wrapper! {
    pub struct ImagePage(ObjectSubclass<imp::ImagePage>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
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
    use glib::{Properties, subclass::InitializingObject};
    use gtk::CompositeTemplate;

    use crate::app::{
        model::{Image, ImageState},
        widgets::ErrorNotificationPage,
    };

    #[derive(Default, CompositeTemplate, Properties)]
    #[properties(wrapper_type = super::ImagePage)]
    #[template(resource = "/de/swsnr/pictureoftheday/ui/image-page.ui")]
    pub struct ImagePage {
        #[property(get, set)]
        image: RefCell<Option<Image>>,
        #[template_child]
        loading: TemplateChild<gtk::Widget>,
        #[template_child]
        picture: TemplateChild<gtk::Widget>,
        #[template_child]
        error: TemplateChild<gtk::Widget>,
    }

    #[gtk::template_callbacks]
    impl ImagePage {
        #[template_callback]
        fn stack_page(&self, state: ImageState) -> gtk::Widget {
            match state {
                ImageState::Pending => self.loading.get(),
                ImageState::Downloaded => self.picture.get(),
                ImageState::DownloadFailed => self.error.get(),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImagePage {
        const NAME: &'static str = "PotDImagePage";

        type Type = super::ImagePage;

        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            Image::ensure_type();
            ImageState::ensure_type();
            ErrorNotificationPage::ensure_type();

            klass.bind_template();
            klass.bind_template_callbacks();
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
