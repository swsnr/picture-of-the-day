// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use glib::Object;

use super::ImageMetadata;

glib::wrapper! {
    pub struct ImageObject(ObjectSubclass<imp::ImageObject>);
}

impl From<ImageMetadata> for ImageObject {
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
    #[properties(wrapper_type = super::ImageObject)]
    pub struct ImageObject {
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
        image_file: RefCell<Option<gio::File>>,
        #[property(get, set, nullable)]
        error_message: RefCell<Option<String>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImageObject {
        const NAME: &'static str = "PotDImage";

        type Type = super::ImageObject;
    }

    #[glib::derived_properties]
    impl ObjectImpl for ImageObject {}
}
