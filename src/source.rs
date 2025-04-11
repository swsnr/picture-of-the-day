// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Sources of wallpapers.

// See <https://github.com/gtk-rs/gtk-rs-core/discussions/1625>
#![allow(clippy::as_conversions)]

use std::path::PathBuf;

use glib::{GString, dpgettext2};

use crate::config::G_LOG_DOMAIN;
use crate::image::DownloadableImage;

mod error;
mod http;

mod apod;
mod bing;
pub mod stalenhag;
mod wikimedia;

#[cfg(test)]
mod testutil;

pub use error::SourceError;

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    glib::Enum,
    glib::Variant,
    strum::EnumIter,
    strum::IntoStaticStr,
)]
#[enum_type(name = "PotDSource")]
#[strum(serialize_all = "kebab-case")]
pub enum Source {
    Apod,
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
            Source::Bing => dpgettext2(None, "source name", "Bing"),
            Source::Wikimedia => dpgettext2(None, "source name", "Wikimedia Picture Of The Day"),
            Source::Stalenhag => dpgettext2(None, "source name", "Simon Stålenhag"),
        }
    }

    pub fn id(self) -> &'static str {
        self.into()
    }

    pub fn url(self) -> &'static str {
        match self {
            Source::Apod => "https://apod.nasa.gov/",
            Source::Bing => "https://bing.com",
            Source::Wikimedia => "https://commons.wikimedia.org/wiki/Main_Page",
            Source::Stalenhag => "https://simonstalenhag.se/",
        }
    }

    pub fn images_directory(self) -> PathBuf {
        crate::config::images_directory().join(self.id())
    }

    /// Get metadata for images offer by this source for today.
    ///
    /// Return a non-empty list of images for today.  Return an error if I/O
    /// failed, or if the source did not provide any image.
    pub async fn get_images(
        self,
        session: &soup::Session,
        date: Option<&glib::DateTime>,
    ) -> Result<Vec<DownloadableImage>, SourceError> {
        let now = glib::DateTime::now_local().unwrap();
        let images = match self {
            Source::Apod => vec![apod::fetch_picture_of_the_day(session, date).await?],
            Source::Bing => {
                date.inspect(|_| {
                    glib::warn!("Bing does not support overriding the date");
                });
                bing::fetch_daily_images(session).await?
            }
            Source::Wikimedia => {
                vec![wikimedia::fetch_featured_image(session, date.unwrap_or(&now)).await?]
            }
            Source::Stalenhag => vec![stalenhag::pick_image_for_date(date.unwrap_or(&now))],
        };

        if images.is_empty() {
            glib::warn!("Source {self:?} returned an empty list of images!");
            Err(SourceError::NoImage)
        } else {
            Ok(images)
        }
    }
}
