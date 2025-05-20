// Copyright Sebastian Wiesner <sebastian@swsnr.de>
//
// Licensed under the EUPL
//
// See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

use glib::Priority;
use gtk::gio::prelude::SettingsExt;
use serde::Deserialize;
use url::Url;

use crate::{
    config::G_LOG_DOMAIN,
    net::http::{HttpError, SoupSessionExt},
};

use super::super::{DownloadableImage, ImageMetadata, Source, SourceError};

#[derive(Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
enum MediaType {
    Image,
    Video,
    #[serde(other)]
    Unknown,
}

/// See <https://github.com/nasa/apod-api#endpoint-versionapod>.
#[derive(Debug, Deserialize)]
struct ApodMetadata {
    /// The title of the image.
    title: String,
    /// Date of image. Included in response because of default values.
    date: jiff::civil::Date,
    /// The URL of the APOD image or video of the day.
    url: String,
    /// The URL for any high-resolution image for that day. Returned regardless of 'hd' param setting but will be omitted in the response IF it does not exist originally at APOD.
    hdurl: Option<String>,
    /// The type of media (data) returned. May either be 'image' or 'video' depending on content.
    media_type: MediaType,
    /// The supplied text explanation of the image.
    explanation: String,
    /// The name of the copyright holder.
    copyright: Option<String>,
}

impl TryFrom<ApodMetadata> for DownloadableImage {
    type Error = SourceError;

    fn try_from(metadata: ApodMetadata) -> Result<Self, Self::Error> {
        if let MediaType::Image = metadata.media_type {
            let url_date = &metadata.date.strftime("%y%m%d");
            let url = format!("https://apod.nasa.gov/apod/ap{url_date}.html");
            Ok(DownloadableImage {
                metadata: ImageMetadata {
                    title: metadata.title,
                    description: Some(metadata.explanation),
                    copyright: metadata.copyright,
                    url: Some(url),
                    source: Source::Apod,
                },
                image_url: metadata.hdurl.unwrap_or(metadata.url),
                pubdate: Some(metadata.date),
                suggested_filename: None,
            })
        } else {
            Err(SourceError::NotAnImage)
        }
    }
}

#[derive(Debug, Deserialize)]
struct ApodErrorDetails {
    code: String,
}

#[derive(Debug, Deserialize)]
struct ApodErrorBody {
    error: ApodErrorDetails,
}

fn to_source_error(error: HttpError) -> SourceError {
    if let HttpError::HttpStatus(_, _, data) = &error {
        if let Ok(body) = serde_json::from_slice::<ApodErrorBody>(data) {
            match body.error.code.as_str() {
                "API_KEY_INVALID" => return SourceError::InvalidApiKey,
                "OVER_RATE_LIMIT" => return SourceError::RateLimited,
                _ => (),
            }
        }
    }
    error.into()
}

fn get_metadata_message(date: Option<jiff::civil::Date>, api_key: &str) -> soup::Message {
    let mut url = Url::parse_with_params(
        "https://api.nasa.gov/planetary/apod",
        &[("api_key", api_key)],
    )
    .unwrap();
    if let Some(date) = date {
        url.query_pairs_mut()
            .append_pair("date", &date.strftime("%Y-%m-%d").to_string());
    }
    glib::info!("Querying APOD image metadata from {url}");
    // We can safely unwrap here, because `Url` already guarantees us that `url` is valid
    soup::Message::new("GET", url.as_str()).unwrap()
}

/// Fetch the astronomy picture of the day.
async fn query_metadata(
    session: &soup::Session,
    date: Option<jiff::civil::Date>,
    api_key: &str,
) -> Result<ApodMetadata, SourceError> {
    let message = get_metadata_message(date, api_key);
    session
        .send_and_read_json::<ApodMetadata>(&message, Priority::DEFAULT)
        .await
        .map_err(to_source_error)
}

pub async fn fetch_picture_of_the_day(
    session: &soup::Session,
    date: Option<jiff::civil::Date>,
) -> Result<DownloadableImage, SourceError> {
    let settings = crate::config::get_settings();
    let api_key = settings.string("apod-api-key");
    query_metadata(session, date, &api_key).await?.try_into()
}

#[cfg(test)]
mod tests {
    use gtk::gio::Cancellable;
    use soup::prelude::SessionExt;

    use super::*;
    use crate::images::source::testutil::soup_session;

    #[test]
    fn fetch_apod() {
        // We use a separate API key for testing, with account ID 431bacf7-4e26-407f-9ca3-06a17d8d7400
        let api_key = "74AFPeibYGYI13Efz7MrgtjJ1ozN3etA1Ggt87r6";
        // See https://apod.nasa.gov/apod/ap250327.html
        let date = jiff::civil::date(2025, 3, 27);
        let message = get_metadata_message(Some(date), api_key);
        let response = soup_session()
            .send_and_read(&message, Cancellable::NONE)
            .unwrap();
        assert_eq!(message.status(), soup::Status::Ok);

        let metadata = serde_json::from_slice::<ApodMetadata>(&response).unwrap();
        let image = DownloadableImage::try_from(metadata).unwrap();
        let metadata = image.metadata;

        assert_eq!(metadata.title, "Messier 81");
        assert!(
            metadata
                .description
                .as_ref()
                .unwrap()
                .starts_with("One of the brightest galaxies"),
            "{:?}",
            &metadata.description
        );
        assert_eq!(metadata.copyright.unwrap(), "Lorand Fenyes");
        assert_eq!(
            metadata.url.unwrap(),
            "https://apod.nasa.gov/apod/ap250327.html"
        );
        assert_eq!(metadata.source, Source::Apod);
        assert_eq!(
            image.image_url,
            "https://apod.nasa.gov/apod/image/2503/291_lorand_fenyes_m81_kicsi.jpg"
        );
        assert_eq!(image.pubdate.unwrap(), date);
        assert!(image.suggested_filename.is_none());
    }
}
