// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use app::PictureOfTheDayApplication;
use gtk::gio;
use gtk::prelude::*;

mod app;
mod config;

fn main() -> glib::ExitCode {
    static GLIB_LOGGER: glib::GlibLogger = glib::GlibLogger::new(
        glib::GlibLoggerFormat::Structured,
        glib::GlibLoggerDomain::CrateTarget,
    );
    let max_level = if std::env::var_os("G_MESSAGES_DEBUG").is_some() {
        log::LevelFilter::Trace
    } else {
        log::LevelFilter::Warn
    };
    log::set_max_level(max_level);
    log::set_logger(&GLIB_LOGGER).unwrap();

    gio::resources_register_include!("picture-of-the-day.gresource").unwrap();
    glib::set_application_name("Picture Of The Day");

    let app = PictureOfTheDayApplication::default();
    app.set_version(config::CARGO_PKG_VERSION);
    app.run()
}
