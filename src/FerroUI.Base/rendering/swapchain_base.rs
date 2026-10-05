use std::cell::RefCell;
use std::rc::Rc;

use crate::reactive::{Disposable, IDisposable};
use crate::PixelSize;

/// The state of the last presentation of a swapchain image.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SwapchainImagePresentStatus {
    /// The presentation has not completed yet.
    Pending,
    /// The presentation completed successfully.
    RanToCompletion,
    /// The presentation was canceled.
    Canceled,
    /// The presentation failed.
    Faulted,
}

/// An image of a composition-backed swapchain.
pub trait ISwapchainImage {
    fn size(&self) -> PixelSize;

    /// The state of the last presentation; `None` when the image has not
    /// been presented yet.
    fn last_present(&self) -> Option<SwapchainImagePresentStatus>;

    fn begin_draw(&self);

    fn present(&self);

    /// Releases the image.
    fn dispose(&self);
}

/// The image factory of a swapchain: the member a concrete swapchain has to
/// provide.
pub trait SwapchainImageFactory<TImage: ISwapchainImage + ?Sized> {
    fn create_image(&self, size: PixelSize) -> Rc<TImage>;
}

/// A helper class for composition-backed swapchains, should not be a public
/// API yet.
///
/// A concrete swapchain embeds this type and provides the image factory.
pub struct SwapchainBase<TImage: ISwapchainImage + ?Sized + 'static> {
    pending_images: Rc<RefCell<Vec<Rc<TImage>>>>,
}

impl<TImage: ISwapchainImage + ?Sized + 'static> Default for SwapchainBase<TImage> {
    fn default() -> Self {
        Self::new()
    }
}

impl<TImage: ISwapchainImage + ?Sized + 'static> SwapchainBase<TImage> {
    pub fn new() -> Self {
        Self { pending_images: Rc::new(RefCell::new(Vec::new())) }
    }

    fn is_broken(image: &TImage) -> bool {
        image.last_present() == Some(SwapchainImagePresentStatus::Faulted)
    }

    fn is_ready(image: &TImage) -> bool {
        matches!(image.last_present(), None | Some(SwapchainImagePresentStatus::RanToCompletion))
    }

    fn cleanup_and_find_next_image(&self, size: PixelSize) -> Option<Rc<TImage>> {
        let mut first_found: Option<Rc<TImage>> = None;
        let mut found_multiple = false;

        let mut c = self.pending_images.borrow().len();
        while c > 0 {
            c -= 1;
            // Cloned out: the image callbacks must not run under the borrow.
            let image = self.pending_images.borrow()[c].clone();
            let ready = Self::is_ready(&image);
            let matches = image.size() == size;
            if Self::is_broken(&image) || (!matches && ready) {
                image.dispose();
                self.pending_images.borrow_mut().remove(c);
            }

            if matches && ready {
                if first_found.is_none() {
                    first_found = Some(image);
                } else {
                    found_multiple = true;
                }
            }
        }

        // We are making sure that there was at least one image of the same size in flight
        // Otherwise we might encounter UI thread lockups
        if found_multiple {
            first_found
        } else {
            None
        }
    }

    /// Starts drawing to an image of the given size, which is either reused
    /// or created by `factory`. Disposing the returned value presents the
    /// image.
    pub fn begin_draw_core(
        &self,
        factory: &(impl SwapchainImageFactory<TImage> + ?Sized),
        size: PixelSize,
    ) -> (Rc<dyn IDisposable>, Rc<TImage>) {
        let img = match self.cleanup_and_find_next_image(size) {
            Some(img) => img,
            None => factory.create_image(size),
        };

        img.begin_draw();
        {
            let mut pending_images = self.pending_images.borrow_mut();
            if let Some(index) = pending_images.iter().position(|pending| Rc::ptr_eq(pending, &img)) {
                pending_images.remove(index);
            }
        }

        let image = img.clone();
        let pending_images = self.pending_images.clone();
        let session = Disposable::create(move || {
            img.present();
            pending_images.borrow_mut().push(img);
        });
        (session, image)
    }

    /// Releases every pending image.
    pub fn dispose(&self) {
        let pending_images: Vec<Rc<TImage>> = self.pending_images.borrow().clone();
        for img in pending_images {
            img.dispose();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    struct Image {
        id: i32,
        size: PixelSize,
        last_present: Cell<Option<SwapchainImagePresentStatus>>,
        begin_draws: Cell<i32>,
        presents: Cell<i32>,
        disposed: Cell<bool>,
    }

    impl ISwapchainImage for Image {
        fn size(&self) -> PixelSize {
            self.size
        }

        fn last_present(&self) -> Option<SwapchainImagePresentStatus> {
            self.last_present.get()
        }

        fn begin_draw(&self) {
            self.begin_draws.set(self.begin_draws.get() + 1);
        }

        fn present(&self) {
            self.presents.set(self.presents.get() + 1);
            self.last_present.set(Some(SwapchainImagePresentStatus::Pending));
        }

        fn dispose(&self) {
            self.disposed.set(true);
        }
    }

    #[derive(Default)]
    struct Swapchain {
        base: SwapchainBase<Image>,
        created: RefCell<Vec<Rc<Image>>>,
    }

    impl SwapchainImageFactory<Image> for Swapchain {
        fn create_image(&self, size: PixelSize) -> Rc<Image> {
            let image = Rc::new(Image {
                id: self.created.borrow().len() as i32,
                size,
                last_present: Cell::new(None),
                begin_draws: Cell::new(0),
                presents: Cell::new(0),
                disposed: Cell::new(false),
            });
            self.created.borrow_mut().push(image.clone());
            image
        }
    }

    impl Swapchain {
        fn begin_draw(&self, size: PixelSize) -> (Rc<dyn IDisposable>, Rc<Image>) {
            self.base.begin_draw_core(self, size)
        }

        /// Draws and presents one frame; returns the id of the image used.
        fn frame(&self, size: PixelSize) -> i32 {
            let (session, image) = self.begin_draw(size);
            session.dispose();
            image.id
        }

        fn complete_all(&self) {
            for image in self.created.borrow().iter() {
                if image.last_present.get() == Some(SwapchainImagePresentStatus::Pending) {
                    image.last_present.set(Some(SwapchainImagePresentStatus::RanToCompletion));
                }
            }
        }

        fn pending(&self) -> Vec<i32> {
            self.base.pending_images.borrow().iter().map(|image| image.id).collect()
        }
    }

    const SIZE: PixelSize = PixelSize::new(100, 50);

    #[test]
    fn draw_session_begins_draws_and_presents_on_dispose() {
        let swapchain = Swapchain::default();
        let (session, image) = swapchain.begin_draw(SIZE);
        assert_eq!(image.size, SIZE);
        assert_eq!((image.begin_draws.get(), image.presents.get()), (1, 0));
        // The image being drawn to is not pending.
        assert!(swapchain.pending().is_empty());

        session.dispose();
        assert_eq!(image.presents.get(), 1);
        assert_eq!(swapchain.pending(), [0]);

        // Disposing the session again does nothing.
        session.dispose();
        assert_eq!(image.presents.get(), 1);
        assert_eq!(swapchain.pending(), [0]);
    }

    #[test]
    fn images_are_not_reused_while_their_presentation_is_pending() {
        let swapchain = Swapchain::default();
        assert_eq!(swapchain.frame(SIZE), 0);
        assert_eq!(swapchain.frame(SIZE), 1);
        assert_eq!(swapchain.frame(SIZE), 2);
        assert_eq!(swapchain.pending(), [0, 1, 2]);
    }

    #[test]
    fn an_image_is_reused_only_when_another_ready_image_of_the_size_exists() {
        let swapchain = Swapchain::default();
        assert_eq!(swapchain.frame(SIZE), 0);
        swapchain.complete_all();
        // Only one ready image: a second one is created.
        assert_eq!(swapchain.frame(SIZE), 1);
        swapchain.complete_all();
        // Two ready images: the most recently presented one is reused.
        assert_eq!(swapchain.frame(SIZE), 1);
        assert_eq!(swapchain.created.borrow().len(), 2);
        assert_eq!(swapchain.pending(), [0, 1]);
        assert_eq!(swapchain.created.borrow()[1].begin_draws.get(), 2);

        // Image 1 is in flight again, so only image 0 is ready.
        assert_eq!(swapchain.frame(SIZE), 2);
        assert_eq!(swapchain.pending(), [0, 1, 2]);
    }

    #[test]
    fn ready_images_of_another_size_are_disposed() {
        let swapchain = Swapchain::default();
        swapchain.frame(SIZE);
        swapchain.frame(SIZE);
        swapchain.complete_all();
        // Still in flight when the size changes.
        swapchain.frame(SIZE);

        let other = PixelSize::new(200, 50);
        assert_eq!(swapchain.frame(other), 2);
        let created = swapchain.created.borrow().clone();
        // Image 1 was reused for the third frame and is still pending.
        assert!(created[0].disposed.get());
        assert!(!created[1].disposed.get());
        assert_eq!(swapchain.pending(), [1, 2]);

        swapchain.complete_all();
        assert_eq!(swapchain.frame(other), 3);
        assert!(created[1].disposed.get());
        assert_eq!(swapchain.pending(), [2, 3]);
    }

    #[test]
    fn broken_images_are_disposed() {
        let swapchain = Swapchain::default();
        swapchain.frame(SIZE);
        swapchain.frame(SIZE);
        let created = swapchain.created.borrow().clone();
        created[0].last_present.set(Some(SwapchainImagePresentStatus::Faulted));
        created[1].last_present.set(Some(SwapchainImagePresentStatus::Canceled));

        assert_eq!(swapchain.frame(SIZE), 2);
        assert!(created[0].disposed.get());
        // A canceled presentation is neither broken nor ready.
        assert!(!created[1].disposed.get());
        assert_eq!(swapchain.pending(), [1, 2]);
    }

    #[test]
    fn dispose_releases_the_pending_images() {
        let swapchain = Swapchain::default();
        swapchain.frame(SIZE);
        let (_session, drawing) = swapchain.begin_draw(SIZE);
        swapchain.base.dispose();
        let created = swapchain.created.borrow().clone();
        assert!(created[0].disposed.get());
        // The image of the open session is not pending.
        assert!(!drawing.disposed.get());
    }
}
