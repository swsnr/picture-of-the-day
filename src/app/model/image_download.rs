// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use glib::Object;

glib::wrapper! {
    /// The result of an image download.
    pub struct ImageDownload(ObjectSubclass<imp::ImageDownload>);
}

impl Default for ImageDownload {
    fn default() -> Self {
        Object::builder().build()
    }
}

mod imp {
    use std::cell::RefCell;

    use glib::Properties;
    use glib::prelude::*;
    use glib::subclass::prelude::*;
    use gtk::gio;

    use crate::app::model::ErrorNotification;

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::ImageDownload)]
    pub struct ImageDownload {
        /// The downloaded file, if the download was successful.
        #[property(get, set, nullable)]
        file: RefCell<Option<gio::File>>,
        /// An error if the download file.
        #[property(get, set, nullable)]
        error: RefCell<Option<ErrorNotification>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImageDownload {
        const NAME: &'static str = "PotDImageDownload";

        type Type = super::ImageDownload;
    }

    #[glib::derived_properties]
    impl ObjectImpl for ImageDownload {}
}
