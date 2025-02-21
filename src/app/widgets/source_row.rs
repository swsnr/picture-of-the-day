// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::Source;

glib::wrapper! {
    pub struct SourceRow(ObjectSubclass<imp::SourceRow>)
        @extends adw::ActionRow, adw::PreferencesRow, gtk::ListBoxRow, gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl SourceRow {
    pub fn new(source: Source) -> Self {
        glib::Object::builder().property("source", source).build()
    }
}

mod imp {
    use std::cell::Cell;

    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use glib::{markup_escape_text, GString, Properties};

    use crate::Source;

    #[derive(Default, Properties, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::SourceRow)]
    #[template(resource = "/de/swsnr/picture-of-the-day/ui/source-row.ui")]
    pub struct SourceRow {
        #[property(get, set, construct, builder(Source::default()))]
        source: Cell<Source>,
    }

    #[gtk::template_callbacks(functions)]
    impl SourceRow {
        #[template_callback]
        fn source_title(source: Source) -> GString {
            source.i18n_name()
        }

        #[template_callback]
        fn source_subtitle(source: Source) -> String {
            let url = markup_escape_text(source.url());
            format!("<a href=\"{url}\">{url}</a>")
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SourceRow {
        const NAME: &'static str = "PotDSourceRow";

        type Type = super::SourceRow;

        type ParentType = adw::ActionRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for SourceRow {}

    impl WidgetImpl for SourceRow {}
    impl ListBoxRowImpl for SourceRow {}
    impl PreferencesRowImpl for SourceRow {}
    impl ActionRowImpl for SourceRow {}
}
