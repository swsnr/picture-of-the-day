// Copyright Sebastian Wiesner <sebastian@swsnr.de>
//
// Licensed under the EUPL
//
// See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

use adw::prelude::*;
use glib::{dgettext, dpgettext2};
use glib::{object::IsA, subclass::types::ObjectSubclassIsExt};
use gtk::UriLauncher;
use gtk::gio;

use crate::app::model::{ErrorNotification, ErrorNotificationActions};
use crate::config::G_LOG_DOMAIN;
use crate::date::BoxedCivilDate;
use crate::services::portal::PortalClient;

glib::wrapper! {
    pub struct ApplicationWindow(ObjectSubclass<imp::ApplicationWindow>)
        @extends adw::ApplicationWindow, gtk::ApplicationWindow, gtk::Window, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap,
            gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget,
            gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl ApplicationWindow {
    /// Create a new window.
    ///
    /// The window belongs to `application` and keeps a hold on `application`.
    /// It uses the `session` to fetch images for the selected source.
    pub fn new(
        application: &impl IsA<gtk::Application>,
        session: soup::Session,
        portal_client: PortalClient,
        date: Option<jiff::civil::Date>,
    ) -> Self {
        glib::Object::builder()
            .property("application", application)
            .property("http-session", session)
            .property("portal-client", portal_client)
            .property("date", date.map(BoxedCivilDate::from))
            .build()
    }

    pub fn cancel_loading(&self) {
        self.imp().cancel_loading();
    }

    /// Load images for the selected source.
    pub async fn load_images(&self) {
        self.cancel_loading();
        let cancellable = self.imp().start_loading();

        let source = self.selected_source();
        let result =
            gio::CancellableFuture::new(self.imp().load_images_for_source(source), cancellable)
                .await;
        // If loading was cancelled or successful we don't need to do anything here
        if let Ok(Err(error)) = result {
            self.imp().show_source_error(source, &error);
        }

        self.imp().finish_loading();
    }

    async fn open_source_url(&self) {
        let url = self.selected_source().url();
        if let Err(error) = UriLauncher::new(url).launch_future(Some(self)).await {
            glib::warn!("Failed to open source URL: {error}");
            let description = dpgettext2(
                None,
                "error-notification.description",
                "An I/O error occurred while opening %1, with the following message: %2. If the issue persists please report the problem.",
            );
            let error = ErrorNotification::builder()
                .title(dpgettext2(
                    None,
                    "error-notification.title",
                    "Failed to open source URL",
                ))
                .description(
                    description
                        .replace("%1", url)
                        .replace("%2", &error.to_string()),
                )
                .actions(ErrorNotificationActions::OPEN_ABOUT_DIALOG)
                .build();
            self.imp().show_error(&error);
        }
    }

    async fn set_current_image_as_wallpaper(&self) {
        if let Err(error) = self.imp().set_current_image_as_wallpaper().await {
            glib::warn!("Failed to set current image as wallaper: {error}");
            let description = dpgettext2(
                None,
                "error-notification.description",
                "An I/O error occurred while setting the wallpaper, with the following message: %1. If the issue persists please report the problem.",
            );
            let error = ErrorNotification::builder()
                .title(dpgettext2(
                    None,
                    "error-notification.title",
                    "Failed to set wallpaper",
                ))
                .description(description.replace("%1", &error.to_string()))
                .actions(ErrorNotificationActions::OPEN_ABOUT_DIALOG)
                .build();
            self.imp().show_error(&error);
        }
    }

    async fn open_current_image(&self) {
        if let Some(file) = self.imp().current_image_file() {
            let launcher = gtk::FileLauncher::new(Some(&file));
            launcher.set_writable(false);
            // Always ask so that I can choose which app I'd like to use the image with.
            launcher.set_always_ask(true);
            if let Err(error) = launcher.launch_future(Some(self)).await {
                if error.matches(gtk::DialogError::Cancelled)
                    || error.matches(gtk::DialogError::Dismissed)
                {
                    // The dialog was cancelled programmatically, or dismissed by the user; in these cases
                    // we assume that this was deliberate, so we don't show an error notification.
                    return;
                }
                glib::warn!("Failed to open current image: {error}");
                let description = dpgettext2(
                    None,
                    "error-notification.description",
                    "An I/O error occurred while opening the current image, with the following message: %1. If the issue persists please report the problem.",
                );
                let error = ErrorNotification::builder()
                    .title(dpgettext2(
                        None,
                        "error-notification.title",
                        "Failed to open image",
                    ))
                    .description(description.replace("%1", &error.to_string()))
                    .actions(ErrorNotificationActions::OPEN_ABOUT_DIALOG)
                    .build();
                self.imp().show_error(&error);
            }
        }
    }

    async fn save_current_image(&self) {
        match self.imp().save_current_image().await {
            Ok(None) => {}
            Ok(Some(name)) => {
                let toast = adw::Toast::builder()
                    .title(dgettext(None, "Saved image %1").replace("%1", &name))
                    .priority(adw::ToastPriority::Normal)
                    .timeout(5)
                    .build();
                self.imp().show_toast(toast);
            }
            Err(error) => {
                if error.matches(gtk::DialogError::Cancelled)
                    || error.matches(gtk::DialogError::Dismissed)
                {
                    // The dialog was cancelled programmatically, or dismissed by the user; in these cases
                    // we assume that this was deliberate, so we don't show an error notification.
                    return;
                }
                glib::warn!("Failed to save current image: {error}");
                let description = dpgettext2(
                    None,
                    "error-notification.description",
                    "An I/O error occurred while saving the current image, with the following message: %1. If the issue persists please report the problem.",
                );
                let error = ErrorNotification::builder()
                    .title(dpgettext2(
                        None,
                        "error-notification.title",
                        "Failed to save image",
                    ))
                    .description(description.replace("%1", &error.to_string()))
                    .actions(ErrorNotificationActions::OPEN_ABOUT_DIALOG)
                    .build();
                self.imp().show_error(&error);
            }
        }
    }

    fn show_error_dialog(&self, error: &ErrorNotification) {
        let actions = error.actions();
        let dialog = adw::AlertDialog::builder()
            .heading(error.title())
            .body(error.description())
            .prefer_wide_layout(!actions.is_empty())
            .build();
        dialog.add_response("close", &dpgettext2(None, "alert.response", "Close"));
        dialog.set_close_response("close");
        dialog.set_default_response(Some("close"));

        for (action_name, action) in actions.iter_names() {
            let (label, action_to_activate) = match action {
                ErrorNotificationActions::OPEN_ABOUT_DIALOG => {
                    let label = dpgettext2(None, "alert.response", "Contact information");
                    (label, "app.about")
                }
                ErrorNotificationActions::OPEN_PREFERENCES => {
                    let label = dpgettext2(None, "alert.response", "Open preferences");
                    (label, "app.preferences")
                }
                ErrorNotificationActions::OPEN_SOURCE_URL => {
                    let label = dpgettext2(None, "alert.response", "Open URL");
                    (label, "win.open-source-url")
                }
                _ => unreachable!(),
            };
            dialog.add_response(action_name, &label);
            dialog.connect_response(
                Some(action_name),
                glib::clone!(
                    #[weak(rename_to = window)]
                    self,
                    move |_, _| {
                        gtk::prelude::WidgetExt::activate_action(&window, action_to_activate, None)
                            .unwrap();
                    }
                ),
            );
        }
        dialog.present(Some(self));
    }
}

mod imp {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use futures::future::join_all;
    use glib::subclass::InitializingObject;
    use glib::{Object, Properties, closure, dpgettext2};
    use gtk::CompositeTemplate;
    use gtk::gdk::{Key, ModifierType};
    use gtk::gio::{self, Cancellable, FileCreateFlags, FileQueryInfoFlags};
    use strum::IntoEnumIterator;

    use crate::app::model::{ErrorNotification, Image};
    use crate::app::widgets::{ErrorNotificationPage, ImagesCarousel, SourceRow};
    use crate::config::G_LOG_DOMAIN;
    use crate::date::BoxedCivilDate;
    use crate::images::{Source, SourceError};
    use crate::io::ensure_directory;
    use crate::services::portal::{
        PortalClient,
        wallpaper::{Preview, SetOn},
        window::PortalWindowHandle,
    };

    #[derive(Default, CompositeTemplate, Properties)]
    #[properties(wrapper_type = super::ApplicationWindow)]
    #[template(resource = "/de/swsnr/pictureoftheday/ui/application-window.ui")]
    pub struct ApplicationWindow {
        #[property(get, construct_only)]
        http_session: RefCell<soup::Session>,
        #[property(get, construct_only)]
        portal_client: RefCell<Option<PortalClient>>,
        #[property(get, construct_only, nullable)]
        date: Cell<Option<BoxedCivilDate>>,
        #[property(get, set, builder(Source::default()))]
        selected_source: Cell<Source>,
        #[property(get, set)]
        set_wallpaper_automatically: Cell<bool>,
        #[property(get, set)]
        show_image_properties: Cell<bool>,
        #[property(get, set)]
        show_update_indicator: Cell<bool>,
        #[property(get = Self::is_loading, type = bool)]
        is_loading: RefCell<Option<Cancellable>>,
        #[template_child]
        sources_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        stack: TemplateChild<gtk::Stack>,
        #[template_child]
        images_view: TemplateChild<adw::OverlaySplitView>,
        #[template_child]
        images_carousel: TemplateChild<ImagesCarousel>,
        #[template_child]
        toasts: TemplateChild<adw::ToastOverlay>,
    }

    #[gtk::template_callbacks]
    impl ApplicationWindow {
        #[template_callback(function)]
        fn non_empty(s: Option<&str>) -> bool {
            s.is_some_and(|s| !s.is_empty())
        }
    }

    impl ApplicationWindow {
        pub fn current_image(&self) -> Option<Image> {
            self.images_carousel.current_image()
        }

        pub fn current_image_file(&self) -> Option<gio::File> {
            self.current_image()
                .and_then(|image| image.downloaded_file())
        }

        fn is_loading(&self) -> bool {
            self.is_loading.borrow().is_some()
        }

        pub fn cancel_loading(&self) {
            if let Some(cancellable) = self.is_loading.replace(None) {
                glib::debug!("Cancelling image loading in window");
                cancellable.cancel();
                self.obj().notify_is_loading();
            }
        }

        fn switch_to_images_view(&self) {
            if self.stack.visible_child().unwrap() != self.images_view.get() {
                // Enable the side bar _before_ switching to the image view and
                // thus realizing the overlay view with its sidebar; this
                // ensures that the overlay view gets rendered with expanded
                // sidebar right from the start, which prevents warnings about
                // invalid widget sizes for all widgets contained in the overlay
                // view.
                //
                // We only expand the properties sidebar once, when we switch
                // away from the empty start page.  Afterwards we always honour
                // the users intention of whether to show the sidebar or not.
                self.obj().set_show_image_properties(true);
                self.stack.set_visible_child(&*self.images_view);
                self.obj()
                    .action_set_enabled("win.show-image-properties", true);
            }
        }

        pub fn start_loading(&self) -> gio::Cancellable {
            let cancellable = gio::Cancellable::new();
            if let Some(old_cancellable) = self.is_loading.replace(Some(cancellable.clone())) {
                old_cancellable.cancel();
            }
            self.obj().notify_is_loading();
            cancellable
        }

        pub fn finish_loading(&self) {
            self.is_loading.replace(None);
            self.obj().notify_is_loading();
        }

        pub fn show_source_error(&self, source: Source, error: &SourceError) {
            glib::error!("Fetching images failed: {error}");
            let error = ErrorNotification::from_error(source, error);
            self.show_error(&error);
        }

        pub fn show_toast(&self, toast: adw::Toast) {
            self.toasts.add_toast(toast);
        }

        pub fn show_error(&self, error: &ErrorNotification) {
            let toast = adw::Toast::builder()
                .title(error.title())
                .priority(adw::ToastPriority::High)
                .timeout(15)
                .button_label(dpgettext2(None, "toast.button.label", "Details"))
                .build();

            toast.connect_button_clicked(glib::clone!(
                #[strong]
                error,
                #[weak(rename_to = window)]
                self.obj(),
                move |toast| {
                    toast.dismiss();
                    window.show_error_dialog(&error);
                }
            ));
            self.show_toast(toast);
        }

        pub async fn load_images_for_source(&self, source: Source) -> Result<(), SourceError> {
            glib::info!("Fetching images for source {source:?}");
            let images = source
                .get_images(&self.obj().http_session(), self.date.get().map(Into::into))
                .await?;

            // Create model objects for all images:  We create an image object
            // to expose the metadata as glib properties, and a download object
            // to model the result of an image download.
            let images = images
                .into_iter()
                .map(|image| {
                    let obj = Image::from(&image);
                    (image, obj)
                })
                .collect::<Vec<_>>();

            // Set images to be shown, and switch to images view, in case we're
            // on the empty start page.
            self.images_carousel.get().set_images(
                &images
                    .iter()
                    .map(|(_, image)| image.clone())
                    .collect::<Vec<_>>(),
            );
            self.switch_to_images_view();

            // Create the download directory for the current source.
            let target_directory = source.images_directory();
            ensure_directory(&target_directory).await?;

            // Download all images
            let http_session = self.http_session.borrow().clone();
            let target_directory = Rc::new(target_directory);
            join_all(images.into_iter().map(move |(image, image_obj)| {
                glib::clone!(
                    #[strong]
                    target_directory,
                    #[weak]
                    http_session,
                    async move {
                        match image
                            .download_to_directory(&target_directory, &http_session)
                            .await
                        {
                            Ok(target) => {
                                glib::info!("Displaying image from {}", target.display());
                                image_obj.set_downloaded_file(Some(&gio::File::for_path(target)));
                            }
                            Err(error) => {
                                glib::warn!(
                                    "Downloading image from {} failed: {error}",
                                    &image.image_url
                                );
                                let error = ErrorNotification::from_error(source, &error.into());
                                image_obj.set_download_error(Some(error));
                            }
                        }
                    }
                )
            }))
            .await;
            Ok(())
        }

        pub async fn set_current_image_as_wallpaper(&self) -> Result<(), glib::Error> {
            if let Some(file) = self.current_image_file() {
                let window_handle = PortalWindowHandle::new_for_native(&*self.obj()).await;
                let result = self
                    .obj()
                    .portal_client()
                    .unwrap()
                    .set_wallpaper(&file, &window_handle, Preview::NoPreview, SetOn::Both)
                    .await?;
                glib::info!("Request finished: {result:?}");
            }
            Ok(())
        }

        pub async fn save_current_image(&self) -> Result<Option<glib::GString>, glib::Error> {
            if let Some(file) = self.current_image_file() {
                let info = file
                    .query_info_future(
                        gio::FILE_ATTRIBUTE_STANDARD_DISPLAY_NAME,
                        FileQueryInfoFlags::NONE,
                        glib::Priority::DEFAULT,
                    )
                    .await?;
                let dialog = gtk::FileDialog::new();
                dialog.set_title(&dpgettext2(None, "file-dialog.title", "Save image"));
                dialog.set_initial_name(Some(info.display_name().as_str()));
                let target = dialog.save_future(Some(&*self.obj())).await?;
                let target_info = file
                    .query_info_future(
                        gio::FILE_ATTRIBUTE_STANDARD_DISPLAY_NAME,
                        FileQueryInfoFlags::NONE,
                        glib::Priority::DEFAULT,
                    )
                    .await?;
                target
                    .replace_future(
                        None,
                        false,
                        FileCreateFlags::PRIVATE | FileCreateFlags::REPLACE_DESTINATION,
                        glib::Priority::DEFAULT,
                    )
                    .await?
                    .splice_future(
                        &file.read_future(glib::Priority::DEFAULT).await?,
                        gio::OutputStreamSpliceFlags::CLOSE_SOURCE
                            | gio::OutputStreamSpliceFlags::CLOSE_TARGET,
                        glib::Priority::DEFAULT,
                    )
                    .await?;
                Ok(Some(target_info.display_name()))
            } else {
                Ok(None)
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ApplicationWindow {
        const NAME: &'static str = "PotDApplicationWindow";

        type Type = super::ApplicationWindow;

        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            ImagesCarousel::ensure_type();
            Image::ensure_type();
            ErrorNotificationPage::ensure_type();

            klass.bind_template();
            klass.bind_template_callbacks();

            klass.install_property_action("win.select-source", "selected-source");
            klass.install_property_action("win.show-image-properties", "show-image-properties");
            klass.install_property_action(
                "win.set-wallpaper-automatically",
                "set-wallpaper-automatically",
            );
            klass.install_action("win.cancel-loading", None, |window, _, _| {
                window.imp().cancel_loading();
            });
            klass.install_action_async("win.load-images", None, |window, _, _| async move {
                window.load_images().await;
            });
            klass.install_action_async("win.open-source-url", None, |window, _, _| async move {
                window.open_source_url().await;
            });

            klass.add_binding_action(Key::F5, ModifierType::NO_MODIFIER_MASK, "win.load-images");
            klass.add_binding_action(
                Key::F9,
                ModifierType::NO_MODIFIER_MASK,
                "win.show-image-properties",
            );
            klass.add_binding_action(
                Key::Escape,
                ModifierType::NO_MODIFIER_MASK,
                "win.cancel-loading",
            );
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for ApplicationWindow {
        fn constructed(&self) {
            self.parent_constructed();

            for source in Source::iter() {
                let row = SourceRow::new(source);
                row.set_action_name(Some("win.select-source"));
                row.set_action_target(Some(source));
                self.sources_list.get().append(&row);
            }

            self.obj().connect_selected_source_notify(|window| {
                glib::info!("Selected source updates: {:?}", window.selected_source());
                gtk::prelude::WidgetExt::activate_action(window, "win.load-images", None).unwrap();
            });

            // We're not showing images initially, so let's disable the sidebar action.
            self.obj()
                .action_set_enabled("win.show-image-properties", false);

            let act_set_wallpaper = gio::SimpleAction::new("set-as-wallpaper", None);
            act_set_wallpaper.connect_activate(glib::clone!(
                #[weak(rename_to = window)]
                self.obj(),
                move |_, _| {
                    glib::spawn_future_local(async move {
                        window.set_current_image_as_wallpaper().await;
                    });
                }
            ));
            let act_open_default = gio::SimpleAction::new("open-with-external-application", None);
            act_open_default.connect_activate(glib::clone!(
                #[weak(rename_to = window)]
                self.obj(),
                move |_, _| {
                    glib::spawn_future_local(async move {
                        window.open_current_image().await;
                    });
                }
            ));
            let act_save_image = gio::SimpleAction::new("save-image", None);
            act_save_image.connect_activate(glib::clone!(
                #[weak(rename_to = window)]
                self.obj(),
                move |_, _| {
                    glib::spawn_future_local(async move {
                        window.save_current_image().await;
                    });
                }
            ));

            for action in &[act_set_wallpaper, act_open_default, act_save_image] {
                self.obj().add_action(action);
                action.set_enabled(false);
                self.images_carousel
                    .property_expression("current-image")
                    .chain_property::<Image>("downloaded-file")
                    .chain_closure::<bool>(closure!(
                        |_: Option<Object>, file: Option<&gio::File>| { file.is_some() }
                    ))
                    .bind(action, "enabled", Object::NONE);
            }
        }
    }

    impl AdwApplicationWindowImpl for ApplicationWindow {}

    impl ApplicationWindowImpl for ApplicationWindow {}

    impl WindowImpl for ApplicationWindow {
        fn close_request(&self) -> glib::Propagation {
            let result = self.parent_close_request();
            if result.is_proceed() {
                self.cancel_loading();
            }
            result
        }
    }

    impl WidgetImpl for ApplicationWindow {}
}
