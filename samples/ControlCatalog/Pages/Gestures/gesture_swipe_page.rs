//! Port of `Pages/Gestures/GestureSwipePage.xaml.cs`: the class of the
//! document `Pages/Gestures/GestureSwipePage.xaml`.

use crate::markup::xaml_class;
use ferroui_base::input::gesture_recognizers::SwipeGestureRecognizer;
use ferroui_base::input::{InputElement, InputElementImpl, SwipeGestureEndedEventArgs, SwipeGestureEventArgs};
use ferroui_base::interactivity::{IRoutedEventArgs, InteractiveImpl, RoutedEventArgs};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::rendering::composition::{CompositionVisual, ElementComposition};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Ref,
    StyledElementImpl, Vector3D, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::primitives::{RangeBaseValueChangedEventArgs, TemplatedControlImpl};
use ferroui_controls::{Border, CheckBox, ContentControlImpl, ControlImpl, Panel, TextBlock, UserControl};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

#[repr(C)]
pub struct GestureSwipePage {
    base: UserControl,
    is_init: Cell<bool>,
    indicator_visual: RefCell<Option<Rc<CompositionVisual>>>,
    swipe_delta: Cell<Vector3D>,
}

ferro_class!(GestureSwipePage: UserControl);
ferro_impl_classes!(
    GestureSwipePage: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(GestureSwipePage {
    new: GestureSwipePage::new,
    markup: {
        methods: [
            fn OnDirectionChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<GestureSwipePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_direction_changed(&sender, e.as_routed_event_args())
                },
            fn OnMouseEnabledChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<GestureSwipePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_mouse_enabled_changed(&sender, e.as_routed_event_args())
                },
            fn OnThresholdChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<GestureSwipePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<RangeBaseValueChangedEventArgs>() {
                        this.on_threshold_changed(&sender, e)
                    }
                },
        ],
    },
});
xaml_class!(GestureSwipePage, "/Pages/Gestures/GestureSwipePage.xaml");

impl VisualImpl for GestureSwipePage {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);

        if this.is_init.get() {
            return;
        }

        this.is_init.set(true);

        // The handlers belong to a child of the page: they hold the page weakly.
        let swipe_panel = this.swipe_panel();
        let weak = this.to_ref().downgrade();
        swipe_panel.add_handler(InputElement::swipe_gesture_event(), move |_s, e: &SwipeGestureEventArgs| {
            if let Some(this) = weak.upgrade() {
                this.on_swipe_gesture(e);
            }
        });
        let weak = this.to_ref().downgrade();
        swipe_panel.add_handler(
            InputElement::swipe_gesture_ended_event(),
            move |_s, e: &SwipeGestureEndedEventArgs| {
                if let Some(this) = weak.upgrade() {
                    this.on_swipe_gesture_ended(e);
                }
            },
        );
    }
}

impl GestureSwipePage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            is_init: Cell::new(false),
            indicator_visual: RefCell::new(None),
            swipe_delta: Cell::new(Vector3D::default()),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    fn swipe_panel(&self) -> Ref<Panel> {
        self.get_control::<Panel>("SwipePanel")
    }

    fn swipe_indicator(&self) -> Ref<Border> {
        self.get_control::<Border>("SwipeIndicator")
    }

    fn direction_text(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("DirectionText")
    }

    fn delta_text(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("DeltaText")
    }

    fn velocity_text(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("VelocityText")
    }

    fn swipe_direction_label(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("SwipeDirectionLabel")
    }

    // The fields below are null while the document loads (the check boxes and the slider raise
    // their change events then), which the handlers test for.

    fn swipe_recognizer(&self) -> Option<Ref<SwipeGestureRecognizer>> {
        self.find_control::<SwipeGestureRecognizer>("SwipeRecognizer")
    }

    fn threshold_text(&self) -> Option<Ref<TextBlock>> {
        self.find_control::<TextBlock>("ThresholdText")
    }

    fn check_box(&self, name: &str) -> Ref<CheckBox> {
        self.get_control::<CheckBox>(name)
    }

    fn ensure_visual(&self) {
        if self.indicator_visual.borrow().is_some() {
            return;
        }

        let indicator_visual = ElementComposition::get_element_visual(&self.swipe_indicator());
        *self.indicator_visual.borrow_mut() = indicator_visual.clone();

        if let Some(indicator_visual) = indicator_visual {
            let compositor = indicator_visual.compositor().clone();
            let offset_animation = compositor.create_vector3_key_frame_animation();
            offset_animation.set_target(Some("Offset".to_owned()));
            offset_animation.insert_expression_key_frame(1.0, "this.FinalValue", None);
            offset_animation.set_duration(Duration::from_millis(150));

            let implicit_animations = compositor.create_implicit_animation_collection();
            implicit_animations.set("Offset", offset_animation);

            indicator_visual.set_implicit_animations(Some(implicit_animations));
        }
    }

    fn on_swipe_gesture(&self, e: &SwipeGestureEventArgs) {
        self.ensure_visual();

        let indicator_visual = self.indicator_visual.borrow().clone();
        if let Some(indicator_visual) = indicator_visual {
            let delta = Vector3D::new(-e.delta().x, -e.delta().y, 0.0);
            self.swipe_delta.set(self.swipe_delta.get() + delta);
            let props = indicator_visual.visual_props();
            props.set_offset(&*indicator_visual, props.offset() + delta);
        }

        self.direction_text().set_text(Some(&format!("Direction: {:?}", e.swipe_direction())));
        self.delta_text().set_text(Some(&format!("Delta: {:.1}, {:.1}", e.delta().x, e.delta().y)));
        self.velocity_text().set_text(Some(&format!("Velocity: {:.0}, {:.0}", e.velocity().x, e.velocity().y)));
        let label = self.swipe_direction_label();
        label.set_text(Some(&format!("{:?}", e.swipe_direction())));
        label.set_opacity(1.0);

        e.set_handled(true);
    }

    fn on_swipe_gesture_ended(&self, e: &SwipeGestureEndedEventArgs) {
        let indicator_visual = self.indicator_visual.borrow().clone();
        if let Some(indicator_visual) = indicator_visual {
            let props = indicator_visual.visual_props();
            props.set_offset(&*indicator_visual, props.offset() - self.swipe_delta.get());
        }

        self.swipe_delta.set(Vector3D::default());

        self.velocity_text().set_text(Some(&format!("Velocity: {:.0}, {:.0} (ended)", e.velocity().x, e.velocity().y)));
        let label = self.swipe_direction_label();
        label.set_text(Some("Swipe here"));
        label.set_opacity(0.5);
    }

    fn on_direction_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let Some(swipe_recognizer) = self.swipe_recognizer() else {
            return;
        };

        swipe_recognizer.set_can_horizontally_swipe(self.check_box("HorizontalCheck").is_checked() == Some(true));
        swipe_recognizer.set_can_vertically_swipe(self.check_box("VerticalCheck").is_checked() == Some(true));
    }

    fn on_mouse_enabled_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let Some(swipe_recognizer) = self.swipe_recognizer() else {
            return;
        };

        swipe_recognizer.set_is_mouse_enabled(self.check_box("MouseCheck").is_checked() == Some(true));
    }

    fn on_threshold_changed(&self, _sender: &Option<BoxedValue>, e: &RangeBaseValueChangedEventArgs) {
        let (Some(swipe_recognizer), Some(threshold_text)) = (self.swipe_recognizer(), self.threshold_text()) else {
            return;
        };

        swipe_recognizer.set_threshold(e.new_value());
        threshold_text.set_text(Some(&format!("{:.0}", e.new_value())));
    }
}
