// Copyright Sebastian Wiesner <sebastian@swsnr.de>
//
// Licensed under the EUPL
//
// See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

mod image;
mod source;
mod sources;

pub use image::{DownloadableImage, ImageMetadata};
pub use source::{Source, SourceError};

pub use sources::stalenhag;
