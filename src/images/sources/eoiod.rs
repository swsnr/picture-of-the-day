// Copyright Sebastian Wiesner <sebastian@swsnr.de>
//
// Licensed under the EUPL
//
// See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

//! Earth Observatory Image Of The Day

use glib::dpgettext2;
use quick_xml::NsReader;
use soup::prelude::SessionExt;

use crate::{
    net::http::HttpError,
    xml::rss::{RssItem, read_rss_channel},
};

use super::super::{DownloadableImage, ImageMetadata, Source, SourceError};

fn get_feed_message() -> soup::Message {
    soup::Message::new(
        "GET",
        "https://earthobservatory.nasa.gov/feeds/image-of-the-day.rss",
    )
    .unwrap()
}

fn image_from_item(item: RssItem) -> Result<DownloadableImage, SourceError> {
    let title = item
        .title
        .ok_or_else(|| SourceError::ScrapingFailed("Missing title in RSS item".into()))?;
    let metadata = ImageMetadata {
        title,
        description: item.description,
        copyright: Some(
            dpgettext2(None, "source.eoiod.copyright", "NASA Earth Observatory").into(),
        ),
        url: item.link,
        source: Source::Eoiod,
    };
    let thumbnail = item.thumbnail.ok_or_else(|| {
        SourceError::ScrapingFailed(
            "Missing thumbnail in RSS item, cannot construct image URL".into(),
        )
    })?;
    let image = DownloadableImage {
        metadata,
        image_url: thumbnail.replace("_th.", "_lrg."),
        pubdate: item.pubdate.map(|dt| dt.date()),
        suggested_filename: None,
    };
    Ok(image)
}

fn get_first_image_from_feed(xml: &[u8]) -> Result<DownloadableImage, SourceError> {
    if let Some(item) = read_rss_channel(NsReader::from_reader(xml))?.next() {
        Ok(image_from_item(item?)?)
    } else {
        Err(SourceError::NoImage)
    }
}

pub async fn fetch_image_of_the_day(
    session: &soup::Session,
) -> Result<DownloadableImage, SourceError> {
    let message = get_feed_message();
    let body = session
        .send_and_read_future(&message, glib::Priority::DEFAULT)
        .await?;
    if message.status() == soup::Status::Ok {
        get_first_image_from_feed(&body)
    } else {
        Err(HttpError::HttpStatus(message.status(), message.reason_phrase(), body).into())
    }
}
