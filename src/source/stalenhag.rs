// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::sync::LazyLock;

use glib::dpgettext2;
use gtk::gio::{self, ResourceLookupFlags, prelude::SettingsExtManual};
use serde::Deserialize;

use crate::image::{DownloadableImage, ImageMetadata};

#[derive(Debug, Deserialize)]
pub struct Image {
    pub src: String,
}

#[derive(Debug, Deserialize)]
pub struct Collection {
    pub title: String,
    pub tag: String,
    pub url: String,
    pub images: Vec<Image>,
}

#[derive(Debug)]
struct ImageInCollection {
    title: &'static str,
    tag: &'static str,
    url: &'static str,
    image: &'static Image,
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

pub fn pick_image_for_date(date: &glib::DateTime) -> DownloadableImage {
    let all_images = images(enabled_collections());
    // The 84th anniversary of Georg Elsner's heroic act of resistance against the nazi regime
    let base_date = glib::DateTime::from_local(2023, 11, 8, 21, 20, 0.0).unwrap();
    let days = date.difference(&base_date).as_days();
    let index = usize::try_from(days.rem_euclid(i64::try_from(all_images.len()).unwrap())).unwrap();
    // The modulus above makes sure we don't index out of bounds here
    #[allow(clippy::indexing_slicing)]
    let image = &all_images[index];
    // The URL of the image is guaranteed to have at least one slash.
    let (_, base_name) = image.image.src.rsplit_once('/').unwrap();
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
        image_url: image.image.src.clone(),
        // We do not add a date to the image here, because we cycle through these
        // images and will eventually hit this image again.
        pubdate: None,
        suggested_filename: Some(format!("{}-{base_name}", image.tag)),
    }
}
