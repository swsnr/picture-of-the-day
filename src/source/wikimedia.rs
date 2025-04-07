// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use glib::{Priority, dpgettext2};
use serde::Deserialize;

use crate::config::G_LOG_DOMAIN;
use crate::image::{DownloadableImage, ImageMetadata};
use crate::source::SourceError;
use crate::source::http::SoupSessionExt;

#[derive(Debug, Deserialize)]
struct FeaturedImageImage {
    source: String,
}

#[derive(Debug, Deserialize)]
struct FeaturedImageArtist {
    text: String,
}

#[derive(Debug, Deserialize)]
struct FeaturedImageCredit {
    text: String,
}

#[derive(Debug, Deserialize)]
struct FeaturedImageLicense {
    r#type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FeaturedImageDescription {
    text: String,
}

#[derive(Debug, Deserialize)]
struct FeaturedImage {
    title: String,
    image: FeaturedImageImage,
    file_page: String,
    artist: Option<FeaturedImageArtist>,
    credit: Option<FeaturedImageCredit>,
    license: Option<FeaturedImageLicense>,
    description: Option<FeaturedImageDescription>,
}

impl FeaturedImage {
    fn copyright(&self) -> String {
        let artist = self.artist.as_ref().map(|a| &a.text);
        let license = self.license.as_ref().and_then(|l| l.r#type.as_ref());
        let credit = self.credit.as_ref().map(|c| &c.text);
        match (artist, license, credit) {
            (Some(artist), Some(license), Some(credit)) => dpgettext2(
                None,
                "source.wikimedia.copyright",
                "%artist (%credit, %license)",
            )
            .replace("%artist", artist)
            .replace("%credit", credit)
            .replace("%license", license),
            (Some(artist), Some(license), _) => {
                dpgettext2(None, "source.wikimedia.copyright", "%artist (%license)")
                    .replace("%artist", artist)
                    .replace("%license", license)
            }
            (Some(artist), None, _) => artist.clone(),
            (None, Some(license), _) => license.clone(),
            _ => dpgettext2(
                None,
                "source.wikimedia.copyright",
                "Unknown, all rights reserved",
            )
            .into(),
        }
    }

    fn pretty_title(&self) -> &str {
        cleanup_title(&self.title)
    }
}

impl From<FeaturedImage> for DownloadableImage {
    fn from(image: FeaturedImage) -> Self {
        let title = image.pretty_title().to_owned();
        let copyright = Some(image.copyright());
        let url = Some(image.file_page);
        let image_url = image.image.source;
        let description = image.description.map(|s| s.text);
        DownloadableImage {
            metadata: ImageMetadata {
                title,
                description,
                copyright,
                url,
                source: super::Source::Wikimedia,
            },
            image_url,
            pubdate: None,
            suggested_filename: None,
        }
    }
}

#[derive(Debug, Deserialize)]
struct FeaturedContent {
    image: Option<FeaturedImage>,
}

/// Clean up title of image, by removing the `File:` prefix and the extension.
fn cleanup_title(title: &str) -> &str {
    let no_prefix = title.strip_prefix("File:").unwrap_or(title);
    if let Some((before_extension, _)) = no_prefix.rsplit_once('.') {
        before_extension
    } else {
        no_prefix
    }
}

async fn fetch_featured_content(
    session: &soup::Session,
    date: &glib::DateTime,
    language_code: &str,
) -> Result<FeaturedContent, SourceError> {
    let url_date = date.format("%Y/%m/%d").unwrap();
    let url = format!("https://{language_code}.wikipedia.org/api/rest_v1/feed/featured/{url_date}");
    glib::info!("Fetching featured wikimedia content from {url}");
    let message = soup::Message::new("GET", &url).unwrap();
    Ok(session
        .send_and_read_json(&message, Priority::DEFAULT)
        .await?)
}

async fn fetch_featured_image_at_date(
    session: &soup::Session,
    date: &glib::DateTime,
    language_code: &str,
) -> Result<DownloadableImage, SourceError> {
    let content = fetch_featured_content(session, date, language_code).await?;
    if let Some(image) = content.image {
        glib::info!("Wikimedia provided featured image from {}", image.file_page);
        Ok(DownloadableImage::from(image).with_pubdate(date))
    } else {
        glib::warn!("Wikimedia returned featured content without a featured image!");
        Err(SourceError::NoImage)
    }
}

pub async fn fetch_featured_image(
    session: &soup::Session,
    date: &glib::DateTime,
) -> Result<DownloadableImage, SourceError> {
    let language_code = crate::locale::language_codes().next();
    // Default to English wikimedia if we cannot derive a language from the locale environment.
    let language_code = language_code.as_ref().map_or("en", |s| s.as_str());
    fetch_featured_image_at_date(session, date, language_code).await
}

#[cfg(test)]
mod tests {
    #[test]
    fn cleanup_title() {
        let s = super::cleanup_title(
            "File:Old peasant with dagger and long smoking pipe, Mestia, Svanetia, Georgia (Republic).jpg",
        );
        assert_eq!(
            s,
            "Old peasant with dagger and long smoking pipe, Mestia, Svanetia, Georgia (Republic)"
        );
    }
}
