// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use glib::{Variant, VariantDict};

use super::{
    client::{PortalCall, PortalResponse, RequestResult},
    window::PortalWindowIdentifier,
};

#[derive(Variant)]
pub struct RequestBackground<'a>(PortalWindowIdentifier<'a>, VariantDict);

#[derive(Debug, Copy, Clone)]
pub enum AutostartMode<'a> {
    NoAutostart,
    CommandLine(Option<&'a [&'a str]>),
    DBusActivate,
}

impl<'a> RequestBackground<'a> {
    pub fn new(
        window: PortalWindowIdentifier<'a>,
        reason: &str,
        auto_start: AutostartMode<'_>,
    ) -> Self {
        let options = VariantDict::new(None);
        options.insert("reason", reason);
        match auto_start {
            AutostartMode::NoAutostart => {}
            AutostartMode::CommandLine(None) => {
                options.insert("autostart", true);
            }
            AutostartMode::CommandLine(Some(command_line)) => {
                options.insert("autostart", true);
                options.insert("commandline", command_line);
            }
            AutostartMode::DBusActivate => {
                options.insert("autostart", true);
                options.insert("dbus-activatable", true);
            }
        }
        Self(window, options)
    }
}

impl PortalCall for RequestBackground<'_> {
    const INTERFACE: &'static str = "org.freedesktop.portal.Background";

    const METHOD_NAME: &'static str = "RequestBackground";

    fn options_mut(&mut self) -> &mut VariantDict {
        &mut self.1
    }
}

#[derive(Debug, Clone)]
pub struct RequestBackgroundResult {
    pub request_result: RequestResult,
    pub background: bool,
    pub autostart: bool,
}

impl From<PortalResponse> for RequestBackgroundResult {
    fn from(response: PortalResponse) -> Self {
        let background = response
            .options()
            .lookup("background")
            .unwrap_or_default()
            .unwrap_or_default();
        let autostart = response
            .options()
            .lookup("autostart")
            .unwrap_or_default()
            .unwrap_or_default();
        Self {
            request_result: response.result(),
            background,
            autostart,
        }
    }
}
