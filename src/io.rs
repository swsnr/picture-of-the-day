// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use gtk::gio::{self, IOErrorEnum, prelude::*};

use std::path::Path;

use crate::config::G_LOG_DOMAIN;

pub async fn ensure_directory<P: AsRef<Path> + Send>(directory: P) -> Result<(), glib::Error> {
    let target_directory = gio::File::for_path(directory);
    gio::GioFuture::new(
        &target_directory,
        |target_directory, cancellable, result| {
            gio::spawn_blocking(glib::clone!(
                #[strong]
                cancellable,
                #[strong]
                target_directory,
                move || {
                    match target_directory.make_directory_with_parents(Some(&cancellable)) {
                        Err(error) if error.matches(IOErrorEnum::Exists) => result.resolve(Ok(())),
                        res => result.resolve(res),
                    }
                }
            ));
        },
    )
    .await
}

pub async fn delete_file_ignore_error(target: &gio::File) {
    if let Err(error) = target.delete_future(glib::Priority::DEFAULT).await {
        // No need to warn of the target doesn't exist, that's what we're here for after all
        if !error.matches(IOErrorEnum::NotFound) {
            glib::warn!("Failed to delete file {}: {error}", target.uri());
        }
    }
}
