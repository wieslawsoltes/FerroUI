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
    Rectangle,
    Row,
    Column,
    Flow,
    Text,
    Button,
    CheckBox,
    TextField,
    TextArea,
}

pub(super) fn register(registry: &mut ControlRegistry) -> Result<(), Error> {
    use Kind::*;
    for (module, types) in [
        (
            "QtQuick",
            &[
                ("Item", Item),
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
                ("TextField", TextField),
                ("TextArea", TextArea),
            ][..],
        ),
    ] {
        registry.types_mut().set_module_version(module, 6, 12);
        registry.types_mut().set_module_version(module, 2, 15);
        for &(name, kind) in types {
            registry.register(module, name, kind.definition(), move || {
                Box::new(kind.create())
            })?;
        }
    }
    Ok(())
}

impl Kind {
    fn definition(self) -> TypeDefinition {
        use Value::{Bool, List, Number, Undefined};
        let mut ty = if matches!(self, Self::Item) {
            TypeDefinition::default()
                .inherits("QtQml", "QtObject")
                .property("visible", "bool", Bool(true))
                .property("enabled", "bool", Bool(true))
                .property("width", "real", Undefined)
                .property("height", "real", Undefined)
                .property("x", "real", Number(0.0))
                .property("y", "real", Number(0.0))
                .property("clip", "bool", Bool(false))
                .property("opacity", "real", Number(1.0))
                .property("anchors.fill", "Item", Value::Null)
                .property("anchors.centerIn", "Item", Value::Null)
                .property("anchors.margins", "real", Number(0.0))
                .property("anchors.horizontalCenterOffset", "real", Number(0.0))
                .property("anchors.verticalCenterOffset", "real", Number(0.0))
                .property("data", "list<QtObject>", List(vec![]))
                .default_property("data")
        } else {
            TypeDefinition::default().inherits("QtQuick", "Item")
        };
        if matches!(
            self,
            Self::Text | Self::Button | Self::CheckBox | Self::TextField | Self::TextArea
        ) {
            ty = ty
                .property("text", "string", Value::String(String::new()))
                .property("font.pixelSize", "int", Undefined)
                .property("font.bold", "bool", Undefined);
        }
        match self {
            Self::Item => ty,
            Self::Rectangle => ty
                .property("color", "color", Value::String("white".into()))
                .property("border.color", "color", Value::String("black".into()))
                .property("border.width", "int", Number(0.0))
                .property("radius", "real", Number(0.0)),
            Self::Row | Self::Column | Self::Flow => ty.property("spacing", "real", Number(0.0)),
            Self::Text => ty.property("color", "color", Undefined),
            Self::Button => ty.signal("clicked", vec![]),
            Self::CheckBox => ty
                .signal("clicked", vec![])
                .property("checked", "bool", Bool(false)),
            Self::TextField | Self::TextArea => ty
                .property("color", "color", Undefined)
                .property("readOnly", "bool", Bool(false))
                .property("placeholderText", "string", Value::String(String::new())),
        }
    }

    fn create(self) -> Peer {
        match self {
            Self::Item => Peer::Canvas(super::anchors::AnchorCanvas::create()),
            Self::Row | Self::Column => {
                let stack = StackPanel::new();
                stack.set_orientation(if matches!(self, Self::Row) {
                    Orientation::Horizontal
                } else {
                    Orientation::Vertical
                });
                Peer::Stack(stack)
            }
            Self::Flow => Peer::Flow(WrapPanel::new()),
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
            Self::CheckBox => Peer::CheckBox(CheckBox::new()),
            Self::TextField | Self::TextArea => {
                let input = TextBox::new();
                input.set_accepts_return(matches!(self, Self::TextArea));
                Peer::Input(input)
            }
        }
    }
}

enum Peer {
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
    CheckBox(Ref<CheckBox>),
    Input(Ref<TextBox>),
}

impl NativeObject for Peer {
    fn control(&self) -> Ref<Control> {
        match self {
            Self::Canvas(c) => c.clone().upcast(),
            Self::Stack(c) => c.clone().upcast(),
            Self::Flow(c) => c.clone().upcast(),
            Self::Rectangle { root, .. } => root.clone().upcast(),
            Self::Text(c) => c.clone().upcast(),
            Self::Button(c) => c.clone().upcast(),
            Self::CheckBox(c) => c.clone().upcast(),
            Self::Input(c) => c.clone().upcast(),
        }
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
            (_, "width" | "height" | "x" | "y" | "opacity")
            | (Self::Rectangle { .. }, "border.width" | "radius")
            | (Self::Flow(_) | Self::Stack(_), "spacing")
            | (
                Self::Text(_) | Self::Button(_) | Self::Input(_) | Self::CheckBox(_),
                "font.pixelSize",
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
            "visible" => control.set_is_visible(boolean(property, value)?),
            "enabled" => control.set_is_enabled(boolean(property, value)?),
            "opacity" => control.set_opacity(number(property, value)?.clamp(0.0, 1.0)),
            "clip" => control.set_clip_to_bounds(boolean(property, value)?),
            _ => match (self, property) {
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
                (Self::CheckBox(c), "text") => {
                    c.set_content(Some(Rc::new(string(property, value)?.to_owned())))
                }
                (Self::CheckBox(c), "checked") => c.set_is_checked(Some(boolean(property, value)?)),
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
                    Self::Text(_) | Self::Button(_) | Self::CheckBox(_) | Self::Input(_),
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
                    Self::Text(_) | Self::Button(_) | Self::CheckBox(_) | Self::Input(_),
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
            let token = button.click(move |_, _| click.emit("clicked", vec![]));
            subscriptions.push(Disposable::create(move || {
                button.remove_handler(Button::click_event(), token);
            }));
        }
        match self {
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
            Self::CheckBox(check) => subscriptions.push(check.property_changed(move |args| {
                if args.is_effective_value_change()
                    && args.property() == ToggleButton::is_checked_property().as_property()
                {
                    endpoint.write(
                        "checked",
                        Value::Bool(args.get_new_value::<Option<bool>>().unwrap_or(false)),
                    );
                }
            })),
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
