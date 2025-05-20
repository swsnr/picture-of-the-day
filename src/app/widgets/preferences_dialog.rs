// Copyright Sebastian Wiesner <sebastian@swsnr.de>
//
// Licensed under the EUPL
//
// See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

use adw::prelude::*;
use glib::Object;
use gtk::gio;

glib::wrapper! {
    pub struct PreferencesDialog(ObjectSubclass<imp::PreferencesDialog>)
        @extends adw::PreferencesDialog, adw::Dialog, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl PreferencesDialog {
    pub fn bind(&self, settings: &gio::Settings) {
        settings.bind("apod-api-key", self, "apod-api-key").build();
        settings
            .bind(
                "stalenhag-disabled-collections",
                self,
                "stalenhag-disabled-collections",
            )
            .build();
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
    use glib::{Properties, StrV, dngettext, subclass::InitializingObject};
    use gtk::CompositeTemplate;

    use crate::images::{Source, stalenhag};

    #[derive(Default, CompositeTemplate, Properties)]
    #[properties(wrapper_type = super::PreferencesDialog)]
    #[template(resource = "/de/swsnr/pictureoftheday/ui/preferences-dialog.ui")]
    pub struct PreferencesDialog {
        #[property(get, set)]
        apod_api_key: RefCell<String>,
        #[property(get, set)]
        stalenhag_disabled_collections: RefCell<StrV>,
        #[template_child]
        group_apod: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        group_stalenhag: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        stalenhag_collections: TemplateChild<adw::ExpanderRow>,
    }

    #[gtk::template_callbacks]
    impl PreferencesDialog {
        #[template_callback(function)]
        #[allow(clippy::needless_pass_by_value)]
        fn label_enabled_collections(disabled_collections: StrV) -> String {
            let n_disabled = disabled_collections.len();
            let n_enabled = stalenhag::COLLECTIONS.len() - n_disabled;
            dngettext(
                None,
                "%1/%2 collection enabled",
                "%1/%2 collections enabled",
                u64::try_from(n_enabled).unwrap(),
            )
            .replace("%1", &n_enabled.to_string())
            .replace("%2", &stalenhag::COLLECTIONS.len().to_string())
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PreferencesDialog {
        const NAME: &'static str = "PotDPreferencesDialog";

        type Type = super::PreferencesDialog;

        type ParentType = adw::PreferencesDialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for PreferencesDialog {
        fn constructed(&self) {
            self.parent_constructed();

            let source_groups = [
                (Source::Apod, &self.group_apod),
                (Source::Stalenhag, &self.group_stalenhag),
            ];
            for (source, group) in source_groups {
                group.set_title(&source.i18n_name());
                group.set_description(Some(&format!("<a href=\"{0}\">{0}</a>", source.url())));
            }

            for collection in stalenhag::COLLECTIONS.iter() {
                let switch = adw::SwitchRow::builder()
                    .title(&collection.title)
                    .subtitle(format!("<a href=\"{0}\">{0}</a>", collection.url))
                    .build();
                self.stalenhag_collections.add_row(&switch);

                self.obj()
                    .bind_property("stalenhag-disabled-collections", &switch, "active")
                    .bidirectional()
                    .transform_to(|_, disabled_collections: StrV| {
                        let is_disabled = disabled_collections.contains(&collection.tag);
                        Some(!is_disabled)
                    })
                    .transform_from(|binding, enabled: bool| {
                        let source = binding
                            .source()
                            .map(|o| o.downcast::<super::PreferencesDialog>().unwrap())?;
                        let mut disabled_collections = source.stalenhag_disabled_collections();
                        if enabled {
                            if let Some(index) = disabled_collections
                                .iter()
                                .position(|tag| tag == &collection.tag)
                            {
                                disabled_collections.remove(index);
                            }
                        } else {
                            disabled_collections.push((&collection.tag).into());
                        }
                        Some(disabled_collections)
                    })
                    .build();
            }
        }
    }

    impl WidgetImpl for PreferencesDialog {}

    impl AdwDialogImpl for PreferencesDialog {}

    impl PreferencesDialogImpl for PreferencesDialog {}
}
