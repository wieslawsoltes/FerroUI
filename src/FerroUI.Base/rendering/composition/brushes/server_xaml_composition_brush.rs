//! The server-side counterpart of a XAML composition brush
//! ([`XamlCompositionBrushBase`](crate::media::XamlCompositionBrushBase),
//! winui-materials.md): it paints the composition brush the XAML brush set,
//! or its fallback colour.

use super::paint_composition_brush;
use crate::media::immutable::ImmutableSolidColorBrush;
use crate::media::{BoxShadows, Color, XamlBrushChanges};
use crate::platform::IDrawingContextImpl;
use crate::rendering::composition::generated::{ServerCompositionSimpleBrushHooks, ServerCompositionSimpleBrushProps};
use crate::rendering::composition::server::{
    impl_simple_server_render_resource, with_brush_light_ids, IServerObject, IServerRenderResource,
    IServerRenderResourceObserver, ServerCompositor, SimpleServerRenderResource,
};
use crate::rendering::composition::transport::BatchStreamReader;
use crate::{Rect, RoundedRect};
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::sync::Arc;
use std::time::Duration;

/// The server-side counterpart of a XAML composition brush.
pub struct ServerXamlCompositionBrush {
    base: SimpleServerRenderResource,
    this: Weak<ServerXamlCompositionBrush>,
    props: ServerCompositionSimpleBrushProps,
    fallback_color: Cell<Color>,
    composition_brush: RefCell<Option<Rc<dyn IServerObject>>>,
    light_target_ids: RefCell<Arc<[String]>>,
    needs_transparent_window: Cell<bool>,
    /// Set while the brush paints: a composition brush that paints this one
    /// is not painted inside itself.
    painting: Cell<bool>,
}

impl ServerXamlCompositionBrush {
    pub fn new(compositor: &Rc<ServerCompositor>) -> Rc<ServerXamlCompositionBrush> {
        Rc::new_cyclic(|this: &Weak<ServerXamlCompositionBrush>| ServerXamlCompositionBrush {
            base: SimpleServerRenderResource::new(compositor, this.clone() as Weak<dyn IServerRenderResource>),
            this: this.clone(),
            props: ServerCompositionSimpleBrushProps::new(),
            fallback_color: Cell::new(Color::default()),
            composition_brush: RefCell::new(None),
            light_target_ids: RefCell::new(Arc::from(Vec::<String>::new())),
            needs_transparent_window: Cell::new(false),
            painting: Cell::new(false),
        })
    }

    pub fn is_disposed(&self) -> bool {
        self.base.is_disposed()
    }

    /// The brush properties (opacity, transforms).
    pub fn brush_props(&self) -> &ServerCompositionSimpleBrushProps {
        &self.props
    }

    /// The fallback colour of the XAML brush.
    pub fn fallback_color(&self) -> Color {
        self.fallback_color.get()
    }

    /// The composition brush the XAML brush paints with.
    pub fn composition_brush(&self) -> Option<Rc<dyn IServerObject>> {
        self.composition_brush.borrow().clone()
    }

    /// The ids of the XAML lights that target the brush.
    pub fn light_target_ids(&self) -> Arc<[String]> {
        self.light_target_ids.borrow().clone()
    }

    fn observer(&self) -> Option<Rc<dyn IServerRenderResourceObserver>> {
        self.this.upgrade().map(|this| this as Rc<dyn IServerRenderResourceObserver>)
    }

    /// Paints over `rect`: the composition brush, with the light ids of the
    /// brush offered to the scene lighting it draws, or the fallback colour.
    pub fn paint(&self, canvas: &mut dyn IDrawingContextImpl, rect: Rect) {
        let opacity = self.props.opacity();
        if opacity <= 0.0 || rect.width <= 0.0 || rect.height <= 0.0 || self.painting.replace(true) {
            return;
        }
        if opacity < 1.0 {
            canvas.push_opacity(opacity, Some(rect));
        }
        // A brush that shows what is behind the window, in a window the
        // platform shows nothing behind: the fallback colour.
        let window_is_opaque = self.needs_transparent_window.get()
            && crate::rendering::composition::server::painting_visual()
                .and_then(|visual| visual.root())
                .is_none_or(|target| {
                    target.transparency_level() == crate::rendering::composition::CompositionTransparencyLevel::None
                });
        match self.composition_brush().filter(|_| !window_is_opaque) {
            Some(brush) => {
                let ids = self.light_target_ids();
                with_brush_light_ids(ids, || paint_composition_brush(canvas, &brush, rect));
            }
            None => {
                let color = self.fallback_color();
                if color.a > 0 {
                    let brush = ImmutableSolidColorBrush::new(color);
                    canvas.draw_rectangle(Some(&brush), None, RoundedRect::from_rect(rect), &BoxShadows::default());
                }
            }
        }
        if opacity < 1.0 {
            canvas.pop_opacity();
        }
        self.painting.set(false);
    }

    fn set_composition_brush(&self, brush: Option<Rc<dyn IServerObject>>) {
        let old = std::mem::replace(&mut *self.composition_brush.borrow_mut(), brush);
        let Some(observer) = self.observer() else { return };
        if let Some(old) = old.and_then(|old| old.as_render_resource()) {
            old.remove_observer(&observer);
        }
        if let Some(new) = self.composition_brush().and_then(|new| new.as_render_resource()) {
            new.add_observer(&observer);
        }
    }
}

impl_simple_server_render_resource!(ServerXamlCompositionBrush, base);

impl ServerCompositionSimpleBrushHooks for ServerXamlCompositionBrush {}

impl IServerObject for ServerXamlCompositionBrush {
    fn deserialize_changes_core(&self, reader: &mut BatchStreamReader<'_>, committed_at: Duration) {
        self.props.deserialize_changes_core(self, reader, committed_at);
        let Some(changes) = reader.read_value::<XamlBrushChanges>() else { return };
        self.fallback_color.set(changes.fallback_color);
        self.needs_transparent_window.set(changes.needs_transparent_window);
        *self.light_target_ids.borrow_mut() = Arc::from(changes.light_target_ids);
        let compositor = self.base.compositor();
        let brush = changes.composition_brush.and_then(|id| compositor.as_ref().and_then(|compositor| compositor.get_object(id)));
        self.set_composition_brush(brush);
    }

    fn values_invalidated(&self) {
        self.base.core().invalidated(self);
    }

    fn dispose(&self) {
        self.set_composition_brush(None);
        self.base.core().dispose();
    }

    fn as_render_resource(self: Rc<Self>) -> Option<Rc<dyn IServerRenderResource>> {
        Some(self)
    }

    fn get_props(&self, type_id: TypeId) -> Option<&dyn Any> {
        self.props.find_props(type_id)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}
