use ferroui_base::Ref;
use ferroui_controls::{Border, Button, ComboBox, Control, TextBox};
use ferroui_designer_tools::token_handoff::{
    Conversion, Direction, Options, Plan, TransitionMapping,
};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    fmt::Write,
    rc::Rc,
};

const COLORS: &[(&str, &str)] = &[("Solid color brush", "SolidColorBrush"), ("Color", "Color")];
const GRADIENTS: &[(&str, &str)] = &[
    ("Linear gradient", "LinearGradientBrush"),
    ("Radial gradient", "RadialGradientBrush"),
    ("Conic gradient", "ConicGradientBrush"),
];
const NUMBERS: &[(&str, &str)] = &[("Number", "number"), ("Logical pixels", "dimension")];

struct Row {
    path: String,
    target: String,
    representations: &'static [(&'static str, &'static str)],
    representation: String,
    transition: Option<TransitionMapping>,
}

pub(super) struct ReviewOptions {
    pub panel: Ref<Border>,
    pub submit: Ref<Button>,
    entry: Ref<ComboBox>,
    mapping: Ref<TextBox>,
    representation: Ref<ComboBox>,
    transition: Ref<TextBox>,
    scale: Ref<TextBox>,
    rows: RefCell<Vec<Row>>,
    populating: Cell<bool>,
}

impl ReviewOptions {
    pub fn new(root: &Ref<Control>) -> Rc<Self> {
        Rc::new(Self {
            panel: root.get_control("MappingOptions"),
            submit: root.get_control("ReviewMapping"),
            entry: root.get_control("TokenEntry"),
            mapping: root.get_control("TokenMapping"),
            representation: root.get_control("TokenRepresentation"),
            transition: root.get_control("TokenTransitionProperty"),
            scale: root.get_control("TokenScale"),
            rows: RefCell::new(Vec::new()),
            populating: Cell::new(false),
        })
    }

    pub fn connect(self: &Rc<Self>, edit: impl Fn() + 'static) {
        let edit = Rc::new(edit);
        let options = Rc::downgrade(self);
        self.entry.selection_changed(move |_, _| {
            if let Some(options) = options
                .upgrade()
                .filter(|options| !options.populating.get())
            {
                options.show_selected();
            }
        });
        for field in [&self.mapping, &self.scale, &self.transition] {
            let options = Rc::downgrade(self);
            let edit = edit.clone();
            field.text_changing(move |_, _| {
                if options
                    .upgrade()
                    .is_some_and(|options| !options.populating.get())
                {
                    edit();
                }
            });
        }
        let options = Rc::downgrade(self);
        self.representation.selection_changed(move |_, _| {
            if options
                .upgrade()
                .is_some_and(|options| !options.populating.get())
            {
                edit();
            }
        });
    }

    pub fn show(&self, direction: Direction, options: &Options, plan: &Plan) {
        let previous = self
            .rows
            .borrow()
            .get(self.entry.selected_index() as usize)
            .map(|row| row.path.clone());
        let mut rows = BTreeMap::new();
        for row in &plan.mappings {
            let (path, target, representations, representation) = match direction {
                Direction::Import => (
                    &row.token,
                    &row.key,
                    import_representations(row),
                    &row.xaml_type,
                ),
                Direction::Export => (
                    &row.key,
                    &row.token,
                    if row.xaml_type == "x:Double" {
                        NUMBERS
                    } else {
                        &[]
                    },
                    &row.kind,
                ),
            };
            rows.insert(
                path.clone(),
                Row {
                    path: path.clone(),
                    target: target.clone(),
                    representations,
                    representation: representation.clone(),
                    transition: (direction == Direction::Import)
                        .then(|| row.transition.clone())
                        .flatten(),
                },
            );
        }
        for issue in plan.issues.iter().filter(|issue| !issue.path.is_empty()) {
            let mapping = options.mappings.get(&issue.path);
            let target = mapping.and_then(|mapping| match direction {
                Direction::Import => mapping.key.clone(),
                Direction::Export => mapping.token.clone(),
            });
            rows.entry(issue.path.clone()).or_insert_with(|| Row {
                path: issue.path.clone(),
                target: target.unwrap_or_default(),
                representations: &[],
                representation: String::new(),
                transition: (direction == Direction::Import
                    && issue.kind.as_deref() == Some("transition"))
                .then(|| {
                    mapping
                        .and_then(|mapping| mapping.transition.clone())
                        .unwrap_or_default()
                }),
            });
        }
        self.populating.set(true);
        self.entry.items().clear();
        let rows: Vec<_> = rows.into_values().collect();
        for row in &rows {
            self.entry.items().add(Some(Rc::new(row.path.clone())));
        }
        let index = previous
            .and_then(|previous| rows.iter().position(|row| row.path == previous))
            .unwrap_or(0);
        self.entry
            .set_selected_index(if rows.is_empty() { -1 } else { index as i32 });
        self.mapping.set_placeholder_text(Some(match direction {
            Direction::Import => "XAML resource key",
            Direction::Export => "Dotted token path",
        }));
        self.scale.set_is_visible(direction == Direction::Import);
        self.scale.set_text(
            options
                .pixels_per_rem
                .map(|value| value.to_string())
                .as_deref(),
        );
        self.rows.replace(rows);
        self.populating.set(false);
        self.show_selected();
        self.panel.set_is_visible(true);
    }

    fn show_selected(&self) {
        self.populating.set(true);
        self.representation.items().clear();
        let rows = self.rows.borrow();
        let row = rows.get(self.entry.selected_index() as usize);
        self.mapping.set_is_enabled(row.is_some());
        self.mapping.set_text(row.map(|row| row.target.as_str()));
        if let Some(row) = row {
            for (label, _) in row.representations {
                self.representation
                    .items()
                    .add(Some(Rc::new((*label).to_owned())));
            }
            let index = row
                .representations
                .iter()
                .position(|(_, value)| *value == row.representation)
                .map_or(-1, |index| index as i32);
            self.representation.set_selected_index(index);
        }
        self.representation
            .set_is_visible(row.is_some_and(|row| !row.representations.is_empty()));
        let transition = row.and_then(|row| row.transition.as_ref());
        self.transition.set_is_visible(transition.is_some());
        self.transition
            .set_text(transition.map(|value| value.property.as_str()));
        self.populating.set(false);
    }

    pub fn edited_options(
        &self,
        direction: Direction,
        current: &Options,
    ) -> Result<Options, String> {
        let mut options = current.clone();
        if direction == Direction::Import {
            let scale = self.scale.text().unwrap_or_default();
            options.pixels_per_rem = if scale.trim().is_empty() {
                None
            } else {
                Some(
                    scale
                        .trim()
                        .parse::<f64>()
                        .ok()
                        .filter(|scale| scale.is_finite() && *scale > 0.0)
                        .ok_or("Logical pixels per rem must be positive and finite")?,
                )
            };
        }
        let rows = self.rows.borrow();
        let Some(row) = rows.get(self.entry.selected_index() as usize) else {
            return Ok(options);
        };
        let target = self.mapping.text().unwrap_or_default();
        let target = target.trim();
        let representation = row
            .representations
            .get(self.representation.selected_index() as usize)
            .map(|(_, value)| *value);
        let transition = self.transition.text().unwrap_or_default();
        let transition = transition.trim();
        if target == row.target
            && representation.is_none_or(|value| value == row.representation)
            && row
                .transition
                .as_ref()
                .is_none_or(|value| value.property == transition)
        {
            return Ok(options);
        }
        let mapping = options.mappings.entry(row.path.clone()).or_default();
        let target = (!target.is_empty()).then(|| target.to_owned());
        match direction {
            Direction::Import => {
                mapping.key = target;
                mapping.xaml_type = representation.map(str::to_owned);
                if let Some(current) = &row.transition {
                    mapping.transition = (!transition.is_empty()).then(|| {
                        if current.property == transition {
                            current.clone()
                        } else {
                            TransitionMapping {
                                property: transition.into(),
                                ..Default::default()
                            }
                        }
                    });
                }
            }
            Direction::Export => {
                mapping.token = target;
                mapping.kind = representation.map(str::to_owned);
            }
        }
        Ok(options)
    }
}

fn import_representations(row: &Conversion) -> &'static [(&'static str, &'static str)] {
    match row.kind.as_str() {
        "color" => COLORS,
        "gradient" => GRADIENTS,
        _ => &[],
    }
}

pub(super) fn mapping_report(plan: &Plan) -> String {
    let mut text = String::new();
    for row in &plan.mappings {
        let _ = writeln!(
            text,
            "{} → {}\n{} · {}\n{}",
            row.token, row.key, row.kind, row.xaml_type, row.value
        );
        if let Some(note) = &row.note {
            let _ = writeln!(text, "{note}");
        }
        text.push('\n');
    }
    for issue in &plan.issues {
        let title = if issue.path.is_empty() {
            "Document"
        } else {
            &issue.path
        };
        let _ = writeln!(text, "{title}\n{}\n", issue.message);
    }
    text.trim_end().into()
}
