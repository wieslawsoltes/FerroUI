//! The rounded rectangle used by composition selection-indicator animations.

use crate::shapes::{Rectangle, Shape, ShapeImpl};
use crate::ControlImpl;
use ferroui_base::input::{InputElement, InputElementImpl};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{Geometry, RectangleGeometry};
use ferroui_base::platform::LtrbRect;
use ferroui_base::rendering::composition::server::{
    IServerVisualContent, ServerCompositionDrawListVisual, ServerCompositionVisual, ServerVisualRenderContext,
};
use ferroui_base::rendering::composition::transport::{
    BatchStreamReader, BatchStreamWriter, IRegisterForSerialization,
};
use ferroui_base::rendering::composition::{
    CompositionDrawListVisual, Compositor, ICompositionDrawListVisualExtension,
};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, FerroObjectImplExt,
    FerroPropertyChangedEventArgs, Rect, Ref, StyledElementImpl, TypeInfo, Vector, Visual, VisualImpl,
};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

/// A rectangle whose rounded caps keep their size during composition scale animations.
/// Layout, fill, stroke and styling remain those of a normal Rectangle.
#[repr(C)]
pub struct SelectionIndicator {
    base: Rectangle,
    state: RefCell<Option<Rc<IndicatorState>>>,
}

ferro_class!(SelectionIndicator: Rectangle);
ferro_class_info!(SelectionIndicator { new: SelectionIndicator::new, markup: { namespace: "FerroUI.Controls" } });
ferro_impl_classes!(SelectionIndicator: LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl);

impl SelectionIndicator {
    fn static_constructor() {
        // A travelling indicator may cover another row, but must never redirect its clicks.
        InputElement::is_hit_test_visible_property().override_default_value::<SelectionIndicator>(false);
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: Rectangle::construct(),
            state: RefCell::new(None),
        })
    }

    fn radius(&self) -> Vector {
        let rect = self.drawing_rect();
        Vector::new(
            self.radius_x().max(0.0).min(rect.width.max(0.0) / 2.0),
            self.radius_y().max(0.0).min(rect.height.max(0.0) / 2.0),
        )
    }

    fn drawing_rect(&self) -> Rect {
        Rect::from_size(self.bounds().size()).deflate(self.stroke_thickness() / 2.0)
    }
}

impl ShapeImpl for SelectionIndicator {
    fn create_defining_geometry(this: &Self) -> Option<Ref<Geometry>> {
        // Fit the base corners before compensating animation scale, so the cached
        // path and the animated primitive have identical cap geometry at handoff.
        let radius = this.radius();
        Some(RectangleGeometry::with_rect_and_radii(this.drawing_rect(), radius.x, radius.y).upcast())
    }
}

impl StyledElementImpl for SelectionIndicator {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        Rectangle::TYPE
    }
}

impl FerroObjectImpl for SelectionIndicator {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if !change.is_effective_value_change() {
            return;
        }
        if change.property() == Rectangle::radius_x_property().as_property()
            || change.property() == Rectangle::radius_y_property().as_property()
            || change.property() == Visual::bounds_property().as_property()
            || change.property() == Shape::stroke_thickness_property().as_property()
        {
            // The state is taken out of the cell first: asking for the composition visual of an
            // indicator that has none yet creates it, which replaces the state.
            let state = this.state.borrow().clone();
            if let Some(state) = state {
                state.radius.set(this.radius());
                // An indicator without a composition visual has nothing to send: the visual
                // reads the radius when it is created.
                if this.has_composition_visual() {
                    if let Some(visual) = this.composition_visual() {
                        visual.register_for_serialization();
                    }
                }
            }
        }
    }
}

impl VisualImpl for SelectionIndicator {
    // The indicator is drawn by the content of its composition visual
    // (docs/porting/desktop-performance.md, section 9.42).
    fn requires_composition_visual(_this: &Self) -> bool {
        true
    }

    fn create_composition_visual(this: &Self, compositor: &Rc<Compositor>) -> CompositionDrawListVisual {
        let state = Rc::new(IndicatorState {
            radius: Cell::new(this.radius()),
        });
        let visual = CompositionDrawListVisual::with_extension(
            compositor,
            this,
            || Box::new(ServerIndicator::default()),
            state.clone(),
        );
        *this.state.borrow_mut() = Some(state);
        visual
    }
}

struct IndicatorState {
    radius: Cell<Vector>,
}

impl ICompositionDrawListVisualExtension for IndicatorState {
    fn serialize_changes_core(&self, writer: &mut BatchStreamWriter<'_>) {
        writer.write(self.radius.get());
    }
}

#[derive(Default)]
struct ServerIndicator {
    draw_list: ServerCompositionDrawListVisual,
    radius: Cell<Vector>,
}

impl IServerVisualContent for ServerIndicator {
    fn deserialize_changes_core(
        &self,
        visual: &ServerCompositionVisual,
        reader: &mut BatchStreamReader<'_>,
        committed_at: Duration,
    ) {
        self.draw_list.deserialize_changes_core(visual, reader, committed_at);
        self.radius.set(reader.read::<Vector>());
        self.draw_list.dependency_queued_invalidate(visual);
    }

    fn compute_own_content_bounds(&self, visual: &ServerCompositionVisual) -> Option<LtrbRect> {
        self.draw_list.compute_own_content_bounds(visual)
    }

    fn render_core(
        &self,
        visual: &ServerCompositionVisual,
        context: &mut ServerVisualRenderContext<'_>,
        clip: LtrbRect,
    ) {
        let scale = visual.scale();
        if scale.x == 1.0 && scale.y == 1.0 {
            self.draw_list.render_core(visual, context, clip);
        } else if scale.x.abs() > 0.000001 && scale.y.abs() > 0.000001 {
            if let Some(commands) = self.draw_list.render_commands() {
                let radius = self.radius.get();
                commands.render_as_rounded_rectangle(
                    context.canvas(),
                    Vector::new(radius.x / scale.x.abs(), radius.y / scale.y.abs()),
                );
            }
        }
    }

    fn dependency_queued_invalidate(&self, visual: &ServerCompositionVisual) {
        self.draw_list.dependency_queued_invalidate(visual);
    }
    fn dispose(&self, visual: &ServerCompositionVisual) {
        self.draw_list.dispose(visual);
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
