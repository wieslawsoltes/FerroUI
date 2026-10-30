//! Small native fields with explicit commit and cancellation. A compound value is staged
//! locally; completing its fields never produces partially authored source.
use super::{PropertyEditorContext, PropertyEditorKind, PropertyEditorMetadata, PropertyValueEdit};
use ferroui_base::input::{InputElement, Key};
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::{LocatorExtensions, Ref, Thickness};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::{
    Button, ColumnDefinitions, ComboBox, Control, DataValidationErrors, Grid, RowDefinitions,
    TextBlock, TextBox, ToolTip,
};
use ferroui_designer_tools::PropertyValueShape;
use std::cell::Cell;
use std::rc::Rc;

fn result(field: &Control, value: Result<(), String>) {
    match value {
        Ok(()) => DataValidationErrors::clear_errors(field),
        Err(error) => DataValidationErrors::set_errors(field, Some(vec![Rc::new(error)])),
    }
}

pub(super) fn text(
    context: &PropertyEditorContext,
    metadata: &PropertyEditorMetadata,
) -> Ref<Control> {
    let field = TextBox::new();
    field.classes().add("designer-property-field");
    field.classes().set(
        "property-expression",
        context
            .value
            .as_deref()
            .is_some_and(super::metadata::is_expression),
    );
    field.set_text(context.value.as_deref());
    field.set_placeholder_text(Some(if context.mixed {
        "Multiple values"
    } else {
        "Inherited"
    }));
    field.set_min_height(26.0);
    field.set_font_size(12.0);
    let hint = match metadata.kind {
        PropertyEditorKind::TimeSpan | PropertyEditorKind::Duration => {
            "hh:mm:ss.fffffff · Enter applies · Escape restores"
        }
        PropertyEditorKind::FontFamily => {
            "Font family, fallback family · Enter applies · Escape restores"
        }
        PropertyEditorKind::Object => "Use the value editor to edit an object or expression",
        _ => "Enter applies · Escape restores",
    };
    ToolTip::set_tip(&field, Some(Rc::new(hint.to_owned())));
    let weak = field.downgrade();
    let context = context.clone();
    let metadata = metadata.clone();
    field.add_handler(InputElement::key_down_event(), move |_, event| {
        let Some(field) = weak.upgrade() else { return };
        match event.key {
            Key::Enter => {
                let value = field.text().unwrap_or_default();
                let edit = if super::metadata::is_expression(&value) {
                    PropertyValueEdit::Expression(value)
                } else {
                    PropertyValueEdit::Literal(value)
                };
                result(&field, metadata.apply(&context, edit));
            }
            Key::Escape => {
                field.set_text(context.value.as_deref());
                DataValidationErrors::clear_errors(&field);
            }
            _ => return,
        }
        event.set_handled(true);
    });
    field.upcast::<Control>()
}

/// Labeled Point/Size/Vector/Rect/Matrix and relative-value editor. Enter commits all
/// components once. Mixed fields remain empty until every component has been supplied.
pub struct PropertyVectorEditor {
    root: Ref<Grid>,
    fields: Vec<Ref<TextBox>>,
    units: Option<Ref<ComboBox>>,
}
impl PropertyVectorEditor {
    pub fn new(
        context: &PropertyEditorContext,
        metadata: &PropertyEditorMetadata,
        shape: PropertyValueShape,
    ) -> Self {
        let root = Grid::new();
        root.classes().add("property-vector-editor");
        let count = shape.components().len();
        if count == 0 {
            root.children().add(text(context, metadata));
            return Self {
                root,
                fields: Vec::new(),
                units: None,
            };
        }
        let columns = if count == 6 { 2 } else { count.min(2) };
        let rows = count.div_ceil(columns);
        root.set_column_definitions(
            ColumnDefinitions::parse(if shape.relative() { "*,*,42" } else { "*,*" }).unwrap(),
        );
        root.set_row_definitions(RowDefinitions::parse(&vec!["Auto"; rows].join(",")).unwrap());
        let parsed = (!context.mixed)
            .then(|| {
                context
                    .value
                    .as_deref()
                    .and_then(|value| shape.parse_components(value).ok())
            })
            .flatten();
        let mut fields = Vec::new();
        for (index, label) in shape.components().iter().enumerate() {
            let pair = Grid::new();
            pair.set_column_definitions(
                ColumnDefinitions::parse(if label.len() > 2 { "29,*" } else { "21,*" }).unwrap(),
            );
            pair.set_margin(Thickness::new(
                0.0,
                0.0,
                3.0,
                if rows > 1 { 2.0 } else { 0.0 },
            ));
            let caption = TextBlock::new();
            caption.set_text(Some(label));
            caption.set_font_size(9.0);
            caption.set_opacity(0.7);
            caption.set_vertical_alignment(VerticalAlignment::Center);
            pair.children().add(caption.upcast::<Control>());
            let field = TextBox::new();
            field.classes().add("designer-property-field");
            field.set_font_size(11.0);
            field.set_min_width(0.0);
            field.set_min_height(26.0);
            field.set_padding(Thickness::new(4.0, 3.0, 3.0, 3.0));
            field.set_text(
                parsed
                    .as_ref()
                    .map(|(values, _)| values[index].to_string())
                    .as_deref(),
            );
            field.set_placeholder_text(Some(if context.mixed { "—" } else { "Auto" }));
            AutomationProperties::set_name(
                &field,
                Some(&format!("{} {}", context.property, label)),
            );
            Grid::set_column(&field, 1);
            pair.children().add(field.clone().upcast::<Control>());
            Grid::set_column(&pair, (index % columns) as i32);
            Grid::set_row(&pair, (index / columns) as i32);
            root.children().add(pair.upcast::<Control>());
            fields.push(field);
        }
        let units = shape.relative().then(|| {
            let units = ComboBox::new();
            units.classes().add("designer-property-field");
            for unit in ["px", "%"] {
                units.items().add(Some(Rc::new(unit.to_owned())));
            }
            units.set_selected_index(if parsed.as_ref().is_some_and(|(_, percent)| *percent) {
                1
            } else {
                0
            });
            units.set_min_width(0.0);
            AutomationProperties::set_name(&units, Some("Coordinate units"));
            Grid::set_column(&units, 2);
            Grid::set_row_span(&units, rows as i32);
            root.children().add(units.clone().upcast::<Control>());
            units
        });
        for field in &fields {
            let weak = field.downgrade();
            let fields = fields.iter().map(Ref::downgrade).collect::<Vec<_>>();
            let units = units.as_ref().map(Ref::downgrade);
            let metadata = metadata.clone();
            let context = context.clone();
            let original = parsed.clone();
            field.add_handler(InputElement::key_down_event(), move |_, event| {
                let Some(field) = weak.upgrade() else { return };
                match event.key {
                    Key::Escape => {
                        for (index, target) in fields.iter().enumerate() {
                            if let Some(target) = target.upgrade() {
                                target.set_text(
                                    original
                                        .as_ref()
                                        .map(|(values, _)| values[index].to_string())
                                        .as_deref(),
                                );
                                DataValidationErrors::clear_errors(&target);
                            }
                        }
                        if let Some(units) = units.as_ref().and_then(|value| value.upgrade()) {
                            units.set_selected_index(
                                if original.as_ref().is_some_and(|(_, percent)| *percent) {
                                    1
                                } else {
                                    0
                                },
                            );
                        }
                    }
                    Key::Enter => {
                        let percent = units
                            .as_ref()
                            .and_then(|value| value.upgrade())
                            .is_some_and(|units| units.selected_index() == 1);
                        let values = fields
                            .iter()
                            .map(|field| {
                                field
                                    .upgrade()
                                    .and_then(|field| field.text())
                                    .and_then(|text| text.trim().parse::<f64>().ok())
                                    .filter(|n| n.is_finite())
                            })
                            .collect::<Option<Vec<_>>>();
                        let value = values
                            .ok_or_else(|| {
                                "Complete every component with a finite number".to_owned()
                            })
                            .and_then(|values| {
                                shape
                                    .format_components(&values, percent)
                                    .map_err(|error| error.to_string())
                            });
                        result(
                            &field,
                            value.and_then(|value| {
                                metadata.apply(&context, PropertyValueEdit::Literal(value))
                            }),
                        );
                    }
                    _ => return,
                }
                event.set_handled(true);
            });
        }
        Self {
            root,
            fields,
            units,
        }
    }
    pub fn control(&self) -> Ref<Control> {
        self.root.clone().upcast::<Control>()
    }
    pub fn fields(&self) -> &[Ref<TextBox>] {
        &self.fields
    }
    pub fn units(&self) -> Option<Ref<ComboBox>> {
        self.units.clone()
    }
}

pub(super) fn enumeration(
    context: &PropertyEditorContext,
    metadata: &PropertyEditorMetadata,
    values: &[String],
) -> Ref<Control> {
    let field = ComboBox::new();
    field.classes().add("designer-property-field");
    field.set_min_width(0.0);
    let mut choices = vec![if context.mixed {
        "Multiple values".to_owned()
    } else {
        "Inherited".to_owned()
    }];
    choices.extend_from_slice(values);
    // Unknown authored values are shown verbatim, never replaced by the first enum item.
    let selected = context
        .value
        .as_deref()
        .and_then(|value| {
            choices
                .iter()
                .position(|choice| choice.eq_ignore_ascii_case(value))
        })
        .or_else(|| {
            context.value.as_ref().map(|value| {
                choices.push(value.clone());
                choices.len() - 1
            })
        })
        .unwrap_or(0);
    for choice in &choices {
        field.items().add(Some(Rc::new(choice.clone())));
    }
    field.set_selected_index(selected as i32);
    let weak = field.downgrade();
    let context = context.clone();
    let metadata = metadata.clone();
    field.selection_changed(move |_, _| {
        let Some(field) = weak.upgrade() else { return };
        let index = field.selected_index();
        if index == 0 {
            if !context.mixed {
                result(&field, metadata.apply(&context, PropertyValueEdit::Reset));
            }
            return;
        }
        if let Some(value) = choices.get(index as usize) {
            result(
                &field,
                metadata.apply(&context, PropertyValueEdit::Literal(value.clone())),
            );
        }
    });
    field.upcast::<Control>()
}

pub(super) fn grid_length(
    context: &PropertyEditorContext,
    metadata: &PropertyEditorMetadata,
) -> Ref<Control> {
    let root = Grid::new();
    root.set_column_definitions(ColumnDefinitions::parse("*,62").unwrap());
    let value = TextBox::new();
    value.classes().add("designer-property-field");
    let current = context.value.as_deref().unwrap_or("");
    value
        .set_text((!current.eq_ignore_ascii_case("Auto")).then_some(current.trim_end_matches('*')));
    value.set_placeholder_text(Some(if context.mixed { "Mixed" } else { "Value" }));
    let unit = ComboBox::new();
    unit.classes().add("designer-property-field");
    for choice in ["px", "*", "Auto"] {
        unit.items().add(Some(Rc::new(choice.to_owned())));
    }
    unit.set_selected_index(if current.eq_ignore_ascii_case("Auto") {
        2
    } else if current.ends_with('*') {
        1
    } else {
        0
    });
    let original_text = value.text();
    let original_unit = unit.selected_index();
    let restoring = Rc::new(Cell::new(false));
    value.set_is_enabled(unit.selected_index() != 2);
    Grid::set_column(&unit, 1);
    root.children().add(value.clone().upcast::<Control>());
    root.children().add(unit.clone().upcast::<Control>());
    let weak_value = value.downgrade();
    let weak_unit = unit.downgrade();
    let context = context.clone();
    let metadata = metadata.clone();
    let commit = Rc::new(move || {
        let Some((value, unit)) = weak_value.upgrade().zip(weak_unit.upgrade()) else {
            return;
        };
        let text = value.text().unwrap_or_default();
        let source = match unit.selected_index() {
            2 => "Auto".to_owned(),
            1 => format!("{}*", if text.is_empty() { "1" } else { text.trim() }),
            _ => text,
        };
        result(
            &value,
            metadata.apply(&context, PropertyValueEdit::Literal(source)),
        );
    });
    let accept = commit.clone();
    value.add_handler(InputElement::key_down_event(), move |_, event| {
        if event.key == Key::Enter {
            accept();
            event.set_handled(true);
        }
    });
    let weak_value = value.downgrade();
    let weak_unit = unit.downgrade();
    let restore_guard = restoring.clone();
    unit.selection_changed(move |_, _| {
        if let Some((value, unit)) = weak_value.upgrade().zip(weak_unit.upgrade()) {
            value.set_is_enabled(unit.selected_index() != 2);
            if !restore_guard.get() {
                commit();
            }
        }
    });
    let weak_value = value.downgrade();
    let weak_unit = unit.downgrade();
    root.add_handler(InputElement::key_down_event(), move |_, event| {
        if event.key == Key::Escape {
            if let Some((value, unit)) = weak_value.upgrade().zip(weak_unit.upgrade()) {
                restoring.set(true);
                value.set_text(original_text.as_deref());
                unit.set_selected_index(original_unit);
                restoring.set(false);
                DataValidationErrors::clear_errors(&value);
            }
            event.set_handled(true);
        }
    });
    root.upcast::<Control>()
}

pub(super) fn font(
    context: &PropertyEditorContext,
    metadata: &PropertyEditorMetadata,
) -> Ref<Control> {
    let root = Grid::new();
    root.set_column_definitions(ColumnDefinitions::parse("*,26").unwrap());
    root.children().add(text(context, metadata));
    let chooser = Button::new();
    chooser.set_content(Some(Rc::new("Aa".to_owned())));
    chooser.set_font_size(10.0);
    chooser.set_padding(Thickness::uniform(2.0));
    chooser.set_min_width(0.0);
    AutomationProperties::set_name(
        &chooser,
        Some(if metadata.kind == PropertyEditorKind::FontWeight {
            "Choose a font weight"
        } else {
            "Choose a font family"
        }),
    );
    let flyout = ferroui_controls::Flyout::new();
    flyout.flyout_presenter_classes().add("designer-chrome");
    chooser.set_flyout(Some(flyout.clone().upcast()));
    let weak = flyout.downgrade();
    let context = context.clone();
    let metadata = metadata.clone();
    flyout.opening(move |_| {
        let Some(flyout) = weak.upgrade() else { return };
        let manager =
            ferroui_base::FerroLocator::current().get_service::<ferroui_base::media::FontManager>();
        let mut families = manager
            .map(|manager| {
                manager
                    .system_fonts()
                    .font_families()
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        families.sort_by_key(|name| name.to_lowercase());
        families.dedup();
        if metadata.kind == PropertyEditorKind::FontWeight {
            families = [
                "Thin",
                "ExtraLight",
                "Light",
                "SemiLight",
                "Normal",
                "Medium",
                "SemiBold",
                "Bold",
                "ExtraBold",
                "Black",
                "ExtraBlack",
            ]
            .map(str::to_owned)
            .to_vec();
        } else if families.is_empty() {
            families = ["Segoe UI", "Arial", "sans-serif", "serif", "monospace"]
                .map(str::to_owned)
                .to_vec();
        }
        let field = enumeration(&context, &metadata, &families);
        field.set_width(280.0);
        flyout.set_content(Some(Control::boxed(&field)));
    });
    Grid::set_column(&chooser, 1);
    root.children().add(chooser.upcast::<Control>());
    root.upcast::<Control>()
}
