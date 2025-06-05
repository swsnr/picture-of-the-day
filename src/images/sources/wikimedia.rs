// Copyright Sebastian Wiesner <sebastian@swsnr.de>
//
// Licensed under the EUPL
//
// See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

//! Get the Wikimedia Picture of the Day.
//!
//! See <https://commons.m.wikimedia.org/wiki/Commons:Picture_of_the_day>.

use glib::{Priority, dpgettext2};
use jiff::civil::Date;
use serde::Deserialize;

use crate::config::G_LOG_DOMAIN;
use crate::net::http::SoupSessionExt;

use super::super::{DownloadableImage, ImageMetadata, Source, SourceError};

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
                source: Source::Wikimedia,
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

/// Create a [`soup::Message`] to get featured content.
///
/// Create a [`soup::Message`] to get the content featured at the given `date`,
/// on the Wikipedia for the given ISO `language_code`.
fn get_feature_content_message(date: Date, language_code: &str) -> soup::Message {
    let url_date = date.strftime("%Y/%m/%d");
    let url = format!("https://{language_code}.wikipedia.org/api/rest_v1/feed/featured/{url_date}");
    soup::Message::new("GET", &url).unwrap()
}

async fn fetch_featured_content(
    session: &soup::Session,
    date: Date,
    language_code: &str,
) -> Result<FeaturedContent, SourceError> {
    let message = get_feature_content_message(date, language_code);
    glib::info!(
        "Fetching featured wikimedia content from {}",
        message.uri().unwrap()
    );
    Ok(session
        .send_and_read_json(&message, Priority::DEFAULT)
        .await?)
}

async fn fetch_featured_image_at_date(
    session: &soup::Session,
    date: Date,
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
    date: Date,
) -> Result<DownloadableImage, SourceError> {
    let language_code = gnome_app_utils::i18n::locale::language_codes().next();
    // Default to English wikimedia if we cannot derive a language from the locale environment.
    let language_code = language_code.as_ref().map_or("en", |s| s.as_str());
    fetch_featured_image_at_date(session, date, language_code).await
}

#[cfg(test)]
mod tests {
    use gtk::gio::Cancellable;
    use jiff::civil::date;
    use soup::prelude::SessionExt;

    use crate::images::source::testutil::soup_session;

    use super::*;

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
    fn featured_image() {
        // See https://commons.m.wikimedia.org/wiki/Template:Potd/2025-03#/media/File%3AGeorge_Sand_by_Nadar%2C_1864.jpg
        let date = date(2025, 3, 8);
        let message = get_feature_content_message(date, "en");
        let response = soup_session()
            .send_and_read(&message, Cancellable::NONE)
            .unwrap();
        assert_eq!(message.status(), soup::Status::Ok);

        let content = serde_json::from_slice::<FeaturedContent>(&response).unwrap();
        let image = content.image.as_ref().unwrap();
        assert_eq!(image.title, "File:George Sand by Nadar, 1864.jpg");
        assert_eq!(
            image.description.as_ref().unwrap().text,
            "Portrait of French author George Sand by photographer Nadar in 1864. \
One of the most popular writers in Europe in her lifetime, she stood up for women, \
criticized marriage and fought against the prejudices of a conservative society."
        );
        assert_eq!(
            image.credit.as_ref().unwrap().text,
            "Galerie Contemporaine, 126 boulevard Magenta, Paris - Photographe Goupil [et] C° \
- Cliché Nadar, 51 rue d'Anjou-Saint-Honoré à Paris.Photographie de George Sand sur \
le site Gallica.Ministère de la Culture (France) - Médiathèque de l'architecture et \
du patrimoine.Diffusion Réunion des musées nationaux."
        );
        assert_eq!(
            image.license.as_ref().unwrap().r#type.as_ref().unwrap(),
            "Public domain"
        );
        assert_eq!(
            &image.file_page,
            "https://commons.wikimedia.org/wiki/File:George_Sand_by_Nadar,_1864.jpg"
        );

        let image = DownloadableImage::from(content.image.unwrap());
        assert_eq!(image.metadata.title, "George Sand by Nadar, 1864");
        assert_eq!(
            image.metadata.description.unwrap(),
            "Portrait of French author George Sand by photographer Nadar in 1864. \
One of the most popular writers in Europe in her lifetime, she stood up for women, \
criticized marriage and fought against the prejudices of a conservative society."
        );
        assert_eq!(
            image.metadata.copyright.unwrap(),
            "Nadar (Galerie Contemporaine, 126 boulevard Magenta, Paris - \
Photographe Goupil [et] C° - Cliché Nadar, 51 rue d'Anjou-Saint-Honoré à Paris.\
Photographie de George Sand sur le site Gallica.Ministère de la Culture (France) - \
Médiathèque de l'architecture et du patrimoine.Diffusion Réunion des musées nationaux., \
Public domain)"
        );
        assert_eq!(
            image.metadata.url.unwrap(),
            "https://commons.wikimedia.org/wiki/File:George_Sand_by_Nadar,_1864.jpg"
        );
        assert_eq!(image.metadata.source, Source::Wikimedia);

        assert_eq!(
            image.image_url,
            "https://upload.wikimedia.org/wikipedia/commons/5/54/George_Sand_by_Nadar%2C_1864.jpg"
        );
        assert!(image.pubdate.is_none());
        assert!(image.suggested_filename.is_none());
    }
}
