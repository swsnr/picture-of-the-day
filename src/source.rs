// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Sources of wallpapers.

// See <https://github.com/gtk-rs/gtk-rs-core/discussions/1625>
#![allow(clippy::as_conversions)]

use glib::{GString, dpgettext2};
use std::path::PathBuf;

use crate::config::G_LOG_DOMAIN;
use crate::image::DownloadableImage;

mod error;

mod apod;
mod bing;
mod eoiod;
mod epod;
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
    Eopd,
    Eoiod,
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
            Source::Apod => dpgettext2(None, "source name", "NASA Astronomy Picture of the Day"),
            Source::Bing => dpgettext2(None, "source name", "Bing"),
            Source::Wikimedia => dpgettext2(None, "source name", "Wikimedia Picture of the Day"),
            Source::Stalenhag => dpgettext2(None, "source name", "Simon Stålenhag"),
            Source::Eopd => dpgettext2(None, "source name", "Earth Science Picture of the Day"),
            Source::Eoiod => dpgettext2(
                None,
                "source name",
                "NASA Earth Observatory Image of the Day",
            ),
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
            Source::Eopd => "https://epod.usra.edu/blog/",
            Source::Eoiod => "https://earthobservatory.nasa.gov",
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
        date: Option<jiff::civil::Date>,
    ) -> Result<Vec<DownloadableImage>, SourceError> {
        let today = crate::date::today_local();
        let images = match self {
            Source::Apod => vec![apod::fetch_picture_of_the_day(session, date).await?],
            Source::Bing => {
                date.inspect(|_| {
                    glib::warn!("Bing does not support overriding the date");
                });
                bing::fetch_daily_images(session).await?
            }
            Source::Wikimedia => {
                vec![wikimedia::fetch_featured_image(session, date.unwrap_or(today)).await?]
            }
            Source::Stalenhag => vec![stalenhag::pick_image_for_date(date.unwrap_or(today))],
            Source::Eopd => {
                date.inspect(|_| {
                    glib::warn!(
                        "Earth Science Picture of the Day does not support overriding the date"
                    );
                });
                epod::fetch_picture_of_the_day(session).await?
            }
            Source::Eoiod => {
                date.inspect(|_| {
                    glib::warn!(
                        "NASA Earth Observatory Image of the Day does not support overriding the date"
                    );
                });
                vec![eoiod::fetch_image_of_the_day(session).await?]
            }
        };

        if images.is_empty() {
            glib::warn!("Source {self:?} returned an empty list of images!");
            Err(SourceError::NoImage)
        } else {
            Ok(images)
        }
    }
}
