// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// See <https://github.com/gtk-rs/gtk-rs-core/discussions/1625>
#![allow(clippy::as_conversions)]

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

#[derive(Copy, Clone, Debug, glib::Enum)]
#[enum_type(name = "PotDImageDownloadState")]
pub enum ImageDownloadState {
    Pending,
    Succeeded,
    Failed,
}

impl Default for ImageDownloadState {
    fn default() -> Self {
        Self::Pending
    }
}

mod imp {
    use std::cell::Cell;
    use std::cell::RefCell;

    use glib::Properties;
    use glib::prelude::*;
    use glib::subclass::prelude::*;
    use gtk::gio;

    use crate::app::model::ErrorNotification;

    use super::ImageDownloadState;

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::ImageDownload)]
    pub struct ImageDownload {
        /// The state of the download
        #[property(get, builder(ImageDownloadState::default()))]
        state: Cell<ImageDownloadState>,
        /// Whether the download is still pending.
        #[property(get)]
        is_pending: Cell<bool>,
        /// The downloaded file, if the download was successful.
        #[property(get, set = Self::set_file, nullable)]
        file: RefCell<Option<gio::File>>,
        /// Whether the download has a file.
        #[property(get)]
        has_file: Cell<bool>,
        /// An error if the download file.
        #[property(get, set = Self::set_error, nullable)]
        error: RefCell<Option<ErrorNotification>>,
    }

    impl ImageDownload {
        fn update_state(&self) {
            self.is_pending
                .set(self.file.borrow().is_none() && self.error.borrow().is_none());

            let state = if self.file.borrow().is_some() {
                ImageDownloadState::Succeeded
            } else if self.error.borrow().is_some() {
                ImageDownloadState::Failed
            } else {
                ImageDownloadState::Pending
            };
            self.state.set(state);

            self.obj().notify_is_pending();
            self.obj().notify_state();
        }

        fn set_file(&self, file: Option<gio::File>) {
            self.has_file.set(file.is_some());
            self.file.replace(file);

            self.obj().notify_has_file();
            self.update_state();
        }

        fn set_error(&self, error: Option<ErrorNotification>) {
            self.error.replace(error);
            self.update_state();
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImageDownload {
        const NAME: &'static str = "PotDImageDownload";

        type Type = super::ImageDownload;
    }

    #[glib::derived_properties]
    impl ObjectImpl for ImageDownload {}
}
