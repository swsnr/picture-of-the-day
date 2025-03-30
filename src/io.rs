// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use glib::object::IsA;
use gtk::gio::{self, Cancellable, IOErrorEnum, prelude::*};

use std::path::Path;

use crate::config::G_LOG_DOMAIN;

pub async fn ensure_directory<P: AsRef<Path> + Send>(
    directory: P,
    cancellable: &impl IsA<Cancellable>,
) -> Result<(), glib::Error> {
    glib::info!("Creating target directory {}", directory.as_ref().display());
    let target_directory = gio::File::for_path(directory);
    gio::spawn_blocking(glib::clone!(
        #[strong(rename_to = cancellable)]
        cancellable.as_ref(),
        move || {
            match target_directory.make_directory_with_parents(Some(&cancellable)) {
                Err(error) if error.matches(IOErrorEnum::Exists) => Ok(()),
                res => res,
            }
        }
    ))
    .await
    .unwrap()
}

pub async fn delete_file_ignore_error(target: &gio::File) {
    if let Err(error) = target.delete_future(glib::Priority::DEFAULT).await {
        glib::warn!("Failed to delete file {}: {error}", target.uri());
    }
}
