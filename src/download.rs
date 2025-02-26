// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::path::Path;

use glib::{object::IsA, Priority};
use gtk::gio::{self, prelude::*, Cancellable, IOErrorEnum};
use soup::prelude::SessionExt;

use crate::config::G_LOG_DOMAIN;

async fn delete_file_ignore_error(target: &Path) {
    if let Err(error) = gio::File::for_path(target)
        .delete_future(glib::Priority::DEFAULT)
        .await
    {
        glib::warn!("Failed to delete file {}: {error}", target.display());
    }
}

/// Download a file from `url` to `target`.
///
/// `cancellable` allows to cancel the ongoing transfer; when the transfer failed
/// or was cancelled, delete any partially downloaded `target` file.
pub async fn download_file<P: AsRef<Path>>(
    session: &soup::Session,
    url: &str,
    target: P,
    cancellable: &impl IsA<Cancellable>,
) -> Result<(), glib::Error> {
    let result = gio::CancellableFuture::new(
        transfer_file(session, url, target.as_ref()),
        cancellable.clone().into(),
    )
    .await;

    match result {
        Err(_) => {
            glib::debug!(
                "Download of {url} to {0} was cancelled, deleting partially downloaded {0}",
                target.as_ref().display()
            );
            delete_file_ignore_error(target.as_ref()).await;
            Err(glib::Error::new(
                IOErrorEnum::Cancelled,
                &format!(
                    "Download of {url} to {} was cancelled",
                    target.as_ref().display()
                ),
            ))
        }
        Ok(Err(error)) => {
            glib::warn!(
                "Download of {url} to {0} failed, deleting partially downloaded {0}: {error}",
                target.as_ref().display()
            );
            delete_file_ignore_error(target.as_ref()).await;
            Err(error)
        }
        Ok(Ok(_)) => Ok(()),
    }
}

/// Return a file from `url` to `target`.
///
/// Fails if `target` already exists.
///
/// Return the amount of bytes transferred.
async fn transfer_file<P: AsRef<Path>>(
    session: &soup::Session,
    url: &str,
    target: P,
) -> Result<isize, glib::Error> {
    glib::debug!("Downloading {url} to {}", target.as_ref().display());
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

    let sink = gio::File::for_path(target.as_ref())
        .create_future(gio::FileCreateFlags::NONE, glib::Priority::DEFAULT)
        .await?;
    sink.splice_future(
        &source,
        gio::OutputStreamSpliceFlags::CLOSE_SOURCE | gio::OutputStreamSpliceFlags::CLOSE_TARGET,
        glib::Priority::DEFAULT,
    )
    .await
}
