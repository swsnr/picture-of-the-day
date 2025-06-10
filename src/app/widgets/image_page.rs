// Copyright Sebastian Wiesner <sebastian@swsnr.de>
//
// Licensed under the EUPL
//
// See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

use glib::Object;

glib::wrapper! {
    pub struct ImagePage(ObjectSubclass<imp::ImagePage>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for ImagePage {
    fn default() -> Self {
        Object::builder().build()
    }
}

mod imp {
    use std::cell::RefCell;

    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use glib::{Properties, subclass::InitializingObject};
    use gtk::{CompositeTemplate, gdk::ContentProvider};

    use crate::app::{
        model::{Image, ImageState},
        widgets::ErrorNotificationPage,
    };

    #[derive(Default, CompositeTemplate, Properties)]
    #[properties(wrapper_type = super::ImagePage)]
    #[template(resource = "/de/swsnr/pictureoftheday/ui/image-page.ui")]
    pub struct ImagePage {
        #[property(get, set)]
        image: RefCell<Option<Image>>,
        #[template_child]
        loading: TemplateChild<gtk::Widget>,
        #[template_child]
        picture: TemplateChild<gtk::Picture>,
        #[template_child]
        error: TemplateChild<gtk::Widget>,
    }

    impl ImagePage {
        fn drag_content_provider(&self) -> Option<ContentProvider> {
            let image_file = self.obj().image()?.downloaded_file()?;
            let paintable = self.picture.paintable()?;
            Some(ContentProvider::new_union(&[
                ContentProvider::for_value(&image_file.to_value()),
                ContentProvider::for_value(&paintable.to_value()),
            ]))
        }
    }

    #[gtk::template_callbacks]
    impl ImagePage {
        #[template_callback]
        fn stack_page(&self, state: ImageState) -> gtk::Widget {
            match state {
                ImageState::Pending => self.loading.get(),
                ImageState::Downloaded => self.picture.get().upcast(),
                ImageState::DownloadFailed => self.error.get(),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImagePage {
        const NAME: &'static str = "PotDImagePage";

        type Type = super::ImagePage;

        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            Image::ensure_type();
            ImageState::ensure_type();
            ErrorNotificationPage::ensure_type();

            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for ImagePage {
        fn constructed(&self) {
            let source = gtk::DragSource::new();
            source.connect_prepare(glib::clone!(
                #[weak(rename_to = page)]
                self.obj(),
                #[upgrade_or_default]
                move |_, _, _| page.imp().drag_content_provider()
            ));
            self.picture.add_controller(source);
        }
    }

    impl WidgetImpl for ImagePage {}

    impl BinImpl for ImagePage {}
}
