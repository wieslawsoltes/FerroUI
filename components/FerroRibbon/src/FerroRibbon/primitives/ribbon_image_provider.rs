//! Scoped application image providers. Native DrawingImage owns the displayed data.
use super::RibbonImageLoadError;
use ferroui_base::{
    FerroObject, FerroObjectImpl, Rect, Ref, Size, ferro_class, ferro_class_info, instantiate,
    media::{
        DrawingContext, DrawingImage, IAffectsRender, IImage, ImageDrawing, ImageLoadHandler,
        imaging::IBitmap,
    },
    reactive::{Disposable, SerialDisposable, Subscription},
    threading::{Dispatcher, DispatcherPriority},
    utilities::{HandlerList, UriRef},
};
use ferroui_controls::Image;
use ferroui_system::task::JoinHandle;
use std::{
    any::Any,
    cell::{Cell, RefCell},
    future::Future,
    pin::Pin,
    rc::Rc,
};

/// The source chooses an SVG or bitmap decoder before starting the request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RibbonImageKind {
    Bitmap,
    Svg,
}

/// A provider request polled on the native UI dispatcher.
pub type RibbonImageFuture =
    Pin<Box<dyn Future<Output = Result<Rc<dyn IImage>, RibbonImageLoadError>>>>;

/// An application image loader registered with FerroLocator.
pub trait RibbonImageProvider: 'static {
    /// Receives the resolved native URI and decoder kind. Return None to use
    /// FerroRibbon's built-in URI loader.
    fn open(&self, uri: &UriRef, kind: RibbonImageKind) -> Option<RibbonImageFuture>;
}

/// The result of an image load or assignment of a decoded native bitmap.
pub type RibbonImageLoadResult = Result<(), Rc<RibbonImageLoadError>>;

/// Observes load outcomes for the source currently assigned to a native image.
///
/// The handler runs on the UI dispatcher and follows completion for the
/// current source. A new subscription can receive a URI failure queued for
/// delivery, even when its error is already available on the source.
/// Subsequent assignments of a native bitmap can report readiness. The
/// returned guard ends observation when released.
pub fn observe_image_loads(
    image: &Image,
    handler: impl Fn(&RibbonImageLoadResult) + 'static,
) -> Subscription {
    let active = Rc::new(RefCell::new(None::<Subscription>));
    let current = active.clone();
    let target = image.to_ref().downgrade();
    let handler = Rc::new(handler);
    let watch = Rc::new(move |source: Option<Rc<dyn IImage>>, include_ready: bool| {
        drop(current.take());
        let Some(source) = source else { return };
        let deferred = source
            .as_object()
            .and_then(|value| value.downcast_ref::<RibbonDeferredImage>());
        let can_notify = match deferred {
            Some(deferred) => deferred.completion.pending.get(),
            None => include_ready && source.as_bitmap().is_some(),
        };
        if !can_notify {
            return;
        }

        let load = Rc::new(SerialDisposable::new());
        current.replace(Some(Subscription::new(load.clone())));
        let token = Rc::downgrade(&load);
        let target = target.clone();
        let source = Rc::downgrade(&source);
        let handler = Rc::downgrade(&handler);
        let notify = move |result: &RibbonImageLoadResult| {
            let (token, target, source, handler, result) = (
                token.clone(),
                target.clone(),
                source.clone(),
                handler.clone(),
                result.clone(),
            );
            Dispatcher::ui_thread().post_local(
                move || {
                    if let (Some(_load), Some(target), Some(source), Some(handler)) = (
                        token.upgrade(),
                        target.upgrade(),
                        source.upgrade(),
                        handler.upgrade(),
                    ) && target.source().as_ref() == Some(&source)
                    {
                        handler(&result);
                    }
                },
                DispatcherPriority::NORMAL,
            );
        };
        if let Some(deferred) = deferred {
            load.set(
                deferred
                    .completion
                    .subscribe(notify)
                    .map(Subscription::detach),
            );
        } else {
            notify(&Ok(()));
        }
    });
    let follow = watch.clone();
    let changes = image.property_changed(move |change| {
        if change.property() == Image::source_property().as_property()
            && let Some(image) = change.sender().downcast_ref::<Image>()
        {
            let source = image.source();
            if source == change.get_new_value::<Option<Rc<dyn IImage>>>() {
                follow(source, true);
            }
        }
    });
    watch(image.source(), false);
    Subscription::new(Disposable::create(move || {
        changes.dispose();
        drop(active.take());
    }))
}

struct ImageCompletion {
    pending: Cell<bool>,
    handlers: HandlerList<dyn Fn(&RibbonImageLoadResult)>,
}

impl ImageCompletion {
    fn new() -> Rc<Self> {
        Rc::new(Self {
            pending: Cell::new(true),
            handlers: HandlerList::new(),
        })
    }

    fn subscribe(
        self: &Rc<Self>,
        handler: impl Fn(&RibbonImageLoadResult) + 'static,
    ) -> Option<Subscription> {
        if !self.pending.get() {
            return None;
        }
        let token = self.handlers.add(Rc::new(handler));
        let weak = Rc::downgrade(self);
        Some(Subscription::new(Disposable::create(move || {
            if let Some(completion) = weak.upgrade() {
                completion.handlers.remove(token);
            }
        })))
    }

    fn queue(self: &Rc<Self>, result: RibbonImageLoadResult) {
        let weak = Rc::downgrade(self);
        Dispatcher::ui_thread().post_local(
            move || {
                if let Some(completion) = weak.upgrade() {
                    completion.publish(&result);
                }
            },
            DispatcherPriority::NORMAL,
        );
    }

    fn publish(&self, result: &RibbonImageLoadResult) {
        self.pending.set(false);
        let handlers = self.handlers.snapshot();
        for (token, _) in handlers.iter() {
            self.handlers.remove(*token);
        }
        for (_, handler) in handlers.iter() {
            handler(result);
        }
    }
}

/// Retains native bitmap and drawing interfaces while adding load completion.
pub(super) struct RibbonUriImage {
    image: Rc<dyn IImage>,
    completion: Rc<ImageCompletion>,
}

impl RibbonUriImage {
    pub(super) fn decoded(image: Rc<dyn IImage>) -> Rc<dyn IImage> {
        let completion = ImageCompletion::new();
        completion.queue(Ok(()));
        Rc::new(Self { image, completion })
    }
}

impl IImage for RibbonUriImage {
    fn size(&self) -> Size {
        self.image.size()
    }

    fn draw(&self, context: &mut DrawingContext<'_>, source: Rect, dest: Rect) {
        self.image.draw(context, source, dest);
    }

    fn as_any(&self) -> &dyn Any {
        self.image.as_any()
    }

    fn as_object(&self) -> Option<&FerroObject> {
        self.image.as_object()
    }

    fn as_bitmap(&self) -> Option<&dyn IBitmap> {
        self.image.as_bitmap()
    }

    fn as_affects_render(&self) -> Option<&dyn IAffectsRender> {
        self.image.as_affects_render()
    }

    fn subscribe_load(&self, handler: ImageLoadHandler) -> Option<Subscription> {
        self.completion.subscribe(move |result| {
            let result = result
                .clone()
                .map_err(|error| error as Rc<dyn std::error::Error>);
            handler(&result);
        })
    }

    fn reference_id(&self) -> *const () {
        self.image.reference_id()
    }
}

/// The image source for a pending request or a completed loading failure.
///
/// Its native drawing is empty until completion. A retained source belongs to
/// its consumers, so completing it never changes another image's Source.
#[repr(C)]
pub struct RibbonDeferredImage {
    base: DrawingImage,
    task: RefCell<Option<JoinHandle<()>>>,
    size_subscription: RefCell<Option<Subscription>>,
    result: RefCell<Option<RibbonImageLoadResult>>,
    completion: Rc<ImageCompletion>,
}
ferro_class!(RibbonDeferredImage:DrawingImage);
ferro_class_info!(RibbonDeferredImage {});
impl FerroObjectImpl for RibbonDeferredImage {}

impl RibbonDeferredImage {
    pub(super) fn start(request: RibbonImageFuture) -> Rc<dyn IImage> {
        let image = Self::with_result(None);
        let weak = image.downgrade();
        let task = Dispatcher::ui_thread().spawn_local(async move {
            let result = request.await;
            if let Some(image) = weak.upgrade() {
                image.complete(result);
            }
        });
        image.task.replace(Some(task));
        Self::as_source(image)
    }

    pub(crate) fn failed(error: RibbonImageLoadError) -> Rc<dyn IImage> {
        let result = Err(Rc::new(error));
        let image = Self::with_result(Some(result.clone()));
        image.completion.queue(result);
        Self::as_source(image)
    }

    fn as_source(image: Ref<Self>) -> Rc<dyn IImage> {
        let completion = image.completion.clone();
        Rc::new(RibbonUriImage {
            image: image.upcast::<DrawingImage>().into(),
            completion,
        })
    }

    fn with_result(result: Option<RibbonImageLoadResult>) -> Ref<Self> {
        instantiate(Self {
            base: DrawingImage::construct(),
            task: RefCell::default(),
            size_subscription: RefCell::default(),
            result: RefCell::new(result),
            completion: ImageCompletion::new(),
        })
    }

    pub fn load_result(&self) -> Option<RibbonImageLoadResult> {
        self.result.borrow().clone()
    }

    /// Observes completion on the UI dispatcher or invokes the handler immediately
    /// with the result if the image is already complete.
    pub fn completed(&self, handler: impl Fn(&RibbonImageLoadResult) + 'static) -> Subscription {
        if let Some(result) = self.load_result() {
            handler(&result);
            return Subscription::new(Disposable::empty());
        }
        self.completion
            .subscribe(handler)
            .expect("pending image load")
    }

    fn complete(&self, result: Result<Rc<dyn IImage>, RibbonImageLoadError>) {
        drop(self.task.take());
        let (drawing, result) = match result {
            Ok(image) => {
                let drawing = ImageDrawing::new();
                drawing.set_rect(Rect::from_size(image.size()));
                if let Some(render) = image.as_affects_render() {
                    // Refresh layout before the drawing forwards render invalidation.
                    let weak = drawing.downgrade();
                    let subscription = render.invalidated(Rc::new(move || {
                        if let Some(drawing) = weak.upgrade()
                            && let Some(image) = drawing.image_source()
                        {
                            let rect = Rect::from_size(image.size());
                            if drawing.rect() != rect {
                                drawing.set_rect(rect);
                            }
                        }
                    }));
                    self.size_subscription
                        .replace(Some(Subscription::new(subscription)));
                }
                drawing.set_image_source(Some(image));
                (Some(drawing), Ok(()))
            }
            Err(error) => (None, Err(Rc::new(error))),
        };
        self.result.replace(Some(result.clone()));
        if let Some(drawing) = drawing {
            self.set_drawing(drawing);
        }
        self.completion.publish(&result);
    }
}

impl Drop for RibbonDeferredImage {
    fn drop(&mut self) {
        if let Some(task) = self.task.get_mut().take() {
            task.abort();
        }
    }
}
