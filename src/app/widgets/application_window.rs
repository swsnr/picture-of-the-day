// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use adw::prelude::*;
use glib::{dgettext, dpgettext2, object::IsA, subclass::types::ObjectSubclassIsExt};
use gtk::gio;

use crate::source::Source;

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
    /// It uses the `session` to fetch images for the selected source, and
    /// initially uses the given `selected_source`.
    pub fn new(
        application: &impl IsA<gtk::Application>,
        session: soup::Session,
        selected_source: Source,
    ) -> Self {
        glib::Object::builder()
            .property("application", application)
            .property("http-session", session)
            .property("selected-source", selected_source)
            .build()
    }

    fn show_about_dialog(&self) {
        let dialog = adw::AboutDialog::from_appdata(
            "/de/swsnr/picture-of-the-day/de.swsnr.picture-of-the-day.metainfo.xml",
            Some(&crate::config::release_notes_version().to_string()),
        );
        dialog.set_version(crate::config::CARGO_PKG_VERSION);

        // TODO translations link to codeberg translate
        dialog.set_developers(&["Sebastian Wiesner https://swsnr.de"]);
        dialog.set_designers(&["Sebastian Wiesner https://swsnr.de"]);
        // Credits for the translator to the current language.
        // Translators: Add your name here, as "Jane Doe <jdoe@example.com>" or "Jane Doe https://jdoe.example.com"
        // Mail address or URL are optional.  Separate multiple translators with a newline, i.e. \n
        dialog.set_translator_credits(&dgettext(None, "translator-credits"));
        dialog.add_acknowledgement_section(
            Some(&dpgettext2(
                None,
                "about-dialog.acknowledgment-section",
                "Help and inspiration",
            )),
            &[
                "Sebastian Dröge https://github.com/sdroege",
                "Bilal Elmoussaoui https://github.com/bilelmoussaoui",
                "Authenticator https://gitlab.gnome.org/World/Authenticator",
                "Decoder https://gitlab.gnome.org/World/decoder/",
            ],
        );
        dialog.add_acknowledgement_section(
            Some(&dpgettext2(
                None,
                "about-dialog.acknowledgment-section",
                "Helpful services",
            )),
            &[
                "Flathub https://flathub.org/",
                "Open Build Service https://build.opensuse.org/",
                "GitHub actions https://github.com/features/actions",
            ],
        );

        dialog.present(Some(self));
    }

    pub fn cancel_loading(&self) {
        self.imp().cancel_loading();
    }

    /// Load images for the selected source.
    pub async fn load_images(&self) {
        self.cancel_loading();
        let cancellable = self.imp().start_loading();

        let source = self.selected_source();
        if let Err(error) = self
            .imp()
            .load_images_for_source(source, &cancellable)
            .await
        {
            self.imp().show_error(source, &error);
        }

        self.imp().finish_loading();
    }
}

mod imp {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use futures_util::future::join_all;
    use glib::Properties;
    use glib::subclass::InitializingObject;
    use gtk::CompositeTemplate;
    use gtk::gdk::{Key, ModifierType};
    use gtk::gio::{self, Cancellable, IOErrorEnum};
    use strum::IntoEnumIterator;

    use crate::Source;
    use crate::app::model::{ErrorNotification, Image, ImageDownload};
    use crate::app::widgets::{ErrorNotificationPage, ImagesCarousel, SourceRow};
    use crate::config::G_LOG_DOMAIN;
    use crate::source::SourceError;

    #[derive(Default, CompositeTemplate, Properties)]
    #[properties(wrapper_type = super::ApplicationWindow)]
    #[template(resource = "/de/swsnr/picture-of-the-day/ui/application-window.ui")]
    pub struct ApplicationWindow {
        #[property(get, construct_only)]
        http_session: RefCell<soup::Session>,
        #[property(get, set, builder(Source::default()))]
        selected_source: Cell<Source>,
        #[property(get, set)]
        show_image_properties: Cell<bool>,
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
        error_page: TemplateChild<ErrorNotificationPage>,
    }

    #[gtk::template_callbacks]
    impl ApplicationWindow {
        #[template_callback(function)]
        fn non_empty(s: Option<&str>) -> bool {
            s.is_some_and(|s| !s.is_empty())
        }
    }

    impl ApplicationWindow {
        fn is_loading(&self) -> bool {
            self.is_loading.borrow().is_some()
        }

        pub fn cancel_loading(&self) {
            if let Some(cancellable) = self.is_loading.replace(None) {
                cancellable.cancel();
            }
            self.obj().notify_is_loading();
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
            self.is_loading.replace(Some(cancellable.clone()));
            self.obj().notify_is_loading();
            cancellable
        }

        pub fn finish_loading(&self) {
            self.is_loading.replace(None);
            self.obj().notify_is_loading();
        }

        pub fn show_error(&self, source: Source, error: &SourceError) {
            if let SourceError::Cancelled = error {
                glib::info!("Fetching images cancelled by user");
                // Do not change the view if the user cancelled the action; just
                // keep the previous view.
            } else {
                glib::error!("Fetching images failed: {error}");
                let error = ErrorNotification::from_error(source, error);
                self.error_page.set_error(Some(&error));
                self.stack.set_visible_child(&self.error_page.get());
            }
        }

        pub async fn load_images_for_source(
            &self,
            source: Source,
            cancellable: &Cancellable,
        ) -> Result<(), SourceError> {
            glib::info!("Fetching images for source {source:?}");
            let images = gio::CancellableFuture::new(
                source.get_images(&self.obj().http_session()),
                cancellable.clone(),
            )
            .await??;

            // Create model objects for all images:  We create an image object
            // to expose the metadata as glib properties, and a download object
            // to model the result of an image download.
            let images = images
                .into_iter()
                .map(|image| {
                    let obj = Image::from(&image);
                    (image, obj, ImageDownload::default())
                })
                .collect::<Vec<_>>();

            // Set images to be shown, and switch to images view, in case we're
            // on the empty start page.
            self.images_carousel.get().set_images(
                &images
                    .iter()
                    .map(|(_, image, download)| (image.clone(), download.clone()))
                    .collect::<Vec<_>>(),
            );
            self.switch_to_images_view();

            // Create the download directory for the current source.
            let target_directory = glib::user_data_dir()
                .join(crate::config::APP_ID)
                .join("images")
                .join(source.id());
            gio::spawn_blocking(glib::clone!(
                #[strong]
                target_directory,
                #[strong]
                cancellable,
                move || {
                    glib::info!("Creating target directory {}", target_directory.display());
                    let target_directory = gio::File::for_path(&*target_directory);
                    match target_directory.make_directory_with_parents(Some(&cancellable)) {
                        Err(error) if error.matches(IOErrorEnum::Exists) => Ok(()),
                        res => res,
                    }
                }
            ))
            .await
            .unwrap()?;

            // Download all images
            let http_session = self.http_session.borrow().clone();
            let target_directory = Rc::new(target_directory);
            join_all(images.into_iter().map(move |(image, _, download)| {
                glib::clone!(
                    #[strong]
                    target_directory,
                    #[weak]
                    http_session,
                    async move {
                        let target = target_directory.join(&*image.filename());
                        match image.download_to(&target, &http_session, cancellable).await {
                            Ok(()) => {
                                glib::info!("Displaying image from {}", target.display());
                                download.set_file(Some(&gio::File::for_path(target)));
                            }
                            Err(error) => {
                                glib::warn!(
                                    "Downloading image from {} failed: {error}",
                                    &image.image_url
                                );
                                let error = ErrorNotification::from_error(source, &error.into());
                                download.set_error(Some(error));
                            }
                        }
                    }
                )
            }))
            .await;
            Ok(())
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

            klass.install_action("win.about-app", None, |window, _, _| {
                window.show_about_dialog();
            });
            klass.install_property_action("win.select-source", "selected-source");
            klass.install_property_action("win.show-image-properties", "show-image-properties");
            klass.install_action("win.cancel-loading", None, |window, _, _| {
                window.imp().cancel_loading();
            });
            klass.install_action_async("win.load-images", None, |window, _, _| async move {
                window.load_images().await;
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
        }
    }

    impl AdwApplicationWindowImpl for ApplicationWindow {}

    impl ApplicationWindowImpl for ApplicationWindow {}

    impl WindowImpl for ApplicationWindow {}

    impl WidgetImpl for ApplicationWindow {}
}
