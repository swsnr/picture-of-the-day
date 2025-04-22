// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

#![allow(unused)]

use gtk::gio;
use scraper::{ElementRef, Html, Node, Selector};
use soup::prelude::SessionExt;

use crate::image::{DownloadableImage, ImageMetadata};

use super::{Source, SourceError};

#[derive(Debug)]
struct ScraperError {
    message: &'static str,
}

impl From<&'static str> for ScraperError {
    fn from(message: &'static str) -> Self {
        Self { message }
    }
}

fn compile_description(paragraphs: &[ElementRef<'_>]) -> String {
    let texts = paragraphs
        .iter()
        .map(|p| p.text().collect::<String>())
        .take_while(|text| !text.trim().starts_with("Related Links"))
        .collect::<Vec<_>>();
    texts.join("\n\n").trim().to_owned()
}

fn extract_copyright_and_description(paragraphs: &[ElementRef<'_>]) -> (Option<String>, String) {
    let copyright_index = paragraphs.iter().position(|element| {
        element
            .text()
            .collect::<String>()
            .trim()
            .starts_with("Photographer:")
    });
    if let Some(copyright_index) = copyright_index {
        // .position() guarantees that `copyright_index` is within bounds.
        #[allow(clippy::indexing_slicing)]
        let copyright_text = paragraphs[copyright_index].text().collect::<String>();
        if let Some(paragraphs) = paragraphs.get((copyright_index + 1)..) {
            (Some(copyright_text), compile_description(paragraphs))
        } else {
            (Some(copyright_text), String::new())
        }
    } else {
        (None, compile_description(paragraphs))
    }
}

/// Replace all `br` tags with line breaks in a document.
///
/// Iterate over all `br` elements in the given `document`, and replace the `br`
/// element with a text node containing a line break.
fn replace_br_with_linebreak(document: &mut Html) {
    let br_node_ids = document
        .select(&Selector::parse("br").unwrap())
        .map(|e| e.id())
        .collect::<Vec<_>>();
    for node_id in br_node_ids {
        let mut node = document.tree.get_mut(node_id).unwrap();
        *node.value() = Node::Text(scraper::node::Text {
            text: "\n".to_owned().into(),
        });
    }
}

fn scrape_page(data: &[u8]) -> Result<Vec<DownloadableImage>, ScraperError> {
    let mut document = Html::parse_document(&String::from_utf8_lossy(data));
    replace_br_with_linebreak(&mut document);

    let header_selector = Selector::parse(".entry > .entry-header").unwrap();
    let header = document
        .select(&header_selector)
        .next()
        .ok_or(".entry > .entry-header not found")?;
    let title = header.text().collect::<String>().trim().to_owned();

    let header_link_selector = Selector::parse(".entry > .entry-header > a").unwrap();
    let url = document
        .select(&header_link_selector)
        .next()
        .and_then(|a| a.attr("href"));

    let body_paragraphs_selector = Selector::parse(".entry .entry-body > p").unwrap();
    let image_paragraph_selector = Selector::parse("p:has(a.asset-img-link)").unwrap();
    let (images, paragraphs) = document
        .select(&body_paragraphs_selector)
        // Skip until we find the first image
        .skip_while(|element| !image_paragraph_selector.matches(element))
        // Take the images and the subsequent text
        .partition::<Vec<_>, _>(|element| image_paragraph_selector.matches(element));

    let (copyright, description) = extract_copyright_and_description(&paragraphs);

    let entry_date_selector = Selector::parse(".entry > .date").unwrap();
    let entry_date = document
        .select(&entry_date_selector)
        .next()
        .ok_or(".entry > .date not found")?
        .text()
        .next()
        .ok_or("No text in .entry > .date")?;
    let pubdate = chrono::NaiveDate::parse_from_str(entry_date.trim(), "%B %d, %Y")
        .map_err(|_| "No valid date in .entry > .date")?;

    let metadata = ImageMetadata {
        title,
        description: Some(description),
        copyright,
        url: url.map(ToOwned::to_owned),
        source: Source::Eopd,
    };

    let image_selector = Selector::parse("a.asset-img-link").unwrap();
    let image_urls = images
        .iter()
        .flat_map(|e| e.select(&image_selector))
        .map(|e| e.attr("href").ok_or("a.asset-img-link had no href"))
        .collect::<Result<Vec<_>, _>>()?;

    let images = image_urls
        .into_iter()
        .map(|image_url| DownloadableImage {
            metadata: metadata.clone(),
            image_url: image_url.to_owned(),
            pubdate: Some(pubdate),
            suggested_filename: None,
        })
        .collect();
    Ok(images)
}

fn get_blog_message() -> soup::Message {
    soup::Message::new("GET", "https://epod.usra.edu/blog/").unwrap()
}

pub async fn fetch_picture_of_the_day(
    session: &soup::Session,
) -> Result<Vec<DownloadableImage>, SourceError> {
    let message = get_blog_message();
    let data = session
        .send_and_read_future(&message, glib::Priority::DEFAULT)
        .await?;
    gio::spawn_blocking(move || scrape_page(&data))
        .await
        .unwrap()
        .map_err(|error| SourceError::ScrapingFailed(error.message.into()))
}

#[cfg(test)]
mod tests {
    use gtk::gio::Cancellable;
    use soup::prelude::SessionExt;

    use crate::{
        image::ImageMetadata,
        source::{Source, testutil::soup_session},
    };

    #[test]
    fn fetch_picture_of_the_day() {
        let session = soup_session();
        let message = super::get_blog_message();
        let data = session.send_and_read(&message, Cancellable::NONE).unwrap();
        let images = super::scrape_page(&data).unwrap();
        assert!(!images.is_empty());
        for image in &images {
            assert_eq!(image.metadata.source, Source::Eopd);
            assert!(image.metadata.url.is_some());
            assert!(image.metadata.copyright.is_some());
            assert!(image.metadata.description.is_some());
            assert!(image.pubdate.is_some());
        }
    }
}
