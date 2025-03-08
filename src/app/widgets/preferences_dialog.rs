// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use glib::{Object, subclass::types::ObjectSubclassIsExt};
use gtk::gio;

glib::wrapper! {
    pub struct PreferencesDialog(ObjectSubclass<imp::PreferencesDialog>)
        @extends adw::PreferencesDialog, adw::Dialog, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl PreferencesDialog {
    pub fn bind(&self, settings: &gio::Settings) {
        self.imp().bind(settings);
    }
}

impl Default for PreferencesDialog {
    fn default() -> Self {
        Object::builder().build()
    }
}

mod imp {
    use std::cell::RefCell;

    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use glib::{Variant, dngettext, subclass::InitializingObject};
    use gtk::{
        CompositeTemplate,
        gio::{self, prelude::SettingsExtManual},
    };

    use crate::source::{
        Source,
        stalenhag::{self, Collection},
    };

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/de/swsnr/pictureoftheday/ui/preferences-dialog.ui")]
    pub struct PreferencesDialog {
        #[template_child]
        apod: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        apod_api_key: TemplateChild<adw::EntryRow>,
        #[template_child]
        stalenhag: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        stalenhag_collections: TemplateChild<adw::ExpanderRow>,
        collection_switches: RefCell<Vec<(&'static Collection, adw::SwitchRow)>>,
    }

    impl PreferencesDialog {
        pub fn bind(&self, settings: &gio::Settings) {
            settings
                .bind("apod-api-key", &*self.apod_api_key, "text")
                .build();

            settings
                .bind(
                    "disabled-collections",
                    &*self.stalenhag_collections,
                    "subtitle",
                )
                .get_only()
                .mapping(|value, _| {
                    let n_disabled = value.n_children();
                    let n_enabled = stalenhag::COLLECTIONS.len() - n_disabled;
                    let label = dngettext(
                        None,
                        "%1/%2 collection enabled",
                        "%1/%2 collections enabled",
                        u64::try_from(n_enabled).unwrap(),
                    )
                    .replace("%1", &n_enabled.to_string())
                    .replace("%2", &stalenhag::COLLECTIONS.len().to_string());
                    Some(label.into())
                })
                .build();

            let collections = self.collection_switches.borrow();
            for (collection, switch) in collections.iter() {
                // Deref to make the rust compiler understand that collection is static
                let collection: &'static Collection = collection;
                settings
                    .bind("disabled-collections", switch, "active")
                    .set_mapping(glib::clone!(
                        #[weak]
                        settings,
                        #[upgrade_or_default]
                        move |value, _| {
                            let is_enabled = value.get::<bool>().ok()?;
                            let disabled_collections =
                                settings.strv("disabled-collections").into_iter();
                            let mapped = if is_enabled {
                                let disabled_collections = disabled_collections
                                    .filter(|c| *c.as_str() != collection.tag)
                                    .map(|s| Variant::from(s.as_str()));
                                Variant::array_from_iter::<String>(disabled_collections)
                            } else {
                                let disabled_collections = disabled_collections
                                    .map(|s| Variant::from(s.as_str()))
                                    .chain(std::iter::once(Variant::from(&collection.tag)));
                                Variant::array_from_iter::<String>(disabled_collections)
                            };
                            Some(mapped)
                        }
                    ))
                    .mapping(|value, _| {
                        let is_enabled = !value.array_iter_str().ok()?.any(|c| c == collection.tag);
                        Some(is_enabled.into())
                    })
                    .build();
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PreferencesDialog {
        const NAME: &'static str = "PotDPreferencesDialog";

        type Type = super::PreferencesDialog;

        type ParentType = adw::PreferencesDialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PreferencesDialog {
        fn constructed(&self) {
            self.parent_constructed();

            let source_groups = [
                (Source::Apod, &self.apod),
                (Source::Stalenhag, &self.stalenhag),
            ];
            for (source, group) in source_groups {
                group.set_title(&source.i18n_name());
                group.set_description(Some(&format!("<a href=\"{0}\">{0}</a>", source.url())));
            }

            self.collection_switches.replace(
                stalenhag::COLLECTIONS
                    .iter()
                    .map(|collection| {
                        let switch = adw::SwitchRow::builder()
                            .title(&collection.title)
                            .subtitle(format!("<a href=\"{0}\">{0}</a>", collection.url))
                            .build();
                        (collection, switch)
                    })
                    .collect(),
            );

            for (_, switch) in self.collection_switches.borrow().iter() {
                self.stalenhag_collections.add_row(switch);
            }
        }
    }

    impl WidgetImpl for PreferencesDialog {}

    impl AdwDialogImpl for PreferencesDialog {}

    impl PreferencesDialogImpl for PreferencesDialog {}
}
