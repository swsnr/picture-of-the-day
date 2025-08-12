// Copyright Sebastian Wiesner <sebastian@swsnr.de>
//
// Licensed under the EUPL
//
// See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

use std::borrow::Cow;

use glib::{Priority, dpgettext2};
use jiff::civil::Date;
use serde::{Deserialize, Deserializer, de};
use url::Url;

use crate::{config::G_LOG_DOMAIN, net::http::SoupSessionExt};

use super::super::{DownloadableImage, ImageMetadata, Source, SourceError};

#[derive(Debug, Deserialize)]
struct BingImage {
    title: String,
    copyright: String,
    copyrightlink: String,
    #[serde(deserialize_with = "deserialize_civil_date")]
    startdate: jiff::civil::Date,
    urlbase: String,
}

pub struct BingDateVisitor;

const BING_DATE_FORMAT: &str = "%Y%m%d";

impl de::Visitor<'_> for BingDateVisitor {
    type Value = jiff::civil::Date;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a date in {BING_DATE_FORMAT}")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Date::strptime(BING_DATE_FORMAT, v).map_err(de::Error::custom)
    }
}

fn deserialize_civil_date<'de, D>(d: D) -> Result<jiff::civil::Date, D::Error>
where
    D: Deserializer<'de>,
{
    d.deserialize_str(BingDateVisitor)
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
                let suggested_filename = image_url
                    .query_pairs()
                    .find_map(|(key, value)| (key == "id").then(|| value.into_owned()));
                DownloadableImage {
                    metadata: ImageMetadata {
                        title: image.title,
                        // The copyright fields really seem to be more of a description really
                        url: Some(image.copyrightlink),
                        description: Some(image.copyright),
                        copyright: Some(
                            dpgettext2(None, "source.bing.copyright", "All rights reserved.")
                                .into(),
                        ),
                        source: Source::Bing,
                    },
                    image_url: image_url.into(),
                    pubdate: Some(image.startdate),
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

async fn fetch_daily_bing_images(
    session: &soup::Session,
    language_code: Option<&str>,
) -> Result<BingResponse, SourceError> {
    // n means number of images, we fetch eight,
    // see https://codeberg.org/swsnr/gnome-shell-extension-picture-of-the-day/issues/27
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
    let message = soup::Message::new("GET", &url).unwrap();
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
    let language_code = gnome_app_utils::i18n::locale::language_and_territory_codes().next();
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
    use crate::images::source::testutil::{block_on_new_main_context, soup_session};

    use super::*;

    #[test]
    fn fetch_daily_images() {
        block_on_new_main_context(async {
            let session = soup_session();
            let images = fetch_daily_bing_images(&session, Some("en_GB"))
                .await
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
        });
    }
}
