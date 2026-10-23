//! Native local style management. Definition edits reuse the validated core
//! lifecycle and preserve effective values, bindings and exact undo boundaries.
#[path = "layout_grids.rs"]
mod layout_grids;

use super::*;
use ferrodesign_core::{LocalStyleKind, Style, StyleId, StyleValue};
use ferroui_base::interactivity::RoutingStrategies;

impl EditorShell {
    pub(super) fn styles_dialog(self: &Rc<Self>) {
        if !self.flush_inspector_drafts() {
            return;
        }
        self.updating.set(true);
        self.modal(
            "Local styles",
            "Apply a style to unlocked compatible layers, or edit its shared definition.",
        );
        self.updating.set(false);
        let styles: Vec<_> = {
            let editor = self.editor.borrow();
            let mut usages = BTreeMap::<StyleId, usize>::new();
            for node in editor.document.nodes.values() {
                let bindings = [node.fill_style, node.text_style, node.effect_style, node.grid_style];
                for (index, id) in bindings.iter().enumerate() {
                    if let Some(id) = id.filter(|id| !bindings[..index].contains(&Some(*id))) {
                        *usages.entry(id).or_default() += 1;
                    }
                }
            }
            editor
                .document
                .styles
                .values()
                .map(|style| {
                    (
                        style.id,
                        style.name.clone(),
                        style.value.kind(),
                        usages.get(&style.id).copied().unwrap_or(0),
                    )
                })
                .collect()
        };
        let host = self.control::<StackPanel>("ModalContent");
        if styles.is_empty() {
            host.children().add(style_label(
                "Create a reusable style from a selected layer.",
            ));
        }
        for (id, name, kind, users) in styles {
            let row = style_grid("*,80");
            let apply = self.style_button(&format!("{name}  ·  {}  ·  {users} uses", kind.label()),
                &format!("StyleApply{id}"), true, move |shell| {
                    let result = { let mut editor = shell.editor.borrow_mut();
                        let ids = editor.selection.clone(); editor.apply_local_style(&ids, id) };
                    match result {
                        Ok(count) if count != 0 => { shell.changed(); shell.close_modal(); }
                        Ok(_) => shell.toast("Select an unlocked compatible layer that does not already use this style."),
                        Err(error) => shell.toast(&error.to_string()),
                    }
                });
            apply.classes().add("layer");
            row.children().add(apply);
            let edit = self.style_button("Edit", &format!("StyleEdit{id}"), true, move |shell| {
                shell.style_dialog(id)
            });
            Grid::set_column(&edit, 1);
            row.children().add(edit);
            host.children().add(row);
        }
        host.children().add(self.style_button("Selected frame grid settings…", "StyleGridSettings", true,
            |shell| shell.layout_grid_dialog()));
        let actions = self.control::<StackPanel>("ModalActions");
        for (label, name, command) in [
            ("Create fill", "StyleCreateFill", "create-fill-style"),
            ("Create text", "StyleCreateText", "create-text-style"),
            ("Create effects", "StyleCreateEffect", "create-effect-style"),
            ("Create grid", "StyleCreateGrid", "create-grid-style"),
        ] {
            actions.children().add(
                self.style_button(label, name, true, move |shell| shell.create_style(command)),
            );
        }
    }

    pub(super) fn create_style(self: &Rc<Self>, command: &str) {
        if !self.flush_inspector_drafts() {
            return;
        }
        let kind = match command {
            "create-text-style" => LocalStyleKind::Text,
            "create-effect-style" => LocalStyleKind::Effect,
            "create-grid-style" => LocalStyleKind::Grid,
            _ => LocalStyleKind::Fill,
        };
        let result = {
            let mut editor = self.editor.borrow_mut();
            let ids = editor.selection.clone();
            let name = ids
                .iter()
                .filter_map(|id| editor.document.nodes.get(id))
                .find(|node| kind.accepts(node) && !editor.document.effective_locked(node.id))
                .map(|node| node.name.clone())
                .unwrap_or_else(|| kind.label().into());
            editor.create_local_style_for_nodes(&ids, kind, &name, "")
        };
        match result {
            Ok(id) => {
                self.changed();
                self.style_dialog(id);
            }
            Err(error) => self.toast(&error.to_string()),
        }
    }

    fn style_dialog(self: &Rc<Self>, id: StyleId) {
        if !self.flush_inspector_drafts() {
            return;
        }
        let style = self.editor.borrow().document.styles.get(&id).cloned();
        let Some(style) = style else {
            self.toast("The local style no longer exists.");
            self.styles_dialog();
            return;
        };
        let kind = style.value.kind();
        self.updating.set(true);
        self.modal(
            "Edit local style",
            "Changes update linked layers across all pages. Detached values stay independent.",
        );
        self.updating.set(false);
        let host = self.control::<StackPanel>("ModalContent");
        host.set_spacing(10.0);
        let usage = style_label(&format!(
            "{} style · {} linked layers",
            kind.label(),
            self.editor.borrow().document.local_style_usage(id)
        ));
        usage.set_name(Some("StyleUsage".into()));
        host.children().add(usage);
        host.children().add(self.style_field(
            id,
            "Name",
            "StyleName",
            &style.name,
            |style, value| {
                style.name = value.trim().into();
                Ok(())
            },
        ));
        host.children().add(self.style_field(
            id,
            "Description",
            "StyleDescription",
            &style.description,
            |style, value| {
                style.description = value.into();
                Ok(())
            },
        ));
        match &style.value {
            StyleValue::Fill { fills } => {
                for (index, fill) in fills.iter().enumerate() {
                    self.style_fill(&host, id, index, fill, fills.len());
                }
                host.children().add(self.style_button(
                    "Add paint",
                    "StyleAddFill",
                    true,
                    move |shell| {
                        shell.style_action(id, |style| {
                            let StyleValue::Fill { fills } = &mut style.value else {
                                return Err(stale_style());
                            };
                            fills.push(Fill::solid(Color::rgb(217, 217, 217)));
                            Ok(())
                        });
                    },
                ));
            }
            StyleValue::Text {
                family,
                size,
                weight,
                line_height,
                letter_spacing,
            } => {
                for (caption, name, value, property) in [
                    ("Font family", "StyleFontFamily", family.clone(), 0),
                    ("Font size", "StyleFontSize", size.to_string(), 1),
                    ("Weight · 1–1000", "StyleFontWeight", weight.to_string(), 2),
                    ("Line height", "StyleLineHeight", line_height.to_string(), 3),
                    (
                        "Letter spacing",
                        "StyleLetterSpacing",
                        letter_spacing.to_string(),
                        4,
                    ),
                ] {
                    host.children().add(self.style_field(
                        id,
                        caption,
                        name,
                        &value,
                        move |style, text| {
                            let StyleValue::Text {
                                family,
                                size,
                                weight,
                                line_height,
                                letter_spacing,
                            } = &mut style.value
                            else {
                                return Err(stale_style());
                            };
                            match property {
                                0 => *family = text.trim().into(),
                                1 => *size = style_number(text)?,
                                2 => {
                                    *weight = text.trim().parse().map_err(|_| {
                                        "Enter a font weight from 1 to 1000.".to_string()
                                    })?
                                }
                                3 => *line_height = style_number(text)?,
                                _ => *letter_spacing = style_number(text)?,
                            }
                            Ok(())
                        },
                    ));
                }
                host.children().add(style_label(
                    "Character-level formatting remains attached to each text layer.",
                ));
            }
            StyleValue::Effect { effects } => {
                for (index, effect) in effects.iter().enumerate() {
                    self.style_effect(&host, id, index, effect, effects.len());
                }
                host.children().add(self.style_button(
                    "Add effect",
                    "StyleAddEffect",
                    true,
                    move |shell| {
                        shell.style_action(id, |style| {
                            let StyleValue::Effect { effects } = &mut style.value else {
                                return Err(stale_style());
                            };
                            effects.push(Effect::for_kind(core::EffectKind::DropShadow));
                            Ok(())
                        });
                    },
                ));
            }
            value @ (StyleValue::Grid { .. } | StyleValue::GridStack { .. }) => {
                self.style_layout_guides(&host, id, value);
            }
        }
        let value_actions = style_row();
        value_actions.children().add(self.style_button(
            "Replace from selected layer",
            "StyleReplaceFromSelection",
            true,
            move |shell| {
                if !shell.flush_inspector_drafts() {
                    return;
                }
                let source = {
                    let editor = shell.editor.borrow();
                    editor
                        .selection
                        .iter()
                        .filter_map(|node| editor.document.nodes.get(node))
                        .find(|node| {
                            kind.accepts(node) && !editor.document.effective_locked(node.id)
                        })
                        .map(|node| StyleValue::from_node(kind, node))
                };
                match source {
                    Some(Ok(value)) => shell.style_action(id, move |style| {
                        style.value = value.clone();
                        Ok(())
                    }),
                    Some(Err(error)) => shell.toast(&error.to_string()),
                    None => shell.toast(
                        "Select an unlocked compatible layer to replace the complete definition.",
                    ),
                }
            },
        ));
        value_actions.children().add(self.style_button(
            "Detach selected",
            "StyleDetachSelected",
            true,
            move |shell| {
                if !shell.flush_inspector_drafts() {
                    return;
                }
                let result = {
                    let mut editor = shell.editor.borrow_mut();
                    let ids: Vec<_> = editor
                        .selection
                        .iter()
                        .copied()
                        .filter(|node| {
                            editor
                                .document
                                .nodes
                                .get(node)
                                .is_some_and(|node| match kind {
                                    LocalStyleKind::Fill => node.fill_style == Some(id),
                                    LocalStyleKind::Text => node.text_style == Some(id),
                                    LocalStyleKind::Effect => node.effect_style == Some(id),
                                    LocalStyleKind::Grid => node.grid_style == Some(id),
                                })
                        })
                        .collect();
                    editor.detach_local_style(&ids, kind)
                };
                match result {
                    Ok(count) => {
                        if count != 0 {
                            shell.changed();
                        }
                        shell.style_dialog(id);
                    }
                    Err(error) => shell.toast(&error.to_string()),
                }
            },
        ));
        host.children().add(value_actions);
        let actions = self.control::<StackPanel>("ModalActions");
        actions
            .children()
            .add(self.style_button("All styles", "StyleBack", true, |shell| {
                shell.styles_dialog()
            }));
        actions.children().add(self.style_button(
            "Delete style",
            "StyleDelete",
            true,
            move |shell| {
                if !shell.flush_inspector_drafts() {
                    return;
                }
                let result = shell.editor.borrow_mut().remove_local_style(id);
                match result {
                    Ok(_) => {
                        shell.changed();
                        shell.styles_dialog();
                    }
                    Err(error) => shell.toast(&error.to_string()),
                }
            },
        ));
        let done = self.style_button("Done", "StyleDone", true, |shell| shell.close_modal());
        done.classes().add("primary");
        actions.children().add(done);
    }

    fn style_fill(
        self: &Rc<Self>,
        host: &Ref<StackPanel>,
        id: StyleId,
        index: usize,
        fill: &Fill,
        count: usize,
    ) {
        let panel = style_card(host, &format!("Paint {}", index + 1));
        self.style_stack_actions(&panel, id, index, count, true);
        panel.children().add(self.style_toggle(
            id,
            "Visible",
            &format!("StyleFillVisible{index}"),
            fill.visible,
            move |style, value| {
                style_fill_mut(style, index)?.visible = value;
                Ok(())
            },
        ));
        let selected = match fill.paint {
            Paint::Solid(_) => 0,
            Paint::LinearGradient { .. } => 1,
            Paint::RadialGradient { .. } => 2,
            Paint::AngularGradient { .. } => 3,
            Paint::Image { .. } => 4,
        };
        panel.children().add(self.style_choice(id, &format!("StyleFillType{index}"),
            &["Solid", "Linear gradient", "Radial gradient", "Angular gradient", "Existing image"], selected, move |style, value| {
                let fill = style_fill_mut(style, index)?;
                // Fields commit in place, so the current paint can differ
                // from the value that created this native choice control.
                if matches!((&fill.paint, value), (Paint::Solid(_), 0)
                    | (Paint::LinearGradient { .. }, 1) | (Paint::RadialGradient { .. }, 2)
                    | (Paint::AngularGradient { .. }, 3) | (Paint::Image { .. }, 4))
                {
                    return Ok(());
                }
                let color = fill.paint.representative_color();
                let stops = match &fill.paint {
                    Paint::LinearGradient { stops, .. } | Paint::RadialGradient { stops, .. }
                    | Paint::AngularGradient { stops, .. } => stops.clone(),
                    _ => style_default_stops(color),
                };
                fill.paint = match value {
                    0 => Paint::Solid(color),
                    1 => Paint::LinearGradient { start: Point::new(0.0, 0.0), end: Point::new(1.0, 1.0), stops },
                    2 => Paint::RadialGradient { center: Point::new(0.5, 0.5), radius: 0.5, stops },
                    3 => Paint::AngularGradient { center: Point::new(0.5, 0.5), angle: 0.0, stops },
                    _ => return Err("Replace from a selected image layer to use its embedded image and source window.".into()),
                }; Ok(())
            }));
        panel.children().add(self.style_field(
            id,
            "Opacity %",
            &format!("StyleFillOpacity{index}"),
            &(fill.opacity * 100.0).to_string(),
            move |style, value| {
                style_fill_mut(style, index)?.opacity = style_number(value)? / 100.0;
                Ok(())
            },
        ));
        match &fill.paint {
            Paint::Solid(color) => panel.children().add(self.style_field(
                id,
                "Color · RGBA hex",
                &format!("StyleFillColor{index}"),
                &color.hex(),
                move |style, value| {
                    let Paint::Solid(color) = &mut style_fill_mut(style, index)?.paint else {
                        return Err(stale_style());
                    };
                    *color = style_color(value)?;
                    Ok(())
                },
            )),
            Paint::LinearGradient { start, end, stops } => {
                self.style_gradient_coordinates(
                    &panel,
                    id,
                    index,
                    &[
                        ("Start X", start.x),
                        ("Start Y", start.y),
                        ("End X", end.x),
                        ("End Y", end.y),
                    ],
                );
                self.style_gradient_stops(&panel, id, index, stops);
            }
            Paint::RadialGradient {
                center,
                radius,
                stops,
            } => {
                self.style_gradient_coordinates(
                    &panel,
                    id,
                    index,
                    &[
                        ("Center X", center.x),
                        ("Center Y", center.y),
                        ("Radius", *radius),
                    ],
                );
                self.style_gradient_stops(&panel, id, index, stops);
            }
            Paint::AngularGradient { center, angle, stops } => {
                self.style_gradient_coordinates(
                    &panel, id, index,
                    &[("Center X", center.x), ("Center Y", center.y), ("Angle (degrees)", *angle)],
                );
                self.style_gradient_stops(&panel, id, index, stops);
            }
            Paint::Image { scale, crop, .. } => {
                let scales = [
                    core::ImageScale::Fill,
                    core::ImageScale::Fit,
                    core::ImageScale::Stretch,
                    core::ImageScale::Tile,
                ];
                panel.children().add(self.style_choice(
                    id,
                    &format!("StyleImageScale{index}"),
                    &["Fill", "Fit", "Stretch", "Tile"],
                    scales.iter().position(|item| item == scale).unwrap_or(0),
                    move |style, value| {
                        let Paint::Image { scale, .. } = &mut style_fill_mut(style, index)?.paint
                        else {
                            return Err(stale_style());
                        };
                        *scale = scales[value];
                        Ok(())
                    },
                ));
                let window = crop.unwrap_or(core::Rect::new(0.0, 0.0, 1.0, 1.0));
                for (property, caption, value) in [
                    (0, "Source X", window.x),
                    (1, "Source Y", window.y),
                    (2, "Source width", window.width),
                    (3, "Source height", window.height),
                ] {
                    panel.children().add(self.style_field(
                        id,
                        caption,
                        &format!("StyleImageCrop{index}_{property}"),
                        &value.to_string(),
                        move |style, text| {
                            let Paint::Image { crop, .. } =
                                &mut style_fill_mut(style, index)?.paint
                            else {
                                return Err(stale_style());
                            };
                            let window = crop.get_or_insert(core::Rect::new(0.0, 0.0, 1.0, 1.0));
                            match property {
                                0 => window.x = style_number(text)?,
                                1 => window.y = style_number(text)?,
                                2 => window.width = style_number(text)?,
                                _ => window.height = style_number(text)?,
                            }
                            Ok(())
                        },
                    ));
                }
            }
        }
    }

    fn style_gradient_coordinates(
        self: &Rc<Self>,
        host: &Ref<StackPanel>,
        id: StyleId,
        index: usize,
        coordinates: &[(&str, f64)],
    ) {
        for (property, (caption, value)) in coordinates.iter().enumerate() {
            host.children().add(self.style_field(
                id,
                caption,
                &format!("StyleGradientCoordinate{index}_{property}"),
                &value.to_string(),
                move |style, text| {
                    let value = style_number(text)?;
                    match &mut style_fill_mut(style, index)?.paint {
                        Paint::LinearGradient { start, end, .. } => match property {
                            0 => start.x = value,
                            1 => start.y = value,
                            2 => end.x = value,
                            _ => end.y = value,
                        },
                        Paint::RadialGradient { center, radius, .. } => match property {
                            0 => center.x = value,
                            1 => center.y = value,
                            _ => *radius = value,
                        },
                        Paint::AngularGradient { center, angle, .. } => match property {
                            0 => center.x = value,
                            1 => center.y = value,
                            _ => *angle = value,
                        },
                        _ => return Err(stale_style()),
                    }
                    Ok(())
                },
            ));
        }
    }

    fn style_gradient_stops(
        self: &Rc<Self>,
        host: &Ref<StackPanel>,
        id: StyleId,
        index: usize,
        stops: &[GradientStop],
    ) {
        for (stop, item) in stops.iter().enumerate() {
            let row = style_grid("*,100,72");
            row.children().add(self.style_field(
                id,
                &format!("Stop {} color", stop + 1),
                &format!("StyleStopColor{index}_{stop}"),
                &item.color.hex(),
                move |style, text| {
                    style_stops_mut(style, index)?
                        .get_mut(stop)
                        .ok_or_else(stale_style)?
                        .color = style_color(text)?;
                    Ok(())
                },
            ));
            let field = self.style_field(
                id,
                "Position %",
                &format!("StyleStopOffset{index}_{stop}"),
                &(item.offset * 100.0).to_string(),
                move |style, text| {
                    style_stops_mut(style, index)?
                        .get_mut(stop)
                        .ok_or_else(stale_style)?
                        .offset = style_number(text)? / 100.0;
                    Ok(())
                },
            );
            Grid::set_column(&field, 1);
            row.children().add(field);
            let remove = self.style_button(
                "Remove",
                &format!("StyleStopRemove{index}_{stop}"),
                stops.len() > 1,
                move |shell| {
                    shell.style_action(id, move |style| {
                        let stops = style_stops_mut(style, index)?;
                        if stops.len() <= 1 || stop >= stops.len() {
                            return Err(stale_style());
                        }
                        stops.remove(stop);
                        Ok(())
                    });
                },
            );
            Grid::set_column(&remove, 2);
            remove.set_vertical_alignment(VerticalAlignment::Bottom);
            row.children().add(remove);
            host.children().add(row);
        }
        host.children().add(self.style_button(
            "Add gradient stop",
            &format!("StyleStopAdd{index}"),
            true,
            move |shell| {
                shell.style_action(id, move |style| {
                    let stops = style_stops_mut(style, index)?;
                    let (offset, color) = stops
                        .windows(2)
                        .max_by(|a, b| {
                            (a[1].offset - a[0].offset).total_cmp(&(b[1].offset - b[0].offset))
                        })
                        .map(|pair| {
                            (
                                (pair[0].offset + pair[1].offset) / 2.0,
                                pair[0].color.lerp(pair[1].color, 0.5),
                            )
                        })
                        .unwrap_or((
                            1.0,
                            stops.first().map(|stop| stop.color).unwrap_or(Color::WHITE),
                        ));
                    stops.push(GradientStop { offset, color });
                    stops.sort_by(|a, b| a.offset.total_cmp(&b.offset));
                    Ok(())
                });
            },
        ));
    }

    fn style_effect(
        self: &Rc<Self>,
        host: &Ref<StackPanel>,
        id: StyleId,
        index: usize,
        effect: &Effect,
        count: usize,
    ) {
        let panel = style_card(host, &format!("Effect {}", index + 1));
        self.style_stack_actions(&panel, id, index, count, false);
        panel.children().add(self.style_toggle(
            id,
            "Visible",
            &format!("StyleEffectVisible{index}"),
            effect.visible,
            move |style, value| {
                style_effect_mut(style, index)?.visible = value;
                Ok(())
            },
        ));
        let kinds = [
            core::EffectKind::DropShadow,
            core::EffectKind::InnerShadow,
            core::EffectKind::LayerBlur,
            core::EffectKind::BackgroundBlur,
        ];
        panel.children().add(
            self.style_choice(
                id,
                &format!("StyleEffectType{index}"),
                &[
                    "Drop shadow",
                    "Inner shadow",
                    "Layer blur",
                    "Background blur",
                ],
                kinds
                    .iter()
                    .position(|kind| *kind == effect.kind)
                    .unwrap_or(0),
                move |style, value| {
                    style_effect_mut(style, index)?.kind = kinds[value];
                    Ok(())
                },
            ),
        );
        for (property, caption, value) in [
            (0, "Blur radius", effect.radius),
            (1, "Offset X", effect.offset.x),
            (2, "Offset Y", effect.offset.y),
            (3, "Spread", effect.spread),
        ] {
            panel.children().add(self.style_field(
                id,
                caption,
                &format!("StyleEffectNumber{index}_{property}"),
                &value.to_string(),
                move |style, text| {
                    let value = style_number(text)?;
                    let effect = style_effect_mut(style, index)?;
                    match property {
                        0 => effect.radius = value,
                        1 => effect.offset.x = value,
                        2 => effect.offset.y = value,
                        _ => effect.spread = value,
                    }
                    Ok(())
                },
            ));
        }
        panel.children().add(self.style_field(
            id,
            "Color · RGBA hex",
            &format!("StyleEffectColor{index}"),
            &effect.color.hex(),
            move |style, text| {
                style_effect_mut(style, index)?.color = style_color(text)?;
                Ok(())
            },
        ));
    }

    fn style_stack_actions(
        self: &Rc<Self>,
        host: &Ref<StackPanel>,
        id: StyleId,
        index: usize,
        count: usize,
        fills: bool,
    ) {
        let row = style_row();
        for (action, caption, enabled) in [
            (0, "Up", index > 0),
            (1, "Down", index + 1 < count),
            (2, "Duplicate", true),
            (3, "Remove", true),
        ] {
            row.children().add(self.style_button(
                caption,
                &format!("StyleStack{action}_{index}"),
                enabled,
                move |shell| {
                    shell.style_action(id, move |style| match &mut style.value {
                        StyleValue::Fill { fills: values } if fills => {
                            style_stack(values, index, action)
                        }
                        StyleValue::Effect { effects: values } if !fills => {
                            style_stack(values, index, action)
                        }
                        _ => Err(stale_style()),
                    });
                },
            ));
        }
        host.children().add(row);
    }

    fn style_action(self: &Rc<Self>, id: StyleId, edit: impl Fn(&mut Style) -> Result<(), String>) {
        if self.updating.get() || !self.flush_inspector_drafts() {
            return;
        }
        match self.apply_style_edit(id, edit) {
            Ok(_) => self.style_dialog(id),
            Err(message) => self.toast(&message),
        }
    }

    fn apply_style_edit(
        self: &Rc<Self>,
        id: StyleId,
        edit: impl Fn(&mut Style) -> Result<(), String>,
    ) -> Result<bool, String> {
        let mut style = self
            .editor
            .borrow()
            .document
            .styles
            .get(&id)
            .cloned()
            .ok_or_else(stale_style)?;
        edit(&mut style)?;
        let kind = style.value.kind();
        let result = self
            .editor
            .borrow_mut()
            .update_local_style(id, &style.name, &style.description, style.value)
            .map_err(|error| error.to_string())?;
        if result.definition_changed {
            self.changed();
        }
        let usage = self.editor.borrow().document.local_style_usage(id);
        // Update only the native label: rebuilding the editor here would
        // replace the focused field and its still-live draft/IME state.
        for visual in self
            .control::<StackPanel>("ModalContent")
            .get_visual_descendants()
        {
            if let Some(label) = visual
                .cast::<TextBlock>()
                .filter(|label| label.name().as_deref() == Some("StyleUsage"))
            {
                label.set_text(Some(&format!(
                    "{} style · {usage} linked layers",
                    kind.label()
                )));
                break;
            }
        }
        if result.detached_overrides != 0 {
            self.toast(&format!(
                "Updated style; retained {} independently edited layers.",
                result.detached_overrides
            ));
        }
        Ok(result.definition_changed)
    }

    fn style_field(
        self: &Rc<Self>,
        id: StyleId,
        caption: &str,
        name: &str,
        value: &str,
        edit: impl Fn(&mut Style, &str) -> Result<(), String> + 'static,
    ) -> Ref<StackPanel> {
        let panel = StackPanel::new();
        panel.set_spacing(4.0);
        panel.children().add(style_label(caption));
        let field = TextBox::new();
        field.set_name(Some(name.into()));
        field.set_text(Some(value));
        panel.children().add(field.clone());
        let error = style_label("");
        error.set_name(Some(format!("{name}Error")));
        error.set_foreground(Some(icons::brush("#C8372B")));
        error.set_is_visible(false);
        panel.children().add(error.clone());
        let committed = Rc::new(RefCell::new(value.to_string()));
        let weak = Rc::downgrade(self);
        let weak_field = field.downgrade();
        let weak_error = error.downgrade();
        let previous = committed.clone();
        let commit: Rc<dyn Fn()> = Rc::new(move || {
            let (Some(shell), Some(field), Some(error)) =
                (weak.upgrade(), weak_field.upgrade(), weak_error.upgrade())
            else {
                return;
            };
            if shell.updating.get()
                || field.is_composing_text()
                || !field.is_attached_to_visual_tree()
                || !field.is_effectively_visible()
            {
                return;
            }
            let text = field.text().unwrap_or_default();
            if text == *previous.borrow() {
                return;
            }
            match shell.apply_style_edit(id, |style| edit(style, &text)) {
                Ok(_) => {
                    *previous.borrow_mut() = text;
                    error.set_is_visible(false);
                    field.set_border_brush(Some(icons::brush("#E5E5E5")));
                }
                Err(message) => {
                    error.set_text(Some(&message));
                    error.set_is_visible(true);
                    field.set_border_brush(Some(icons::brush("#C8372B")));
                }
            }
            shell.sync_document_draft_dirty();
        });
        let blur = commit.clone();
        field.add_handler(InputElement::lost_focus_event(), move |_, _| blur());
        let enter = commit.clone();
        let weak_field = field.downgrade();
        field.add_handler_with(
            InputElement::key_down_event(),
            move |_, event| {
                if event.key == Key::Enter
                    && !weak_field
                        .upgrade()
                        .is_some_and(|field| field.is_composing_text())
                {
                    enter();
                    event.set_handled(true);
                }
            },
            RoutingStrategies::TUNNEL,
            false,
        );
        self.bind_inspector_draft(&field, committed, commit, Some(&error));
        panel
    }

    fn style_choice(
        self: &Rc<Self>,
        id: StyleId,
        name: &str,
        labels: &[&str],
        selected: usize,
        edit: impl Fn(&mut Style, usize) -> Result<(), String> + 'static,
    ) -> Ref<ComboBox> {
        let choice = ComboBox::new();
        choice.set_name(Some(name.into()));
        for label in labels {
            choice.items().add(Some(Rc::new(label.to_string())));
        }
        choice.set_selected_index(selected as i32);
        let weak = Rc::downgrade(self);
        let weak_choice = choice.downgrade();
        let length = labels.len();
        choice.selection_changed(move |_, _| {
            let (Some(shell), Some(choice)) = (weak.upgrade(), weak_choice.upgrade()) else {
                return;
            };
            let Ok(index) = usize::try_from(choice.selected_index()) else {
                return;
            };
            if index >= length || shell.updating.get() || !choice.is_attached_to_visual_tree() {
                return;
            }
            if !shell.flush_inspector_drafts() {
                shell.updating.set(true);
                choice.set_selected_index(selected as i32);
                shell.updating.set(false);
                return;
            }
            match shell.apply_style_edit(id, |style| edit(style, index)) {
                Ok(_) => shell.style_dialog(id),
                Err(message) => {
                    shell.updating.set(true);
                    choice.set_selected_index(selected as i32);
                    shell.updating.set(false);
                    shell.toast(&message);
                }
            }
        });
        choice
    }

    fn style_toggle(
        self: &Rc<Self>,
        id: StyleId,
        caption: &str,
        name: &str,
        checked: bool,
        edit: impl Fn(&mut Style, bool) -> Result<(), String> + 'static,
    ) -> Ref<CheckBox> {
        let toggle = CheckBox::new();
        toggle.set_name(Some(name.into()));
        toggle.set_content(Some(Rc::new(caption.to_string())));
        toggle.set_is_checked(Some(checked));
        let weak = Rc::downgrade(self);
        let weak_toggle = toggle.downgrade();
        toggle.is_checked_changed(move |_, _| {
            let (Some(shell), Some(toggle)) = (weak.upgrade(), weak_toggle.upgrade()) else {
                return;
            };
            if shell.updating.get() || !toggle.is_attached_to_visual_tree() {
                return;
            }
            if !shell.flush_inspector_drafts() {
                shell.updating.set(true);
                toggle.set_is_checked(Some(checked));
                shell.updating.set(false);
                return;
            }
            match shell.apply_style_edit(id, |style| edit(style, toggle.is_checked() == Some(true)))
            {
                Ok(_) => shell.style_dialog(id),
                Err(message) => {
                    shell.updating.set(true);
                    toggle.set_is_checked(Some(checked));
                    shell.updating.set(false);
                    shell.toast(&message);
                }
            }
        });
        toggle
    }

    pub(super) fn style_button(
        self: &Rc<Self>,
        caption: &str,
        name: &str,
        enabled: bool,
        action: impl Fn(&Rc<Self>) + 'static,
    ) -> Ref<Button> {
        let button = Button::new();
        button.set_name(Some(name.into()));
        button.set_content(Some(Rc::new(caption.to_string())));
        button.set_is_enabled(enabled);
        let weak = Rc::downgrade(self);
        button.click(move |_, _| {
            if let Some(shell) = weak.upgrade() {
                if !shell.updating.get() {
                    action(&shell);
                }
            }
        });
        button
    }
}

fn stale_style() -> String {
    "This style entry changed. Reopen its definition before editing.".into()
}
fn style_number(value: &str) -> Result<f64, String> {
    value
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
        .ok_or_else(|| "Enter a finite number.".into())
}
fn style_color(value: &str) -> Result<Color, String> {
    Color::from_hex(value).ok_or_else(|| "Enter an RGB or RGBA hexadecimal color.".into())
}
fn style_fill_mut(style: &mut Style, index: usize) -> Result<&mut Fill, String> {
    let StyleValue::Fill { fills } = &mut style.value else {
        return Err(stale_style());
    };
    fills.get_mut(index).ok_or_else(stale_style)
}
fn style_effect_mut(style: &mut Style, index: usize) -> Result<&mut Effect, String> {
    let StyleValue::Effect { effects } = &mut style.value else {
        return Err(stale_style());
    };
    effects.get_mut(index).ok_or_else(stale_style)
}
fn style_stops_mut(style: &mut Style, index: usize) -> Result<&mut Vec<GradientStop>, String> {
    match &mut style_fill_mut(style, index)?.paint {
        Paint::LinearGradient { stops, .. } | Paint::RadialGradient { stops, .. }
        | Paint::AngularGradient { stops, .. } => Ok(stops),
        _ => Err(stale_style()),
    }
}
fn style_stack<T: Clone>(values: &mut Vec<T>, index: usize, action: usize) -> Result<(), String> {
    if index >= values.len() {
        return Err(stale_style());
    }
    match action {
        0 if index > 0 => values.swap(index, index - 1),
        1 if index + 1 < values.len() => values.swap(index, index + 1),
        2 => values.insert(index + 1, values[index].clone()),
        3 => {
            values.remove(index);
        }
        _ => return Err(stale_style()),
    }
    Ok(())
}
fn style_default_stops(color: Color) -> Vec<GradientStop> {
    vec![
        GradientStop { offset: 0.0, color },
        GradientStop {
            offset: 1.0,
            color: Color::WHITE,
        },
    ]
}
pub(super) fn style_label(value: &str) -> Ref<TextBlock> {
    let label = TextBlock::new();
    label.set_text(Some(value));
    label.set_font_size(11.0);
    label.set_text_wrapping(TextWrapping::Wrap);
    label
}
fn style_row() -> Ref<StackPanel> {
    let row = StackPanel::new();
    row.set_orientation(Orientation::Horizontal);
    row.set_spacing(8.0);
    row
}
fn style_grid(columns: &str) -> Ref<Grid> {
    let grid = Grid::new();
    grid.set_column_definitions(ColumnDefinitions::parse(columns).expect("static style columns"));
    grid.set_column_spacing(8.0);
    grid
}
fn style_card(host: &Ref<StackPanel>, caption: &str) -> Ref<StackPanel> {
    let border = Border::new();
    border.set_padding(Thickness::uniform(10.0));
    border.set_border_thickness(Thickness::uniform(1.0));
    border.set_border_brush(Some(icons::brush("#E5E5E5")));
    let panel = StackPanel::new();
    panel.set_spacing(8.0);
    let title = style_label(caption);
    title.set_font_weight(FontWeight::SemiBold);
    panel.children().add(title);
    border.set_child(panel.clone());
    host.children().add(border);
    panel
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_style_fields_reject_nonfinite_numbers_and_parse_alpha_colors() {
        for value in ["NaN", "inf", "-inf", "1e999", ""] {
            assert!(style_number(value).is_err());
        }
        assert_eq!(style_number(" -12.5 "), Ok(-12.5));
        assert_eq!(style_color("#1234"), Ok(Color::rgba(17, 34, 51, 68)));
        assert!(style_color("not a color").is_err());
    }
    #[test]
    fn style_stack_operations_preserve_order_and_reject_stale_indices() {
        let mut values = vec![1, 2, 3];
        style_stack(&mut values, 1, 2).unwrap();
        assert_eq!(values, [1, 2, 2, 3]);
        style_stack(&mut values, 3, 0).unwrap();
        assert_eq!(values, [1, 2, 3, 2]);
        style_stack(&mut values, 0, 1).unwrap();
        assert_eq!(values, [2, 1, 3, 2]);
        style_stack(&mut values, 2, 3).unwrap();
        assert_eq!(values, [2, 1, 2]);
        assert!(style_stack(&mut values, 100, 3).is_err());
        assert_eq!(values, [2, 1, 2]);
    }
}
