// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use glib::{Object, subclass::types::ObjectSubclassIsExt};

use crate::app::model::{Image, ImageDownload};

glib::wrapper! {
    pub struct ImagesCarousel(ObjectSubclass<imp::ImagesCarousel>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl ImagesCarousel {
    pub fn nth_image(&self, n: u32) -> Image {
        self.imp().nth_image(n)
    }

    pub fn set_images(&self, images: &[(Image, ImageDownload)]) {
        self.imp().set_images(images);
    }
}

impl Default for ImagesCarousel {
    fn default() -> Self {
        Object::builder().build()
    }
}

mod imp {
    use std::{cell::RefCell, cmp::Ordering};

    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use glib::{Properties, subclass::InitializingObject};
    use gtk::CompositeTemplate;

    use crate::app::{
        model::{Image, ImageDownload},
        widgets::ImagePage,
    };

    #[derive(Default, CompositeTemplate, Properties)]
    #[properties(wrapper_type = super::ImagesCarousel)]
    #[template(resource = "/de/swsnr/picture-of-the-day/ui/images-carousel.ui")]
    pub struct ImagesCarousel {
        #[property(get)]
        current_image: RefCell<Option<Image>>,
        #[template_child]
        images_carousel: TemplateChild<adw::Carousel>,
    }

    impl ImagesCarousel {
        pub fn nth_image(&self, n: u32) -> Image {
            self.images_carousel
                .nth_page(n)
                .downcast::<ImagePage>()
                .unwrap()
                .image()
                // We guarantee that all our image pages have an image behind them
                .unwrap()
        }

        /// Set images to show.
        ///
        /// Create as many pages as there are `images`, all in loading state.
        pub fn set_images(&self, images: &[(Image, ImageDownload)]) {
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

            // Assign an image to all pages
            for (n, (image, download)) in images.iter().enumerate() {
                let page = carousel
                    .nth_page(u32::try_from(n).unwrap())
                    .downcast::<ImagePage>()
                    .unwrap();
                page.set_image(image);
                page.set_download(download);
            }

            // Then navigate to first page
            self.images_carousel
                .scroll_to(&carousel.nth_page(0), adw::is_animations_enabled(&carousel));
            self.update_current_image(0);
        }

        fn update_current_image(&self, n: u32) {
            self.current_image.replace(Some(self.nth_image(n)));
            self.obj().notify_current_image();
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

    #[glib::derived_properties]
    impl ObjectImpl for ImagesCarousel {
        fn constructed(&self) {
            self.images_carousel.connect_page_changed(glib::clone!(
                #[weak(rename_to = carousel)]
                self.obj(),
                move |_, n| {
                    carousel.imp().update_current_image(n);
                }
            ));
        }
    }

    impl WidgetImpl for ImagesCarousel {}

    impl BinImpl for ImagesCarousel {}
}
