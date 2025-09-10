// Copyright Sebastian Wiesner <sebastian@swsnr.de>
//
// Licensed under the EUPL
//
// See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

use crate::images::{Source, SourceError};

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
    use formatx::formatx;
    use glib::dpgettext2;
    use glib::translate::IntoGlib;
    use gtk::gio;
    use gtk::gio::prelude::*;

    use crate::images::Source;

    use super::{ErrorNotification, ErrorNotificationActions};

    pub fn invalid_api_key(source: Source) -> ErrorNotification {
        let title = dpgettext2(
            None,
            "error-notification.title",
            "Picture of the Day needs an API key",
        );
        let description = formatx!(
            dpgettext2(
                None,
                "error-notification.description",
                "Please configure a valid API key for {source_name} in the \
application preferences. The current key appears to be invalid.",
            ),
            source_name = source.i18n_name()
        )
        .unwrap();
        ErrorNotification::builder()
            .title(title)
            .description(description)
            .actions(ErrorNotificationActions::OPEN_PREFERENCES)
            .needs_attention()
            .build()
    }

    pub fn rate_limited(source: Source) -> ErrorNotification {
        let title = dpgettext2(
            None,
            "error-notification.title",
            "Picture of the Day was rate-limited",
        );
        let description = formatx!(
            dpgettext2(
                None,
                "error-notification.description",
                "The {source_name} server is rate-limited and does currently not \
allow you to fetch an image. Try again later, or try a different source.",
            ),
            source_name = source.i18n_name()
        )
        .unwrap();
        ErrorNotification::builder()
            .title(title)
            .description(description)
            .build()
    }

    pub fn no_image(source: Source) -> ErrorNotification {
        let title = dpgettext2(None, "error-notification.title", "No image today");
        let description = formatx!(
            dpgettext2(
                None,
                "error-notification.description",
                "Currently, {source_name} does not provide an image for today. \
Try again later, or try a different source.",
            ),
            source_name = source.i18n_name()
        )
        .unwrap();
        ErrorNotification::builder()
            .title(title)
            .description(description)
            .build()
    }

    pub fn not_an_image(source: Source) -> ErrorNotification {
        let title = dpgettext2(None, "error-notification.title", "No image today");
        let description = formatx!(
            dpgettext2(
                None,
                "error-notification.description",
                "Today, {source_name} provides a video or another type of media \
which this application does not support. Try again tomorrow, or try a \
different source.",
            ),
            source_name = source.i18n_name()
        )
        .unwrap();
        ErrorNotification::builder()
            .title(title)
            .description(description)
            .actions(ErrorNotificationActions::OPEN_SOURCE_URL)
            // Needs attention because the user can navigate to the source site and e.g. watch the video
            .needs_attention()
            .build()
    }

    pub fn http_status(source: Source, status: soup::Status) -> ErrorNotification {
        let title = dpgettext2(None, "error-notification.title", "Fetching images failed");
        let description = formatx!(
            dpgettext2(
                None,
                "error-notification.description",
                "The {source_name} server replied with HTTP status {http_status}. \
The server may have issues currently. Try again later, or try a different \
source. If the issue persists please report the problem.",
            ),
            source_name = source.i18n_name(),
            http_status = status.into_glib()
        )
        .unwrap();
        ErrorNotification::builder()
            .title(title)
            .description(description)
            .actions(ErrorNotificationActions::OPEN_ABOUT_DIALOG)
            .build()
    }

    pub fn invalid_data(source: Source) -> ErrorNotification {
        let title = dpgettext2(None, "error-notification.title", "Fetching images failed");
        let description = formatx!(
            dpgettext2(
                None,
                "error-notification.description",
                "The {source_name} server responded with invalid data. The server \
may have issues currently. Try again later, or try a different source. If the \
issue persists please report the problem.",
            ),
            source_name = source.i18n_name()
        )
        .unwrap();
        ErrorNotification::builder()
            .title(title)
            .description(description)
            .actions(ErrorNotificationActions::OPEN_ABOUT_DIALOG)
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
            let description = formatx!(
                dpgettext2(
                    None,
                    "error-notification.description",
                    "An I/O error occurred while fetching today's image from \
{source_name}, with the following message: {error}. Your system appears to \
have only limited network connectivity. Try to connect your system to the \
internet.",
                ),
                source_name = source.i18n_name(),
                error = error
            )
            .unwrap();
            (title, description)
        } else {
            let title = dpgettext2(None, "error-notification.title", "Fetching images failed");
            let description = formatx!(
                dpgettext2(
                    None,
                    "error-notification.description",
                    "An I/O error occurred while fetching today's image from \
{source_name}, with the following message: {error}. Try again later, or try a \
different source. If the issue persists please report the problem.",
                ),
                source_name = source.i18n_name(),
                error = error
            )
            .unwrap();
            (title, description)
        };
        ErrorNotification::builder()
            .title(title)
            .description(description)
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
            SourceError::InvalidJson(_)
            | SourceError::ScrapingFailed(_)
            | SourceError::InvalidRss(_) => errors::invalid_data(source),
            SourceError::IO(error) => errors::io_error(source, error),
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
        #[property(get, construct_only)]
        needs_attention: Cell<bool>,
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

    /// Indicate that this error needs immediate user attention.
    ///
    /// In this case an error notification should be shown immediately, even if
    /// the error occurred during background refresh.
    pub fn needs_attention(mut self) -> Self {
        self.builder = self.builder.property("needs-attention", true);
        self
    }

    pub fn build(self) -> ErrorNotification {
        self.builder.build()
    }
}
