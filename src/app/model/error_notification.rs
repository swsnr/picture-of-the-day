// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::source::{Source, SourceError};

#[glib::flags(name = "PotDErrorNotificationActions")]
pub enum ErrorNotificationActions {
    NONE = 0,
    OPEN_PREFERENCES = 1,
    OPEN_SOURCE_URL = 1 << 2,
    OPEN_ABOUT_DIALOG = 1 << 3,
}

impl Default for ErrorNotificationActions {
    fn default() -> Self {
        Self::NONE
    }
}

glib::wrapper! {
    pub struct ErrorNotification(ObjectSubclass<imp::ErrorNotification>);
}

mod errors {
    use glib::dpgettext2;
    use glib::translate::IntoGlib;
    use gtk::gio;
    use gtk::gio::prelude::*;

    use crate::source::Source;

    use super::{ErrorNotification, ErrorNotificationActions};

    pub fn invalid_api_key(source: Source) -> ErrorNotification {
        let title = dpgettext2(
            None,
            "error-notification.title",
            "Picture Of The Day needs an API key",
        );
        let description = dpgettext2(
            None,
            "error-notification.description",
            "Please configure a valid API key for %1 in the application preferences. The current key appears to be invalid.",
        );
        ErrorNotification::builder()
            .title(title)
            .description(description.replace("%1", &source.i18n_name()))
            .actions(ErrorNotificationActions::OPEN_PREFERENCES)
            .build()
    }

    pub fn rate_limited(source: Source) -> ErrorNotification {
        let title = dpgettext2(
            None,
            "error-notification.title",
            "Picture Of The Day was rate-limited",
        );
        let description = dpgettext2(
            None,
            "error-notification.description",
            "The %1 server is rate-limited and does currently not allow you to fetch an image. Try again later, or try a different source.",
        );
        ErrorNotification::builder()
            .title(title)
            .description(description.replace("%1", &source.i18n_name()))
            .build()
    }

    pub fn no_image(source: Source) -> ErrorNotification {
        let title = dpgettext2(None, "error-notification.title", "No image today");
        let description = dpgettext2(
            None,
            "error-notification.description",
            "Currently, %1 does not provide an image for today. Try again later, or try a different source.",
        );
        ErrorNotification::builder()
            .title(title)
            .description(description.replace("%1", &source.i18n_name()))
            .build()
    }

    pub fn not_an_image(source: Source) -> ErrorNotification {
        let title = dpgettext2(None, "error-notification.title", "No image today");
        let description = dpgettext2(
            None,
            "error-notification.description",
            "Today, %1 provides a video or another type of media which this application does not support. Try again tomorrow, or try a different source.",
        );
        ErrorNotification::builder()
            .title(title)
            .description(description.replace("%1", &source.i18n_name()))
            .actions(ErrorNotificationActions::OPEN_SOURCE_URL)
            .build()
    }

    pub fn http_status(source: Source, status: soup::Status) -> ErrorNotification {
        let title = dpgettext2(None, "error-notification.title", "Fetching images failed");
        let description = dpgettext2(
            None,
            "error-notification.description",
            "The %s server replied with HTTP status %2. The server may have issues currently. Try again later, or try a different source. If the issue persists please report the problem.",
        );
        ErrorNotification::builder()
            .title(title)
            .description(
                description
                    .replace("%1", &source.i18n_name())
                    .replace("%2", &status.into_glib().to_string()),
            )
            .actions(ErrorNotificationActions::OPEN_ABOUT_DIALOG)
            .build()
    }

    pub fn invalid_data(source: Source) -> ErrorNotification {
        let title = dpgettext2(None, "error-notification.title", "Fetching images failed");
        let description = dpgettext2(
            None,
            "error-notification.description",
            "The %1 server responded with invalid data. The server may have issues currently. Try again later, or try a different source. If the issue persists please report the problem.",
        );
        ErrorNotification::builder()
            .title(title)
            .description(description.replace("%1", &source.i18n_name()))
            .actions(ErrorNotificationActions::OPEN_ABOUT_DIALOG)
            .build()
    }

    pub fn cancelled(source: Source) -> ErrorNotification {
        let title = dpgettext2(None, "error-notification.title", "Cancelled");
        let description = dpgettext2(
            None,
            "error-notification.description",
            "You cancelled loading images from %1.",
        );
        ErrorNotification::builder()
            .title(title)
            .description(description.replace("%1", &source.i18n_name()))
            .build()
    }

    pub fn io_error(source: Source, error: &glib::Error) -> ErrorNotification {
        let connectivity = gio::NetworkMonitor::default().connectivity();
        let (title, description) = if connectivity == gio::NetworkConnectivity::Full {
            let title = dpgettext2(
                None,
                "error-notification.title",
                "Limited network connectivity",
            );
            let description = dpgettext2(
                None,
                "error-notification.description",
                "An I/O error occurred while fetching today's image from %1, with the following message: %2. The system appears to have limited network connectivity.  Try to connect to the internet.",
            );
            (title, description)
        } else {
            let title = dpgettext2(None, "error-notification.title", "Fetching images failed");
            let description = dpgettext2(
                None,
                "error-notification.description",
                "An I/O error occurred while fetching today's image from %1, with the following message: %2.",
            );
            (title, description)
        };
        ErrorNotification::builder()
            .title(title)
            .description(
                description
                    .replace("%1", &source.i18n_name())
                    .replace("%2", &error.to_string()),
            )
            .build()
    }
}

impl ErrorNotification {
    pub fn builder() -> ErrorNotificationBuilder {
        ErrorNotificationBuilder::default()
    }

    pub fn from_error(source: Source, error: &SourceError) -> Self {
        match error {
            SourceError::InvalidApiKey => errors::invalid_api_key(source),
            SourceError::RateLimited => errors::rate_limited(source),
            SourceError::NoImage => errors::no_image(source),
            SourceError::NotAnImage => errors::not_an_image(source),
            SourceError::HttpStatus(status, _) => errors::http_status(source, *status),
            SourceError::InvalidJson(_) => errors::invalid_data(source),
            SourceError::IO(error) => errors::io_error(source, error),
            SourceError::Cancelled => errors::cancelled(source),
        }
    }
}

mod imp {
    use std::cell::Cell;
    use std::cell::RefCell;

    use glib::Properties;
    use glib::prelude::*;
    use glib::subclass::prelude::*;

    use super::ErrorNotificationActions;

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::ErrorNotification)]
    pub struct ErrorNotification {
        #[property(get, construct_only)]
        title: RefCell<String>,
        #[property(get, construct_only)]
        description: RefCell<String>,
        #[property(get, construct_only)]
        actions: Cell<ErrorNotificationActions>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ErrorNotification {
        const NAME: &'static str = "PotDErrorNotification";

        type Type = super::ErrorNotification;
    }

    #[glib::derived_properties]
    impl ObjectImpl for ErrorNotification {}
}

#[must_use = "The builder must be built to be used"]
pub struct ErrorNotificationBuilder {
    builder: glib::object::ObjectBuilder<'static, ErrorNotification>,
}

impl Default for ErrorNotificationBuilder {
    fn default() -> Self {
        Self {
            builder: glib::object::Object::builder(),
        }
    }
}

impl ErrorNotificationBuilder {
    pub fn title(mut self, title: impl Into<glib::GString>) -> Self {
        self.builder = self.builder.property("title", title.into());
        self
    }

    pub fn description(mut self, description: impl Into<glib::GString>) -> Self {
        self.builder = self.builder.property("description", description.into());
        self
    }

    pub fn actions(mut self, actions: ErrorNotificationActions) -> Self {
        self.builder = self.builder.property("actions", actions);
        self
    }

    pub fn build(self) -> ErrorNotification {
        self.builder.build()
    }
}
