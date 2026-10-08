use crate::primitives::converters::AccentColorConverter;
use crate::ColorChangedEventArgs;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::{InputElement, InputElementImpl, PointerPressedEventArgs};
use ferroui_base::interactivity::{Interactive, InteractiveImpl};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::HsvColor;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::span_helpers::{try_parse_int, NumberStyles};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, FerroObjectImplExt, FerroPropertyChangedEventArgs,
    Ref, StaticType, StyledElementImpl, VisualImpl,
};
use ferroui_controls::metadata::TemplatePartAttribute;
use ferroui_controls::primitives::{TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt};
use ferroui_controls::{Border, ControlImpl};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Presents a preview color with optional accent colors.
#[repr(C)]
pub struct ColorPreviewer {
    base: TemplatedControl,
    color_changed: HandlerList<dyn Fn(&ColorChangedEventArgs)>,
    events_connected: Cell<bool>,
    /// The handlers connected by `connect_events` (the original removes its
    /// handlers one by one with `-=`).
    event_disposables: RefCell<Vec<Rc<dyn IDisposable>>>,

    // XAML template parts
    accent_decrement1_border: RefCell<Option<Ref<Border>>>,
    accent_decrement2_border: RefCell<Option<Ref<Border>>>,
    accent_increment1_border: RefCell<Option<Ref<Border>>>,
    accent_increment2_border: RefCell<Option<Ref<Border>>>,
}

ferro_class! {
    ColorPreviewer: TemplatedControl, virtuals ColorPreviewerImpl: TemplatedControlImpl {
        /// Raises the color changed event.
        ///
        /// `e` defines the old/new colors.
        fn on_color_changed(this, e: &ColorChangedEventArgs);
    }
}
ferro_impl_classes!(ColorPreviewer: StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl);
ferroui_base::ferro_class_info!(ColorPreviewer { new: ColorPreviewer::new });

impl FerroObjectImpl for ColorPreviewer {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        if change.property() == Self::hsv_color_property().as_property() {
            this.on_color_changed(&ColorChangedEventArgs::new(
                change.get_old_value::<HsvColor>().unwrap_or_default().to_rgb(),
                change.get_new_value::<HsvColor>().to_rgb(),
            ));
        }

        Self::parent_on_property_changed(this, change);
    }
}

impl TemplatedControlImpl for ColorPreviewer {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        // Remove any existing events present if the control was previously loaded then unloaded
        this.connect_events(false);

        *this.accent_decrement1_border.borrow_mut() = e.name_scope().find_as::<Border>("PART_AccentDecrement1Border");
        *this.accent_decrement2_border.borrow_mut() = e.name_scope().find_as::<Border>("PART_AccentDecrement2Border");
        *this.accent_increment1_border.borrow_mut() = e.name_scope().find_as::<Border>("PART_AccentIncrement1Border");
        *this.accent_increment2_border.borrow_mut() = e.name_scope().find_as::<Border>("PART_AccentIncrement2Border");

        // Must connect after controls are found
        this.connect_events(true);

        Self::parent_on_apply_template(this, e);
    }
}

impl ColorPreviewerImpl for ColorPreviewer {
    fn on_color_changed(this: &Self, e: &ColorChangedEventArgs) {
        for (_, handler) in this.color_changed.snapshot().iter() {
            handler(e);
        }
    }
}

impl ColorPreviewer {
    /// The named parts expected in the control template.
    pub const TEMPLATE_PARTS: &'static [TemplatePartAttribute] = &[
        TemplatePartAttribute::new("PART_AccentDecrement1Border", <Border as StaticType>::TYPE),
        TemplatePartAttribute::new("PART_AccentDecrement2Border", <Border as StaticType>::TYPE),
        TemplatePartAttribute::new("PART_AccentIncrement1Border", <Border as StaticType>::TYPE),
        TemplatePartAttribute::new("PART_AccentIncrement2Border", <Border as StaticType>::TYPE),
    ];

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            color_changed: HandlerList::new(),
            events_connected: Cell::new(false),
            event_disposables: RefCell::new(Vec::new()),
            accent_decrement1_border: RefCell::new(None),
            accent_decrement2_border: RefCell::new(None),
            accent_increment1_border: RefCell::new(None),
            accent_increment2_border: RefCell::new(None),
        }
    }

    /// Initializes a new instance of the [`ColorPreviewer`] class.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Event for when the selected color changes within the previewer.
    /// This occurs when an accent color is pressed. Disposing the returned
    /// handle unsubscribes.
    pub fn color_changed(&self, handler: impl Fn(&ColorChangedEventArgs) + 'static) -> Rc<dyn IDisposable> {
        let token = self.color_changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.color_changed.remove(token);
            }
        })
    }

    /// Connects or disconnects all control event handlers.
    ///
    /// `connected` is true to connect event handlers, otherwise false.
    fn connect_events(&self, connected: bool) {
        if connected && !self.events_connected.get() {
            // Add all events
            let borders = [
                self.accent_decrement1_border.borrow().clone(),
                self.accent_decrement2_border.borrow().clone(),
                self.accent_increment1_border.borrow().clone(),
                self.accent_increment2_border.borrow().clone(),
            ];
            let mut disposables = Vec::new();
            for border in borders.into_iter().flatten() {
                let weak = self.to_ref().downgrade();
                disposables.push(border.add_disposable_handler(
                    InputElement::pointer_pressed_event(),
                    move |sender, e: &PointerPressedEventArgs| {
                        if let Some(this) = weak.upgrade() {
                            this.accent_border_pointer_pressed(sender, e);
                        }
                    },
                ));
            }
            *self.event_disposables.borrow_mut() = disposables;

            self.events_connected.set(true);
        } else if !connected && self.events_connected.get() {
            // Remove all events
            let disposables = std::mem::take(&mut *self.event_disposables.borrow_mut());
            for disposable in disposables {
                disposable.dispose();
            }

            self.events_connected.set(false);
        }
    }

    /// Event handler for when an accent color border is pressed.
    /// This will update the color to the background of the pressed panel.
    fn accent_border_pointer_pressed(&self, sender: &Interactive, _e: &PointerPressedEventArgs) {
        let border = sender.to_ref().cast::<Border>();
        let mut accent_step = 0;
        let hsv_color = self.hsv_color();

        // Get the value component delta
        let tag = border.and_then(|border| border.tag()).map(|tag| ValueTypes::to_display_string(Some(&tag)));
        if let Some(parsed) = try_parse_int(tag.as_deref().unwrap_or("0"), NumberStyles::INTEGER) {
            accent_step = parsed;
        }

        if accent_step != 0 {
            // ColorChanged will be invoked in OnPropertyChanged if the value is different
            self.set_current_value(Self::hsv_color_property(), AccentColorConverter::get_accent(hsv_color, accent_step));
        }
    }
}
