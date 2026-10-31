use crate::{TextureFrameImporter, TexturePresenter};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties,
    input::InputElementImpl,
    instantiate,
    interactivity::InteractiveImpl,
    layout::LayoutableImpl,
    media::{Brushes, Stretch},
    reactive::IDisposable,
    threading::{Dispatcher, DispatcherPriority, DispatcherTimer},
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Point, Ref,
    StyledElementImpl, StyledProperty, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::{ControlImpl, Panel, PanelImpl, TopLevel};
use ferroui_media::{
    frames::*, MediaError, MediaErrorKind, MediaEvent, MediaSubscription, RenderSchedule,
    TextureRenderer, TextureRendererHost,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
    time::Duration,
};

/// A normal composition control which schedules a client-owned renderer. Input
/// mailboxes borrow already-acquired frames; this control never opens a source.
#[repr(C)]
pub struct RendererHost {
    base: Panel,
    presenter: RefCell<Option<Ref<TexturePresenter>>>,
    host: RefCell<Option<TextureRendererHost>>,
    inputs: RefCell<Vec<Arc<FrameMailbox>>>,
    last_inputs: RefCell<Option<Vec<FrameIdentity>>>,
    timer: RefCell<Option<Rc<dyn IDisposable>>>,
    errors: MediaEvent<MediaError>,
    presenter_errors: RefCell<Option<MediaSubscription>>,
    failed: Cell<bool>,
    generation: Cell<u64>,
}
ferro_class!(RendererHost:Panel);
ferro_class_info!(RendererHost {
    new: RendererHost::new
});
ferro_impl_classes!(RendererHost:StyledElementImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,PanelImpl);
impl ControlImpl for RendererHost {
    fn on_create_automation_peer(this: &Self) -> Ref<ferroui_controls::automation::peers::AutomationPeer> {
        ferroui_controls::automation::peers::ControlAutomationPeer::new(this).upcast()
    }
}
ferro_properties! {impl RendererHost {
    pub fn schedule_property()->StyledProperty<RenderSchedule>{FerroProperty::register::<Self,_>("Schedule",RenderSchedule::OnInvalidation)}
}}
impl FerroObjectImpl for RendererHost {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        let presenter = TexturePresenter::new();
        presenter.set_stretch(Stretch::Fill);
        let weak = this.to_ref().downgrade();
        this.presenter_errors
            .replace(Some(presenter.errors().subscribe(move |error| {
                if let Some(view) = weak.upgrade() {
                    view.failed.set(true);
                    if let Some(host) = view.host.borrow().as_ref() {
                        host.set_active(false);
                    }
                    view.errors.emit(error);
                }
            })));
        this.children().add(&presenter);
        this.presenter.replace(Some(presenter));
        this.set_clip_to_bounds(true);
        this.set_background(Some(Brushes::transparent()));
        this.set_focusable(true);
    }
    fn on_property_changed(this: &Self, args: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, args);
        if args.property() == Self::schedule_property().as_property() {
            if let Some(host) = this.host.borrow().as_ref() {
                host.set_schedule(this.schedule());
            }
        }
    }
}
impl VisualImpl for RendererHost {
    fn on_attached_to_visual_tree(this: &Self, args: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, args);
        this.start();
    }
    fn on_detached_from_visual_tree(this: &Self, args: &VisualTreeAttachmentEventArgs) {
        this.stop();
        Self::parent_on_detached_from_visual_tree(this, args);
    }
}
impl Drop for RendererHost {
    fn drop(&mut self) {
        if let Some(timer) = self.timer.get_mut().take() {
            timer.dispose();
        }
        if let Some(host) = self.host.get_mut().take() {
            host.set_active(false);
        }
    }
}
impl RendererHost {
    pub fn construct() -> Self {
        Self {
            base: Panel::construct(),
            presenter: RefCell::new(None),
            host: RefCell::new(None),
            inputs: RefCell::new(Vec::new()),
            last_inputs: RefCell::new(None),
            timer: RefCell::new(None),
            errors: MediaEvent::default(),
            presenter_errors: RefCell::new(None),
            failed: Cell::new(false),
            generation: Cell::new(0),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn schedule(&self) -> RenderSchedule {
        self.get_value(Self::schedule_property())
    }
    pub fn set_schedule(&self, value: RenderSchedule) {
        self.set_value(Self::schedule_property(), value);
    }
    pub fn errors(&self) -> MediaEvent<MediaError> {
        self.errors.clone()
    }
    /// The renderer's output identity must use source and the viewport's
    /// generation. The framework does not own its scenes or rendering library.
    pub fn set_renderer(
        &self,
        renderer: Option<Rc<dyn TextureRenderer>>,
        source: u64,
        importer: Option<Rc<dyn TextureFrameImporter>>,
    ) {
        self.stop();
        let host = renderer.map(|renderer| TextureRendererHost::new(renderer, source));
        if let Some(host) = host.as_ref() {
            host.set_schedule(self.schedule());
        }
        if let Some(view) = self.presenter.borrow().as_ref() {
            view.set_frame_source(host.as_ref().map(|h| h.output()), importer);
        }
        self.host.replace(host);
        self.failed.set(false);
        self.last_inputs.borrow_mut().take();
        if self.is_attached_to_visual_tree() {
            self.start();
        }
    }
    pub fn output(&self) -> Option<Arc<FrameMailbox>> {
        self.host.borrow().as_ref().map(|h| h.output())
    }
    pub fn set_inputs(&self, inputs: Vec<Arc<FrameMailbox>>) {
        self.inputs.replace(inputs);
        if let Some(host) = self.host.borrow().as_ref() {
            host.clear_output();
        }
        self.last_inputs.borrow_mut().take();
        self.invalidate_renderer();
    }
    pub fn invalidate_renderer(&self) {
        self.failed.set(false);
        if let Some(host) = self.host.borrow().as_ref() {
            host.invalidate();
        }
    }
    /// Map local pointer coordinates after normal control transforms. The
    /// caller subscribes to ordinary routed input and owns hit testing.
    pub fn viewport_point(&self, point: Point) -> Option<Point> {
        let bounds = self.bounds();
        self.host
            .borrow()
            .as_ref()?
            .viewport_point(point.x, point.y, bounds.width, bounds.height)
            .map(|(x, y)| Point::new(x, y))
    }
    fn stop(&self) {
        self.generation.set(
            self.generation
                .get()
                .checked_add(1)
                .expect("renderer control generation exhausted"),
        );
        if let Some(timer) = self.timer.borrow_mut().take() {
            timer.dispose();
        }
        if let Some(host) = self.host.borrow().as_ref() {
            host.set_active(false);
        }
        self.last_inputs.borrow_mut().take();
    }
    fn start(&self) {
        if self.timer.borrow().is_some() || self.host.borrow().is_none() {
            return;
        }
        let weak = self.to_ref().downgrade();
        self.timer.replace(Some(DispatcherTimer::run(
            move || {
                let Some(view) = weak.upgrade() else {
                    return false;
                };
                view.tick();
                true
            },
            Duration::from_millis(16),
            DispatcherPriority::RENDER,
        )));
        self.tick();
    }
    fn tick(&self) {
        if self.failed.get() {
            return;
        }
        let Some(host) = self.host.borrow().clone() else {
            return;
        };
        if !self.is_attached_to_visual_tree() || !self.is_effectively_visible() {
            host.set_active(false);
            if let Some(view) = self.presenter.borrow().as_ref() {
                view.present_latest();
            }
            return;
        }
        let bounds = self.bounds();
        let scale = TopLevel::get_top_level(Some(self))
            .map(|t| t.render_scaling())
            .unwrap_or(1.);
        if let Err(error) = host.resize(bounds.width, bounds.height, scale, 8192) {
            self.errors.emit(&error);
            return;
        }
        // set_active(true) invalidates; only call when transitioning from hidden.
        host.activate_if_needed();
        let inputs: Option<Vec<_>> = self.inputs.borrow().iter().map(|s| s.latest()).collect();
        let Some(inputs) = inputs else {
            if self.last_inputs.borrow_mut().take().is_some() {
                host.clear_output();
            }
            if let Some(view) = self.presenter.borrow().as_ref() {
                view.present_latest();
            }
            return;
        };
        let ids = inputs
            .iter()
            .map(|v| v.metadata().identity)
            .collect::<Vec<_>>();
        if self.last_inputs.borrow().as_ref() != Some(&ids) {
            host.invalidate();
            self.last_inputs.replace(Some(ids));
        }
        if let Some(pending) = host.render(inputs) {
            let weak = self.to_ref().downgrade();
            let generation = self.generation.get();
            Dispatcher::ui_thread().invoke_async_task_local(move || async move {
                let result = pending.await;
                if let Some(view) = weak.upgrade().filter(|v| v.generation.get() == generation) {
                    if let Err(error) = result {
                        if error.kind != MediaErrorKind::Cancelled {
                            view.failed.set(true);
                            if let Some(host) = view.host.borrow().as_ref() {
                                host.set_active(false);
                            }
                            view.errors.emit(&error);
                        }
                    }
                    if let Some(presenter) = view.presenter.borrow().as_ref() {
                        presenter.present_latest();
                    }
                }
            });
        }
        if let Some(view) = self.presenter.borrow().as_ref() {
            view.present_latest();
        }
    }
}
