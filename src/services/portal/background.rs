// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::collections::HashMap;

use glib::{Variant, bitflags::bitflags, variant::ToVariant};

use super::{
    client::{PortalCall, PortalResponse, RequestResult},
    window::PortalWindowIdentifier,
};

#[derive(Variant)]
pub struct RequestBackground<'a>(PortalWindowIdentifier<'a>, HashMap<String, Variant>);

bitflags! {
    #[derive(Copy, Clone)]
    pub struct RequestBackgroundFlags: u8 {
        const AUTOSTART = 1;
        const DBUS_ACTIVATE = 2;
    }
}

impl<'a> RequestBackground<'a> {
    pub fn new(
        window: PortalWindowIdentifier<'a>,
        reason: &str,
        command_line: Option<&[&str]>,
        flags: RequestBackgroundFlags,
    ) -> Self {
        let mut options = HashMap::from([("reason".to_string(), reason.to_variant())]);
        if let Some(command_line) = command_line {
            options.insert("commandline".to_string(), command_line.to_variant());
        }
        if flags.contains(RequestBackgroundFlags::AUTOSTART) {
            options.insert("autostart".to_string(), true.to_variant());
        }
        if flags.contains(RequestBackgroundFlags::DBUS_ACTIVATE) {
            options.insert("dbus-activatable".to_string(), true.to_variant());
        }
        Self(window, options)
    }
}

impl PortalCall for RequestBackground<'_> {
    const INTERFACE: &'static str = "org.freedesktop.portal.Background";

    const METHOD_NAME: &'static str = "RequestBackground";

    fn with_option(mut self, key: &str, value: Variant) -> Self {
        self.1.insert(key.to_string(), value);
        self
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
            .get("background")
            .and_then(glib::Variant::get)
            .unwrap_or_default();
        let autostart = response
            .options()
            .get("autostart")
            .and_then(glib::Variant::get)
            .unwrap_or_default();
        Self {
            request_result: response.result(),
            background,
            autostart,
        }
    }
}
