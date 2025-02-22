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
    use glib::subclass::InitializingObject;
    use glib::Properties;
    use gtk::gdk::{Key, ModifierType};
    use gtk::CompositeTemplate;
    use strum::IntoEnumIterator;

    use crate::app::widgets::SourceRow;
    use crate::config::G_LOG_DOMAIN;
    use crate::image::DownloadableImage;
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
        #[template_child]
        sources_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        stack: TemplateChild<gtk::Stack>,
        #[template_child]
        images_view: TemplateChild<adw::OverlaySplitView>,
        images: RefCell<Vec<DownloadableImage>>,
    }

    #[gtk::template_callbacks(functions)]
    impl ApplicationWindow {
        #[template_callback]
        fn non_empty(s: &str) -> bool {
            !s.is_empty()
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ApplicationWindow {
        const NAME: &'static str = "PotDApplicationWindow";

        type Type = super::ApplicationWindow;

        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();

            klass.install_action("win.about-app", None, |window, _, _| {
                window.show_about_dialog();
            });
            klass.install_property_action("win.select-source", "selected-source");
            klass.install_action_async("win.refresh-images", None, |window, _, _| async move {
                let source = window.selected_source();
                glib::info!("Fetching images for source {source:?}");
                match source.get_images(&window.http_session()).await {
                    Ok(images) => {
                        glib::info!("Fetched images for {source:?}: {images:?}");
                        window
                            .imp()
                            .stack
                            .get()
                            .set_visible_child(&window.imp().images_view.get());
                        window.imp().images.replace(images);
                        let images = window.imp().images.borrow();
                        window.show_image_metadata_in_sidebar(Some(&images[0].metadata));
                    }
                    Err(error) => {
                        glib::error!("Failed to fetch images for {source:?}: {error}");
                    }
                }
            });

            klass.add_binding_action(
                Key::F5,
                ModifierType::NO_MODIFIER_MASK,
                "win.refresh-images",
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
                gtk::prelude::WidgetExt::activate_action(window, "win.refresh-images", None)
                    .unwrap();
            });
        }
    }

    impl AdwApplicationWindowImpl for ApplicationWindow {}

    impl ApplicationWindowImpl for ApplicationWindow {}

    impl WindowImpl for ApplicationWindow {}

    impl WidgetImpl for ApplicationWindow {}
}
