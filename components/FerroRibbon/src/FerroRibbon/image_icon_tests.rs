use crate::{
    compat::ImageIcon, controls::RibbonItemHelper, primitives::RibbonIconPresenter, tests::run,
};
use ferroui_base::{
    Rect, Ref, Size,
    media::{DrawingContext, IImage},
    threading::Dispatcher,
};
use ferroui_controls::{Control, Grid, Image, Viewbox, Window};
use std::rc::Rc;
struct TestImage(Size);
impl IImage for TestImage {
    fn size(&self) -> Size {
        self.0
    }
    fn draw(&self, _: &mut DrawingContext<'_>, _: Rect, _: Rect) {}
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
fn root(icon: &ImageIcon) -> Ref<Grid> {
    icon.visual_children().get(0).cast::<Grid>().unwrap()
}
#[test]
fn image_uri_failures_leave_the_ribbon_presenter_replaceable() {
    use crate::primitives::{RibbonDeferredImage, RibbonImageLoadError};

    run(|| {
        let presenter = RibbonIconPresenter::new();
        presenter.set_icon(Some(Rc::new("ribbon-probe://local/icon.png".to_owned())));
        let image = presenter.children().get(0).cast::<Image>().unwrap();
        let source = image.source().unwrap();
        let failed = source
            .as_object()
            .unwrap()
            .downcast_ref::<RibbonDeferredImage>()
            .unwrap();
        assert!(matches!(
            failed.load_result().unwrap().unwrap_err().as_ref(),
            RibbonImageLoadError::UnsupportedScheme(_)
        ));
        assert_eq!(source.size(), Size::default());

        presenter.set_icon(Some(Rc::new("M0,0 L10,0 L10,10 Z".to_owned())));
        assert!(presenter.has_icon());
        assert!(presenter.children().get(0).cast::<Image>().is_none());
    });
}

#[test]
fn native_image_icon_retains_tree_across_reload_and_updates_shared_source() {
    run(|| {
        let a: Rc<dyn IImage> = Rc::new(TestImage(Size::new(30., 20.)));
        let b: Rc<dyn IImage> = Rc::new(TestImage(Size::new(60., 40.)));
        let icon = ImageIcon::new();
        assert!(icon.source().is_none());
        icon.set_source(Some(a.clone()));
        assert!(icon.visual_children().is_empty());
        icon.measure(Size::INFINITY);
        assert_eq!(icon.desired_size(), Size::default());
        let window = Window::new();
        window.set_content(Some(Control::boxed(icon.clone())));
        window.show();
        Dispatcher::ui_thread().run_jobs(None);
        assert!(icon.is_loaded());
        let grid = root(&icon);
        let image = grid.children().get(0).cast::<Image>().unwrap();
        assert!(Rc::ptr_eq(&image.source().unwrap(), &a));
        assert_eq!(image.stretch(), ferroui_base::media::Stretch::Uniform);
        icon.set_source(Some(b.clone()));
        assert!(Rc::ptr_eq(&image.source().unwrap(), &b));
        icon.set_source(None);
        assert!(image.source().is_none());
        window.set_content(None);
        assert!(!icon.is_loaded());
        icon.set_source(Some(a.clone()));
        assert!(Rc::ptr_eq(&image.source().unwrap(), &a));
        window.set_content(Some(Control::boxed(icon.clone())));
        Dispatcher::ui_thread().run_jobs(None);
        assert!(root(&icon).ptr_eq(&grid));
        assert!(grid.children().get(0).ptr_eq(&image));
        assert_eq!(icon.visual_children().count(), 1);
        window.set_content(None);
        window.close();
        let weak = icon.downgrade();
        drop(icon);
        assert!(weak.upgrade().is_none());
    });
}
#[test]
fn image_icon_presenter_and_menu_clones_share_images_without_stealing_controls() {
    run(|| {
        let image: Rc<dyn IImage> = Rc::new(TestImage(Size::new(30., 20.)));
        let source = ImageIcon::new();
        source.set_source(Some(image.clone()));
        let parent = ferroui_controls::Panel::new();
        parent.children().add(source.clone());
        let presenter = RibbonIconPresenter::new();
        presenter.set_icon(Some(Control::boxed(source.clone())));
        let clone = presenter
            .children()
            .get(0)
            .cast::<Viewbox>()
            .unwrap()
            .child()
            .unwrap()
            .cast::<ImageIcon>()
            .unwrap();
        assert!(!clone.ptr_eq(&source));
        assert!(source.parent().unwrap().ptr_eq(&parent));
        assert!(Rc::ptr_eq(&clone.source().unwrap(), &image));
        assert_eq!((clone.width(), clone.height()), (16., 16.));
        let menu = RibbonItemHelper::create_menu_icon(Some(Control::boxed(source.clone())))
            .unwrap()
            .unwrap()
            .cast::<ImageIcon>()
            .unwrap();
        assert!(!menu.ptr_eq(&source));
        assert!(menu.width().is_nan() && menu.height().is_nan());
        assert!(Rc::ptr_eq(&menu.source().unwrap(), &image));
        source.set_source(None);
        assert!(clone.source().is_some() && menu.source().is_some());
        presenter.set_icon_size(24.);
        let resized = presenter
            .children()
            .get(0)
            .cast::<Viewbox>()
            .unwrap()
            .child()
            .unwrap()
            .cast::<ImageIcon>()
            .unwrap();
        assert!(resized.source().is_none());
        assert_eq!((resized.width(), resized.height()), (24., 24.));
        let weak = Rc::downgrade(&image);
        presenter.set_icon(None);
        menu.set_source(None);
        drop(image);
        drop(clone);
        drop(menu);
        assert!(weak.upgrade().is_none());
    });
}

#[test]
fn source_menu_uri_whitespace_uses_the_exact_selected_native_resource() {
    run(|| crate::menu_image_uri_cases::assert_source_resource_routes(None));
}

#[test]
fn uri_controls_use_deferred_provider_results_and_cancel_released_sources() {
    run(|| {
        use crate::{
            compat::BitmapIcon,
            primitives::{
                RibbonDeferredImage, RibbonImageFuture, RibbonImageKind, RibbonImageLoadError,
                RibbonImageProvider, observe_image_loads,
            },
        };
        use ferroui_base::{
            FerroLocator,
            media::{DrawingImage, GeometryDrawing, RectangleGeometry},
            utilities::{Uri, UriRef},
        };
        use futures_channel::oneshot;
        use std::{
            cell::{Cell, RefCell},
            collections::HashMap,
        };

        type ImageResult = Result<Rc<dyn IImage>, RibbonImageLoadError>;
        #[derive(Default)]
        struct Provider {
            requests: RefCell<Vec<(String, RibbonImageKind)>>,
            responses: RefCell<HashMap<String, oneshot::Receiver<ImageResult>>>,
        }
        impl Provider {
            fn request(&self, path: &str) -> oneshot::Sender<ImageResult> {
                let (sender, receiver) = oneshot::channel();
                self.responses
                    .borrow_mut()
                    .insert(path.to_owned(), receiver);
                sender
            }
        }
        impl RibbonImageProvider for Provider {
            fn open(&self, uri: &UriRef, kind: RibbonImageKind) -> Option<RibbonImageFuture> {
                let path = uri.as_url()?.path();
                self.requests.borrow_mut().push((path.to_owned(), kind));
                let receiver = self.responses.borrow_mut().remove(path)?;
                Some(Box::pin(async move {
                    receiver.await.expect("test image response")
                }))
            }
        }

        let _scope = FerroLocator::enter_scope();
        let provider = Rc::new(Provider::default());
        FerroLocator::current_mutable()
            .bind::<dyn RibbonImageProvider>()
            .to_constant(provider.clone() as Rc<dyn RibbonImageProvider>);
        let settle = || Dispatcher::ui_thread().run_jobs(None);
        let source = |presenter: &RibbonIconPresenter| {
            presenter
                .children()
                .get(0)
                .cast::<Image>()
                .unwrap()
                .source()
                .unwrap()
        };
        let deferred = |image: &Rc<dyn IImage>| {
            image
                .as_object()
                .unwrap()
                .downcast_ref::<RibbonDeferredImage>()
                .unwrap()
                .to_ref()
        };

        let presenter = RibbonIconPresenter::new();
        let ready = provider.request("/presenter.svg");
        presenter.set_icon(Some(Rc::new(
            "https://example.invalid/presenter.svg".to_owned(),
        )));
        let visible = source(&presenter);
        let pending = deferred(&visible);
        assert!(pending.load_result().is_none());
        assert_eq!(visible.size(), Size::default());
        assert!(provider.requests.borrow().is_empty());
        let window = Window::new();
        window.set_content(Some(Control::boxed(presenter.clone())));
        window.show();
        let image = presenter.children().get(0).cast::<Image>().unwrap();
        image.measure(Size::INFINITY);
        assert!(image.is_attached_to_visual_tree() && image.is_measure_valid());
        let control_opened = Rc::new(Cell::new(0));
        let control_count = control_opened.clone();
        let _control = observe_image_loads(&image, move |result| {
            assert!(result.is_ok());
            control_count.set(control_count.get() + 1);
        });
        let opened = Rc::new(Cell::new(0));
        let count = opened.clone();
        let _completion = pending.completed(move |result| {
            assert!(result.is_ok());
            count.set(count.get() + 1);
        });
        settle();
        assert_eq!(provider.requests.borrow()[0].1, RibbonImageKind::Svg);
        let size = Size::new(24., 12.);
        let drawing = GeometryDrawing::new();
        drawing.set_geometry(RectangleGeometry::with_rect(Rect::from_size(size)));
        let loaded = DrawingImage::with_drawing(&drawing);
        assert!(ready.send(Ok(loaded.clone().into())).is_ok());
        settle();
        assert_eq!(opened.get(), 1);
        assert_eq!(control_opened.get(), 1);
        assert_eq!(source(&presenter).size(), size);
        assert!(Rc::ptr_eq(&source(&presenter), &visible));
        let cached = opened.clone();
        let _cached = pending.completed(move |result| {
            assert!(result.is_ok());
            cached.set(cached.get() + 1);
        });
        assert_eq!(opened.get(), 2);
        assert!(image.is_measure_valid());
        assert_eq!(image.bounds().size(), Size::new(16., 8.));
        let first_size = Rc::new(Cell::new(None));
        let observed_size = first_size.clone();
        let source_ref = pending.downgrade();
        let _invalidation = pending.invalidated(move || {
            if observed_size.get().is_none() {
                observed_size.set(source_ref.upgrade().map(|source| source.size()));
            }
        });
        loaded.set_viewbox(Some(Rect::new(0., 0., 12., 24.)));
        assert_eq!(first_size.get(), Some(Size::new(12., 24.)));
        settle();
        assert_eq!(visible.size(), Size::new(12., 24.));
        assert_eq!(image.bounds().size(), Size::new(8., 16.));
        assert!(Rc::ptr_eq(&source(&presenter), &visible));
        loaded.set_viewbox(None);
        loaded.set_drawing(None);
        settle();
        assert_eq!(visible.size(), Size::default());
        assert_eq!(opened.get(), 2);
        assert_eq!(control_opened.get(), 1);
        window.set_content(None);
        window.close();
        drop(image);

        let menu_result = provider.request("/menu.svg");
        let menu = RibbonItemHelper::create_menu_icon(Some(Rc::new(
            "https://example.invalid/menu.svg".to_owned(),
        )))
        .unwrap()
        .unwrap()
        .cast::<ImageIcon>()
        .unwrap();
        let menu_source = menu.source().unwrap();
        settle();
        assert_eq!(provider.requests.borrow()[1].1, RibbonImageKind::Bitmap);
        assert!(menu_result.send(Ok(Rc::new(TestImage(size)))).is_ok());
        settle();
        assert_eq!(menu_source.size(), size);

        let bitmap_result = provider.request("/broken.png");
        let bitmap = BitmapIcon::new();
        bitmap.set_uri_source(Some(
            Uri::absolute("https://example.invalid/broken.png").unwrap(),
        ));
        settle();
        assert!(bitmap.load_error().is_none());
        assert!(
            bitmap_result
                .send(Err(RibbonImageLoadError::Load(std::io::Error::other(
                    "decode failed"
                ))))
                .is_ok()
        );
        settle();
        assert!(matches!(
            bitmap.load_error().as_deref(),
            Some(RibbonImageLoadError::Load(_))
        ));

        let retained = provider.request("/retained.png");
        presenter.set_icon(Some(Rc::new(
            "https://example.invalid/retained.png".to_owned(),
        )));
        let retired_source = source(&presenter);
        let replacement = provider.request("/replacement.png");
        presenter.set_icon(Some(Rc::new(
            "https://example.invalid/replacement.png".to_owned(),
        )));
        let current_source = source(&presenter);
        let control = presenter.children().get(0).cast::<Image>().unwrap();
        control.set_source(Some(retired_source.clone()));
        let control_completions = Rc::new(Cell::new(0));
        let observed = control_completions.clone();
        let _control_events = observe_image_loads(&control, move |result| {
            assert!(result.is_ok());
            observed.set(observed.get() + 1);
        });
        control.set_source(Some(current_source.clone()));
        let completions = Rc::new(RefCell::new(Vec::new()));
        let subscriptions: Vec<_> = [(&retired_source, "retired"), (&current_source, "current")]
            .into_iter()
            .map(|(source, label)| {
                let completions = completions.clone();
                deferred(source).completed(move |result| {
                    assert!(result.is_ok());
                    completions.borrow_mut().push(label);
                })
            })
            .collect();
        settle();
        assert!(replacement.send(Ok(Rc::new(TestImage(size)))).is_ok());
        settle();
        assert_eq!(&*completions.borrow(), &["current"]);
        assert_eq!(control_completions.get(), 1);
        let retired_size = Size::new(8., 8.);
        assert!(retained.send(Ok(Rc::new(TestImage(retired_size)))).is_ok());
        settle();
        assert!(deferred(&retired_source).load_result().unwrap().is_ok());
        assert_eq!(retired_source.size(), retired_size);
        assert_eq!(current_source.size(), size);
        assert!(Rc::ptr_eq(&source(&presenter), &current_source));
        assert_eq!(&*completions.borrow(), &["current", "retired"]);
        assert_eq!(control_completions.get(), 1);
        control.set_source(Some(retired_source));
        settle();
        control.set_source(Some(current_source));
        settle();
        assert_eq!(control_completions.get(), 1);
        drop(subscriptions);

        let superseded = provider.request("/retired.png");
        presenter.set_icon(Some(Rc::new(
            "https://example.invalid/retired.png".to_owned(),
        )));
        let _events = {
            let image = presenter.children().get(0).cast::<Image>().unwrap();
            observe_image_loads(&image, |_| {
                panic!("the discarded image has no load outcome")
            })
        };
        settle();
        presenter.set_icon(None);
        settle();
        assert!(superseded.is_canceled());
        assert!(presenter.children().is_empty());

        ferroui_base::platform::register_assets(
            "ribbon_deferred_fallback",
            &[(
                "fallback.png",
                include_bytes!("../../tests/fixtures/images/bitmap-mask-3x2.png"),
            )],
        );
        presenter.set_icon(Some(Rc::new(
            "ferres://ribbon_deferred_fallback/fallback.png".to_owned(),
        )));
        let fallback = source(&presenter);
        assert!(deferred(&fallback).load_result().is_none());
        settle();
        assert!(deferred(&fallback).load_result().unwrap().is_ok());
        assert!(fallback.size().width > 0. && fallback.size().height > 0.);
    });
}
