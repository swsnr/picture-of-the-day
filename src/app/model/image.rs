// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use glib::Object;

use crate::image::ImageMetadata;

glib::wrapper! {
    pub struct Image(ObjectSubclass<imp::Image>);
}

impl From<ImageMetadata> for Image {
    fn from(metadata: ImageMetadata) -> Self {
        Object::builder()
            .property("title", metadata.title)
            .property("description", metadata.description)
            .property("copyright", metadata.copyright)
            .property("url", metadata.url)
            .property("source-name", metadata.source.i18n_name())
            .property("source-url", metadata.source.url())
            .build()
    }
}

mod imp {
    use std::cell::RefCell;

    use glib::prelude::*;
    use glib::subclass::prelude::*;
    use gtk::gio;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::Image)]
    pub struct Image {
        #[property(get, construct_only)]
        title: RefCell<String>,
        #[property(get, construct_only, nullable)]
        description: RefCell<Option<String>>,
        #[property(get, construct_only, nullable)]
        copyright: RefCell<Option<String>>,
        #[property(get, construct_only, nullable)]
        url: RefCell<Option<String>>,
        #[property(get, construct_only)]
        source_name: RefCell<String>,
        #[property(get, construct_only)]
        source_url: RefCell<String>,
        #[property(get, set, nullable)]
        #[allow(clippy::struct_field_names)]
        image_file: RefCell<Option<gio::File>>,
        #[property(get, set, nullable)]
        error_message: RefCell<Option<String>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Image {
        const NAME: &'static str = "PotDImage";

        type Type = super::Image;
    }

    #[glib::derived_properties]
    impl ObjectImpl for Image {}
}
