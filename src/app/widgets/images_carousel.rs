// Copyright Sebastian Wiesner <sebastian@swsnr.de>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use glib::{Object, subclass::types::ObjectSubclassIsExt};

use crate::app::model::Image;

glib::wrapper! {
    pub struct ImagesCarousel(ObjectSubclass<imp::ImagesCarousel>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl ImagesCarousel {
    pub fn nth_image(&self, n: u32) -> Image {
        self.imp().nth_image(n)
    }

    pub fn set_images(&self, images: &[Image]) {
        self.imp().set_images(images);
    }

    pub fn scroll_next_image(&self, animate: bool) {
        self.imp().scroll_image_offset(1, animate);
    }

    pub fn scroll_previous_image(&self, animate: bool) {
        self.imp().scroll_image_offset(-1, animate);
    }
}

impl Default for ImagesCarousel {
    fn default() -> Self {
        Object::builder().build()
    }
}

mod imp {
    use std::cell::{Cell, RefCell};
    use std::cmp::Ordering;

    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use glib::{Properties, SourceId, subclass::InitializingObject};
    use gtk::{
        CompositeTemplate,
        gdk::{Key, ModifierType},
    };

    use crate::app::{model::Image, widgets::ImagePage};

    #[derive(Default, CompositeTemplate, Properties)]
    #[properties(wrapper_type = super::ImagesCarousel)]
    #[template(resource = "/de/swsnr/pictureoftheday/ui/images-carousel.ui")]
    pub struct ImagesCarousel {
        #[property(get)]
        current_image: RefCell<Option<Image>>,
        /// Whether to show navigation.
        ///
        /// When `true` show navigation, but only if the carousel has more than one page.
        #[property(get, set = Self::set_show_nav)]
        show_nav: Cell<bool>,
        #[template_child]
        carousel: TemplateChild<adw::Carousel>,
        nav_autohide_timeout: RefCell<Option<SourceId>>,
    }

    #[gtk::template_callbacks]
    impl ImagesCarousel {
        #[template_callback(function)]
        fn show_nav(number_of_pages: u32, show_nav: bool) -> bool {
            show_nav && 1 < number_of_pages
        }

        #[template_callback]
        fn pointer_enter_or_move(&self) {
            self.flash_nav_bar();
        }

        #[template_callback]
        fn pointer_leave(&self) {
            self.obj().set_show_nav(false);
        }
    }

    impl ImagesCarousel {
        pub fn nth_image(&self, n: u32) -> Image {
            self.carousel
                .nth_page(n)
                .downcast::<ImagePage>()
                .unwrap()
                .image()
                // We guarantee that all our image pages have an image behind them
                .unwrap()
        }

        fn set_show_nav(&self, show_nav: bool) {
            self.show_nav.set(show_nav);
            if show_nav {
                // Cancel any auto-hide timer
                if let Some(source_id) = self.nav_autohide_timeout.take() {
                    source_id.remove();
                }
            }
        }

        /// Show navigation bar for a few seconds.
        fn flash_nav_bar(&self) {
            self.obj().set_show_nav(true);
            self.nav_autohide_timeout
                .replace(Some(glib::timeout_add_seconds_local_once(
                    3,
                    glib::clone!(
                        #[weak(rename_to = carousel)]
                        self.obj(),
                        move || {
                            carousel.set_show_nav(false);
                            carousel.imp().nav_autohide_timeout.replace(None);
                        }
                    ),
                )));
        }

        /// Scroll to the n'th page.
        fn scroll_to_nth_page(&self, n: u32, animate: bool) {
            let n_pages = self.carousel.n_pages();
            if n_pages == 0 {
                return;
            }
            let n = n.clamp(0, n_pages - 1);
            let page = self.carousel.nth_page(n);
            self.carousel.scroll_to(&page, animate);
        }

        pub fn scroll_image_offset(&self, offset: i32, animate: bool) {
            let position = if 0 < offset {
                self.carousel.position().floor()
            } else {
                self.carousel.position().ceil()
            };
            // A carousel only has u32::MAX pages at most, so this won't truncate.
            #[allow(clippy::cast_possible_truncation, clippy::as_conversions)]
            let position = position as i64;
            let new_position = (position + i64::from(offset)).max(0).try_into().unwrap();
            self.scroll_to_nth_page(new_position, animate);
            self.flash_nav_bar();
        }

        /// Set images to show.
        ///
        /// Create as many pages as there are `images`, all in loading state.
        pub fn set_images(&self, images: &[Image]) {
            let carousel = self.carousel.get();

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
            for (n, image) in images.iter().enumerate() {
                let page = carousel
                    .nth_page(u32::try_from(n).unwrap())
                    .downcast::<ImagePage>()
                    .unwrap();
                page.set_image(image);
            }

            // Then navigate to first page
            self.carousel
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
            klass.bind_template_callbacks();

            klass.install_action("image.next", None, |carousel, _, _| {
                carousel.scroll_next_image(false);
            });
            klass.install_action("image.previous", None, |carousel, _, _| {
                carousel.scroll_previous_image(false);
            });

            klass.add_binding_action(Key::Left, ModifierType::NO_MODIFIER_MASK, "image.previous");
            klass.add_binding_action(
                Key::Page_Up,
                ModifierType::NO_MODIFIER_MASK,
                "image.previous",
            );
            klass.add_binding_action(Key::Right, ModifierType::NO_MODIFIER_MASK, "image.next");
            klass.add_binding_action(Key::Page_Down, ModifierType::NO_MODIFIER_MASK, "image.next");
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for ImagesCarousel {
        fn constructed(&self) {
            self.carousel.connect_page_changed(glib::clone!(
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
