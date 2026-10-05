use crate::platform::{IInputPane, InputPaneStateEventArgs};
use crate::{ControlImpl, Decorator, InputPaneAwareBehavior, TopLevel};
use ferroui_base::animation::easings::{Easing, LinearEasing};
use ferroui_base::animation::{DoubleTransition, ITransition, TimeSpan, Transitions};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{Layoutable, LayoutableImpl, LayoutableImplExt};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, DirectProperty, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Matrix, Point, Rect, Ref, Size,
    StyledElementImpl, StyledProperty, Thickness, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A control that automatically adjusts the position or the height of its
/// child based on the height of the input pane to ensure the content is
/// visible.
#[repr(C)]
pub struct InputPaneAwareDecorator {
    base: Decorator,
    behavior: Cell<InputPaneAwareBehavior>,
    input_pane: RefCell<Option<Rc<dyn IInputPane>>>,
    /// The subscription to the state changed event of the input pane.
    input_pane_state_changed: RefCell<Option<Rc<dyn IDisposable>>>,
    first_layout_done: Cell<bool>,
}

ferro_class!(InputPaneAwareDecorator: Decorator);
ferro_class_info!(InputPaneAwareDecorator { new: InputPaneAwareDecorator::new });
ferro_impl_classes!(InputPaneAwareDecorator: StyledElementImpl, InteractiveImpl, InputElementImpl, ControlImpl);

impl FerroObjectImpl for InputPaneAwareDecorator {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::behavior_property().as_property() {
            this.ensure_input_pane_padding_applied();
        }
    }
}

impl LayoutableImpl for InputPaneAwareDecorator {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        if this.input_pane.borrow().is_some()
            && this.first_layout_done.get()
            && this.visual_root().is_some()
            && this.behavior() == InputPaneAwareBehavior::Resize
        {
            this.set_current_value(
                Decorator::padding_property(),
                Thickness::new(0.0, 0.0, 0.0, this.current_input_pane_padding()),
            );
        }
        Self::parent_measure_override(this, available_size)
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        this.first_layout_done.set(true);

        if this.behavior() == InputPaneAwareBehavior::Pan {
            if let Some(child) = this.child() {
                child.arrange(Rect::from_position_size(
                    Point::new(0.0, -this.current_input_pane_padding()),
                    final_size,
                ));
            }

            final_size
        } else {
            Self::parent_arrange_override(this, final_size)
        }
    }
}

impl VisualImpl for InputPaneAwareDecorator {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);

        let input_pane = TopLevel::get_top_level(Some(this)).and_then(|top_level| top_level.input_pane());
        if let Some(input_pane) = &input_pane {
            // The handler refers to the decorator weakly: the input pane
            // belongs to the platform and must not keep the decorator alive.
            let weak = this.to_ref().downgrade();
            let subscription = input_pane.state_changed(Rc::new(move |e| {
                if let Some(this) = weak.upgrade() {
                    this.input_pane_aware_view_state_changed(e);
                }
            }));
            *this.input_pane_state_changed.borrow_mut() = Some(subscription);
        }
        *this.input_pane.borrow_mut() = input_pane;
        this.ensure_input_pane_padding_applied();
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);
        let subscription = this.input_pane_state_changed.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
        *this.input_pane.borrow_mut() = None;
        this.set_current_value(Decorator::padding_property(), Thickness::default());
        this.set_current_input_pane_padding(0.0);
    }
}

ferro_properties! {
    impl InputPaneAwareDecorator {
        /// Defines the `Behavior` property.
        pub fn behavior_property() -> DirectProperty<InputPaneAwareDecorator, InputPaneAwareBehavior> {
            FerroProperty::register_direct::<InputPaneAwareDecorator, _>(
                "Behavior",
                |o| o.behavior(),
                Some(|o, x| o.set_behavior(x)),
                InputPaneAwareBehavior::None,
            )
        }

        fn current_input_pane_padding_property() -> StyledProperty<f64> {
            FerroProperty::register::<InputPaneAwareDecorator, _>("CurrentInputPanePadding", 0.0)
        }
    }
}

impl InputPaneAwareDecorator {
    fn static_constructor() {
        Layoutable::affects_measure::<InputPaneAwareDecorator>(&[
            Self::behavior_property().as_property(),
            Self::current_input_pane_padding_property().as_property(),
        ]);
        Layoutable::affects_arrange::<InputPaneAwareDecorator>(&[
            Self::current_input_pane_padding_property().as_property()
        ]);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: Decorator::construct(),
            behavior: Cell::new(InputPaneAwareBehavior::None),
            input_pane: RefCell::new(None),
            input_pane_state_changed: RefCell::new(None),
            first_layout_done: Cell::new(false),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// How the view reacts to the input pane state.
    pub fn behavior(&self) -> InputPaneAwareBehavior {
        self.behavior.get()
    }

    pub fn set_behavior(&self, value: InputPaneAwareBehavior) {
        self.set_and_raise_cell(Self::behavior_property(), &self.behavior, value);
    }

    fn current_input_pane_padding(&self) -> f64 {
        self.get_value(Self::current_input_pane_padding_property())
    }

    fn set_current_input_pane_padding(&self, value: f64) {
        self.set_value(Self::current_input_pane_padding_property(), value)
    }

    fn ensure_input_pane_padding_applied(&self) {
        self.set_current_value(Decorator::padding_property(), Thickness::default());

        let input_pane = self.input_pane.borrow().clone();
        match (input_pane, self.visual_root()) {
            (Some(input_pane), Some(root))
                if self.first_layout_done.get() && self.behavior() != InputPaneAwareBehavior::None =>
            {
                let occluded_rect = input_pane.occluded_rect();

                let transform_matrix = self.transform_to_visual(&root).unwrap_or(Matrix::IDENTITY);

                let translated_rect =
                    Rect::from_size(self.bounds().size()).transform_to_aabb(transform_matrix);

                let intersect = occluded_rect.intersect(translated_rect);
                self.set_current_input_pane_padding(intersect.height);
            }
            _ => self.set_current_input_pane_padding(0.0),
        }
    }

    fn input_pane_aware_view_state_changed(&self, e: &InputPaneStateEventArgs) {
        let transition = DoubleTransition::new();
        transition.set_property(Some(Self::current_input_pane_padding_property().as_property()));
        transition.set_duration(TimeSpan::from(e.animation_duration()));
        // Only an easing class is used; anything else is replaced by the
        // linear easing.
        transition.set_easing(match e.easing() {
            Some(easing) if easing.derives_from_easing() => Easing::from_rc(easing.clone()),
            _ => Easing::new(LinearEasing::new()),
        });

        let transition: Rc<dyn ITransition> = transition.into();
        self.set_transitions(Some(Transitions::from_items([transition])));

        self.ensure_input_pane_padding_applied();

        self.invalidate_measure();
    }
}
