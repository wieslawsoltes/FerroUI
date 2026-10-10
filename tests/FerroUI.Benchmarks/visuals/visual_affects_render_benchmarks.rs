//! Setting and clearing a property of a visual that affects its rendering.

use crate::harness::Registry;
use ferroui_base::media::{Brushes, IBrush, IPen, Pen};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_properties, instantiate, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, StyledProperty, Visual, VisualImpl,
};
use ferroui_controls::Border;
use std::rc::Rc;

pub struct VisualAffectsRenderBenchmarks {
    target: Ref<TestVisual>,
    pen: Rc<dyn IPen>,
}

impl VisualAffectsRenderBenchmarks {
    pub fn new() -> Self {
        let target = TestVisual::new();
        let pen: Rc<dyn IPen> = Pen::with_brush(Some(Brushes::black() as Rc<dyn IBrush>), 1.0).into();

        Self { target, pen }
    }

    pub fn set_property_that_affects_render(&self) {
        self.target.set_pen(Some(self.pen.clone()));
        self.target.set_pen(None);
    }
}

impl Default for VisualAffectsRenderBenchmarks {
    fn default() -> Self {
        Self::new()
    }
}

#[repr(C)]
struct TestVisual {
    base: Visual,
}

ferro_class!(TestVisual: Visual);
ferro_impl_classes!(TestVisual: FerroObjectImpl, StyledElementImpl, VisualImpl);

ferro_properties! {
    impl TestVisual {
        /// Defines the `Pen` property. Its owner is the border class, as
        /// upstream.
        pub fn pen_property() -> StyledProperty<Option<Rc<dyn IPen>>> {
            FerroProperty::register::<Border, _>("Pen", None)
        }
    }
}

impl TestVisual {
    fn static_constructor() {
        Visual::affects_render::<TestVisual>(&[Self::pen_property().as_property()]);
    }

    fn construct() -> Self {
        Self { base: Visual::construct() }
    }

    fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    #[allow(dead_code)]
    fn pen(&self) -> Option<Rc<dyn IPen>> {
        self.get_value(Self::pen_property())
    }

    fn set_pen(&self, value: Option<Rc<dyn IPen>>) {
        self.set_value(Self::pen_property(), value)
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("visuals", "VisualAffectsRenderBenchmarks");
    class.benchmark("set_property_that_affects_render", "", VisualAffectsRenderBenchmarks::new, |b| {
        b.set_property_that_affects_render()
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn visual_affects_render_benchmarks() {
        crate::harness::smoke_class(super::register, "VisualAffectsRenderBenchmarks");
    }
}
