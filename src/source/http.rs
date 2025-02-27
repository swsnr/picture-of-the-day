// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::source::SourceError;
use serde::de::DeserializeOwned;
use soup::prelude::SessionExt;

pub trait SoupSessionExt {
    /// Send a `message` with `priority` and read a JSON response.
    ///
    /// ## Errors
    ///
    /// Return [`SourceError::HttpStatus`] if the request returns a status other
    /// than [`soup::Status::Ok`], or [`SourceError::InvalidJson`] if the status
    /// was good, but the body contained either invalid JSON, or did not
    /// deserialize to the given type `T`.
    async fn send_and_read_json<T: DeserializeOwned>(
        &self,
        message: &soup::Message,
        priority: glib::Priority,
    ) -> Result<T, SourceError>;
}

impl SoupSessionExt for soup::Session {
    async fn send_and_read_json<T: DeserializeOwned>(
        &self,
        message: &soup::Message,
        priority: glib::Priority,
    ) -> Result<T, SourceError> {
        let body = self.send_and_read_future(message, priority).await?;
        if message.status() == soup::Status::Ok {
            Ok(serde_json::from_slice(&body)?)
        } else {
            Err(SourceError::HttpStatus(
                message.status(),
                message.reason_phrase(),
            ))
        }
    }
}
