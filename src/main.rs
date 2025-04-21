// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

#![deny(warnings, clippy::all, clippy::pedantic,
    // Do cfg(test) right
    clippy::cfg_not_test,
    clippy::tests_outside_test_module,
    // Guard against left-over debugging output
    clippy::dbg_macro,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::unimplemented,
    clippy::use_debug,
    clippy::todo,
    // Require correct safety docs
    clippy::undocumented_unsafe_blocks,
    clippy::unnecessary_safety_comment,
    clippy::unnecessary_safety_doc,
    // We must use Gtk's APIs to exit the app.
    clippy::exit,
    // Don't panic carelessly
    clippy::get_unwrap,
    clippy::unused_result_ok,
    clippy::unwrap_in_result,
    clippy::indexing_slicing,
    // Do not carelessly ignore errors
    clippy::let_underscore_must_use,
    clippy::let_underscore_untyped,
    // Code smells
    clippy::float_cmp_const,
    clippy::string_to_string,
    clippy::if_then_some_else_none,
    clippy::large_include_file,
    // Disable as casts
    clippy::as_conversions,
)]
#![allow(clippy::enum_glob_use, clippy::module_name_repetitions)]

use app::Application;
use glib::dpgettext2;
use gtk::gio;
use gtk::prelude::*;

mod app;
mod config;
mod date;
mod gettext;
mod http;
mod image;
mod io;
mod locale;
mod portal;
mod rss;
mod source;
mod xml;

use config::G_LOG_DOMAIN;
use source::Source;

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

    glib::info!(
        "Running in flatpak?: {}",
        crate::config::running_in_flatpak()
    );

    let locale_dir = config::locale_directory();
    glib::debug!("Initializing gettext with locale directory {}", locale_dir);
    if let Err(error) = gettext::init_gettext(config::APP_ID, locale_dir) {
        glib::warn!("Failed to initialize gettext: {error}");
    }

    gio::resources_register_include!("pictureoftheday.gresource").unwrap();
    glib::set_application_name(&dpgettext2(None, "application-name", "Picture Of The Day"));

    let app = Application::default();
    app.set_version(config::CARGO_PKG_VERSION);
    app.run()
}
