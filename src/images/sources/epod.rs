// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use gtk::gio;
use scraper::{ElementRef, Html, Node, Selector};
use soup::prelude::SessionExt;

use super::super::{DownloadableImage, ImageMetadata, Source, SourceError};

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
    let image_paragraph_selector =
        Selector::parse("p:has(a.asset-img-link), p:has(img:only-child)").unwrap();
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
    let pubdate = jiff::civil::Date::strptime("%B %d, %Y", entry_date.trim())
        .map_err(|_| "No valid date in .entry > .date")?;

    let metadata = ImageMetadata {
        title,
        description: Some(description),
        copyright,
        url: url.map(ToOwned::to_owned),
        source: Source::Eopd,
    };

    let asset_link_selector = Selector::parse("a.asset-img-link").unwrap();
    let image_urls = {
        let asset_link_hrefs = images
            .iter()
            .flat_map(|e| e.select(&asset_link_selector))
            .map(|e| e.attr("href").ok_or("a.asset-img-link had no href"))
            .collect::<Result<Vec<_>, _>>()?;
        if asset_link_hrefs.is_empty() {
            // If we had no asset links to high-res images, look images themselves
            let image_selector = Selector::parse("img:only-child").unwrap();
            images
                .iter()
                .flat_map(|e| e.select(&image_selector))
                .map(|e| e.attr("src").ok_or("img:only-child had no src"))
                .collect::<Result<Vec<_>, _>>()?
        } else {
            asset_link_hrefs
        }
    };

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
    use jiff::civil::date;
    use soup::prelude::SessionExt;

    use crate::images::source::testutil::soup_session;

    use super::*;

    fn scrape_page_at_url(url: &str) -> Vec<DownloadableImage> {
        let session = soup_session();
        let message = soup::Message::new("GET", url).unwrap();
        let data = session.send_and_read(&message, Cancellable::NONE).unwrap();
        scrape_page(&data).unwrap()
    }

    #[test]
    fn scrape_page_with_asset_link_and_photographer() {
        let mut images = scrape_page_at_url(
            "https://epod.usra.edu/blog/2025/01/aurora-borealis-and-an-east-west-oriented-arc-september-1314-2024.html",
        );
        let image = images.pop().unwrap();
        assert!(images.is_empty(), "More than one image returned!");

        let metadata = image.metadata;
        assert_eq!(
            metadata.title,
            "Aurora Borealis and an East-West Oriented Arc"
        );
        assert_eq!(
            metadata.copyright.unwrap(),
            "Photographer:\u{a0}Geir T. Birkeland Øye\u{a0}
Summary Authors:\u{a0}Geir T. Birkeland Øye, Jim Foster"
        );
        assert_eq!(metadata.source, Source::Eopd);
        assert_eq!(
            metadata.description.unwrap(),
            "\
The\u{a0}northern lights display shown above was observed from Ørsta, Norway \
on the night of September 13/14, 2024. It was indeed a colorful display, \
captured between clouds and above a developing fog. However, the most \
interesting feature was the east-west oriented arc at right, referred to as \
a\u{a0}STEVE, an acronym for\u{a0}Strong Thermal Emission Velocity \
Enhancements. It formed at about 10:25 p.m. local time (UTC 21.25) and lasted \
approximately 11 minutes. It was then followed by an intense aurora. This \
splendid arc appeared in the same portion of the sky where\u{a0}I've \
previously seen the STEVE phenomenon.\u{a0}

Photo Details: Canon 650D camera; Samyang 8 mm fisheye-lens.

\u{a0}

Ørsta, Norway Coordinates: 62.2611, 6.2922"
        );
        assert!(metadata.url.is_none());

        assert_eq!(
            image.image_url,
            "https://epod.usra.edu/.a/6a0105371bb32c970b02c8d3c1bc3f200c-pi"
        );
        assert_eq!(image.pubdate.unwrap(), date(2025, 1, 3));
    }

    #[test]
    fn scrape_page_without_asset_link_and_copyright() {
        let mut images = scrape_page_at_url(
            "https://epod.usra.edu/blog/2025/04/archive-earth-day-and-red-deer-bridge.html",
        );

        let image = images.pop().unwrap();
        assert!(images.is_empty(), "More than one image returned!");

        let metadata = image.metadata;
        assert_eq!(metadata.title, "Archive - Earth Day and Red Deer Bridge");
        assert!(metadata.copyright.is_none(),);
        assert_eq!(metadata.source, Source::Eopd);
        assert_eq!(
            metadata.description.unwrap(),
            "\
This EPOD was originally published April 22, 2005

Provided by: Peg Zenko
Summary authors & editors: Peg Zenko

The photo above was taken on October 3, 2004 and shows the Red Deer River and \
bridge at Yaha Tinda Ranch on the border of Banff National Park in Alberta, \
Canada. Yaha Tinda is cited as one of the last remaining unspoiled mountain \
elk habitats in Alberta, containing miles of beautiful winding horse trails. \
These trails have a fair amount of traffic from recreational riders and \
hunters, but everyone is very respectful of keeping it pristine. Our 15-mile \
(24 km) trek took place on an absolutely stunning day. I've never been \
anywhere that equals it for overall beauty, even Glacier National Park (in \
Montana) or Jasper National Park (also in Alberta).

In the 1960s, former U.S. Senator Gaylord Nelson first proposed that there \
should be a designated day set aside to raise the concern about environmental \
issues and to consciously conserve our natural resources. The very first Earth \
Day was celebrated on April 22, 1970, so today (April 22, 2025) we're \
celebrating its 55th observance. Note that while Earth Day is always on April \
22, International Earth Day occurs on the day of the Vernal Equinox."
        );
        assert!(metadata.url.is_none());

        assert_eq!(
            image.image_url,
            "https://epod.typepad.com/.a/6a0105371bb32c970b01157114b027970c-750wi"
        );
        assert_eq!(image.pubdate.unwrap(), date(2025, 4, 22));
    }

    #[test]
    fn fetch_picture_of_the_day() {
        let session = soup_session();
        let message = get_blog_message();
        let data = session.send_and_read(&message, Cancellable::NONE).unwrap();
        let images = scrape_page(&data).unwrap();

        assert!(!images.is_empty());
        for image in &images {
            assert_eq!(image.metadata.source, Source::Eopd);
            assert!(image.metadata.url.is_some());
            assert!(image.metadata.description.is_some());
            assert!(image.pubdate.is_some());
        }
    }
}
