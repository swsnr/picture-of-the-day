// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use glib::dpgettext2;
use gtk::gio::{self, ResourceLookupFlags, prelude::SettingsExtManual};
use jiff::civil::Date;
use serde::Deserialize;
use std::sync::LazyLock;

use crate::image::{DownloadableImage, ImageMetadata};

#[derive(Debug, Deserialize)]
pub struct Collection {
    pub title: String,
    pub tag: String,
    pub url: String,
    pub images: Vec<String>,
}

#[derive(Debug)]
struct ImageInCollection {
    title: &'static str,
    tag: &'static str,
    url: &'static str,
    image: &'static str,
}

pub static COLLECTIONS: LazyLock<Vec<Collection>> = LazyLock::new(|| {
    let data = gio::resources_lookup_data(
        "/de/swsnr/pictureoftheday/stalenhag/collections.json",
        ResourceLookupFlags::NONE,
    )
    .unwrap();
    serde_json::from_slice(&data).unwrap()
});

// See https://stackoverflow.com/a/38406885
fn some_kind_of_uppercase_first_letter(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

fn pretty_title(title: &str) -> String {
    let no_ext = if let Some((prefix, _)) = title.rsplit_once('.') {
        prefix
    } else {
        title
    };
    no_ext
        .split('_')
        .filter(|s| !s.is_empty())
        .map(some_kind_of_uppercase_first_letter)
        .collect::<Vec<_>>()
        .join(" ")
}

fn enabled_collections() -> impl Iterator<Item = &'static Collection> {
    let settings = crate::config::get_settings();
    let disabled_collections = settings.strv("stalenhag-disabled-collections");
    COLLECTIONS
        .iter()
        .filter(move |collection| !disabled_collections.contains(&collection.tag))
}

fn images(collections: impl Iterator<Item = &'static Collection>) -> Vec<ImageInCollection> {
    collections
        .flat_map(|c| {
            c.images.iter().map(|i| ImageInCollection {
                title: &c.title,
                tag: &c.tag,
                url: &c.url,
                image: i,
            })
        })
        .collect()
}

fn pick_image_for_date(date: Date, images: &[ImageInCollection]) -> DownloadableImage {
    // The 84th anniversary of Georg Elsner's heroic act of resistance against the nazi regime
    let base_date = jiff::civil::date(2023, 11, 8);
    let days = (date - base_date).get_days();
    let index = usize::try_from(days.rem_euclid(i32::try_from(images.len()).unwrap())).unwrap();
    // The modulus above makes sure we don't index out of bounds here
    #[allow(clippy::indexing_slicing)]
    let image = &images[index];
    // The URL of the image is guaranteed to have at least one slash.
    let (_, base_name) = image.image.rsplit_once('/').unwrap();
    let copyright = dpgettext2(None, "source.stalenhag.copyright", "All rights reserved.");
    let description = dpgettext2(None, "source.stalenhag.description", "Collection: %s")
        .replace("%s", image.title);
    DownloadableImage {
        metadata: ImageMetadata {
            title: pretty_title(base_name),
            description: Some(description),
            copyright: Some(copyright.into()),
            url: Some(image.url.to_owned()),
            source: super::Source::Stalenhag,
        },
        image_url: image.image.to_owned(),
        // We do not add a date to the image here, because we cycle through these
        // images and will eventually hit this image again.
        pubdate: None,
        suggested_filename: Some(format!("{}-{base_name}", image.tag)),
    }
}

fn pick_image_for_date_from_collections(
    date: Date,
    collections: impl Iterator<Item = &'static Collection>,
) -> DownloadableImage {
    pick_image_for_date(date, &images(collections))
}

pub fn pick_image_for_date_from_configured_collections(date: Date) -> DownloadableImage {
    pick_image_for_date_from_collections(date, enabled_collections())
}

#[cfg(test)]
mod tests {
    use gtk::gio;
    use jiff::civil::date;

    use crate::source::Source;

    use super::COLLECTIONS;

    #[test]
    fn pick_image_for_date_from_collections() {
        gio::resources_register_include!("pictureoftheday.gresource").unwrap();
        let collections = COLLECTIONS
            .iter()
            .filter(move |collection| collection.tag != "paleo");
        let image = super::pick_image_for_date_from_collections(date(2025, 4, 24), collections);
        let metadata = image.metadata;
        assert_eq!(metadata.title, "Svema 19 Big");
        assert_eq!(metadata.copyright.unwrap(), "All rights reserved.");
        assert_eq!(
            metadata.description.unwrap(),
            "Collection: SWEDISH MACHINES (2024)"
        );
        assert_eq!(
            metadata.url.unwrap(),
            "https://simonstalenhag.se/svema.html"
        );
        assert_eq!(metadata.source, Source::Stalenhag);
        assert!(image.pubdate.is_none());
        assert_eq!(
            image.image_url,
            "https://simonstalenhag.se/4k/svema_19_big.jpg"
        );
        assert_eq!(image.suggested_filename.unwrap(), "svema-svema_19_big.jpg");
    }
}
