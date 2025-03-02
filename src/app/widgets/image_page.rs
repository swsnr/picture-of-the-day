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
    use gtk::{CompositeTemplate, gio};

    use crate::app::model::{Image, ImageDownload};

    #[derive(Default, CompositeTemplate, Properties)]
    #[properties(wrapper_type = super::ImagePage)]
    #[template(resource = "/de/swsnr/picture-of-the-day/ui/image-page.ui")]
    pub struct ImagePage {
        #[property(get, set = Self::set_image)]
        image: RefCell<Option<Image>>,
        #[property(get)]
        download: RefCell<Option<ImageDownload>>,
        #[template_child]
        loading: TemplateChild<gtk::Widget>,
        #[template_child]
        picture: TemplateChild<gtk::Widget>,
        #[template_child]
        error: TemplateChild<gtk::Widget>,
    }

    impl ImagePage {
        fn set_image(&self, image: Option<Image>) {
            self.download
                .replace(image.as_ref().map(|i| i.download().clone()));
            self.image.replace(image);
            self.obj().notify_download();
        }
    }

    #[gtk::template_callbacks]
    impl ImagePage {
        #[template_callback(function)]
        fn is_loading(file: Option<&gio::File>, error_message: Option<&str>) -> bool {
            file.is_none() && error_message.is_none()
        }

        #[template_callback]
        fn stack_page(&self, file: Option<&gio::File>, error_message: Option<&str>) -> gtk::Widget {
            match (file, error_message) {
                (Some(_), _) => self.picture.get(),
                (_, Some(_)) => self.error.get(),
                (None, None) => self.loading.get(),
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
            ImageDownload::ensure_type();

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
