use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{Layoutable, LayoutableImpl, LayoutableImplExt};
use ferroui_base::{ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, Size, StyledElementImpl, VisualImpl};
use ferroui_controls::{ControlImpl, Decorator};

/// A decorator that asks for no space and measures its child with the
/// space it is arranged in.
#[repr(C)]
pub struct ChildFitter {
    base: Decorator,
}

ferro_class!(ChildFitter: Decorator);
ferro_impl_classes!(ChildFitter: StyledElementImpl, VisualImpl, InteractiveImpl, InputElementImpl, ControlImpl);
ferro_class_info!(ChildFitter { new: ChildFitter::new });

impl FerroObjectImpl for ChildFitter {}

impl LayoutableImpl for ChildFitter {
    fn measure_override(_this: &Self, _available_size: Size) -> Size {
        Size::new(0.0, 0.0)
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        if let Some(child) = this.child() {
            Layoutable::measure(&child, final_size);
        }
        Self::parent_arrange_override(this, final_size);
        final_size
    }
}

impl ChildFitter {
    pub fn construct() -> Self {
        Self { base: Decorator::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the upstream project has no tests.
    use super::*;
    use ferroui_controls::Border;

    #[test]
    fn measures_to_nothing_and_measures_the_child_with_the_arranged_size() {
        let fitter = ChildFitter::new();
        let child = Border::new();
        child.set_width(150.0);
        fitter.set_child(child.clone());

        fitter.measure(Size::new(500.0, 500.0));
        assert_eq!(Size::new(0.0, 0.0), fitter.desired_size());

        fitter.arrange(ferroui_base::Rect::new(0.0, 0.0, 200.0, 100.0));
        assert_eq!(Size::new(150.0, 0.0), child.desired_size());
        assert_eq!(Size::new(200.0, 100.0), fitter.bounds().size());
    }
}
