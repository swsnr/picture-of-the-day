// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

mod image;
mod source;
mod sources;

pub use image::{DownloadableImage, ImageMetadata};
pub use source::{Source, SourceError};

pub use sources::stalenhag;
