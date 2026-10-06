//! Port of `Pages/Gestures/GesturePullPage.xaml.cs`: the class of the
//! document `Pages/Gestures/GesturePullPage.xaml`.

use crate::markup::xaml_class;
use ferroui_base::input::{InputElement, InputElementImpl, PullGestureEndedEventArgs, PullGestureEventArgs};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::rendering::composition::{CompositionVisual, ElementComposition};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, StyledElementImpl, Vector3D,
    VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{Border, ContentControlImpl, Control, ControlImpl, UserControl};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

#[repr(C)]
pub struct GesturePullPage {
    base: UserControl,
    is_init: Cell<bool>,
}

ferro_class!(GesturePullPage: UserControl);
ferro_impl_classes!(
    GesturePullPage: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(GesturePullPage { new: GesturePullPage::new });
xaml_class!(GesturePullPage, "/Pages/Gestures/GesturePullPage.xaml");

impl VisualImpl for GesturePullPage {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);

        if this.is_init.get() {
            return;
        }

        this.is_init.set(true);

        Self::set_pull_handlers(&this.get_control::<Control>("TopPullZone"), false);
        Self::set_pull_handlers(&this.get_control::<Control>("BottomPullZone"), true);
        Self::set_pull_handlers(&this.get_control::<Control>("RightPullZone"), true);
        Self::set_pull_handlers(&this.get_control::<Control>("LeftPullZone"), false);
    }
}

impl GesturePullPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), is_init: Cell::new(false) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    fn set_pull_handlers(control: &Control, inverse: bool) {
        let Some(ball) = control.find_logical_descendant_of_type::<Border>(false) else {
            return;
        };

        // The locals the handlers share. The handlers belong to `control`, which holds `ball`:
        // they hold the ball weakly.
        let default_offset = Rc::new(Cell::new(Vector3D::default()));
        let ball_composition_visual: Rc<RefCell<Option<Rc<CompositionVisual>>>> = Rc::new(RefCell::new(None));
        let weak_ball = ball.downgrade();

        fn init_composition(ball_composition_visual: &RefCell<Option<Rc<CompositionVisual>>>, ball: &Border) {
            let visual = ElementComposition::get_element_visual(ball);
            *ball_composition_visual.borrow_mut() = visual.clone();

            if let Some(visual) = visual {
                let compositor = visual.compositor().clone();
                let offset_animation = compositor.create_vector3_key_frame_animation();
                offset_animation.set_target(Some("Offset".to_owned()));
                offset_animation.insert_expression_key_frame(1.0, "this.FinalValue", None);
                offset_animation.set_duration(Duration::from_millis(100));

                let implicit_animations = compositor.create_implicit_animation_collection();
                implicit_animations.set("Offset", offset_animation);

                visual.set_implicit_animations(Some(implicit_animations));
            }
        }

        init_composition(&ball_composition_visual, &ball);

        {
            let (weak_ball, default_offset) = (weak_ball.clone(), default_offset.clone());
            let ball_composition_visual = ball_composition_visual.clone();
            let _layout_updated = control.layout_updated(move || {
                let Some(ball) = weak_ball.upgrade() else {
                    return;
                };
                init_composition(&ball_composition_visual, &ball);
                if let Some(visual) = ball_composition_visual.borrow().as_ref() {
                    default_offset.set(visual.visual_props().offset());
                }
            });
        }

        {
            let (weak_ball, default_offset) = (weak_ball.clone(), default_offset.clone());
            let ball_composition_visual = ball_composition_visual.clone();
            control.add_handler(InputElement::pull_gesture_event(), move |_s, e: &PullGestureEventArgs| {
                let Some(ball) = weak_ball.upgrade() else {
                    return;
                };
                init_composition(&ball_composition_visual, &ball);
                let visual = ball_composition_visual.borrow().clone();
                if let Some(visual) = visual {
                    // The factor is single precision, as in the original.
                    let factor = f64::from(0.4f32);
                    let delta = Vector3D::new(e.delta().x * factor, e.delta().y * factor, 0.0)
                        * if inverse { -1.0 } else { 1.0 };
                    visual.visual_props().set_offset(&*visual, default_offset.get() + delta);
                    e.set_handled(true);
                }
            });
        }

        control.add_handler(InputElement::pull_gesture_ended_event(), move |_s, _e: &PullGestureEndedEventArgs| {
            let Some(ball) = weak_ball.upgrade() else {
                return;
            };
            init_composition(&ball_composition_visual, &ball);
            let visual = ball_composition_visual.borrow().clone();
            if let Some(visual) = visual {
                visual.visual_props().set_offset(&*visual, default_offset.get());
            }
        });
    }
}
