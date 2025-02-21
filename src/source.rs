// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Sources of wallpapers.

// See <https://github.com/gtk-rs/gtk-rs-core/discussions/1625>
#![allow(clippy::as_conversions)]

use glib::{dpgettext2, GString};
use gtk::gio::IOErrorEnum;

use crate::image::DownloadableImage;

mod error;
mod wikimedia;

pub use error::SourceError;

#[derive(Debug, Clone, Copy, glib::Enum, glib::Variant, strum::EnumIter)]
#[enum_type(name = "PictureOfTheDaySource")]
pub enum Source {
    Apod,
    Eopod,
    Bing,
    Wikimedia,
    Stalenhag,
}

impl Default for Source {
    fn default() -> Self {
        // This source provides images under free licenses
        Self::Wikimedia
    }
}

#[deny(clippy::as_conversions)]
impl Source {
    pub fn i18n_name(self) -> GString {
        match self {
            Source::Apod => dpgettext2(None, "source name", "NASA Astronomy Picture Of The Day"),
            Source::Eopod => dpgettext2(
                None,
                "source name",
                "NASA Earth Observatory Image Of The Day",
            ),
            Source::Bing => dpgettext2(None, "source name", "Bing"),
            Source::Wikimedia => dpgettext2(None, "source name", "Wikimedia Picture Of The Day"),
            Source::Stalenhag => dpgettext2(None, "source name", "Simon Stålenhag"),
        }
    }

    pub fn url(self) -> &'static str {
        match self {
            Source::Apod => "https://apod.nasa.gov/",
            Source::Eopod => "https://earthobservatory.nasa.gov/topic/image-of-the-day",
            Source::Bing => "https://bing.com",
            Source::Wikimedia => "https://commons.wikimedia.org/wiki/Main_Page",
            Source::Stalenhag => "https://simonstalenhag.se/",
        }
    }

    pub async fn get_images(
        self,
        session: &soup::Session,
    ) -> Result<Vec<DownloadableImage>, SourceError> {
        let not_supported = glib::Error::new(
            IOErrorEnum::NotSupported,
            &format!("Source {self:?} not supported yet!"),
        );
        #[allow(clippy::match_same_arms)]
        match self {
            Source::Apod => Err(not_supported.into()),
            Source::Eopod => Err(not_supported.into()),
            Source::Bing => Err(not_supported.into()),
            Source::Wikimedia => Ok(vec![wikimedia::fetch_featured_image(session).await?]),
            Source::Stalenhag => Err(not_supported.into()),
        }
    }
}
