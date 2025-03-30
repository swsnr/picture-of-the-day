// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use gtk::gio::{
    self, Cancellable, FileCopyFlags, IOErrorEnum,
    prelude::{FileExt, FileExtManual},
};
use rand::distributions::{Alphanumeric, DistString};

use crate::{
    config::G_LOG_DOMAIN, image::download::download_file, io::delete_file_ignore_error,
    rng::GlibRng, source::Source,
};

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
        let filename = self
            .suggested_filename
            .as_deref()
            .map_or_else(|| self.guess_filename(), Cow::Borrowed);
        match &self.pubdate {
            Some(pubdate) => Cow::Owned(format!("{pubdate}-{filename}")),
            None => filename,
        }
    }

    /// Download this image to a directory.
    ///
    /// Download this image to `target_directory`, using the provided HTTP
    /// `session.`
    ///
    /// Return the full path to the downloaded image if successful.
    pub async fn download_to_directory(
        &self,
        target_directory: &Path,
        session: &soup::Session,
        cancellable: &Cancellable,
    ) -> Result<PathBuf, glib::Error> {
        let file_name = self.filename();
        let target_file = target_directory.join(file_name.as_ref());
        let exists = {
            let target_file = gio::File::for_path(&target_file);
            gio::spawn_blocking(glib::clone!(
                #[strong]
                cancellable,
                move || target_file.query_exists(Some(&cancellable))
            ))
            .await
            .unwrap()
        };
        if exists {
            // If the target file exists already just return it
            glib::debug!("Using existing file at {}", target_file.display());
            Ok(target_file)
        } else {
            // Download to a random temp file in the target directory first
            let temp_file = gio::File::for_path(target_directory.join(format!(
                ".{file_name}.download.{}",
                Alphanumeric.sample_string(&mut GlibRng, 5)
            )));
            glib::debug!("Downloading {} to {}", &self.image_url, temp_file.uri());
            download_file(session, &self.image_url, &temp_file, cancellable).await?;

            // Then attempt to atomically (NO_FALLBACK_FOR_MOVE) move the temp
            // file to the target file
            let flags = FileCopyFlags::NOFOLLOW_SYMLINKS | FileCopyFlags::NO_FALLBACK_FOR_MOVE;
            glib::debug!("Moving {} to {}", temp_file.uri(), target_file.display());
            match temp_file
                .move_future(
                    &gio::File::for_path(&target_file),
                    flags,
                    glib::Priority::DEFAULT,
                )
                .0
                .await
            {
                Err(error) if error.matches(IOErrorEnum::Exists) => {
                    // If the target file already exists, assume that a parallel download
                    // finished first, and just delete our temp file.
                    delete_file_ignore_error(&temp_file).await;
                    Ok(target_file)
                }
                Err(error) => Err(error),
                Ok(()) => Ok(target_file),
            }
        }
    }
}
