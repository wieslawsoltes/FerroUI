//! Port of `Layout/FluidMoveBehavior.cs`.

use ferroui_base::animation::TimeSpan;
use ferroui_base::layout::Layoutable;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_markup_enum, ferro_properties, instantiate, FerroObjectImpl, FerroProperty,
    PixelPoint, Point, Ref, StaticType, StyledProperty, Visual,
};
use ferroui_controls::{Control, Panel};
use ferroui_xaml_behaviors_animations::FluidMoveAnimation;
use ferroui_xaml_interactivity::{BehaviorImpl, BehaviorImplExt, BehaviorOfT};
use std::cell::RefCell;
use std::rc::Rc;

/// Determines if the behavior applies to the associated element or its children.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FluidMoveScope {
    /// Apply behavior to the element itself.
    #[default]
    Self_,
    /// Apply behavior to the children of the element.
    Children,
}

// Deviation (DEVIATIONS.md, Behaviors): `Self` is a keyword in Rust; the member is named `Self_`
// in Rust and `Self` in markup.
ferro_markup_enum!(FluidMoveScope { Self = Self_, Children });

impl std::fmt::Display for FluidMoveScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Self_ => "Self",
            Self::Children => "Children",
        })
    }
}

/// Behavior that animates position changes of a control or its children.
#[repr(C)]
pub struct FluidMoveBehavior {
    base: BehaviorOfT,
    positions: RefCell<Vec<(Ref<Control>, PixelPoint)>>,
    // The subscription of the layout updated event (`LayoutUpdated -= OnLayoutUpdated` of the
    // reference).
    layout_updated: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class!(FluidMoveBehavior: BehaviorOfT);
ferro_class_info!(FluidMoveBehavior { new: FluidMoveBehavior::new });
ferroui_base::ferro_impl_classes!(FluidMoveBehavior: FerroObjectImpl);

ferro_properties! {
    impl FluidMoveBehavior {
        /// Identifies the `AppliesTo` property.
        pub fn applies_to_property() -> StyledProperty<FluidMoveScope> {
            FerroProperty::register::<FluidMoveBehavior, _>("AppliesTo", FluidMoveScope::Self_)
        }

        /// Identifies the `Duration` property.
        pub fn duration_property() -> StyledProperty<TimeSpan> {
            FerroProperty::register::<FluidMoveBehavior, _>("Duration", TimeSpan::from_milliseconds(300.0))
        }
    }
}

ferroui_base::ferro_overrides! {
    impl BehaviorImpl for FluidMoveBehavior {
        fn on_attached(this: &Self) {
            Self::parent_on_attached(this);
            if let Some(layoutable) = this.associated_object_as::<Layoutable>() {
                let behavior = this.to_ref().downgrade();
                let subscription = layoutable.layout_updated(move || {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.on_layout_updated();
                    }
                });
                *this.layout_updated.borrow_mut() = Some(subscription);
            }
        }

        fn on_detaching(this: &Self) {
            let subscription = this.layout_updated.borrow_mut().take();
            if let Some(subscription) = subscription {
                subscription.dispose();
            }
            Self::parent_on_detaching(this);
        }
    }
}

impl FluidMoveBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: BehaviorOfT::construct(<Visual as StaticType>::TYPE),
            positions: RefCell::new(Vec::new()),
            layout_updated: RefCell::new(None),
        }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets how the behavior is applied.
    pub fn applies_to(&self) -> FluidMoveScope {
        self.get_value(Self::applies_to_property())
    }

    pub fn set_applies_to(&self, value: FluidMoveScope) {
        self.set_value(Self::applies_to_property(), value)
    }

    /// Gets or sets animation duration.
    pub fn duration(&self) -> TimeSpan {
        self.get_value(Self::duration_property())
    }

    pub fn set_duration(&self, value: TimeSpan) {
        self.set_value(Self::duration_property(), value)
    }

    fn on_layout_updated(&self) {
        let Some(root) = self.associated_object_as::<Visual>() else {
            return;
        };

        let applies_to = self.applies_to();
        if applies_to == FluidMoveScope::Self_ {
            if let Some(control) = root.clone().cast::<Control>() {
                self.update_control(&control, &root);
            }
        } else if applies_to == FluidMoveScope::Children {
            if let Some(panel) = root.clone().cast::<Panel>() {
                for child in panel.children().to_vec() {
                    self.update_control(&child, &root);
                }
            }
        }
    }

    fn update_control(&self, control: &Ref<Control>, root: &Visual) {
        let Some(p) = control.translate_point(Point::new(0.0, 0.0), root) else {
            return;
        };

        let current = PixelPoint::new(p.x as i32, p.y as i32);

        let previous = {
            let mut positions = self.positions.borrow_mut();
            match positions.iter_mut().find(|(known, _)| known.ptr_eq(control)) {
                Some((_, previous)) => {
                    let value = *previous;
                    *previous = current;
                    value
                }
                None => {
                    positions.push((control.clone(), current));
                    return;
                }
            }
        };

        if previous == current {
            return;
        }

        let dx = (previous.x - current.x) as f64;
        let dy = (previous.y - current.y) as f64;

        FluidMoveAnimation::try_run(Some(&**control), dx, dy, self.duration());
    }
}
