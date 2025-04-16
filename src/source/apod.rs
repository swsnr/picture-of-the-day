// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use chrono::NaiveDate;
use glib::Priority;
use gtk::gio::prelude::SettingsExt;
use serde::Deserialize;
use url::Url;

use crate::{
    config::G_LOG_DOMAIN,
    image::{DownloadableImage, ImageMetadata},
    source::{
        SourceError,
        http::{HttpError, SoupSessionExt},
    },
};

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
    date: String,
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

/// Fetch the astronomy picture of the day.
async fn query_metadata(
    session: &soup::Session,
    date: Option<NaiveDate>,
    api_key: &str,
) -> Result<ApodMetadata, SourceError> {
    let mut url = Url::parse_with_params(
        "https://api.nasa.gov/planetary/apod",
        &[("api_key", api_key)],
    )
    .unwrap();
    if let Some(date) = date {
        url.query_pairs_mut()
            .append_pair("date", &date.format("%Y-%m-%d").to_string());
    }
    glib::info!("Querying APOD image metadata from {url}");
    // We can safely unwrap here, because `Url` already guarantees us that `url` is valid
    let message = soup::Message::new("GET", url.as_str()).unwrap();

    session
        .send_and_read_json::<ApodMetadata>(&message, Priority::DEFAULT)
        .await
        .map_err(to_source_error)
}

async fn fetch_apod(
    session: &soup::Session,
    date: Option<NaiveDate>,
    api_key: &str,
) -> Result<DownloadableImage, SourceError> {
    let metadata = query_metadata(session, date, api_key).await?;
    let url_date = &metadata.date.replace('-', "")[2..];
    let url = format!("https://apod.nasa.gov/apod/ap{url_date}.html");
    if let MediaType::Image = metadata.media_type {
        Ok(DownloadableImage {
            metadata: ImageMetadata {
                title: metadata.title,
                description: Some(metadata.explanation),
                copyright: metadata.copyright,
                url: Some(url),
                source: super::Source::Apod,
            },
            image_url: metadata.hdurl.unwrap_or(metadata.url),
            pubdate: Some(metadata.date),
            suggested_filename: None,
        })
    } else {
        Err(SourceError::NotAnImage)
    }
}

pub async fn fetch_picture_of_the_day(
    session: &soup::Session,
    date: Option<NaiveDate>,
) -> Result<DownloadableImage, SourceError> {
    let settings = crate::config::get_settings();
    let api_key = settings.string("apod-api-key");
    fetch_apod(session, date, &api_key).await
}
