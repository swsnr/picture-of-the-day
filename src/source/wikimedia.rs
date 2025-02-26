// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use glib::{Priority, dpgettext2};
use serde::Deserialize;
use soup::prelude::SessionExt;

use crate::config::G_LOG_DOMAIN;
use crate::image::{DownloadableImage, ImageMetadata};
use crate::source::SourceError;

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

fn create_message(date: &glib::DateTime, language_code: &str) -> soup::Message {
    let url_date = date.format("%Y/%m/%d").unwrap();
    let url =
        format!("https://api.wikimedia.org/feed/v1/wikipedia/{language_code}/featured/{url_date}");
    soup::Message::new("GET", &url).unwrap()
}

async fn fetch_featured_content(
    session: &soup::Session,
    date: &glib::DateTime,
    language_code: &str,
) -> Result<FeaturedContent, SourceError> {
    let message = create_message(date, language_code);
    glib::info!(
        "Fetching featured wikimedia content from {}",
        message.uri().unwrap()
    );
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
) -> Result<DownloadableImage, SourceError> {
    let locales = glib::language_names_with_category("LC_MESSAGES");
    let language_code = locales
        .first()
        .map_or("en", |l| l.split('_').next().unwrap());
    fetch_featured_image_at_date(
        session,
        &glib::DateTime::now_local().unwrap(),
        language_code,
    )
    .await
}

#[cfg(test)]
mod tests {
    use gtk::gio::Cancellable;
    use soup::prelude::SessionExt;

    use crate::image::DownloadableImage;

    use super::{FeaturedContent, create_message};

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

    #[test]
    fn image_from_featured_content() {
        // We use deliberately use the sync API here and test the individual parts
        // separately, because the async soup API suffers from weird deadlocks
        // when used with separate main contexts.
        let session = soup::Session::new();
        let date = glib::DateTime::new(&glib::TimeZone::utc(), 2025, 2, 21, 12, 0, 0.0).unwrap();
        let message = create_message(&date, "en");
        let response = session.send_and_read(&message, Cancellable::NONE).unwrap();
        assert_eq!(message.status(), soup::Status::Ok);

        let featured_content = serde_json::from_slice::<FeaturedContent>(&response).unwrap();
        let image = DownloadableImage::from(featured_content.image.unwrap()).with_pubdate(&date);

        assert_eq!(
            image.metadata.title,
            "Bicudas (Sphyraena viridensis), Cabo de Palos, España, 2022-07-15, DD 09"
        );
        assert_eq!(
            image.metadata.description.unwrap(),
            "Yellowmouth barracudas (Sphyraena viridensis), Cabo de Palos, Spain"
        );
        assert_eq!(
            image.metadata.copyright.unwrap(),
            "Diego Delso (Own work, CC BY-SA 4.0)"
        );
        assert_eq!(
            image.metadata.url.unwrap(),
            "https://commons.wikimedia.org/wiki/File:Bicudas_(Sphyraena_viridensis),_Cabo_de_Palos,_Espa%C3%B1a,_2022-07-15,_DD_09.jpg"
        );

        assert_eq!(
            image.image_url,
            "https://upload.wikimedia.org/wikipedia/commons/7/70/Bicudas_%28Sphyraena_viridensis%29%2C_Cabo_de_Palos%2C_Espa%C3%B1a%2C_2022-07-15%2C_DD_09.jpg"
        );
        assert_eq!(image.pubdate.unwrap(), "2025-02-21");
    }
}
