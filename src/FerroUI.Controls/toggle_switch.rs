use crate::presenters::ContentPresenter;
use crate::primitives::{TemplateAppliedEventArgs, TemplatedControlImpl, TemplatedControlImplExt, ToggleButton, ToggleButtonImpl};
use crate::templates::IDataTemplate;
use crate::{ButtonImpl, Canvas, ContentControlImpl, ContentControlImplExt, Control, ControlImpl, ControlImplExt, Panel};
use ferroui_base::animation::Transitions;
use ferroui_base::input::{
    InputElement, InputElementImpl, PointerEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs,
};
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventArgs};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::utilities::math_utilities::{max, min};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, BoxedValue, FerroObjectImpl,
    FerroProperty, FerroPropertyChangedEventArgs, Point, Ref, StyledElement, StyledElementImpl, StyledProperty, Visual,
    VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const PC_DRAGGING: &str = ":dragging";

/// A toggle switch control.
#[repr(C)]
pub struct ToggleSwitch {
    base: ToggleButton,
    knobs_panel: RefCell<Option<Ref<Panel>>>,
    switch_knob: RefCell<Option<Ref<Panel>>>,
    knobs_panel_pressed: Cell<bool>,
    switch_start_point: Cell<Point>,
    init_left: Cell<f64>,
    is_dragging: Cell<bool>,
    off_content_presenter: RefCell<Option<Ref<ContentPresenter>>>,
    on_content_presenter: RefCell<Option<Ref<ContentPresenter>>>,
}

ferro_class!(ToggleSwitch: ToggleButton);

ferro_class_info!(ToggleSwitch {
    new: ToggleSwitch::new,
    markup: {
        attributes: [
            TemplatePart("PART_MovingKnobs", type(Ref<Panel>), IsRequired = true),
            TemplatePart("PART_OffContentPresenter", type(Ref<ContentPresenter>)),
            TemplatePart("PART_OnContentPresenter", type(Ref<ContentPresenter>)),
            TemplatePart("PART_SwitchKnob", type(Ref<Panel>)),
            PseudoClasses(":dragging"),
        ],
    },
});

ferro_impl_classes!(
    ToggleSwitch: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ButtonImpl,
    ToggleButtonImpl
);

impl ControlImpl for ToggleSwitch {
    fn on_loaded(this: &Self, e: &RoutedEventArgs) {
        Self::parent_on_loaded(this, e);
        this.update_knob_transitions();
    }
}

impl TemplatedControlImpl for ToggleSwitch {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);

        *this.switch_knob.borrow_mut() = e.name_scope().find_as::<Panel>("PART_SwitchKnob");
        let knobs_panel = e.name_scope().get_as::<Panel>("PART_MovingKnobs");
        *this.knobs_panel.borrow_mut() = Some(knobs_panel.clone());

        // The handlers refer to the switch weakly: the panel is a part of
        // the switch and must not keep it alive.
        let weak = this.to_ref().downgrade();
        knobs_panel.add_handler(InputElement::pointer_pressed_event(), {
            let weak = weak.clone();
            move |_, e: &PointerPressedEventArgs| {
                if let Some(this) = weak.upgrade() {
                    this.knobs_panel_pointer_pressed(e);
                }
            }
        });
        knobs_panel.add_handler(InputElement::pointer_released_event(), {
            let weak = weak.clone();
            move |_, e: &PointerReleasedEventArgs| {
                if let Some(this) = weak.upgrade() {
                    this.knobs_panel_pointer_released(e);
                }
            }
        });
        knobs_panel.add_handler(InputElement::pointer_moved_event(), move |_, e: &PointerEventArgs| {
            if let Some(this) = weak.upgrade() {
                this.knobs_panel_pointer_moved(e);
            }
        });

        if let Some(is_checked) = this.is_checked() {
            this.update_knob_pos(is_checked);
        }
    }
}

impl ContentControlImpl for ToggleSwitch {
    fn register_content_presenter(this: &Self, presenter: &ContentPresenter) -> bool {
        let mut result = Self::parent_register_content_presenter(this, presenter);

        let name = presenter.name();
        if name.as_deref() == Some("PART_OnContentPresenter") {
            *this.on_content_presenter.borrow_mut() = Some(presenter.to_ref());
            result = true;
        } else if name.as_deref() == Some("PART_OffContentPresenter") {
            *this.off_content_presenter.borrow_mut() = Some(presenter.to_ref());
            result = true;
        }

        result
    }
}

ferro_properties! {
    impl ToggleSwitch {
        /// Defines the `OffContent` property.
        pub fn off_content_property() -> StyledProperty<Option<BoxedValue>> {
            let default: BoxedValue = Rc::new("Off".to_string());
            FerroProperty::register::<ToggleSwitch, _>("OffContent", Some(default))
        }

        /// Defines the `OffContentTemplate` property.
        pub fn off_content_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
            FerroProperty::register::<ToggleSwitch, _>("OffContentTemplate", None)
        }

        /// Defines the `OnContent` property.
        pub fn on_content_property() -> StyledProperty<Option<BoxedValue>> {
            let default: BoxedValue = Rc::new("On".to_string());
            FerroProperty::register::<ToggleSwitch, _>("OnContent", Some(default))
        }

        /// Defines the `OnContentTemplate` property.
        pub fn on_content_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
            FerroProperty::register::<ToggleSwitch, _>("OnContentTemplate", None)
        }

        /// Defines the `KnobTransitions` property.
        pub fn knob_transitions_property() -> StyledProperty<Option<Transitions>> {
            FerroProperty::register::<ToggleSwitch, _>("KnobTransitions", None)
        }
    }
}

impl ToggleSwitch {
    fn static_constructor() {
        Self::off_content_property().changed().add_class_handler::<ToggleSwitch>(|x, e| x.off_content_changed(e));
        Self::on_content_property().changed().add_class_handler::<ToggleSwitch>(|x, e| x.on_content_changed(e));
        ToggleButton::is_checked_property().changed().add_class_handler::<ToggleSwitch>(|x, e| {
            if let Some(value) = e.get_new_value::<Option<bool>>() {
                x.update_knob_pos(value);
            }
        });

        Visual::bounds_property().changed().add_class_handler::<ToggleSwitch>(|x, _| {
            if let Some(is_checked) = x.is_checked() {
                x.update_knob_pos(is_checked);
            }
        });
        Self::knob_transitions_property().changed().add_class_handler::<ToggleSwitch>(|x, _| {
            x.update_knob_transitions();
        });
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: ToggleButton::construct(),
            knobs_panel: RefCell::new(None),
            switch_knob: RefCell::new(None),
            knobs_panel_pressed: Cell::new(false),
            switch_start_point: Cell::new(Point::default()),
            init_left: Cell::new(-1.0),
            is_dragging: Cell::new(false),
            off_content_presenter: RefCell::new(None),
            on_content_presenter: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The content that is displayed when in the on state.
    pub fn on_content(&self) -> Option<BoxedValue> {
        self.get_value(Self::on_content_property())
    }

    pub fn set_on_content(&self, value: Option<BoxedValue>) {
        self.set_value(Self::on_content_property(), value)
    }

    /// The content that is displayed when in the off state.
    pub fn off_content(&self) -> Option<BoxedValue> {
        self.get_value(Self::off_content_property())
    }

    pub fn set_off_content(&self, value: Option<BoxedValue>) {
        self.set_value(Self::off_content_property(), value)
    }

    /// The presenter of the off content from the control's template.
    pub fn off_content_presenter(&self) -> Option<Ref<ContentPresenter>> {
        self.off_content_presenter.borrow().clone()
    }

    /// The presenter of the on content from the control's template.
    pub fn on_content_presenter(&self) -> Option<Ref<ContentPresenter>> {
        self.on_content_presenter.borrow().clone()
    }

    /// The data template used to display the off content.
    pub fn off_content_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::off_content_template_property())
    }

    pub fn set_off_content_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::off_content_template_property(), value)
    }

    /// The data template used to display the on content.
    pub fn on_content_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::on_content_template_property())
    }

    pub fn set_on_content_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::on_content_template_property(), value)
    }

    /// The transitions of the switching knob.
    pub fn knob_transitions(&self) -> Option<Transitions> {
        self.get_value(Self::knob_transitions_property())
    }

    pub fn set_knob_transitions(&self, value: Option<Transitions>) {
        self.set_value(Self::knob_transitions_property(), value)
    }

    fn off_content_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        self.logical_content_changed(e);
    }

    fn on_content_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        self.logical_content_changed(e);
    }

    /// The body shared by the two content changed handlers.
    fn logical_content_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        let (old_value, new_value) = e.get_old_and_new_value::<Option<BoxedValue>>();

        if let Some(old_child) = old_value.as_ref().and_then(Control::logical_from_boxed) {
            StyledElement::logical_children(self).remove(&old_child);
        }

        if let Some(new_child) = new_value.as_ref().and_then(Control::logical_from_boxed) {
            StyledElement::logical_children(self).add(new_child);
        }
    }

    fn update_knob_transitions(&self) {
        let knobs_panel = self.knobs_panel.borrow().clone();
        if let Some(knobs_panel) = knobs_panel {
            knobs_panel.set_transitions(self.knob_transitions());
        }
    }

    fn switch_knob_visual(&self) -> Option<Ref<Visual>> {
        self.switch_knob.borrow().clone().map(Ref::upcast)
    }

    fn knobs_panel_pointer_pressed(&self, e: &PointerPressedEventArgs) {
        let knobs_panel = self.knobs_panel.borrow().clone().expect("the knobs panel raised the event");
        self.switch_start_point.set(e.get_position(self.switch_knob_visual().as_deref()));
        self.init_left.set(Canvas::get_left(&knobs_panel));
        self.is_dragging.set(false);
        self.knobs_panel_pressed.set(true);
    }

    fn knobs_panel_pointer_released(&self, e: &PointerReleasedEventArgs) {
        if self.is_dragging.get() {
            e.set_handled(true);

            let knobs_panel = self.knobs_panel.borrow().clone().expect("the knobs panel raised the event");
            let switch_knob = self.switch_knob.borrow().clone().expect("dragging requires the switch knob");
            let should_become_checked = Canvas::get_left(&knobs_panel) >= (switch_knob.bounds().width / 2.0);
            knobs_panel.clear_value(Canvas::left_property());

            self.pseudo_classes().set(PC_DRAGGING, false);

            if Some(should_become_checked) == self.is_checked() {
                self.update_knob_pos(should_become_checked);
            } else {
                self.set_current_value(ToggleButton::is_checked_property(), Some(should_become_checked));
            }
            self.update_knob_transitions();
        }

        self.is_dragging.set(false);

        self.knobs_panel_pressed.set(false);
    }

    fn knobs_panel_pointer_moved(&self, e: &PointerEventArgs) {
        if self.knobs_panel_pressed.get() {
            let difference = e.get_position(self.switch_knob_visual().as_deref()) - self.switch_start_point.get();

            if !self.is_dragging.get() && difference.x.abs() > 3.0 {
                let knobs_panel = self.knobs_panel.borrow().clone();
                if let Some(knobs_panel) = knobs_panel {
                    knobs_panel.set_transitions(None);
                }
                self.is_dragging.set(true);
                self.pseudo_classes().set(PC_DRAGGING, true);
            }

            if self.is_dragging.get() {
                let knobs_panel = self.knobs_panel.borrow().clone().expect("the knobs panel raised the event");
                let switch_knob = self.switch_knob.borrow().clone().expect("dragging requires the switch knob");
                // The minimum and the maximum propagate a position that is
                // not a number (the knob of an indeterminate switch has none).
                Canvas::set_left(
                    &knobs_panel,
                    min(switch_knob.bounds().width, max(0.0, self.init_left.get() + difference.x)),
                );
            }
        }
    }

    fn update_knob_pos(&self, value: bool) {
        let switch_knob = self.switch_knob.borrow().clone();
        let knobs_panel = self.knobs_panel.borrow().clone();
        if let (Some(switch_knob), Some(knobs_panel)) = (switch_knob, knobs_panel) {
            if value {
                Canvas::set_left(&knobs_panel, switch_knob.bounds().width);
            } else {
                Canvas::set_left(&knobs_panel, 0.0);
            }
        }
    }
}
