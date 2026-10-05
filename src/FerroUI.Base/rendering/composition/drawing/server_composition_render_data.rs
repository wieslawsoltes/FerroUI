use super::{RenderDataStream, ReplayScope};
use crate::platform::{IDrawingContextImpl, LtrbRect};
use crate::rendering::composition::server::{
    IServerObject, IServerRenderResource, IServerRenderResourceHost, IServerRenderResourceObserver, ServerCompositor,
    ServerRenderResourceCore,
};
use crate::rendering::composition::transport::BatchStreamReader;
use crate::Rect;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::time::Duration;

/// What a visual drew, on the server: the stream to replay, and the
/// subscriptions to the server-side resources it draws with.
pub struct ServerCompositionRenderData {
    this: Weak<ServerCompositionRenderData>,
    compositor: Weak<ServerCompositor>,
    core: ServerRenderResourceCore,
    stream: RefCell<Option<RenderDataStream>>,
    referenced_resources: RefCell<Vec<Rc<dyn IServerRenderResource>>>,
    bounds: Cell<Option<LtrbRect>>,
    bounds_valid: Cell<bool>,
    replay_scopes: RefCell<Vec<ReplayScope>>,
}

impl ServerCompositionRenderData {
    pub fn new(compositor: &Rc<ServerCompositor>) -> Rc<ServerCompositionRenderData> {
        Rc::new_cyclic(|this| ServerCompositionRenderData {
            this: this.clone(),
            compositor: Rc::downgrade(compositor),
            core: ServerRenderResourceCore::new(),
            stream: RefCell::new(None),
            referenced_resources: RefCell::new(Vec::new()),
            bounds: Cell::new(None),
            bounds_valid: Cell::new(false),
            replay_scopes: RefCell::new(Vec::new()),
        })
    }

    pub fn is_disposed(&self) -> bool {
        self.core.is_disposed()
    }

    fn as_observer(&self) -> Option<Rc<dyn IServerRenderResourceObserver>> {
        self.this.upgrade().map(|this| this as Rc<dyn IServerRenderResourceObserver>)
    }

    /// The bounds of the drawn content, rounded outwards to whole units;
    /// `None` if nothing is drawn.
    pub fn bounds(&self) -> Option<LtrbRect> {
        if !self.bounds_valid.get() {
            self.bounds.set(self.calculate_render_bounds());
            self.bounds_valid.set(true);
        }
        self.bounds.get()
    }

    fn calculate_render_bounds(&self) -> Option<LtrbRect> {
        let bounds = self.stream.borrow().as_ref().and_then(RenderDataStream::calculate_bounds);
        Self::apply_render_bounds_rounding(bounds.map(LtrbRect::from_rect))
    }

    /// Rounds render bounds outwards to whole units.
    pub fn apply_render_bounds_rounding_rect(rect: Option<Rect>) -> Option<Rect> {
        Self::apply_render_bounds_rounding(rect.map(LtrbRect::from_rect)).map(|r| r.to_rect())
    }

    /// Rounds render bounds outwards to whole units.
    pub fn apply_render_bounds_rounding(rect: Option<LtrbRect>) -> Option<LtrbRect> {
        rect.map(|r| LtrbRect { left: r.left.floor(), top: r.top.floor(), right: r.right.ceil(), bottom: r.bottom.ceil() })
    }

    /// Replays the drawn content onto a platform drawing context.
    pub fn render(&self, context: &mut dyn IDrawingContextImpl) {
        if let Some(stream) = self.stream.borrow().as_ref() {
            stream.replay_with_scopes(context, &mut self.replay_scopes.borrow_mut());
        }
    }

    fn reset(&self) {
        self.bounds.set(None);
        self.bounds_valid.set(false);
        let referenced = std::mem::take(&mut *self.referenced_resources.borrow_mut());
        if let Some(observer) = self.as_observer() {
            for resource in &referenced {
                resource.remove_observer(&observer);
            }
        }
        if let Some(mut stream) = self.stream.borrow_mut().take() {
            stream.dispose_resources();
            stream.dispose();
        }
    }
}

impl IServerObject for ServerCompositionRenderData {
    fn deserialize_changes_core(&self, reader: &mut BatchStreamReader<'_>, _committed_at: Duration) {
        self.reset();
        let mut stream = RenderDataStream::new();
        let compositor = self.compositor.upgrade();
        let server_ids = RefCell::new(Vec::new());
        stream.deserialize_from(reader, &|id| {
            server_ids.borrow_mut().push(id);
            compositor.as_ref().and_then(|c| c.get_object(id))
        });
        let server_ids = server_ids.into_inner();

        // Server-side brushes, pens and geometries are render resources:
        // follow their changes.
        if let (Some(compositor), Some(observer)) = (&compositor, self.as_observer()) {
            let mut referenced = self.referenced_resources.borrow_mut();
            for id in &server_ids {
                if let Some(resource) = compositor.get_object(*id).and_then(|o| o.as_render_resource()) {
                    resource.add_observer(&observer);
                    referenced.push(resource);
                }
            }
        }
        *self.stream.borrow_mut() = Some(stream);
    }

    fn values_invalidated(&self) {
        self.core.invalidated(self);
    }

    fn dispose(&self) {
        self.reset();
        self.core.dispose();
    }

    fn as_render_resource(self: Rc<Self>) -> Option<Rc<dyn IServerRenderResource>> {
        Some(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}

impl IServerRenderResourceObserver for ServerCompositionRenderData {
    fn dependency_queued_invalidate(&self, _sender: &dyn IServerRenderResource) {
        self.bounds_valid.set(false);
        self.core.dependency_queued_invalidate(self);
    }
}

impl IServerRenderResource for ServerCompositionRenderData {
    fn add_observer(&self, observer: &Rc<dyn IServerRenderResourceObserver>) {
        self.core.add_observer(observer);
    }

    fn remove_observer(&self, observer: &Rc<dyn IServerRenderResourceObserver>) {
        self.core.remove_observer(observer);
    }

    fn queued_invalidate(&self) {
        self.core.queued_invalidate(self);
    }
}

impl IServerRenderResourceHost for ServerCompositionRenderData {
    fn compositor(&self) -> Option<Rc<ServerCompositor>> {
        self.compositor.upgrade()
    }

    fn as_render_resource_rc(&self) -> Option<Rc<dyn IServerRenderResource>> {
        self.this.upgrade().map(|this| this as Rc<dyn IServerRenderResource>)
    }
}
