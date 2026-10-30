//! Port of `Debugging/VisualDebugBehavior.cs`.

use super::visual_debug_adorner::VisualDebugAdorner;
use ferroui_base::animation::TimeSpan;
use ferroui_base::media::{Color, Colors, SolidColorBrush};
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, StyledProperty, Visual,
};
use ferroui_controls::primitives::AdornerLayer;
use ferroui_controls::Control;
use ferroui_xaml_interactivity::{
    EventTriggerBase, EventTriggerBaseImpl, EventTriggerBaseImplExt, StyledElementBehaviorImpl,
};

/// A behavior that visualizes events on the attached control for debugging purposes: each time
/// the event fires, the actions run and the control is highlighted for
/// [`duration`](Self::duration).
#[repr(C)]
pub struct VisualDebugBehavior {
    base: EventTriggerBase,
}

ferro_class!(VisualDebugBehavior: EventTriggerBase);
ferro_class_info!(VisualDebugBehavior { new: VisualDebugBehavior::new });
ferroui_base::ferro_impl_classes!(
    VisualDebugBehavior: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
);

ferro_properties! {
    impl VisualDebugBehavior {
        /// Identifies the `HighlightColor` property.
        pub fn highlight_color_property() -> StyledProperty<Color> {
            FerroProperty::register::<VisualDebugBehavior, _>("HighlightColor", Colors::RED)
        }

        /// Identifies the `Duration` property.
        pub fn duration_property() -> StyledProperty<TimeSpan> {
            FerroProperty::register::<VisualDebugBehavior, _>("Duration", TimeSpan::from_seconds(0.5))
        }
    }
}

ferroui_base::ferro_overrides! {
    impl EventTriggerBaseImpl for VisualDebugBehavior {
        fn on_event(this: &Self, event_args: Option<&BoxedValue>) {
            // Execute any associated actions.
            Self::parent_on_event(this, event_args);

            if let Some(control) = this.associated_object().and_then(|associated_object| associated_object.cast::<Control>()) {
                this.show_highlight(&control);
            }
        }
    }
}

impl VisualDebugBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: EventTriggerBase::construct() }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the color used to highlight the control when the event fires.
    pub fn highlight_color(&self) -> Color {
        self.get_value(Self::highlight_color_property())
    }

    pub fn set_highlight_color(&self, value: Color) {
        self.set_value(Self::highlight_color_property(), value)
    }

    /// Gets or sets the duration of the highlight.
    pub fn duration(&self) -> TimeSpan {
        self.get_value(Self::duration_property())
    }

    pub fn set_duration(&self, value: TimeSpan) {
        self.set_value(Self::duration_property(), value)
    }

    fn show_highlight(&self, control: &Ref<Control>) {
        let visual: &Visual = control;
        let Some(adorner_layer) = AdornerLayer::get_adorner_layer(visual) else {
            return;
        };

        let brush = SolidColorBrush::with_color(self.highlight_color());
        let adorner = VisualDebugAdorner::new(control.clone(), brush);

        let adorner_visual: &Visual = &adorner;
        AdornerLayer::set_adorned_element(adorner_visual, control);
        adorner_layer.children().add(adorner.clone());

        let _ = DispatcherTimer::run_once(
            move || {
                adorner_layer.children().remove(adorner.clone());
            },
            self.duration().to_duration().unwrap_or_default(),
            DispatcherPriority::BACKGROUND,
        );
    }
}
