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
    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::{
        CompositeTemplate,
        gio::{self, prelude::SettingsExtManual},
    };

    use crate::source::Source;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/de/swsnr/picture-of-the-day/ui/preferences-dialog.ui")]
    pub struct PreferencesDialog {
        #[template_child]
        apod: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        apod_api_key: TemplateChild<adw::EntryRow>,
    }

    impl PreferencesDialog {
        pub fn bind(&self, settings: &gio::Settings) {
            settings
                .bind("apod-api-key", &*self.apod_api_key, "text")
                .build();
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
            self.apod.set_title(&Source::Apod.i18n_name());
            self.apod.set_description(Some(&format!(
                "<a href=\"{0}\">{0}</a>",
                Source::Apod.url()
            )));
        }
    }

    impl WidgetImpl for PreferencesDialog {}

    impl AdwDialogImpl for PreferencesDialog {}

    impl PreferencesDialogImpl for PreferencesDialog {}
}
