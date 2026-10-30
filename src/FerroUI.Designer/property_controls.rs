//! Reusable, document-independent controls for compact property inspectors.
//! Controls retain drafts locally and report one semantic commit to their host.
use ferroui_base::input::{
    Cursor, DataFormat, DataFormatOf, IPointer, InputElement, InputElementImpl,
    InputElementImplExt, Key, KeyEventArgs, KeyModifiers, MouseButton, PointerCaptureLostEventArgs,
    PointerEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs, StandardCursorType,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{HorizontalAlignment, LayoutableImpl, VerticalAlignment};
use ferroui_base::media::{
    BoxShadows, DrawingContext, FlowDirection, FormattedText, IBrush, SolidColorBrush, Typeface,
};
use ferroui_base::utilities::CultureInfo;
use ferroui_base::{
    CornerRadius, FerroObjectImpl, Point, Rect, Ref, StyledElementImpl, Thickness, VisualImpl,
    WeakRef, ferro_class, ferro_class_info, ferro_impl_classes, instantiate,
};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::{
    Button, ColumnDefinitions, Control, ControlImpl, DataValidationErrors, Grid, RowDefinitions,
    TextBlock, TextBox, ToolTip,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

type NumberAction = Rc<dyn Fn(f64)>;
type ComponentsAction = Rc<dyn Fn([f64; 4])>;

/// Small filled vectors keep the inspector independent of symbol fonts and platform glyphs.
pub(crate) fn property_icon(data: &str, size: f64) -> Ref<Control> {
    crate::icons::vector_icon(data, size).upcast::<Control>()
}

pub(crate) const RESET_ICON: &str = "F1 M4,2 L4,5.2 A5,5 0 1 1 3.2,9 L1.7,9 A6.5,6.5 0 1 0 4,3.2 L4,2 Z M1,2 L6,2 6,3.5 2.5,3.5 2.5,7 1,7 Z";
pub(crate) const CHEVRON_DOWN_ICON: &str = "F1 M2,5 L8,11 14,5 12.8,3.8 8,8.6 3.2,3.8 Z";
pub(crate) const CHEVRON_RIGHT_ICON: &str = "F1 M5,2 L11,8 5,14 3.8,12.8 8.6,8 3.8,3.2 Z";

/// Cross-process native drag format for a single XAML property value, such as a color,
/// resource expression or binding. Targets still validate against their active document.
pub fn property_value_data_format() -> DataFormatOf<String> {
    DataFormat::create_string_application_format("FerroUI.Designer.PropertyValue")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PropertyReferenceKind {
    Resource,
    Element,
}

/// Native tree references preserve their source identity for the duration of the drag.
/// This format is deliberately in-process: an AST identity is not a serialized token.
#[derive(Clone)]
pub(crate) struct PropertyReference {
    pub document_uri: String,
    pub revision: u64,
    pub ast: Arc<ferroui_designer_core::Ast>,
    pub node: ferroui_designer_core::NodeId,
    pub kind: PropertyReferenceKind,
}
pub(crate) fn property_reference_data_format() -> DataFormatOf<PropertyReference> {
    DataFormat::create_in_process_format("FerroUI.Designer.PropertyReference")
}

/// Numeric interaction policy shared by labels, keyboard nudging and component editors.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PropertyNumberOptions {
    pub minimum: f64,
    pub maximum: f64,
    pub step: f64,
    pub integer: bool,
}
impl Default for PropertyNumberOptions {
    fn default() -> Self {
        Self {
            minimum: -f64::MAX,
            maximum: f64::MAX,
            step: 1.0,
            integer: false,
        }
    }
}
impl PropertyNumberOptions {
    pub(crate) fn adjusted(self, original: f64, delta: f64, modifiers: KeyModifiers) -> f64 {
        let scale = if modifiers.contains(KeyModifiers::SHIFT) {
            10.0
        } else if modifiers.contains(KeyModifiers::CONTROL) {
            0.1
        } else {
            1.0
        };
        let value = original + delta * self.step * scale;
        let value = if self.integer {
            value.round()
        } else if value.abs() >= 1e15 {
            // Decimal cleanup must not overflow a large, otherwise finite draft.
            value
        } else {
            (value * 1e9).round() / 1e9
        };
        if value.is_nan() {
            original
        } else {
            value.clamp(self.minimum, self.maximum)
        }
    }
    fn validate(self) -> bool {
        self.minimum.is_finite()
            && self.maximum.is_finite()
            && self.minimum <= self.maximum
            && self.step.is_finite()
            && self.step > 0.0
    }
}

#[derive(Clone, Copy)]
struct NumberDrag {
    start_x: f64,
    start_value: f64,
    value: f64,
}

/// A property-name label that also scrubs a numeric value. Dragging shows a local draft;
/// releasing commits once. Escape, pointer capture loss and reconfiguration cancel.
#[repr(C)]
pub struct PropertyNumberScrubber {
    base: Control,
    label: RefCell<String>,
    value: Cell<f64>,
    options: Cell<PropertyNumberOptions>,
    drag: Cell<Option<NumberDrag>>,
    pointer: RefCell<Option<Rc<dyn IPointer>>>,
    commit: RefCell<Option<NumberAction>>,
}
ferro_class!(PropertyNumberScrubber:Control);
ferro_class_info!(PropertyNumberScrubber {
    new: PropertyNumberScrubber::new
});
ferro_impl_classes!(PropertyNumberScrubber:FerroObjectImpl,StyledElementImpl,LayoutableImpl,InteractiveImpl,ControlImpl);
impl PropertyNumberScrubber {
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self {
            base: Control::construct(),
            label: RefCell::new(String::new()),
            value: Cell::new(0.0),
            options: Cell::new(PropertyNumberOptions::default()),
            drag: Cell::new(None),
            pointer: RefCell::new(None),
            commit: RefCell::new(None),
        });
        this.set_height(26.0);
        this.set_min_width(32.0);
        this.set_focusable(true);
        this.set_cursor(Some(Cursor::new(StandardCursorType::SizeWestEast)));
        ToolTip::set_tip(&this, Some(Rc::new("Drag horizontally to adjust. Shift: ×10. Ctrl: precision. Escape cancels. Arrow keys nudge.".to_owned())));
        this
    }
    pub fn configure(
        &self,
        label: &str,
        value: f64,
        options: PropertyNumberOptions,
    ) -> Result<(), String> {
        if !value.is_finite() || !options.validate() {
            return Err(
                "A numeric scrubber needs a finite value, ordered bounds and a positive step"
                    .into(),
            );
        }
        self.cancel();
        *self.label.borrow_mut() = label.to_owned();
        self.value.set(value);
        self.options.set(options);
        AutomationProperties::set_name(self, Some(&format!("Adjust {label}")));
        self.invalidate_visual();
        Ok(())
    }
    pub fn value(&self) -> f64 {
        self.value.get()
    }
    pub fn draft_value(&self) -> Option<f64> {
        self.drag.get().map(|drag| drag.value)
    }
    pub fn on_commit(&self, callback: impl Fn(f64) + 'static) {
        *self.commit.borrow_mut() = Some(Rc::new(callback));
    }
    fn owns_pointer(&self, pointer: &Rc<dyn IPointer>) -> bool {
        self.pointer
            .borrow()
            .as_ref()
            .is_some_and(|active| active.id() == pointer.id())
    }
    fn cancel(&self) {
        self.drag.set(None);
        let pointer = self.pointer.borrow_mut().take();
        if let Some(pointer) = pointer {
            if pointer.captured().as_ref() == Some(&self.to_ref().upcast::<InputElement>()) {
                pointer.capture(None);
            }
        }
        self.invalidate_visual();
    }
    fn request(&self, value: f64) {
        let callback = self.commit.borrow().clone();
        if value != self.value.get() {
            if let Some(callback) = callback {
                callback(value);
            }
        }
    }
}
impl InputElementImpl for PropertyNumberScrubber {
    fn on_pointer_pressed(this: &Self, event: &PointerPressedEventArgs) {
        if !event.properties().is_left_button_pressed {
            return;
        }
        this.cancel();
        this.focus();
        this.drag.set(Some(NumberDrag {
            start_x: event.get_position(Some(this)).x,
            start_value: this.value.get(),
            value: this.value.get(),
        }));
        *this.pointer.borrow_mut() = Some(event.pointer().clone());
        event
            .pointer()
            .capture(Some(&this.to_ref().upcast::<InputElement>()));
        event.set_handled(true);
        this.invalidate_visual();
    }
    fn on_pointer_moved(this: &Self, event: &PointerEventArgs) {
        if !this.owns_pointer(event.pointer()) {
            return;
        }
        if let Some(mut drag) = this.drag.get() {
            drag.value = this.options.get().adjusted(
                drag.start_value,
                event.get_position(Some(this)).x - drag.start_x,
                event.key_modifiers(),
            );
            this.drag.set(Some(drag));
            event.set_handled(true);
            this.invalidate_visual();
        }
    }
    fn on_pointer_released(this: &Self, event: &PointerReleasedEventArgs) {
        if event.initial_press_mouse_button() != MouseButton::Left
            || !this.owns_pointer(event.pointer())
        {
            return;
        }
        let drag = this.drag.get();
        this.cancel();
        if let Some(drag) = drag {
            let delta = event.get_position(Some(this)).x - drag.start_x;
            if delta.abs() >= 2.0 {
                this.request(this.options.get().adjusted(
                    drag.start_value,
                    delta,
                    event.key_modifiers(),
                ));
            }
        }
        event.set_handled(true);
    }
    fn on_pointer_capture_lost(this: &Self, event: &PointerCaptureLostEventArgs) {
        if this.owns_pointer(event.pointer()) {
            this.pointer.borrow_mut().take();
            this.drag.set(None);
            this.invalidate_visual();
        }
        Self::parent_on_pointer_capture_lost(this, event);
    }
    fn on_key_down(this: &Self, event: &KeyEventArgs) {
        match event.key {
            Key::Escape if this.drag.get().is_some() => this.cancel(),
            Key::Left | Key::Down | Key::Right | Key::Up => {
                this.cancel();
                let delta = if matches!(event.key, Key::Left | Key::Down) {
                    -1.0
                } else {
                    1.0
                };
                this.request(this.options.get().adjusted(
                    this.value.get(),
                    delta,
                    event.key_modifiers,
                ));
            }
            _ => {
                Self::parent_on_key_down(this, event);
                return;
            }
        }
        event.set_handled(true);
    }
}
impl VisualImpl for PropertyNumberScrubber {
    fn render(this: &Self, context: &mut DrawingContext) {
        let bounds = Rect::new(0.0, 0.0, this.bounds().width, this.bounds().height);
        let clip = context.push_clip(bounds);
        let dragging = this.drag.get();
        let fill: Rc<dyn IBrush> = SolidColorBrush::from_uint32(if dragging.is_some() {
            0xff2d4061
        } else {
            0x00000000
        })
        .into();
        context.draw_rectangle(Some(&fill), None, bounds, 3.0, 3.0, &BoxShadows::default());
        let label = if let Some(drag) = dragging {
            format!("{}  {}", this.label.borrow(), number_text(drag.value))
        } else {
            this.label.borrow().clone()
        };
        let text = FormattedText::new(
            &label,
            CultureInfo::current_culture(),
            FlowDirection::LeftToRight,
            Typeface::new(ferroui_controls::documents::TextElement::get_font_family(
                this,
            )),
            11.0,
            Some(
                SolidColorBrush::from_uint32(if dragging.is_some() {
                    0xffa8c7ff
                } else {
                    0xffa9adba
                })
                .into(),
            ),
        );
        context.draw_text(
            &text,
            Point::new(2.0, ((bounds.height - text.height()) / 2.0).max(0.0)),
        );
        context.pop(clip);
    }
}

/// Four-component property ordering. The two-value XAML shorthand differs between
/// thickness (horizontal/vertical) and corner radius (top/bottom).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PropertyComponentsKind {
    Thickness,
    CornerRadius,
}
impl PropertyComponentsKind {
    pub fn parse(self, value: &str) -> Option<[f64; 4]> {
        let values = match self {
            Self::Thickness => {
                let v = Thickness::parse(value).ok()?;
                [v.left, v.top, v.right, v.bottom]
            }
            Self::CornerRadius => {
                let v = CornerRadius::parse(value).ok()?;
                [v.top_left, v.top_right, v.bottom_right, v.bottom_left]
            }
        };
        values
            .iter()
            .all(|value| value.is_finite())
            .then_some(values)
    }
    fn labels(self) -> [&'static str; 4] {
        match self {
            Self::Thickness => ["L", "T", "R", "B"],
            Self::CornerRadius => ["TL", "TR", "BR", "BL"],
        }
    }
    fn descriptions(self) -> [&'static str; 4] {
        match self {
            Self::Thickness => ["Left", "Top", "Right", "Bottom"],
            Self::CornerRadius => ["Top left", "Top right", "Bottom right", "Bottom left"],
        }
    }
}
struct ComponentsState {
    root: WeakRef<Grid>,
    fields: [WeakRef<TextBox>; 4],
    kind: PropertyComponentsKind,
    original: Cell<Option<[f64; 4]>>,
    linked: Cell<bool>,
    non_negative: bool,
    commit: RefCell<Option<ComponentsAction>>,
}
impl ComponentsState {
    fn refresh_layout(&self) {
        let linked = self.linked.get();
        if let Some(root) = self.root.upgrade() {
            root.set_row_definitions(
                RowDefinitions::parse(if linked { "28,0" } else { "28,28" }).unwrap(),
            );
            for control in root.children().to_vec() {
                if control.classes().contains("property-link") {
                    Grid::set_row_span(&control, if linked { 1 } else { 2 });
                }
            }
        }
        for (index, field) in self.fields.iter().enumerate() {
            if let Some(field) = field.upgrade() {
                field.set_is_visible(!linked || index == 0);
                Grid::set_column_span(&field, if linked && index == 0 { 2 } else { 1 });
                if let Some(label) = field
                    .inner_left_content()
                    .and_then(|content| Control::from_boxed(&content))
                    .and_then(|control| control.cast::<TextBlock>())
                {
                    label.set_text(Some(if linked && index == 0 {
                        "All"
                    } else {
                        self.kind.labels()[index]
                    }));
                }
                AutomationProperties::set_name(
                    &field,
                    Some(if linked {
                        "All components"
                    } else {
                        self.kind.descriptions()[index]
                    }),
                );
            }
        }
    }
    fn restore(&self) {
        for (index, weak) in self.fields.iter().enumerate() {
            if let Some(field) = weak.upgrade() {
                field.set_text(
                    self.original
                        .get()
                        .map(|values| number_text(values[index]))
                        .as_deref(),
                );
                DataValidationErrors::clear_errors(&field);
            }
        }
    }
    fn read(&self, changed: usize) -> Option<[f64; 4]> {
        let mut values = [0.0; 4];
        let mut valid = true;
        for field in &self.fields {
            if let Some(field) = field.upgrade() {
                DataValidationErrors::clear_errors(&field);
            }
        }
        for (index, value) in values.iter_mut().enumerate() {
            let source = if self.linked.get() { changed } else { index };
            let field = self.fields[source].upgrade()?;
            if let Some(parsed) = field
                .text()
                .and_then(|text| text.trim().parse::<f64>().ok())
                .filter(|value| value.is_finite() && (!self.non_negative || *value >= 0.0))
            {
                *value = parsed;
                DataValidationErrors::clear_errors(&field);
            } else {
                valid = false;
                DataValidationErrors::set_errors(
                    &field,
                    Some(vec![Rc::new(if self.non_negative {
                        "Enter a finite, non-negative value".to_owned()
                    } else {
                        "Enter a finite numeric value".to_owned()
                    })]),
                );
            }
        }
        valid.then_some(values)
    }
    fn commit(&self, changed: usize) {
        let Some(values) = self.read(changed) else {
            return;
        };
        if self.original.get() != Some(values) {
            let callback = self.commit.borrow().clone();
            if let Some(callback) = callback {
                callback(values);
            }
        }
    }
}

/// A linked value or compact 2 × 2 editor for Margin, Padding, BorderThickness or
/// CornerRadius. All fields commit together on Enter; Escape restores the accepted value.
/// A linked edit deliberately sets every component and is also valid for mixed selections.
pub struct PropertyComponentsEditor {
    root: Ref<Grid>,
    state: Rc<ComponentsState>,
    link: Ref<Button>,
}
impl PropertyComponentsEditor {
    pub fn new(kind: PropertyComponentsKind, non_negative: bool) -> Self {
        let root = Grid::new();
        root.classes().add("property-components");
        root.set_column_definitions(ColumnDefinitions::parse("*,*,24").unwrap());
        root.set_row_definitions(RowDefinitions::parse("28,28").unwrap());
        let fields: [Ref<TextBox>; 4] = std::array::from_fn(|index| {
            let label = TextBlock::new();
            label.set_text(Some(kind.labels()[index]));
            label.set_font_size(9.0);
            label.set_foreground(Some(SolidColorBrush::from_uint32(0xff979caa).into()));
            label.set_width(21.0);
            label.set_vertical_alignment(VerticalAlignment::Center);
            label.set_horizontal_alignment(HorizontalAlignment::Center);
            let field = TextBox::new();
            field.classes().add("designer-property-field");
            field.classes().add("property-component-field");
            field.set_inner_left_content(Some(Control::boxed(&label)));
            field.set_font_size(11.0);
            field.set_min_height(26.0);
            field.set_min_width(20.0);
            field.set_padding(Thickness::new(2.0, 3.0, 5.0, 3.0));
            field.set_margin(Thickness::new(0.0, 1.0, 3.0, 1.0));
            field.set_placeholder_text(Some("—"));
            AutomationProperties::set_name(&field, Some(kind.descriptions()[index]));
            ToolTip::set_tip(
                &field,
                Some(Rc::new(format!(
                    "{} · Enter applies all components; Escape restores; Up/Down nudges (Shift ×10, Ctrl ÷10)",
                    kind.descriptions()[index]
                ))),
            );
            Grid::set_row(&field, (index / 2) as i32);
            Grid::set_column(&field, (index % 2) as i32);
            root.children().add(field.clone().upcast::<Control>());
            field
        });
        let state = Rc::new(ComponentsState {
            root: root.downgrade(),
            fields: std::array::from_fn(|index| fields[index].downgrade()),
            kind,
            original: Cell::new(None),
            linked: Cell::new(false),
            non_negative,
            commit: RefCell::new(None),
        });
        for (index, field) in fields.iter().enumerate() {
            let weak = field.downgrade();
            field.text_changing(move |_, _| {
                if let Some(field) = weak.upgrade() {
                    DataValidationErrors::clear_errors(&field);
                }
            });
            let state = state.clone();
            let weak = field.downgrade();
            field.add_handler(InputElement::key_down_event(), move |_, event| {
                match event.key {
                    Key::Enter => state.commit(index),
                    Key::Escape => state.restore(),
                    Key::Up | Key::Down => {
                        let Some(field) = weak.upgrade() else {
                            return;
                        };
                        let Some(value) = field
                            .text()
                            .and_then(|text| text.trim().parse::<f64>().ok())
                            .filter(|value| value.is_finite())
                        else {
                            return;
                        };
                        let options = PropertyNumberOptions {
                            minimum: if state.non_negative { 0.0 } else { -f64::MAX },
                            ..Default::default()
                        };
                        let value = options.adjusted(
                            value,
                            if event.key == Key::Up { 1.0 } else { -1.0 },
                            event.key_modifiers,
                        );
                        field.set_text(Some(&number_text(value)));
                    }
                    _ => return,
                }
                event.set_handled(true);
            });
        }
        let link = Button::new();
        link.classes().add("property-link");
        let icon = property_icon(
            "F1 M5.5,3 L7,3 7,4.5 5.5,4.5 A3.5,3.5 0 0 0 5.5,11.5 L7,11.5 7,13 5.5,13 A5,5 0 0 1 5.5,3 Z M9,3 L10.5,3 A5,5 0 0 1 10.5,13 L9,13 9,11.5 10.5,11.5 A3.5,3.5 0 0 0 10.5,4.5 L9,4.5 Z M5,7.25 L11,7.25 11,8.75 5,8.75 Z",
            14.0,
        );
        link.set_content(Some(Control::boxed(&icon)));
        link.set_width(24.0);
        link.set_min_width(0.0);
        link.set_min_height(24.0);
        link.set_padding(Thickness::uniform(1.0));
        link.set_vertical_alignment(VerticalAlignment::Center);
        Grid::set_column(&link, 2);
        Grid::set_row_span(&link, 2);
        let weak = link.downgrade();
        let linked = state.clone();
        link.click(move |_, _| {
            linked.linked.set(!linked.linked.get());
            linked.refresh_layout();
            if let Some(link) = weak.upgrade() {
                update_link(&link, linked.linked.get());
            }
        });
        root.children().add(link.clone().upcast::<Control>());
        update_link(&link, false);
        Self { root, state, link }
    }
    pub fn control(&self) -> Ref<Control> {
        self.root.clone().upcast::<Control>()
    }
    pub fn configure(&self, value: Option<[f64; 4]>) -> Result<(), String> {
        if value.is_some_and(|value| {
            value
                .iter()
                .any(|value| !value.is_finite() || (self.state.non_negative && *value < 0.0))
        }) {
            return Err(
                "Property components must be finite and within their permitted range".into(),
            );
        }
        self.state.original.set(value);
        self.state.restore();
        Ok(())
    }
    pub fn set_linked(&self, linked: bool) {
        self.state.linked.set(linked);
        self.state.refresh_layout();
        update_link(&self.link, linked);
    }
    pub fn is_linked(&self) -> bool {
        self.state.linked.get()
    }
    pub fn on_commit(&self, callback: impl Fn([f64; 4]) + 'static) {
        *self.state.commit.borrow_mut() = Some(Rc::new(callback));
    }
}
fn update_link(button: &Button, linked: bool) {
    button.classes().set("active", linked);
    AutomationProperties::set_name(
        button,
        Some(if linked {
            "Unlink property components"
        } else {
            "Link property components"
        }),
    );
    ToolTip::set_tip(
        button,
        Some(Rc::new(
            if linked {
                "Linked: editing one component sets all four"
            } else {
                "Independent components · click to link"
            }
            .to_owned(),
        )),
    );
}
pub(crate) fn number_text(value: f64) -> String {
    if value == 0.0 {
        "0".into()
    } else {
        value.to_string()
    }
}
pub(crate) fn components_text(values: [f64; 4]) -> String {
    values.map(number_text).join(",")
}

#[cfg(test)]
#[path = "property_controls_tests.rs"]
mod tests;
