// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use gtk::gio::{self, Cancellable, prelude::FileExt};

use crate::{config::G_LOG_DOMAIN, image::download::download_file, source::Source};

pub mod download;

/// Metadata of an image.
#[derive(Debug, Eq, PartialEq)]
pub struct ImageMetadata {
    /// The image title.
    pub title: String,
    /// The image description.
    pub description: Option<String>,
    /// Copyright information, if known.
    pub copyright: Option<String>,
    /// The direct URL for this image.
    pub url: Option<String>,
    /// The source this image comes from.
    pub source: Source,
}

#[derive(Debug)]
pub struct DownloadableImage {
    /// Metadata for this image.
    pub metadata: ImageMetadata,
    /// Download URL for this image.
    pub image_url: String,
    /// The date this image was published at, as `YYYY-MM-DD`.
    ///
    /// If set the downloaded users this data as prefix for filenames.
    pub pubdate: Option<String>,
    /// The suggested file name for this image.
    pub suggested_filename: Option<String>,
}

impl DownloadableImage {
    fn guess_filename(&self) -> Cow<str> {
        self.image_url
            .split('/')
            .next_back()
            .filter(|s| !s.is_empty())
            .map_or_else(
                || Cow::Owned(self.metadata.title.replace(['/', '\n'], "_")),
                Cow::Borrowed,
            )
    }

    pub fn with_pubdate(mut self, date: &glib::DateTime) -> Self {
        self.pubdate = Some(date.format("%Y-%m-%d").unwrap().into());
        self
    }

    pub fn filename(&self) -> Cow<str> {
        self.suggested_filename
            .as_deref()
            .map_or_else(|| self.guess_filename(), Cow::Borrowed)
    }

    pub fn prepare_download<P: AsRef<Path>>(
        self,
        target_directory: P,
    ) -> (ImageMetadata, ImageDownload) {
        let filename = match &self.pubdate {
            Some(pubdate) => Cow::Owned(format!("{pubdate}-{}", self.filename())),
            None => self.filename(),
        };
        let target = target_directory.as_ref().join(&*filename);
        let download = ImageDownload {
            url: self.image_url,
            target,
        };
        (self.metadata, download)
    }
}

/// An image to download.
#[derive(Debug)]
pub struct ImageDownload {
    /// The URL to download from.
    pub url: String,
    /// The file to download to.
    pub target: PathBuf,
}

impl ImageDownload {
    pub async fn download(
        &self,
        session: &soup::Session,
        cancellable: &Cancellable,
    ) -> Result<(), glib::Error> {
        let target_file = gio::File::for_path(&self.target);
        let exists = gio::spawn_blocking(glib::clone!(
            #[strong]
            cancellable,
            move || target_file.query_exists(Some(&cancellable))
        ))
        .await
        .unwrap();
        if exists {
            glib::debug!("Using existing file at {}", self.target.display());
        } else {
            download_file(session, &self.url, &self.target, cancellable).await?;
        }
        Ok(())
    }
}
