// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::borrow::Cow;

use glib::Priority;
use serde::Deserialize;
use url::Url;

use crate::{
    config::G_LOG_DOMAIN,
    image::{DownloadableImage, ImageMetadata},
    source::http::SoupSessionExt,
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

fn bing_base_url() -> Url {
    // This will never panic because the URL is valid
    Url::parse("https://www.bing.com").unwrap()
}

impl TryFrom<BingImage> for DownloadableImage {
    type Error = url::ParseError;

    fn try_from(image: BingImage) -> Result<Self, Self::Error> {
        let urlbase = format!("{}_UHD.jpg", &image.urlbase);
        bing_base_url()
            .join(&urlbase)
            .map(|image_url| {
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
            .inspect_err(|error| {
                glib::error!(
                    "Failed to compile image URL from {urlbase}, skipping this image: {error}"
                );
            })
    }
}

#[derive(Debug, Deserialize)]
struct BingResponse {
    images: Vec<BingImage>,
}

fn get_daily_bing_images_message(language_code: Option<&str>) -> soup::Message {
    // n means number of images, we fetch eight,
    // see https://github.com/swsnr/gnome-shell-extension-picture-of-the-day/issues/27
    let url = "https://www.bing.com/HPImageArchive.aspx?format=js&idx=0&n=8";
    // Bing has locale-dependent images; we take the current locale for this GNOME
    // shell process, and turn it into a format Bing understands (no encoding, and
    // no underscores).
    //
    // With an invalid locale bing seems to fall back to geo-IP, and return an
    // image for the geopgraphic location of the user.
    let locale = language_code.map(|c| c.replace('_', "-"));
    let url = if let Some(locale) = locale {
        Cow::Owned(format!(
            "{url}&mkt={}",
            glib::Uri::escape_string(&locale, None, false)
        ))
    } else {
        Cow::Borrowed(url)
    };
    soup::Message::new("GET", &url).unwrap()
}

async fn fetch_daily_bing_images(
    session: &soup::Session,
    language_code: Option<&str>,
) -> Result<BingResponse, SourceError> {
    let message = get_daily_bing_images_message(language_code);
    glib::debug!(
        "Querying latest bing images from {}",
        message.uri().unwrap()
    );
    Ok(session
        .send_and_read_json(&message, Priority::DEFAULT)
        .await?)
}

pub async fn fetch_daily_images(
    session: &soup::Session,
) -> Result<Vec<DownloadableImage>, SourceError> {
    let language_code = crate::locale::language_and_territory_codes().next();
    let images = fetch_daily_bing_images(session, language_code.as_deref())
        .await?
        .images;
    if images.is_empty() {
        glib::warn!("No images received from bing!");
        return Err(SourceError::NoImage);
    }
    Ok(images
        .into_iter()
        .filter_map(|image| DownloadableImage::try_from(image).ok())
        .collect())
}

#[cfg(test)]
mod tests {
    use gtk::gio::Cancellable;
    use soup::prelude::SessionExt;

    use crate::{image::DownloadableImage, source::testutil::soup_session};

    use super::BingResponse;

    #[test]
    fn fetch_daily_images() {
        let message = super::get_daily_bing_images_message(Some("en_GB"));
        let response = soup_session()
            .send_and_read(&message, Cancellable::NONE)
            .unwrap();
        assert_eq!(message.status(), soup::Status::Ok);
        let images = serde_json::from_slice::<BingResponse>(&response)
            .unwrap()
            .images;
        assert_eq!(images.len(), 8);

        let images = images
            .into_iter()
            .map(DownloadableImage::try_from)
            .map(Result::unwrap)
            .collect::<Vec<_>>();
        for image in images {
            assert!(image.pubdate.is_some());
            assert!(image.suggested_filename.is_some());
        }
    }
}
