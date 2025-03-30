// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use glib::{Priority, object::IsA};
use gtk::gio::{self, Cancellable, IOErrorEnum, prelude::*};
use soup::prelude::SessionExt;

use crate::{config::G_LOG_DOMAIN, io::delete_file_ignore_error};

/// Download a file from `url` to `target`.
///
/// `cancellable` allows to cancel the ongoing transfer; when the transfer failed
/// or was cancelled, delete any partially downloaded `target` file.
pub async fn download_file(
    session: &soup::Session,
    url: &str,
    target: &gio::File,
    cancellable: &impl IsA<Cancellable>,
) -> Result<(), glib::Error> {
    let result = gio::CancellableFuture::new(
        transfer_file(session, url, target),
        cancellable.clone().into(),
    )
    .await
    .map_err(|_| {
        glib::Error::new(
            IOErrorEnum::Cancelled,
            &format!("Download of {url} to {} was cancelled", target.uri()),
        )
    })
    // Result::flatten is nightly only, see https://github.com/rust-lang/rust/issues/70142
    .and_then(|r| r);

    match result {
        Err(error) => {
            glib::warn!(
                "Download of {url} to {0} failed, deleting partially downloaded {0}: {error}",
                target.uri()
            );
            delete_file_ignore_error(target).await;
            Err(error)
        }
        Ok(_) => Ok(()),
    }
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
