// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// See <https://github.com/gtk-rs/gtk-rs-core/discussions/1625>
#![allow(clippy::as_conversions)]

use glib::Object;

use crate::image::{DownloadableImage, ImageMetadata};

#[derive(Copy, Clone, Debug, glib::Enum)]
#[enum_type(name = "PotDImageDownloadState")]
pub enum ImageState {
    Pending,
    Downloaded,
    DownloadFailed,
}

impl Default for ImageState {
    fn default() -> Self {
        Self::Pending
    }
}

glib::wrapper! {
    pub struct Image(ObjectSubclass<imp::Image>);
}

impl From<&ImageMetadata> for Image {
    fn from(metadata: &ImageMetadata) -> Self {
        Object::builder()
            .property("title", &metadata.title)
            .property("description", &metadata.description)
            .property("copyright", &metadata.copyright)
            .property("url", &metadata.url)
            .property("source-name", metadata.source.i18n_name())
            .property("source-url", metadata.source.url())
            .build()
    }
}

impl From<&DownloadableImage> for Image {
    fn from(image: &DownloadableImage) -> Self {
        Image::from(&image.metadata)
    }
}

mod imp {
    use std::cell::{Cell, RefCell};

    use glib::prelude::*;
    use glib::subclass::prelude::*;
    use gtk::gio;

    use crate::app::model::ErrorNotification;
    use crate::app::model::image::ImageState;

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
        /// The state of the download
        #[property(get, builder(ImageState::default()))]
        state: Cell<ImageState>,
        /// The downloaded file, if the download was successful.
        #[property(get, set = Self::set_downloaded_file, nullable)]
        downloaded_file: RefCell<Option<gio::File>>,
        /// An error if the download file.
        #[property(get, set = Self::set_download_error, nullable)]
        download_error: RefCell<Option<ErrorNotification>>,
    }

    impl Image {
        fn update_state(&self) {
            let state = if self.downloaded_file.borrow().is_some() {
                ImageState::Downloaded
            } else if self.download_error.borrow().is_some() {
                ImageState::DownloadFailed
            } else {
                ImageState::Pending
            };
            self.state.set(state);

            self.obj().notify_state();
        }

        fn set_downloaded_file(&self, file: Option<gio::File>) {
            self.downloaded_file.replace(file);
            self.update_state();
            if self.downloaded_file.borrow().is_some() {
                self.obj().set_download_error(None::<ErrorNotification>);
            }
        }

        fn set_download_error(&self, error: Option<ErrorNotification>) {
            self.download_error.replace(error);
            self.update_state();
            if self.download_error.borrow().is_some() {
                self.obj().set_downloaded_file(None::<gio::File>);
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Image {
        const NAME: &'static str = "PotDImage";

        type Type = super::Image;
    }

    #[glib::derived_properties]
    impl ObjectImpl for Image {}
}
