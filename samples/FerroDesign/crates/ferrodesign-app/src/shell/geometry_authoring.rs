//! Native affine and frame authoring. Existing document transforms remain
//! editable matrices; no geometry is flattened by these controls.
use super::*;
use ferrodesign_core::{Constraint, Transform};

const MATRIX_FIELDS: [&str; 6] = [
    "AffineA", "AffineB", "AffineC", "AffineD", "AffineX", "AffineY",
];
const PRESETS: &[(&str, &str, f64, f64)] = &[
    ("Phone", "Compact phone", 320.0, 568.0),
    ("Phone", "Phone", 390.0, 844.0),
    ("Phone", "Large phone", 430.0, 932.0),
    ("Tablet", "Tablet", 768.0, 1024.0),
    ("Tablet", "Large tablet", 1024.0, 1366.0),
    ("Desktop", "Laptop", 1440.0, 900.0),
    ("Desktop", "Desktop", 1920.0, 1080.0),
    ("Desktop", "Wide desktop", 2560.0, 1440.0),
    ("Presentation", "Presentation 16:9", 1920.0, 1080.0),
    ("Presentation", "Presentation 4:3", 1024.0, 768.0),
    ("Social", "Square", 1080.0, 1080.0),
    ("Social", "Portrait", 1080.0, 1350.0),
    ("Social", "Story", 1080.0, 1920.0),
];

impl EditorShell {
    pub(super) fn wire_geometry_authoring(self: &Rc<Self>) {
        self.hook("AffineButton", "affine-transform");
        self.hook("FramePresetsButton", "frame-presets");
        self.hook("PageFramePresetsButton", "frame-presets");
        for (name, horizontal, end) in [
            ("ConstraintLeft", true, false),
            ("ConstraintRight", true, true),
            ("ConstraintTop", false, false),
            ("ConstraintBottom", false, true),
        ] {
            let weak = Rc::downgrade(self);
            self.button(name).click(move |_, _| {
                let Some(shell) = weak.upgrade() else {
                    return;
                };
                shell.modify("Change constraint", |node| {
                    let value = if horizontal {
                        &mut node.constraints.horizontal
                    } else {
                        &mut node.constraints.vertical
                    };
                    *value = toggle_edge(*value, end);
                    node.overridden_properties.insert("constraints".into());
                });
                shell.changed();
            });
        }
        for (name, horizontal) in [("ConstraintCenterX", true), ("ConstraintCenterY", false)] {
            let weak = Rc::downgrade(self);
            self.button(name).click(move |_, _| {
                let Some(shell) = weak.upgrade() else {
                    return;
                };
                shell.modify("Center constraint", |node| {
                    if horizontal {
                        node.constraints.horizontal = Constraint::Center;
                    } else {
                        node.constraints.vertical = Constraint::Center;
                    }
                    node.overridden_properties.insert("constraints".into());
                });
                shell.changed();
            });
        }
    }

    pub(super) fn refresh_constraint_diagram(&self, node: Option<&Node>) {
        for (name, active) in [
            (
                "ConstraintLeft",
                node.is_some_and(|n| {
                    matches!(
                        n.constraints.horizontal,
                        Constraint::Start | Constraint::Stretch
                    )
                }),
            ),
            (
                "ConstraintRight",
                node.is_some_and(|n| {
                    matches!(
                        n.constraints.horizontal,
                        Constraint::End | Constraint::Stretch
                    )
                }),
            ),
            (
                "ConstraintTop",
                node.is_some_and(|n| {
                    matches!(
                        n.constraints.vertical,
                        Constraint::Start | Constraint::Stretch
                    )
                }),
            ),
            (
                "ConstraintBottom",
                node.is_some_and(|n| {
                    matches!(
                        n.constraints.vertical,
                        Constraint::End | Constraint::Stretch
                    )
                }),
            ),
            (
                "ConstraintCenterX",
                node.is_some_and(|n| n.constraints.horizontal == Constraint::Center),
            ),
            (
                "ConstraintCenterY",
                node.is_some_and(|n| n.constraints.vertical == Constraint::Center),
            ),
        ] {
            self.button(name).classes().set("active", active);
            self.button(name).set_is_enabled(node.is_some());
        }
    }

    pub(super) fn frame_presets_dialog(self: &Rc<Self>) {
        self.modal("Frame presets", "Create an editable frame at the center of the canvas. All dimensions are design pixels.");
        let host = self.control::<StackPanel>("ModalContent");
        let mut category = "";
        for (index, &(group, name, width, height)) in PRESETS.iter().enumerate() {
            if category != group {
                category = group;
                host.children().add(geometry_label(group));
            }
            let button = self.geometry_button(
                &format!("{name}   {} × {}", number(width), number(height)),
                &format!("FramePreset{index}"),
                move |shell| shell.create_preset_frame(name, width, height),
            );
            button
                .set_horizontal_content_alignment(ferroui_base::layout::HorizontalAlignment::Left);
            host.children().add(button);
        }
        self.modal_button("Done", "close-modal");
    }

    fn create_preset_frame(self: &Rc<Self>, name: &str, width: f64, height: f64) {
        let viewport = self.canvas.viewport.get();
        let center =
            viewport.screen_to_world(Point::new(viewport.width / 2.0, viewport.height / 2.0));
        let mut node = Node::new(
            NodeKind::Frame,
            name,
            center.x - width / 2.0,
            center.y - height / 2.0,
            width,
            height,
        );
        node.clip_content = true;
        node.fills = vec![Fill::solid(Color::WHITE)];
        let result = {
            let mut editor = self.editor.borrow_mut();
            let page = editor.active_page;
            editor.insert_node(page, node)
        };
        match result {
            Ok(_) => {
                self.close_modal();
                self.changed();
                self.canvas.zoom_to_selection();
            }
            Err(error) => self.toast(&error.to_string()),
        }
    }

    pub(super) fn affine_dialog(self: &Rc<Self>) {
        let node = {
            let editor = self.editor.borrow();
            (editor.selection.len() == 1)
                .then(|| editor.selection[0])
                .and_then(|id| editor.document.nodes.get(&id).cloned())
        };
        let Some(node) = node else {
            self.toast("Select one layer to edit its affine transform.");
            return;
        };
        let enabled = !self.editor.borrow().document.effective_locked(node.id);
        let transform = node.local_transform();
        self.modal("Affine transform", "Edit the complete transform relative to this layer's parent, or apply a centered skew. Changes retain vector geometry, children and one-step undo.");
        let host = self.control::<StackPanel>("ModalContent");
        let grid = Grid::new();
        grid.set_column_definitions(ColumnDefinitions::parse("*,12,*").unwrap());
        let left = StackPanel::new();
        left.set_spacing(8.0);
        let right = StackPanel::new();
        right.set_spacing(8.0);
        let values = [
            transform.a,
            transform.b,
            transform.c,
            transform.d,
            transform.e,
            transform.f,
        ];
        let fields: Vec<_> = MATRIX_FIELDS
            .iter()
            .zip(values)
            .map(|(name, value)| {
                let field = TextBox::new();
                field.set_name(Some((*name).into()));
                field.set_text(Some(&value.to_string()));
                field.set_is_enabled(enabled);
                field
            })
            .collect();
        for (index, label) in [
            "A · horizontal scale",
            "B · vertical shear",
            "C · horizontal shear",
            "D · vertical scale",
            "X · translation",
            "Y · translation",
        ]
        .iter()
        .enumerate()
        {
            let column = if index % 2 == 0 { &left } else { &right };
            column.children().add(geometry_label(label));
            column.children().add(fields[index].clone());
        }
        grid.children().add(left);
        Grid::set_column(&right, 2);
        grid.children().add(right);
        host.children().add(grid);
        let error = geometry_label("");
        error.set_name(Some("AffineError".into()));
        error.set_foreground(Some(icons::brush("#C8372B")));
        error.set_is_visible(false);
        host.children().add(error.clone());
        let committed: Vec<_> = fields
            .iter()
            .map(|field| Rc::new(RefCell::new(field.text().unwrap_or_default())))
            .collect();
        let weak_fields: Vec<_> = fields.iter().map(Ref::downgrade).collect();
        let previous = committed.clone();
        let weak_shell = Rc::downgrade(self);
        let weak_error = error.downgrade();
        let id = node.id;
        let commit: Rc<dyn Fn()> = Rc::new(move || {
            let (Some(shell), Some(error)) = (weak_shell.upgrade(), weak_error.upgrade()) else {
                return;
            };
            let Some(fields) = weak_fields
                .iter()
                .map(WeakRef::upgrade)
                .collect::<Option<Vec<_>>>()
            else {
                return;
            };
            if shell.updating.get() || fields.iter().any(|field| field.is_composing_text()) {
                return;
            }
            let values: Vec<_> = fields
                .iter()
                .map(|field| field.text().unwrap_or_default())
                .collect();
            if values.iter().zip(&previous).all(|(a, b)| *a == *b.borrow()) {
                return;
            }
            let parsed = values
                .iter()
                .map(|value| {
                    value
                        .trim()
                        .parse::<f64>()
                        .map_err(|_| "Enter six finite matrix values.".to_string())
                })
                .collect::<Result<Vec<_>, _>>()
                .and_then(|v| {
                    validate_affine(Transform {
                        a: v[0],
                        b: v[1],
                        c: v[2],
                        d: v[3],
                        e: v[4],
                        f: v[5],
                    })
                });
            let result = parsed
                .and_then(|transform| apply_affine(&mut shell.editor.borrow_mut(), id, transform));
            match result {
                Ok(changed) => {
                    for (previous, value) in previous.iter().zip(values) {
                        *previous.borrow_mut() = value;
                    }
                    error.set_is_visible(false);
                    if changed {
                        shell.changed();
                    }
                }
                Err(message) => {
                    error.set_text(Some(&message));
                    error.set_is_visible(true);
                }
            }
            shell.sync_document_draft_dirty();
        });
        for (field, previous) in fields.iter().zip(committed) {
            let blur = commit.clone();
            field.add_handler(InputElement::lost_focus_event(), move |_, _| blur());
            let enter = commit.clone();
            field.add_handler(InputElement::key_down_event(), move |_, event| {
                if event.key == Key::Enter {
                    enter();
                    event.set_handled(true);
                }
            });
            self.bind_inspector_draft(field, previous, commit.clone(), Some(&error));
        }
        host.children().add(geometry_label(
            "Centered skew · positive or negative degrees",
        ));
        let angle = TextBox::new();
        angle.set_name(Some("AffineSkewAngle".into()));
        angle.set_text(Some("15"));
        host.children().add(angle.clone());
        let row = StackPanel::new();
        row.set_orientation(Orientation::Horizontal);
        row.set_spacing(8.0);
        for (caption, name, horizontal) in [
            ("Skew horizontally", "AffineSkewX", true),
            ("Skew vertically", "AffineSkewY", false),
        ] {
            let angle = angle.downgrade();
            let button = self.geometry_button(caption, name, move |shell| {
                if !shell.flush_inspector_drafts() {
                    return;
                }
                let degrees = angle
                    .upgrade()
                    .and_then(|field| field.text())
                    .and_then(|text| text.parse::<f64>().ok());
                let Some(degrees) = degrees.filter(|angle| angle.is_finite() && angle.abs() < 89.0)
                else {
                    shell.toast("Choose a skew angle between −89 and 89 degrees.");
                    return;
                };
                let result = {
                    let mut editor = shell.editor.borrow_mut();
                    let Some(node) = editor.document.nodes.get(&id) else {
                        return;
                    };
                    let center = Point::new(node.width / 2.0, node.height / 2.0);
                    let shear = degrees.to_radians().tan();
                    let skew = Transform {
                        b: if horizontal { 0.0 } else { shear },
                        c: if horizontal { shear } else { 0.0 },
                        ..Transform::IDENTITY
                    };
                    let transformed = node.local_transform()
                        * Transform::translation(center.x, center.y)
                        * skew
                        * Transform::translation(-center.x, -center.y);
                    apply_affine(&mut editor, id, transformed)
                };
                match result {
                    Ok(changed) => {
                        if changed {
                            shell.changed();
                        }
                        shell.affine_dialog();
                    }
                    Err(error) => shell.toast(&error),
                }
            });
            button.set_is_enabled(enabled);
            row.children().add(button);
        }
        host.children().add(row);
        self.modal_button("Done", "close-modal");
    }

    fn geometry_button(
        self: &Rc<Self>,
        caption: &str,
        name: &str,
        action: impl Fn(&Rc<Self>) + 'static,
    ) -> Ref<Button> {
        let button = Button::new();
        button.set_name(Some(name.into()));
        button.set_content(Some(Rc::new(caption.to_string())));
        let weak = Rc::downgrade(self);
        button.click(move |_, _| {
            if let Some(shell) = weak.upgrade() {
                action(&shell);
            }
        });
        button
    }
}

fn geometry_label(value: &str) -> Ref<TextBlock> {
    let label = TextBlock::new();
    label.set_text(Some(value));
    label.set_font_size(11.0);
    label.set_text_wrapping(TextWrapping::Wrap);
    label
}
fn toggle_edge(value: Constraint, end: bool) -> Constraint {
    match (value, end) {
        (Constraint::Stretch, false) => Constraint::End,
        (Constraint::Stretch, true) => Constraint::Start,
        (Constraint::Start, true) | (Constraint::End, false) => Constraint::Stretch,
        (_, true) => Constraint::End,
        (_, false) => Constraint::Start,
    }
}
fn validate_affine(value: Transform) -> Result<Transform, String> {
    if [value.a, value.b, value.c, value.d, value.e, value.f]
        .iter()
        .any(|v| !v.is_finite() || v.abs() > 1e7)
    {
        return Err("Use finite matrix values between −10,000,000 and 10,000,000.".into());
    }
    if value.inverse().is_none() {
        return Err("The matrix must preserve a nonzero area.".into());
    }
    Ok(value)
}
fn apply_affine(editor: &mut Editor, id: NodeId, transform: Transform) -> Result<bool, String> {
    let transform = validate_affine(transform)?;
    if editor.selection != [id] {
        return Err("Selection changed. Open the transform editor for the new layer.".into());
    }
    if editor.has_active_transaction() {
        return Err("Finish the current gesture before editing this transform.".into());
    }
    if editor.document.effective_locked(id) {
        return Err("Unlock the layer and its ancestors before transforming it.".into());
    }
    let node = editor
        .document
        .nodes
        .get(&id)
        .ok_or("The layer no longer exists.")?;
    let current = node.local_transform();
    // The model decomposes a matrix into editable rotation/reflection and a
    // residual affine matrix. Reconstructing that representation can differ
    // by a few machine units, which must not turn Enter into another edit.
    if [
        current.a, current.b, current.c, current.d, current.e, current.f,
    ]
    .into_iter()
    .zip([
        transform.a,
        transform.b,
        transform.c,
        transform.d,
        transform.e,
        transform.f,
    ])
    .all(|(a, b)| (a - b).abs() <= 16.0 * f64::EPSILON * a.abs().max(b.abs()).max(1.0))
    {
        return Ok(false);
    }
    editor.update_node(id, "Edit affine transform", |node| {
        node.set_local_transform(transform);
        for property in ["x", "y", "rotation", "flip_x", "flip_y", "affine_transform"] {
            node.overridden_properties.insert(property.into());
        }
    });
    Ok(true)
}

#[cfg(test)]
#[path = "geometry_authoring_tests.rs"]
mod tests;
