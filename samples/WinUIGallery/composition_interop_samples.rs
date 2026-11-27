//! The code behind of the XamlCompInterop page (the handlers of `XamlCompInteropPage.xaml.cs`)
//! over FerroUI's composition layer.
//!
//! WinUI's `UIElement.StartAnimation` starts a composition animation on the element's `Scale` or
//! `Translation`; here the animation is started with the animation's `Target` on the composition
//! visual of the element (`ElementCompositionPreview::get_element_visual`), whose `Scale` and
//! `Translation` are those of the element while its facades are not set. An element given to
//! `SetExpressionReferenceParameter` is its composition visual, as everywhere in the port
//! (winui-composition.md, section 7): `ActualSize` and `ActualOffset` of the element are the
//! `Size` and `Offset` of that visual, which the renderer writes from the element's layout. The
//! animations run on the compositor, which draws the visuals with each backend's GPU renderer.

use ferroui_base::numerics::Vector3;
use ferroui_base::rendering::composition::animations::{
    ICompositionAnimation, SpringVector3NaturalMotionAnimation,
};
use ferroui_base::rendering::composition::{
    AsCompositionObject, CompositionVisual, Compositor, ICompositionObjectAnimations,
};
use ferroui_base::{Ref, Thickness, Visual, WeakRef};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::hosting::ElementCompositionPreview;
use ferroui_controls::primitives::Popup;
use ferroui_controls::shapes::{Ellipse, Rectangle};
use ferroui_controls::{
    Button, Control, Grid, RadioButton, RadioButtons, Slider, StackPanel, TextBlock,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

/// The page's text where its window renders without a compositor (a test host).
pub(crate) const NO_COMPOSITOR: &str =
    "This window has no compositor: the animations of this page run on the composition visuals of its elements.";

/// The number of buttons of `ActualSizeExample_Loaded` (`totalElements`).
pub(crate) const TOTAL_ELEMENTS: usize = 8;

/// The damping ratio of `GetDampingRatio` without a selected option.
pub(crate) const DEFAULT_DAMPING_RATIO: f32 = 0.6;

/// The elements whose scale the pointer animates (`element_PointerEntered` and
/// `element_PointerExited` in the source's markup).
const SCALED_ELEMENTS: &[&str] = &[
    "NaturalMotionButton",
    "rectangle",
    "ExpressionButton1",
    "ExpressionButton2",
    "ExpressionButton3",
    "ExpressionButton4",
];

/// `radius` of `ActualSizeExample_Loaded`: half the width of the source element (the source's
/// `(source.ActualSize.X / 2)`).
const RADIUS: &str = "(source.Size.X / 2)";

/// The expression of `ActualOffsetExample_Loaded`: the source's
/// `Vector3(source.ActualOffset.X + source.ActualSize.X, source.ActualOffset.Y +
/// source.ActualSize.Y / 2 - 25, 0)` over the visual's `Offset` and `Size`.
pub(crate) const POPUP_EXPRESSION: &str =
    "Vector3(source.Offset.X + source.Size.X, source.Offset.Y + source.Size.Y / 2 - 25, 0)";

pub(crate) fn wire(page_id: &str, root: &Ref<Control>) {
    if page_id != "XamlCompInterop" {
        return;
    }
    let page = Rc::new(XamlCompInteropPage {
        root: root.downgrade(),
        spring: RefCell::new(None),
    });

    let loaded = |name: &str, handler: fn(&XamlCompInteropPage)| {
        let page = page.clone();
        root.get_control::<StackPanel>(name)
            .loaded(move |_, _| handler(&page));
    };
    loaded("NaturalMotionExample", XamlCompInteropPage::natural_motion_example_loaded);
    loaded("ExpressionSample", XamlCompInteropPage::expression_sample_loaded);
    loaded("StackedButtonsExample", XamlCompInteropPage::stacked_buttons_example_loaded);
    loaded("ActualSizeExample", XamlCompInteropPage::actual_size_example_loaded);
    loaded("ActualOffsetExample", XamlCompInteropPage::actual_offset_example_loaded);

    for name in SCALED_ELEMENTS {
        let element = root.get_control::<Control>(name);
        let entered = page.clone();
        element.pointer_entered(move |sender, _| entered.element_pointer(sender, 1.5));
        let exited = page.clone();
        element.pointer_exited(move |sender, _| exited.element_pointer(sender, 1.0));
    }

    // `RadiusSlider_ValueChanged`.
    let weak_panel = root.get_control::<Grid>("LayoutPanel").downgrade();
    root.get_control::<Slider>("RadiusSlider")
        .value_changed(move |_, e| {
            if let Some(panel) = weak_panel.upgrade() {
                panel.set_width(e.new_value());
                panel.set_height(e.new_value());
            }
        });

    // The source's `Margin="{x:Bind MarginSlider.Value, Converter={StaticResource
    // doubleToThicknessConverter}, Mode=OneWay}"`: a uniform margin of the value.
    let target = root.get_control::<TextBlock>("PopupTarget");
    let margin_slider = root.get_control::<Slider>("MarginSlider");
    target.set_margin(Thickness::uniform(margin_slider.value()));
    let weak_target = target.downgrade();
    margin_slider.value_changed(move |_, e| {
        if let Some(target) = weak_target.upgrade() {
            target.set_margin(Thickness::uniform(e.new_value()));
        }
    });

    // `Popup.StartAnimation(anim)` of `ActualOffsetExample_Loaded`: the popup shows its child, so
    // the translation is that of the child, started once the child is in the overlay layer.
    let popup = root.get_control::<Popup>("Popup");
    let opened = page.clone();
    popup.opened(move || opened.position_popup()).detach();
}

/// The fields of `XamlCompInteropPage`. The root is held weakly: the handlers on the page's
/// elements hold this state.
struct XamlCompInteropPage {
    root: WeakRef<Control>,
    /// `_springAnimation`.
    spring: RefCell<Option<Rc<SpringVector3NaturalMotionAnimation>>>,
}

impl XamlCompInteropPage {
    fn root(&self) -> Option<Ref<Control>> {
        self.root.upgrade()
    }

    /// The composition visual of `element`; where the window has no compositor, the page says
    /// so and the example does nothing, as the source cannot run without one.
    fn visual_of(&self, element: &Visual) -> Option<Rc<CompositionVisual>> {
        let visual = ElementCompositionPreview::get_element_visual(element);
        if visual.is_none() {
            if let Some(root) = self.root() {
                let status = root.get_control::<TextBlock>("CompositionStatus");
                status.set_text(Some(NO_COMPOSITOR));
                status.set_is_visible(true);
            }
        }
        visual
    }

    /// The source's `_compositor`: the compositor of the page's elements.
    fn compositor(&self, element: &Visual) -> Option<Rc<Compositor>> {
        self.visual_of(element).map(|visual| visual.compositor().clone())
    }

    /// `NaturalMotionExample_Loaded`.
    fn natural_motion_example_loaded(&self) {
        let Some(root) = self.root() else { return };
        let button = root.get_control::<Button>("NaturalMotionButton");
        if self.spring.borrow().is_none() {
            let Some(compositor) = self.compositor(&button) else { return };
            let spring = compositor.create_spring_vector3_animation();
            spring.set_target(Some("Scale".to_owned()));
            *self.spring.borrow_mut() = Some(spring);
        }
        self.update_spring_animation(1.0);
    }

    /// `UpdateSpringAnimation`: the final value, the damping ratio and the period of the options.
    fn update_spring_animation(&self, final_value: f32) {
        let Some(root) = self.root() else { return };
        let spring = self.spring.borrow();
        let Some(spring) = spring.as_ref() else { return };
        spring.set_final_value(Some(Vector3::new(final_value, final_value, final_value)));
        spring.set_damping_ratio(damping_ratio(&root.get_control::<RadioButtons>("DampingStackPanel")));
        spring.set_period(period(&root.get_control::<Slider>("PeriodSlider")));
    }

    /// `element_PointerEntered` (1.5) and `element_PointerExited` (1): the spring to the scale,
    /// started on the element (`StartAnimationIfAPIPresent`).
    fn element_pointer(&self, sender: &Visual, final_value: f32) {
        if self.spring.borrow().is_none() {
            return;
        }
        self.update_spring_animation(final_value);
        let spring = self.spring.borrow().clone();
        if let Some(spring) = spring {
            start_animation(sender, &*spring);
        }
    }

    /// `ExpressionSample_Loaded`: the scale of the ellipse is the inverse of the rectangle's.
    fn expression_sample_loaded(&self) {
        let Some(root) = self.root() else { return };
        let rectangle = root.get_control::<Rectangle>("rectangle");
        let ellipse = root.get_control::<Ellipse>("ellipse");
        let (Some(compositor), Some(scale_element)) = (self.compositor(&ellipse), self.visual_of(&rectangle))
        else {
            return;
        };
        let anim = compositor.create_expression_animation();
        anim.set_expression(Some("Vector3(1/scaleElement.Scale.X, 1/scaleElement.Scale.Y, 1)".to_owned()));
        anim.set_target(Some("Scale".to_owned()));
        anim.set_expression_reference_parameter("scaleElement", reference(scale_element));
        start_animation(&ellipse, &*anim);
    }

    /// `StackedButtonsExample_Loaded`: each button moves by the scale and the translation of the
    /// one above it. The parameters are copied when the animation starts, so one animation serves
    /// the three buttons, as in the source.
    fn stacked_buttons_example_loaded(&self) {
        let Some(root) = self.root() else { return };
        let buttons: Vec<Ref<Button>> =
            (1..=4).map(|index| root.get_control::<Button>(&format!("ExpressionButton{index}"))).collect();
        let Some(compositor) = self.compositor(&buttons[0]) else { return };
        let anim = compositor.create_expression_animation();
        anim.set_expression(Some("(above.Scale.Y - 1) * 50 + above.Translation.Y % (50 * index)".to_owned()));
        anim.set_target(Some("Translation.Y".to_owned()));
        for index in 1..buttons.len() {
            let Some(above) = self.visual_of(&buttons[index - 1]) else { return };
            anim.set_expression_reference_parameter("above", reference(above));
            anim.set_scalar_parameter("index", index as f32);
            start_animation(&buttons[index], &*anim);
        }
    }

    /// `ActualSizeExample_Loaded`: eight buttons on a circle whose radius is half the width of the
    /// panel. The buttons are added once: a page loaded again restarts their animations.
    fn actual_size_example_loaded(&self) {
        let Some(root) = self.root() else { return };
        let panel = root.get_control::<Grid>("LayoutPanel");
        if panel.children().count() == 0 {
            for i in 0..TOTAL_ELEMENTS {
                let element = Button::new();
                element.set_content(Some(Rc::new("Button".to_string())));
                AutomationProperties::set_name(&element, Some(&format!("Button {i}")));
                panel.children().add(element);
            }
        }
        let Some(source) = self.visual_of(&panel) else { return };
        let compositor = source.compositor().clone();
        let expression = circle_expression();
        let children = panel.children();
        for i in 0..children.count() {
            let element = children.get(i);
            let anim = compositor.create_expression_animation();
            anim.set_expression(Some(expression.clone()));
            anim.set_scalar_parameter("index", (i + 1) as f32);
            anim.set_scalar_parameter("total", TOTAL_ELEMENTS as f32);
            anim.set_target(Some("Translation".to_owned()));
            anim.set_expression_reference_parameter("source", reference(source.clone()));
            start_animation(&element, &*anim);
        }
    }

    /// `ActualOffsetExample_Loaded`: opens the popup; [`position_popup`](Self::position_popup)
    /// starts its animation.
    fn actual_offset_example_loaded(&self) {
        let Some(root) = self.root() else { return };
        let popup = root.get_control::<Popup>("Popup");
        if popup.is_open() {
            self.position_popup();
        } else {
            popup.set_is_open(true);
        }
    }

    /// The animation of `ActualOffsetExample_Loaded`: the popup is right of the target, centred
    /// on it vertically.
    fn position_popup(&self) {
        let Some(root) = self.root() else { return };
        let target = root.get_control::<TextBlock>("PopupTarget");
        let Some(child) = root.get_control::<Popup>("Popup").child() else { return };
        let (Some(compositor), Some(source)) = (self.compositor(&child), self.visual_of(&target)) else {
            return;
        };
        let anim = compositor.create_expression_animation();
        anim.set_expression(Some(POPUP_EXPRESSION.to_owned()));
        anim.set_target(Some("Translation".to_owned()));
        anim.set_expression_reference_parameter("source", reference(source));
        start_animation(&child, &*anim);
    }
}

/// `UIElement.StartAnimation`: starts `animation` on its `Target` of the composition visual of
/// `element`. Returns whether the element has one.
pub(crate) fn start_animation(element: &Visual, animation: &dyn ICompositionAnimation) -> bool {
    match ElementCompositionPreview::get_element_visual(element) {
        Some(visual) => {
            visual.start_animation_group(animation);
            true
        }
        None => false,
    }
}

/// An element as a reference parameter of an expression: its composition visual.
fn reference(visual: Rc<CompositionVisual>) -> Rc<dyn AsCompositionObject> {
    visual
}

/// The expression of `ActualSizeExample_Loaded`: `Vector3({0}*cos({1})+{2}, {0}*sin({1})+{3},0)`
/// with the radius, the angle, and the offsets (the radius in X: the buttons are laid out at the
/// left edge; none in Y: they are centred vertically).
pub(crate) fn circle_expression() -> String {
    let radius = RADIUS;
    let theta = format!(".02 * {radius} + ((2 * Pi)/total)*index");
    let x_offset = radius;
    let y_offset = "0";
    format!("Vector3({radius}*cos({theta})+{x_offset}, {radius}*sin({theta})+{y_offset},0)")
}

/// `GetDampingRatio`: the content of the selected option, else 0.6. The content is parsed with
/// the invariant decimal point, as the source does.
pub(crate) fn damping_ratio(options: &RadioButtons) -> f32 {
    options
        .selected_item()
        .and_then(|item| Control::from_boxed(&item))
        .and_then(|item| item.cast::<RadioButton>())
        .and_then(|item| item.content())
        .and_then(|content| content.downcast_ref::<String>().and_then(|text| text.parse::<f32>().ok()))
        .unwrap_or(DEFAULT_DAMPING_RATIO)
}

/// `GetPeriod`: the slider's value in milliseconds.
pub(crate) fn period(slider: &Slider) -> Duration {
    Duration::from_secs_f64(slider.value().max(0.0) / 1000.0)
}
