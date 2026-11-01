//! Mappings use typed FerroUI setters, native panel collections and routed events.
use super::*;
use ferroui_base::{
    layout::{Layoutable, Orientation},
    media::{Brush, FontWeight, IBrush},
    reactive::Disposable,
    CornerRadius, Thickness,
};
use ferroui_controls::{
    primitives::ToggleButton, Border, Button, Canvas, CheckBox, Grid, StackPanel, TextBlock,
    TextBox, WrapPanel,
};

#[derive(Clone, Copy)]
enum Kind {
    Item,
    Loader,
    Repeater,
    ListView,
    Rectangle,
    Row,
    Column,
    Flow,
    Text,
    Button,
    CheckBox,
    Slider,
    ProgressBar,
    SpinBox,
    RadioButton,
    Switch,
    TextField,
    TextArea,
}

pub(super) fn register(registry: &mut ControlRegistry) -> Result<(), Error> {
    registry.types_mut().register_quick_states()?;
    registry.types_mut().register(
        "QtQuick.Controls",
        "ButtonGroup",
        TypeDefinition::quick_button_group(),
    )?;
    use Kind::*;
    for (module, types) in [
        (
            "QtQuick",
            &[
                ("Item", Item),
                ("Loader", Loader),
                ("Repeater", Repeater),
                ("ListView", ListView),
                ("Rectangle", Rectangle),
                ("Row", Row),
                ("Column", Column),
                ("Flow", Flow),
                ("Text", Text),
                ("TextInput", TextField),
                ("TextEdit", TextArea),
            ][..],
        ),
        (
            "QtQuick.Controls",
            &[
                ("Label", Text),
                ("Button", Button),
                ("CheckBox", CheckBox),
                ("Slider", Slider),
                ("SpinBox", SpinBox),
                ("ProgressBar", ProgressBar),
                ("Switch", Switch),
                ("RadioButton", RadioButton),
                ("TextField", TextField),
                ("TextArea", TextArea),
            ][..],
        ),
    ] {
        registry.types_mut().set_module_version(module, 6, 12);
        registry.types_mut().set_module_version(module, 2, 15);
        for &(name, kind) in types {
            registry.register(module, name, kind.definition(), move || {
                let peer = kind.create();
                super::positioner::Positioner::mark_item(&peer.control());
                Box::new(peer)
            })?;
        }
    }
    registry.types_mut().reexport("QtQuick", "QtQml")?;
    Ok(())
}

impl Kind {
    fn definition(self) -> TypeDefinition {
        use Value::{Bool, List, Number, Undefined};
        let mut ty = if matches!(self, Self::Item) {
            TypeDefinition::default()
                .with_quick_states()
                .inherits("QtQml", "QtObject")
                .property_group("Accessible")
                .property("Accessible.name", "string", Value::String(String::new()))
                .property("visible", "bool", Bool(true))
                .property("enabled", "bool", Bool(true))
                .property("width", "real", Undefined)
                .property("height", "real", Undefined)
                .property("implicitWidth", "real", Number(0.0))
                .property("implicitHeight", "real", Number(0.0))
                .property("x", "real", Number(0.0))
                .property("y", "real", Number(0.0))
                .property("clip", "bool", Bool(false))
                .property("opacity", "real", Number(1.0))
                .property("rotation", "real", Number(0.0))
                .quick_anchors()
                .property("data", "list<QtObject>", List(vec![]))
                .default_property("data")
                .visual_children("data")
        } else {
            TypeDefinition::default().inherits("QtQuick", "Item")
        };
        if matches!(
            self,
            Self::Text
                | Self::Button
                | Self::CheckBox
                | Self::RadioButton
                | Self::Switch
                | Self::TextField
                | Self::TextArea
        ) {
            ty = ty
                .property("text", "string", Value::String(String::new()))
                .property("font.pixelSize", "int", Undefined)
                .property("font.bold", "bool", Undefined);
        }
        match self {
            Self::Item => ty,
            Self::Loader => TypeDefinition::quick_loader(),
            Self::Repeater => TypeDefinition::quick_repeater(),
            Self::ListView => TypeDefinition::quick_list_view(),
            Self::Rectangle => ty
                .property("color", "color", Value::String("white".into()))
                .property("border.color", "color", Value::String("black".into()))
                .property("border.width", "int", Number(0.0))
                .property("radius", "real", Number(0.0)),
            Self::Column => ty.property("spacing", "real", Number(0.0)),
            Self::Row | Self::Flow => ty
                .property("spacing", "real", Number(0.0))
                .property("layoutDirection", "int", Number(0.))
                .readonly_property("effectiveLayoutDirection", "int", Number(0.)),
            Self::Text => ty.property("color", "color", Undefined),
            Self::Button => ty.quick_button("Button"),
            Self::CheckBox => ty.quick_button("CheckBox"),
            Self::Slider => ty.quick_slider(),
            Self::ProgressBar => ty.quick_progress_bar(),
            Self::SpinBox => ty.quick_spin_box(),
            Self::RadioButton => ty.quick_button("RadioButton"),
            Self::Switch => ty.quick_button("Switch"),
            Self::TextField | Self::TextArea => ty
                .property("color", "color", Undefined)
                .property("readOnly", "bool", Bool(false))
                .property("placeholderText", "string", Value::String(String::new())),
        }
    }

    fn create(self) -> Peer {
        match self {
            Self::Repeater => Peer::Repeater(Canvas::new()),
            Self::ListView => Peer::ListView(super::list_view::Viewport::new()),
            Self::Item | Self::Loader => Peer::Canvas(super::anchors::AnchorCanvas::create()),
            Self::Row | Self::Column => {
                let stack = super::positioner::Positioner::create(if matches!(self, Self::Row) {
                    Orientation::Horizontal
                } else {
                    Orientation::Vertical
                });
                Peer::Stack(stack)
            }
            Self::Flow => Peer::Flow(super::positioner::Flow::create()),
            Self::Rectangle => {
                // A native border consumes layout space inside its stroke. Overlay a
                // separate canvas so a QML child's origin stays at the outer edge.
                let root = Grid::new();
                let paint = Border::new();
                let children = super::anchors::AnchorCanvas::create();
                root.children().add(&paint);
                root.children().add(&children);
                Peer::Rectangle {
                    root,
                    paint,
                    children,
                }
            }
            Self::Text => Peer::Text(TextBlock::new()),
            Self::Button => Peer::Button(Button::new()),
            Self::CheckBox => Peer::Toggle(super::quick_buttons::QuickCheck::create()),
            Self::Slider => Peer::Range(super::quick_ranges::RangePeer::slider()),
            Self::ProgressBar => Peer::Range(super::quick_ranges::RangePeer::progress()),
            Self::SpinBox => Peer::Range(super::quick_ranges::RangePeer::spin()),
            Self::Switch => Peer::Toggle(super::quick_buttons::QuickSwitch::create()),
            Self::RadioButton => {
                let toggle = super::quick_buttons::QuickRadio::create();
                // QML groups can span parents or allow multiple selections. Isolate
                // the native radio from the toolkit's implicit sibling group.
                toggle
                    .cast::<ferroui_controls::RadioButton>()
                    .unwrap()
                    .set_group_name(Some(format!("qml-radio-{:p}", &*toggle)));
                Peer::Toggle(toggle)
            }
            Self::TextField | Self::TextArea => {
                let input = TextBox::new();
                input.set_accepts_return(matches!(self, Self::TextArea));
                Peer::Input(input)
            }
        }
    }
}

enum Peer {
    Range(super::quick_ranges::RangePeer),
    ListView(super::list_view::Viewport),
    Repeater(Ref<Canvas>),
    Canvas(Ref<Canvas>),
    Stack(Ref<StackPanel>),
    Flow(Ref<WrapPanel>),
    Rectangle {
        root: Ref<Grid>,
        paint: Ref<Border>,
        children: Ref<Canvas>,
    },
    Text(Ref<TextBlock>),
    Button(Ref<Button>),
    Toggle(Ref<ToggleButton>),
    Input(Ref<TextBox>),
}

impl NativeObject for Peer {
    fn control(&self) -> Ref<Control> {
        match self {
            Self::Canvas(c) | Self::Repeater(c) => c.clone().upcast(),
            Self::Range(r) => r.control(),
            Self::ListView(v) => v.control.clone().upcast(),
            Self::Stack(c) => c.clone().upcast(),
            Self::Flow(c) => c.clone().upcast(),
            Self::Rectangle { root, .. } => root.clone().upcast(),
            Self::Text(c) => c.clone().upcast(),
            Self::Button(c) => c.clone().upcast(),
            Self::Toggle(c) => c.clone().upcast(),
            Self::Input(c) => c.clone().upcast(),
        }
    }

    fn has_stable_visual_host(&self) -> bool {
        true
    }
    fn uses_item_anchors(&self) -> bool {
        true
    }
    fn anchor_host(&self) -> Option<Ref<Canvas>> {
        match self {
            Self::Canvas(canvas)
            | Self::Rectangle {
                children: canvas, ..
            } => Some(canvas.clone()),
            _ => None,
        }
    }

    fn children_property(&self) -> Option<&str> {
        Some("data")
    }

    fn content(&self) -> Option<(&str, Controls)> {
        let children = match self {
            Self::ListView(v) => v.canvas.children(),
            Self::Canvas(c) => c.children(),
            Self::Stack(c) => c.children(),
            Self::Flow(c) => c.children(),
            Self::Rectangle { children, .. } => children.children(),
            _ => return None,
        };
        Some(("data", children))
    }

    fn validate(&self, property: &str, value: &Value) -> Result<(), Error> {
        // Types are checked by the engine. Native domains add finite geometry
        // and parseable colors. Custom QML properties have no native constraints.
        if *value == Value::Undefined {
            return Ok(());
        }
        match (self, property) {
            (_, "width" | "height" | "x" | "y" | "opacity" | "rotation")
            | (Self::Rectangle { .. }, "border.width" | "radius")
            | (Self::Flow(_) | Self::Stack(_), "spacing")
            | (
                Self::Text(_) | Self::Button(_) | Self::Input(_) | Self::Toggle(_),
                "font.pixelSize",
            ) => {
                number(property, value)?;
            }
            (
                Self::ListView(_),
                "contentX" | "contentY" | "contentWidth" | "contentHeight" | "originX" | "originY",
            ) => {
                number(property, value)?;
            }
            (Self::Rectangle { .. }, "color" | "border.color")
            | (Self::Text(_) | Self::Input(_), "color") => {
                brush(property, value)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn apply(&self, property: &str, value: &Value) -> Result<(), Error> {
        let control = self.control();
        match property {
            "width" if *value == Value::Undefined => {
                control.clear_value(Layoutable::width_property())
            }
            "height" if *value == Value::Undefined => {
                control.clear_value(Layoutable::height_property())
            }
            "width" => control.set_width(number(property, value)?.max(0.0)),
            "height" => control.set_height(number(property, value)?.max(0.0)),
            "x" => Canvas::set_left(&control, number(property, value)?),
            "y" => Canvas::set_top(&control, number(property, value)?),
            // A Repeater supplies siblings and occupies no slot in a positioner.
            "visible" if matches!(self, Self::Repeater(_)) => control.set_is_visible(false),
            "visible" => control.set_is_visible(boolean(property, value)?),
            "enabled" => control.set_is_enabled(boolean(property, value)?),
            "opacity" => control.set_opacity(number(property, value)?.clamp(0.0, 1.0)),
            "Accessible.name" => {
                let name = string(property, value)?;
                ferroui_controls::automation::AutomationProperties::set_name(
                    &control,
                    (!name.is_empty()).then_some(name),
                );
            }
            "clip" => control.set_clip_to_bounds(boolean(property, value)?),
            "rotation" => {
                use ferroui_base::media::RotateTransform;
                let angle = number(property, value)?;
                if let Some(rotate) = control
                    .render_transform()
                    .and_then(|t| t.as_object()?.to_ref().cast::<RotateTransform>())
                {
                    rotate.set_angle(angle);
                } else if angle != 0. {
                    control.set_render_transform(Some(RotateTransform::with_angle(angle).into()));
                }
            }
            _ => match (self, property) {
                (
                    Self::ListView(v),
                    "contentX" | "contentY" | "contentWidth" | "contentHeight" | "originX"
                    | "originY",
                ) => {
                    v.apply(property, number(property, value)?);
                }
                (Self::Range(r), _) => r.apply(property, value),
                (Self::Rectangle { paint, .. }, "color") => {
                    paint.set_background(brush(property, value)?)
                }
                (Self::Rectangle { paint, .. }, "border.color") => {
                    paint.set_border_brush(brush(property, value)?)
                }
                (Self::Rectangle { paint, .. }, "border.width") => paint
                    .set_border_thickness(Thickness::uniform(number(property, value)?.max(0.0))),
                (Self::Rectangle { paint, .. }, "radius") => paint
                    .set_corner_radius(CornerRadius::uniform(number(property, value)?.max(0.0))),
                (Self::Stack(_) | Self::Flow(_), "effectiveLayoutDirection") => {
                    super::positioner::Positioner::set_reversed(
                        &control,
                        number(property, value)? == 1.,
                    );
                }
                (Self::Stack(c), "spacing") => c.set_spacing(number(property, value)?),
                (Self::Flow(c), "spacing") => {
                    let spacing = number(property, value)?;
                    c.set_item_spacing(spacing);
                    c.set_line_spacing(spacing);
                }
                (Self::Text(c), "text") => c.set_text(Some(string(property, value)?)),
                (Self::Text(c), "color") => {
                    if *value == Value::Undefined {
                        c.clear_value(TextBlock::foreground_property());
                    } else {
                        c.set_foreground(brush(property, value)?);
                    }
                }
                (Self::Button(c), "text") => {
                    c.set_content(Some(Rc::new(string(property, value)?.to_owned())))
                }
                (Self::Toggle(c), "text") => {
                    c.set_content(Some(Rc::new(string(property, value)?.to_owned())))
                }
                (Self::Toggle(c), "checked") if c.cast::<CheckBox>().is_none() => {
                    c.set_is_checked(Some(boolean(property, value)?))
                }
                (Self::Toggle(c), "checkState") => {
                    c.set_is_checked(match number(property, value)? as i32 {
                        1 => None,
                        2 => Some(true),
                        _ => Some(false),
                    })
                }
                (Self::Toggle(c), "tristate") => c.set_is_three_state(boolean(property, value)?),
                (Self::Input(c), "text") => c.set_text(Some(string(property, value)?)),
                (Self::Input(c), "placeholderText") => {
                    c.set_placeholder_text(Some(string(property, value)?))
                }
                (Self::Input(c), "readOnly") => c.set_is_read_only(boolean(property, value)?),
                (Self::Input(c), "color") => {
                    use ferroui_controls::primitives::TemplatedControl;
                    if *value == Value::Undefined {
                        c.clear_value(TemplatedControl::foreground_property());
                    } else {
                        c.set_foreground(brush(property, value)?);
                    }
                }
                (
                    Self::Text(_) | Self::Button(_) | Self::Toggle(_) | Self::Input(_),
                    "font.pixelSize",
                ) => {
                    if *value == Value::Undefined {
                        control.clear_value(TextBlock::font_size_property());
                    } else {
                        control.set_value(
                            TextBlock::font_size_property(),
                            number(property, value)?.max(1.0),
                        );
                    }
                }
                (
                    Self::Text(_) | Self::Button(_) | Self::Toggle(_) | Self::Input(_),
                    "font.bold",
                ) => {
                    if *value == Value::Undefined {
                        control.clear_value(TextBlock::font_weight_property());
                    } else {
                        let weight = if boolean(property, value)? {
                            FontWeight::Bold
                        } else {
                            FontWeight::Normal
                        };
                        control.set_value(TextBlock::font_weight_property(), weight);
                    }
                }
                // A declaration local to the QML component has no corresponding native setter.
                _ => {}
            },
        }
        Ok(())
    }

    fn connect(&self, endpoint: NativeEndpoint) -> Vec<Rc<dyn IDisposable>> {
        let mut subscriptions = Vec::new();
        if let Some(button) = self.control().cast::<Button>() {
            let click = endpoint.clone();
            let weak = button.downgrade();
            let token = button.click(move |_, args| {
                if weak.upgrade().is_some_and(|button| args.is_source(&button))
                    || args.source().is_none()
                {
                    click.dispatch(|view| view.call(click.object, "click", vec![]).map(|_| ()));
                }
            });
            subscriptions.push(Disposable::create(move || {
                button.remove_handler(Button::click_event(), token);
            }));
        }
        match self {
            Self::Range(r) => subscriptions.extend(r.connect(endpoint)),
            Self::ListView(v) => subscriptions.push(v.connect(endpoint)),
            Self::Input(input) => subscriptions.push(input.property_changed(move |args| {
                if args.is_effective_value_change()
                    && args.property() == TextBox::text_property().as_property()
                {
                    endpoint.write(
                        "text",
                        Value::String(args.get_new_value::<Option<String>>().unwrap_or_default()),
                    );
                }
            })),
            Self::Toggle(check) => {
                let tri = check.cast::<CheckBox>().is_some();
                subscriptions.push(check.property_changed(move |args| {
                    if args.is_effective_value_change()
                        && args.property() == ToggleButton::is_checked_property().as_property()
                    {
                        let checked = args.get_new_value::<Option<bool>>();
                        if tri {
                            endpoint.write(
                                "checkState",
                                Value::Number(match checked {
                                    None => 1.,
                                    Some(true) => 2.,
                                    Some(false) => 0.,
                                }),
                            );
                        } else {
                            endpoint.write("checked", Value::Bool(checked.unwrap_or(false)));
                        }
                    }
                }));
            }
            _ => {}
        }
        subscriptions
    }
}

fn number(name: &str, value: &Value) -> Result<f64, Error> {
    match value {
        Value::Number(n) if n.is_finite() => Ok(*n),
        _ => Err(Error::new(format!("{name} requires a finite number"))),
    }
}

fn boolean(name: &str, value: &Value) -> Result<bool, Error> {
    match value {
        Value::Bool(v) => Ok(*v),
        _ => Err(Error::new(format!("{name} requires a boolean"))),
    }
}

fn string<'a>(name: &str, value: &'a Value) -> Result<&'a str, Error> {
    match value {
        Value::String(s) => Ok(s),
        _ => Err(Error::new(format!("{name} requires a string"))),
    }
}

fn brush(name: &str, value: &Value) -> Result<Option<Rc<dyn IBrush>>, Error> {
    Brush::parse(string(name, value)?)
        .map(Some)
        .map_err(|err| Error::new(format!("{name}: {err}")))
}
