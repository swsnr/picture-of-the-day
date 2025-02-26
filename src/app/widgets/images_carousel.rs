// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::cell::Ref;

use glib::{Object, subclass::types::ObjectSubclassIsExt};
use gtk::gio;

use crate::image::ImageMetadata;

glib::wrapper! {
    pub struct ImagesCarousel(ObjectSubclass<imp::ImagesCarousel>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl ImagesCarousel {
    pub fn nth_image(&self, n: u32) -> Ref<'_, ImageMetadata> {
        self.imp().nth_image(n)
    }

    pub fn set_images(&self, images: Vec<ImageMetadata>) {
        self.imp().set_images(images);
    }

    pub fn set_image_file(&self, index: u32, file: &gio::File) {
        self.imp().nth_page(index).set_image_file(Some(file));
    }

    pub fn set_error_message(&self, index: u32, message: &str) {
        self.imp().nth_page(index).set_error_message(Some(message));
    }
}

impl Default for ImagesCarousel {
    fn default() -> Self {
        Object::builder().build()
    }
}

mod imp {
    use std::{
        cell::{Ref, RefCell},
        cmp::Ordering,
        sync::OnceLock,
    };

    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use glib::subclass::{InitializingObject, Signal};
    use gtk::CompositeTemplate;

    use crate::{app::widgets::ImagePage, image::ImageMetadata};

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/de/swsnr/picture-of-the-day/ui/images-carousel.ui")]
    pub struct ImagesCarousel {
        images: RefCell<Vec<ImageMetadata>>,
        #[template_child]
        images_carousel: TemplateChild<adw::Carousel>,
    }

    impl ImagesCarousel {
        pub fn nth_image(&self, n: u32) -> Ref<'_, ImageMetadata> {
            #[allow(clippy::indexing_slicing)]
            Ref::map(self.images.borrow(), |i| &i[usize::try_from(n).unwrap()])
        }

        /// Set images to show.
        ///
        /// Create as many pages as there are `images`, all in loading state.
        pub fn set_images(&self, images: Vec<ImageMetadata>) {
            self.images.replace(images);

            let images = self.images.borrow();
            let carousel = self.images_carousel.get();

            let n_pages = usize::try_from(carousel.n_pages()).unwrap();
            match n_pages.cmp(&images.len()) {
                Ordering::Less => {
                    // We have less pages than images, so add missing pages
                    std::iter::repeat_with(ImagePage::default)
                        .take(images.len() - n_pages)
                        .for_each(|page| carousel.append(&page));
                }
                Ordering::Greater => {
                    // We have too many pages so remove pages from the end
                    for _ in 0..(n_pages - images.len()) {
                        carousel.remove(&carousel.nth_page(carousel.n_pages() - 1));
                    }
                }
                Ordering::Equal => (),
            }
            debug_assert_eq!(images.len(), usize::try_from(carousel.n_pages()).unwrap());

            // Reset all pages
            for n in 0..carousel.n_pages() {
                carousel
                    .nth_page(n)
                    .downcast::<ImagePage>()
                    .unwrap()
                    .reset();
            }

            // Then navigate to first page
            self.images_carousel
                .scroll_to(&carousel.nth_page(0), adw::is_animations_enabled(&carousel));
            // And be extra sure to notify the parent
            self.obj().emit_by_name::<()>("image-changed", &[&0u32]);
        }

        pub fn nth_page(&self, n: u32) -> ImagePage {
            self.images_carousel
                .nth_page(n)
                .downcast::<ImagePage>()
                .unwrap()
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImagesCarousel {
        const NAME: &'static str = "PotDImagesCarousel";

        type Type = super::ImagesCarousel;

        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ImagesCarousel {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("image-changed")
                        .param_types([u32::static_type()])
                        .build(),
                ]
            })
        }

        fn constructed(&self) {
            self.images_carousel.connect_page_changed(glib::clone!(
                #[weak(rename_to = carousel)]
                self.obj(),
                move |_, n| {
                    carousel.emit_by_name::<()>("image-changed", &[&n]);
                }
            ));
        }
    }

    impl WidgetImpl for ImagesCarousel {}

    impl BinImpl for ImagesCarousel {}
}
