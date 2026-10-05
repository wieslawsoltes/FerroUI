use super::renderer_factory::{create_renderer, ITopLevelRenderer};
use crate::platform::ITopLevelImpl;
use ferroui_base::input::{Cursor, FocusManager, IInputManager, IInputRoot, InputElement, PointerOverPreProcessor};
use ferroui_base::layout::{LayoutHelper, LayoutManager};
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::platform::IPlatformSettings;
use ferroui_base::reactive::{IDisposable, IObserver};
use ferroui_base::rendering::{IHitTester, IPresentationSource, IRenderer};
use ferroui_base::{
    FerroLocator, IFerroDependencyResolver, LocatorExtensions, PixelPoint, Point, Ref, Size, Visual, WeakRef,
};
use std::cell::{Cell, OnceCell, RefCell};
use std::rc::{Rc, Weak};

/// Connects the root visual of a top-level to its platform implementation,
/// renderer, layout manager and input.
pub struct PresentationSource {
    pub(super) this: Weak<PresentationSource>,
    pub(super) platform_impl: RefCell<Option<Rc<dyn ITopLevelImpl>>>,
    pub(super) pointer_over_pre_processor: RefCell<Option<Rc<PointerOverPreProcessor>>>,
    pointer_over_pre_processor_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    pub(super) input_manager: Option<Rc<dyn IInputManager>>,
    focus_manager: Rc<FocusManager>,
    root_visual: RefCell<Option<Ref<InputElement>>>,
    pub(super) focus_root: WeakRef<InputElement>,
    /// The scaling callback of the platform implementation that was in
    /// place before the source chained its own.
    scaling_changed_subscription: RefCell<Option<Rc<dyn IDisposable>>>,

    // Cursor
    pub(super) cursor: RefCell<Option<Rc<Cursor>>>,
    pub(super) cursor_override: RefCell<Option<Rc<Cursor>>>,
    pub(super) pointer_over_element: RefCell<Option<Ref<InputElement>>>,
    pub(super) cursor_element: RefCell<Option<Ref<InputElement>>>,
    pub(super) cursor_element_subscription: RefCell<Option<Rc<dyn IDisposable>>>,

    // Layout
    pub(super) layout_manager: Rc<LayoutManager>,

    // Render root
    pub(super) renderer: OnceCell<Rc<dyn ITopLevelRenderer>>,
    pub(super) scene_invalidated_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    pub(super) hit_tester_override: RefCell<Option<Rc<dyn IHitTester>>>,
    pub(super) render_scaling: Cell<f64>,
}

impl PresentationSource {
    /// Creates the presentation source of a top-level.
    ///
    /// `root_visual` becomes the root of the visual tree, `default_focus_visual`
    /// receives keyboard input when nothing is focused, `platform_impl` is
    /// the platform implementation of the top-level and the services of the
    /// source are taken from `dependency_resolver`.
    pub fn new(
        root_visual: Ref<InputElement>,
        default_focus_visual: &Ref<InputElement>,
        platform_impl: Rc<dyn ITopLevelImpl>,
        dependency_resolver: &dyn IFerroDependencyResolver,
    ) -> Rc<PresentationSource> {
        let input_manager = try_get_service::<dyn IInputManager>(dependency_resolver);

        let source = Rc::new_cyclic(|this: &Weak<PresentationSource>| {
            let layout_root: Weak<dyn ferroui_base::layout::ILayoutRoot> = this.clone();
            PresentationSource {
                this: this.clone(),
                platform_impl: RefCell::new(Some(platform_impl.clone())),
                pointer_over_pre_processor: RefCell::new(None),
                pointer_over_pre_processor_subscription: RefCell::new(None),
                input_manager,
                focus_manager: FocusManager::new(),
                root_visual: RefCell::new(None),
                focus_root: default_focus_visual.downgrade(),
                scaling_changed_subscription: RefCell::new(None),
                cursor: RefCell::new(None),
                cursor_override: RefCell::new(None),
                pointer_over_element: RefCell::new(None),
                cursor_element: RefCell::new(None),
                cursor_element_subscription: RefCell::new(None),
                layout_manager: Self::create_layout_manager(layout_root),
                renderer: OnceCell::new(),
                scene_invalidated_subscription: RefCell::new(None),
                hit_tester_override: RefCell::new(None),
                render_scaling: Cell::new(1.0),
            }
        });

        platform_impl.set_input_root(source.clone());
        let weak = Rc::downgrade(&source);
        platform_impl.set_input(Some(Rc::new(move |e| {
            if let Some(source) = weak.upgrade() {
                source.handle_input(e);
            }
        })));

        source.render_scaling.set(LayoutHelper::validate_scaling(platform_impl.render_scaling()));
        let weak = Rc::downgrade(&source);
        *source.scaling_changed_subscription.borrow_mut() = Some(add_scaling_changed(
            &platform_impl,
            Rc::new(move |scaling| {
                if let Some(source) = weak.upgrade() {
                    source.handle_scaling_changed(scaling);
                }
            }),
        ));

        let pointer_over_pre_processor = Rc::new(PointerOverPreProcessor::new(source.clone()));
        *source.pointer_over_pre_processor.borrow_mut() = Some(pointer_over_pre_processor.clone());
        *source.pointer_over_pre_processor_subscription.borrow_mut() =
            source.input_manager.as_ref().map(|input_manager| input_manager.pre_process().subscribe(pointer_over_pre_processor));

        let weak_impl = Rc::downgrade(&platform_impl);
        let renderer = create_renderer(
            source.clone(),
            platform_impl.compositor(),
            Rc::new(move || weak_impl.upgrade().map(|platform_impl| platform_impl.surfaces()).unwrap_or_default()),
        );
        let weak = Rc::downgrade(&source);
        *source.scene_invalidated_subscription.borrow_mut() = Some(renderer.scene_invalidated(Rc::new(move |e| {
            if let Some(source) = weak.upgrade() {
                source.scene_invalidated(e);
            }
        })));
        if source.renderer.set(renderer).is_err() {
            unreachable!("the renderer is created once");
        }

        source.set_root_visual(Some(root_visual));
        source
    }

    /// The platform implementation of the top-level; `None` once the source
    /// has been disposed.
    pub fn platform_impl(&self) -> Option<Rc<dyn ITopLevelImpl>> {
        self.platform_impl.borrow().clone()
    }

    /// The focus manager of the tree.
    pub fn focus_manager(&self) -> &Rc<FocusManager> {
        &self.focus_manager
    }

    /// The root of the visual tree; `None` after the top-level has closed.
    pub fn root_visual(&self) -> Option<Ref<InputElement>> {
        self.root_visual.borrow().clone()
    }

    /// Sets the root of the visual tree: the old root is detached from the
    /// source and the new one attached to it.
    pub fn set_root_visual(&self, value: Option<Ref<InputElement>>) {
        let old = self.root_visual.borrow().clone();
        if let Some(old) = old {
            old.set_presentation_source_for_root_visual(None);
        }
        *self.root_visual.borrow_mut() = value.clone();

        if let Some(new) = &value {
            let this: Rc<dyn IPresentationSource> = self.rc();
            new.set_presentation_source_for_root_visual(Some(this));
        }
        self.typed_renderer().set_root(value.clone().map(Ref::upcast));

        self.focus_manager.set_content_root(value);
    }

    /// The root input element; the root visual.
    ///
    /// # Panics
    /// Panics after the top-level has closed.
    pub fn root_element(&self) -> Ref<InputElement> {
        self.root_visual().expect("the presentation source has no root visual")
    }

    /// The presentation source whose root visual is `root_visual`, if the
    /// visual is the root of one (upstream casts the source of the root
    /// visual to this class).
    pub fn from_root_visual(root_visual: &ferroui_base::Visual) -> Option<Rc<PresentationSource>> {
        let host = root_visual.downcast_ref::<crate::top_level_host::TopLevelHost>()?;
        Some(host.top_level()?.presentation_source().clone())
    }

    pub(super) fn rc(&self) -> Rc<PresentationSource> {
        self.this.upgrade().expect("the presentation source is alive while it is used")
    }

    /// The platform settings of the application.
    pub fn platform_settings(&self) -> Option<Rc<dyn IPlatformSettings>> {
        FerroLocator::current().get_service::<dyn IPlatformSettings>()
    }

    /// Detaches the source from its platform implementation and releases
    /// the renderer, the layout manager and the input subscriptions.
    pub fn dispose(&self) {
        if let Some(platform_impl) = self.platform_impl() {
            platform_impl.set_input(None);
        }
        if let Some(subscription) = self.scaling_changed_subscription.borrow_mut().take() {
            subscription.dispose();
        }

        ferroui_base::layout::ILayoutManager::dispose(&*self.layout_manager);
        if let Some(subscription) = self.scene_invalidated_subscription.borrow_mut().take() {
            subscription.dispose();
        }
        // We need to wait for the renderer to complete any in-flight operations
        IRenderer::dispose(&**self.typed_renderer());

        *self.platform_impl.borrow_mut() = None;
        let pointer_over_pre_processor = self.pointer_over_pre_processor.borrow_mut().take();
        if let Some(pointer_over_pre_processor) = pointer_over_pre_processor {
            pointer_over_pre_processor.on_completed();
        }
        if let Some(subscription) = self.pointer_over_pre_processor_subscription.borrow_mut().take() {
            subscription.dispose();
        }
        if let Some(subscription) = self.cursor_element_subscription.borrow_mut().take() {
            subscription.dispose();
        }
    }

    /// The last known position of the pointer, in screen coordinates.
    pub fn get_last_pointer_position(&self, _top_level: &Visual) -> Option<PixelPoint> {
        self.pointer_over_pre_processor.borrow().as_ref().and_then(|p| p.last_position())
    }
}

impl PresentationSource {
    pub(super) fn chrome_hit_test_filter(visual: &Visual) -> bool {
        if !(visual.is_visible() && visual.is_attached_to_visual_tree()) {
            return false;
        }
        if !visual
            .downcast_ref::<InputElement>()
            .is_some_and(|element| element.is_effectively_visible() && element.is_hit_test_visible())
        {
            return false;
        }

        // Allow traversal into any container that might contain chrome elements
        true
    }

    pub(super) fn get_chrome_role_from_visual(
        mut visual: Option<Ref<Visual>>,
    ) -> Option<ferroui_base::input::WindowDecorationsElementRole> {
        use ferroui_base::input::WindowDecorationsElementRole;

        while let Some(current) = visual {
            let role = crate::chrome::WindowDecorationProperties::get_element_role(&current);
            if role != WindowDecorationsElementRole::None {
                return Some(role);
            }
            visual = current.visual_parent();
        }
        None
    }
}

/// Chains `handler` onto the scaling-changed callback of a platform
/// implementation (C# `impl.ScalingChanged += handler`). Disposing the
/// returned handle stops `handler` from being called.
pub(crate) fn add_scaling_changed(platform_impl: &Rc<dyn ITopLevelImpl>, handler: Rc<dyn Fn(f64)>) -> Rc<dyn IDisposable> {
    let previous = platform_impl.scaling_changed();
    let active = Rc::new(Cell::new(true));
    let is_active = active.clone();
    platform_impl.set_scaling_changed(Some(Rc::new(move |scaling| {
        if let Some(previous) = &previous {
            previous(scaling);
        }
        if is_active.get() {
            handler(scaling);
        }
    })));
    ferroui_base::reactive::Disposable::create(move || active.set(false))
}

/// Tries to get a service from a dependency resolver, logging a warning if
/// not found.
pub(crate) fn try_get_service<T: ?Sized + 'static>(resolver: &dyn IFerroDependencyResolver) -> Option<Rc<T>> {
    let result = resolver.get_service::<T>();

    if result.is_none() {
        if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::CONTROL) {
            logger.log_with_values(
                None,
                "Could not create {Service} : maybe Application.RegisterServices() wasn't called?",
                &[&std::any::type_name::<T>()],
            );
        }
    }

    result
}

impl IPresentationSource for PresentationSource {
    fn root_visual(&self) -> Option<Ref<Visual>> {
        PresentationSource::root_visual(self).map(Ref::upcast)
    }

    fn render_scaling(&self) -> f64 {
        self.render_scaling.get()
    }

    fn renderer(&self) -> Rc<dyn IRenderer> {
        self.typed_renderer().clone()
    }

    fn layout_root(&self) -> Rc<dyn ferroui_base::layout::ILayoutRoot> {
        self.rc()
    }

    fn client_size(&self) -> Size {
        PresentationSource::client_size(self)
    }

    fn platform_settings(&self) -> Option<Rc<dyn IPlatformSettings>> {
        PresentationSource::platform_settings(self)
    }

    fn hit_tester(&self) -> Rc<dyn IHitTester> {
        PresentationSource::hit_tester(self)
    }

    fn input_root(&self) -> Rc<dyn IInputRoot> {
        self.rc()
    }

    fn point_to_screen(&self, point: Point) -> Option<PixelPoint> {
        self.platform_impl().map(|platform_impl| platform_impl.point_to_screen(point))
    }

    fn point_to_client(&self, point: PixelPoint) -> Option<Point> {
        self.platform_impl().map(|platform_impl| platform_impl.point_to_client(point))
    }
}
