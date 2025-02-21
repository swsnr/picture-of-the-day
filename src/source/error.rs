// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::{error::Error, fmt::Display};

use glib::GString;

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

impl Display for SourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceError::IO(error) => write!(f, "{error}"),
            // soup::Status has no Display impl
            #[allow(clippy::use_debug)]
            SourceError::HttpStatus(status, None) => write!(f, "HTTP status {status:?}"),
            #[allow(clippy::use_debug)]
            SourceError::HttpStatus(status, Some(reason)) => {
                write!(f, "HTTP status {status:?} {reason}")
            }
            SourceError::InvalidJson(error) => write!(f, "Invalid JSON: {error}"),
            SourceError::NoImage => write!(f, "No image available"),
        }
    }
}

impl Error for SourceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            SourceError::IO(error) => Some(error),
            SourceError::HttpStatus(_, _) | SourceError::NoImage => None,
            SourceError::InvalidJson(error) => Some(error),
        }
    }
}
