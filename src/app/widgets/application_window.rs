// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use adw::prelude::*;
use glib::{dgettext, dpgettext2, object::IsA};
use gtk::gio;

use crate::{image::ImageMetadata, source::Source};

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

    /// Show the given metadata in the image sidebar.
    ///
    /// Show title, URL, description, and copyright information in the sidebar,
    /// if set, or clear out current information if `None`.
    fn show_image_metadata_in_sidebar(&self, metadata: Option<&ImageMetadata>) {
        if let Some(metadata) = metadata {
            self.set_image_title(&*metadata.title);
            self.set_image_url(metadata.url.as_deref().unwrap_or_default());
            self.set_image_description(metadata.description.as_deref().unwrap_or_default());
            self.set_image_copyright(metadata.copyright.as_deref().unwrap_or_default());
            self.set_image_source_name(metadata.source.i18n_name());
            self.set_image_source_url(metadata.source.url());
        } else {
            self.set_image_title("");
            self.set_image_url("");
            self.set_image_description("");
            self.set_image_copyright("");
            self.set_image_source_name("");
            self.set_image_source_url("");
        }
    }
}

mod imp {
    use std::cell::{Cell, RefCell};

    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use futures_util::future::join_all;
    use glib::subclass::InitializingObject;
    use glib::Properties;
    use gtk::gdk::{Key, ModifierType};
    use gtk::gio::{self, Cancellable, IOErrorEnum};
    use gtk::CompositeTemplate;
    use strum::IntoEnumIterator;

    use crate::app::widgets::{ImagesCarousel, SourceRow};
    use crate::config::G_LOG_DOMAIN;
    use crate::source::SourceError;
    use crate::Source;

    #[derive(Default, CompositeTemplate, Properties)]
    #[properties(wrapper_type = super::ApplicationWindow)]
    #[template(resource = "/de/swsnr/picture-of-the-day/ui/application-window.ui")]
    pub struct ApplicationWindow {
        #[property(get, construct_only)]
        http_session: RefCell<soup::Session>,
        #[property(get, set, builder(Source::default()))]
        selected_source: Cell<Source>,
        #[property(get, set)]
        image_url: RefCell<String>,
        #[property(get, set)]
        image_title: RefCell<String>,
        #[property(get, set)]
        image_copyright: RefCell<String>,
        #[property(get, set)]
        image_description: RefCell<String>,
        #[property(get, set)]
        image_source_name: RefCell<String>,
        #[property(get, set)]
        image_source_url: RefCell<String>,
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
    }

    #[gtk::template_callbacks]
    impl ApplicationWindow {
        #[template_callback(function)]
        fn non_empty(s: &str) -> bool {
            !s.is_empty()
        }

        #[template_callback]
        fn image_changed(&self, n: u32, carousel: &ImagesCarousel) {
            let image = carousel.nth_image(n);
            self.obj().show_image_metadata_in_sidebar(Some(&*image));
        }
    }

    impl ApplicationWindow {
        fn is_loading(&self) -> bool {
            self.is_loading.borrow().is_some()
        }

        fn cancel_loading(&self) {
            if let Some(cancellable) = self.is_loading.replace(None) {
                cancellable.cancel();
            }
            self.obj().notify_is_loading();
        }

        fn switch_to_images_view(&self) {
            self.stack.set_visible_child(&*self.images_view);
            self.obj()
                .action_set_enabled("win.show-image-properties", true);
            self.obj().set_show_image_properties(true);
        }

        async fn load_images(&self, cancellable: &Cancellable) -> Result<(), SourceError> {
            let source = self.selected_source.get();
            glib::info!("Fetching images for source {source:?}");
            let images = gio::CancellableFuture::new(
                source.get_images(&self.obj().http_session()),
                cancellable.clone(),
            )
            .await??;
            glib::info!("Fetched images for {source:?}: {images:?}");

            // Prepare downloads for all images
            let target_directory = glib::user_data_dir()
                .join(crate::config::APP_ID)
                .join("images")
                .join(source.id());
            let mut metadatas = Vec::with_capacity(images.len());
            let mut downloads = Vec::with_capacity(images.len());
            for image in images {
                let (metadata, download) = image.prepare_download(&target_directory);
                metadatas.push(metadata);
                downloads.push(download);
            }

            // Set images to be shown, and switch to images view, in case we're
            // on the empty start page.
            let carousel = self.images_carousel.get();
            carousel.set_images(metadatas);
            self.switch_to_images_view();

            let target_directory_file = gio::File::for_path(&target_directory);
            gio::spawn_blocking(glib::clone!(
                #[strong]
                cancellable,
                move || {
                    glib::info!(
                        "Creating target directory {}",
                        target_directory_file.path().unwrap().display()
                    );
                    match target_directory_file.make_directory_with_parents(Some(&cancellable)) {
                        Err(error) if error.matches(IOErrorEnum::Exists) => Ok(()),
                        res => res,
                    }
                }
            ))
            .await
            .unwrap()?;

            let http_session = self.http_session.borrow().clone();
            join_all(downloads.into_iter().enumerate().map(|(n, download)| {
                glib::clone!(
                    #[strong]
                    http_session,
                    #[weak]
                    carousel,
                    #[upgrade_or]
                    Ok(()),
                    async move {
                        match download.download(&http_session, cancellable).await {
                            Ok(()) => {
                                glib::info!("Displaying image from {}", download.target.display());
                                carousel.set_image_file(
                                    u32::try_from(n).unwrap(),
                                    &gio::File::for_path(&download.target),
                                );
                                Ok(())
                            }
                            Err(error) => {
                                glib::error!(
                                    "Downloading image from {} failed: {error}",
                                    &download.url
                                );
                                // TODO: Provide a human-readable and translated error message here!
                                carousel.set_error_message(
                                    u32::try_from(n).unwrap(),
                                    &format!("Download failed: {error}"),
                                );
                                Err(error)
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
                window.imp().cancel_loading();
                let cancellable = gio::Cancellable::new();
                window.imp().is_loading.replace(Some(cancellable.clone()));
                window.notify_is_loading();
                if let Err(error) = window.imp().load_images(&cancellable).await {
                    if error.is_cancelled() {
                        glib::info!("Image loading cancelled!");
                    } else {
                        glib::error!("Image loading failed: {error}");
                    }
                }
                window.imp().is_loading.replace(None);
                window.notify_is_loading();
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
