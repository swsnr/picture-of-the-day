// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::{error::Error, fmt::Display};

use glib::GString;

use super::http::HttpError;

#[derive(Debug)]
pub enum SourceError {
    /// IO failed.
    IO(glib::Error),
    /// An unexpected HTTP status code, with an optional reason.
    HttpStatus(soup::Status, Option<GString>),
    /// A deserialization error.
    InvalidJson(serde_json::Error),
    /// No image was available.
    NoImage,
    /// Invalid API key for the source
    InvalidApiKey,
    /// The client was rate-limited.
    RateLimited,
    /// The source did provide data, but the data does not denote an image.
    ///
    /// The source may have returned a video, for instance.
    NotAnImage,
}

impl From<glib::Error> for SourceError {
    fn from(error: glib::Error) -> Self {
        Self::IO(error)
    }
}

impl From<serde_json::Error> for SourceError {
    fn from(value: serde_json::Error) -> Self {
        Self::InvalidJson(value)
    }
}

impl From<HttpError> for SourceError {
    fn from(error: HttpError) -> Self {
        match error {
            HttpError::IO(error) => Self::from(error),
            // We deliberately discard the body here: At this point we should never inspect the
            // source-specific body again; if there was anything interesting in the body the
            // source backend itself should've analyzed it by now.
            HttpError::HttpStatus(status, reason, _) => Self::HttpStatus(status, reason),
            HttpError::InvalidJson(error) => Self::from(error),
        }
    }
}

impl Display for SourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceError::IO(error) => write!(f, "{error}"),
            #[allow(clippy::use_debug)]
            SourceError::HttpStatus(status, None) => write!(f, "HTTP status {status:?}"),
            #[allow(clippy::use_debug)]
            SourceError::HttpStatus(status, Some(reason)) => {
                write!(f, "HTTP status {status:?} {reason}")
            }
            SourceError::InvalidJson(error) => write!(f, "Invalid JSON: {error}"),
            SourceError::NoImage => write!(f, "No image available"),
            SourceError::InvalidApiKey => write!(f, "The API key used was invalid"),
            SourceError::RateLimited => write!(f, "The client was rate limited"),
            SourceError::NotAnImage => {
                write!(f, "The source return no image data but e.g. a video")
            }
        }
    }
}

impl Error for SourceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            SourceError::IO(error) => Some(error),
            SourceError::InvalidJson(error) => Some(error),
            _ => None,
        }
    }
}
