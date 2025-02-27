// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::borrow::Cow;

use glib::Priority;
use serde::Deserialize;
use soup::prelude::SessionExt;
use url::Url;

use crate::{
    config::G_LOG_DOMAIN,
    image::{DownloadableImage, ImageMetadata},
};

use super::{Source, SourceError};

#[derive(Debug, Deserialize)]
struct BingImage {
    title: String,
    copyright: String,
    copyrightlink: String,
    startdate: String,
    urlbase: String,
}

#[derive(Debug, Deserialize)]
struct BingResponse {
    images: Vec<BingImage>,
}

async fn fetch_daily_bing_images(session: &soup::Session) -> Result<BingResponse, SourceError> {
    // n means number of images, we fetch eight,
    // see https://github.com/swsnr/gnome-shell-extension-picture-of-the-day/issues/27
    let url = "https://www.bing.com/HPImageArchive.aspx?format=js&idx=0&n=8";
    // Bing has locale-dependent images; we take the current locale for this GNOME
    // shell process, and turn it into a format Bing understands (no encoding, and
    // no underscores).
    //
    // With an invalid locale bing seems to fall back to geo-IP, and return an
    // image for the geopgraphic location of the user.
    let locale = glib::language_names_with_category("LC_MESSAGES")
        .first()
        .and_then(|l| l.split_once('.'))
        .map(|(h, _)| h)
        .map(|s| s.replace('_', "-"));
    let url = if let Some(locale) = locale {
        Cow::Owned(format!(
            "{url}&mkt={}",
            glib::Uri::escape_string(&locale, None, false)
        ))
    } else {
        Cow::Borrowed(url)
    };
    glib::debug!("Querying latest bing images from {url}");
    let message = soup::Message::new("GET", &url).unwrap();
    let body = session
        .send_and_read_future(&message, Priority::DEFAULT)
        .await?;
    if message.status() == soup::Status::Ok {
        Ok(serde_json::from_slice(&body)?)
    } else {
        Err(SourceError::HttpStatus(
            message.status(),
            message.reason_phrase(),
        ))
    }
}

pub async fn fetch_daily_images(
    session: &soup::Session,
) -> Result<Vec<DownloadableImage>, SourceError> {
    let images = fetch_daily_bing_images(session).await?.images;
    if images.is_empty() {
        glib::warn!("No images received from bing!");
        return Err(SourceError::NoImage);
    }
    let images = images
        .into_iter()
        .map(|image| {
            let image_url = Url::parse("https://www.bing.com")
                .unwrap()
                .join(&format!("{}_UHD.jpg", &image.urlbase))
                // TODO: Log error and skip this image!
                .unwrap();
            let pubdate = format!(
                "{}-{}-{}",
                &image.startdate[0..4],
                &image.startdate[4..6],
                &image.startdate[6..]
            );
            let suggested_filename = image_url
                .query_pairs()
                .find_map(|(key, value)| (key == "id").then(|| value.into_owned()));
            DownloadableImage {
                metadata: ImageMetadata {
                    title: image.title,
                    // The copyright fields really seem to be more of a description really
                    url: Some(image.copyrightlink),
                    description: Some(image.copyright),
                    copyright: None,
                    source: Source::Bing,
                },
                image_url: image_url.into(),
                pubdate: Some(pubdate),
                suggested_filename,
            }
        })
        .collect();

    Ok(images)
}
