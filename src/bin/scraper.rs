// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use gtk::gio::Cancellable;
use indexmap::IndexSet;
use serde::Serialize;
use soup::prelude::SessionExt;
use url::Url;

#[derive(Debug, Serialize)]
struct Collection {
    title: &'static str,
    tag: &'static str,
}

const KNOWN_COLLECTIONS: [Collection; 7] = [
    Collection {
        title: "SWEDISH MACHINES (2024)",
        tag: "svema",
    },
    Collection {
        title: "THE LABYRINTH (2020)",
        tag: "labyrinth",
    },
    Collection {
        title: "THE ELECTRIC STATE (2017)",
        tag: "es",
    },
    Collection {
        title: "THINGS FROM THE FLOOD (2016)",
        tag: "tftf",
    },
    Collection {
        title: "TALES FROM THE LOOP (2014)",
        tag: "tftl",
    },
    Collection {
        title: "PALEOART",
        tag: "paleo",
    },
    Collection {
        title: "COMMISSIONS, UNPUBLISHED WORK AND SOLO PIECES",
        tag: "other",
    },
];

#[derive(Debug, Serialize)]
struct CollectionWithImages {
    #[serde(flatten)]
    collection: &'static Collection,
    images: IndexSet<Url>,
    url: Url,
}

fn scrape_collection(
    session: &soup::Session,
    base_url: &Url,
    collection: &'static Collection,
) -> CollectionWithImages {
    let url = base_url.join(&format!("{}.html", collection.tag)).unwrap();
    let message = soup::Message::new("GET", url.as_str()).unwrap();
    let contents = session.send_and_read(&message, Cancellable::NONE).unwrap();
    assert_eq!(message.status(), soup::Status::Ok);
    let body = std::str::from_utf8(&contents).unwrap();
    let document = scraper::Html::parse_document(body);

    let mut images = IndexSet::new();
    for img_a in document.select(&scraper::Selector::parse("a:has(> img)").unwrap()) {
        let href = img_a.attr("href").unwrap();
        if href.ends_with(".jpg") {
            let src = base_url.join(href).unwrap();
            images.insert(src);
        }
    }

    CollectionWithImages {
        collection,
        images,
        url,
    }
}

fn main() {
    let session = soup::Session::new();
    session.set_user_agent(concat!(
        env!("CARGO_PKG_NAME"),
        "/",
        env!("CARGO_PKG_VERSION"),
        " (",
        env!("CARGO_PKG_HOMEPAGE"),
        ")"
    ));
    let base_url = Url::parse("https://simonstalenhag.se/").unwrap();

    let collections = KNOWN_COLLECTIONS
        .iter()
        .map(|c| scrape_collection(&session, &base_url, c))
        .collect::<Vec<_>>();
    println!("{}", serde_json::to_string_pretty(&collections).unwrap())
}
