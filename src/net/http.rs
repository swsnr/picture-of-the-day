// Copyright Sebastian Wiesner <sebastian@swsnr.de>
//
// Licensed under the EUPL
//
// See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

use glib::{Bytes, GString};
use serde::de::DeserializeOwned;
use soup::prelude::SessionExt;

pub mod download;

/// An error during a HTTP request.
#[derive(Debug)]
pub enum HttpError {
    /// An IO error.
    IO(glib::Error),
    /// An unexpected HTTP status.
    HttpStatus(soup::Status, Option<GString>, Bytes),
    /// An invalid JSON body.
    InvalidJson(serde_json::Error),
}

impl From<glib::Error> for HttpError {
    fn from(error: glib::Error) -> Self {
        Self::IO(error)
    }
}

impl From<serde_json::Error> for HttpError {
    fn from(error: serde_json::Error) -> Self {
        Self::InvalidJson(error)
    }
}

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
    ) -> Result<T, HttpError>;
}

impl SoupSessionExt for soup::Session {
    async fn send_and_read_json<T: DeserializeOwned>(
        &self,
        message: &soup::Message,
        priority: glib::Priority,
    ) -> Result<T, HttpError> {
        let body = self.send_and_read_future(message, priority).await?;
        if message.status() == soup::Status::Ok {
            Ok(serde_json::from_slice(&body)?)
        } else {
            Err(HttpError::HttpStatus(
                message.status(),
                message.reason_phrase(),
                body,
            ))
        }
    }
}
