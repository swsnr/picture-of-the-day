// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::path::Path;

use glib::Priority;
use gtk::gio::{self, FileCopyFlags, IOErrorEnum, prelude::*};
use soup::prelude::SessionExt;

use crate::{config::G_LOG_DOMAIN, io::delete_file_ignore_error};

/// A temporary target file for a download.
///
/// When dropped the temporary file is scheduled to be deleted asynchronously
/// on the glib main loop.
struct TemporaryDownloadFile {
    temp_file: gio::File,
}

impl TemporaryDownloadFile {
    pub fn new(directory: &Path, name: &str) -> Self {
        let temp_file =
            gio::File::for_path(directory.join(format!(".{name}.download.{}", glib::random_int())));
        Self { temp_file }
    }

    /// Move this temporary file to a final destination.
    ///
    /// Move this temporary file to `target` which must be on the same file system
    /// or the move will fail.
    ///
    /// Consumes the temporary file since it no longer needs to be deleted
    /// automatically on drop.
    pub async fn move_to(self, target: &gio::File) -> Result<(), glib::Error> {
        // Attempt to atomically (NO_FALLBACK_FOR_MOVE) move the temp file to the target file
        let flags = FileCopyFlags::NOFOLLOW_SYMLINKS | FileCopyFlags::NO_FALLBACK_FOR_MOVE;
        self.temp_file
            .move_future(target, flags, glib::Priority::DEFAULT)
            .0
            .await?;
        // Forget about
        std::mem::forget(self);
        Ok(())
    }
}

impl Drop for TemporaryDownloadFile {
    fn drop(&mut self) {
        let file = self.temp_file.clone();

        glib::spawn_future_local(async move {
            glib::debug!("Deleting temporary download file {}", file.uri());
            delete_file_ignore_error(&file).await;
        });
    }
}

impl AsRef<gio::File> for TemporaryDownloadFile {
    fn as_ref(&self) -> &gio::File {
        &self.temp_file
    }
}

/// Download a file from an URL to a directory.
///
/// Download the contents of `url` to a new file named `filename` in the given
/// `directory`.  Contents are written to a temporary file in `directory`, and
/// atomically moved to `filename` only after the download is finished.
pub async fn download_file_to_directory(
    session: &soup::Session,
    url: &str,
    directory: &Path,
    filename: &str,
) -> Result<(), glib::Error> {
    let temp_file = TemporaryDownloadFile::new(directory, filename);
    transfer_file(session, url, temp_file.as_ref()).await?;
    let target = gio::File::for_path(directory.join(filename));
    temp_file.move_to(&target).await
}

/// Return a file from `url` to `target`.
///
/// Fails if `target` already exists.
///
/// Return the amount of bytes transferred.
async fn transfer_file(
    session: &soup::Session,
    url: &str,
    target: &gio::File,
) -> Result<isize, glib::Error> {
    let message = soup::Message::new("GET", url).map_err(|error| {
        glib::Error::new(
            IOErrorEnum::InvalidArgument,
            &format!("Invalid URL: {url}: {error}"),
        )
    })?;

    let source = session.send_future(&message, Priority::DEFAULT).await?;
    if message.status() == soup::Status::NotFound {
        return Err(glib::Error::new(
            IOErrorEnum::NotFound,
            &format!("URL {url} responded with 404"),
        ));
    }
    if message.status() != soup::Status::Ok {
        return Err(glib::Error::new(
            IOErrorEnum::Failed,
            &format!("URL {url} responded with status {:?}", message.status()),
        ));
    }

    let sink = target
        .create_future(gio::FileCreateFlags::NONE, glib::Priority::DEFAULT)
        .await?;
    sink.splice_future(
        &source,
        gio::OutputStreamSpliceFlags::CLOSE_SOURCE | gio::OutputStreamSpliceFlags::CLOSE_TARGET,
        glib::Priority::DEFAULT,
    )
    .await
}
