//! Native paint-stack controls. Every accepted edit is one document transaction.
use super::*;
use ferrodesign_core::Rect;
use ferroui_base::media::immutable::{
    ImmutableConicGradientBrush, ImmutableGradientStop, ImmutableLinearGradientBrush, ImmutableRadialGradientBrush,
    ImmutableSolidColorBrush,
};
use ferroui_base::media::{GradientSpreadMethod, IBrush};
use ferroui_base::{CornerRadius, RelativePoint, RelativeUnit};

/// Clearing a focused modal can raise LostFocus while its old controls still exist.
/// Keep their callbacks inert until the replacement tree is complete.
struct UpdatingPaints<'a>(&'a Cell<bool>, bool);
impl Drop for UpdatingPaints<'_> {
    fn drop(&mut self) {
        self.0.set(self.1);
    }
}

#[derive(Clone, Copy)]
enum Coordinate {
    StartX,
    StartY,
    EndX,
    EndY,
    CenterX,
    CenterY,
    Radius,
    Angle,
}

#[derive(Clone, Copy)]
enum PaintEdit {
    Visible(bool),
    Opacity(f64),
    Kind(usize),
    Solid(Color),
    Coordinate(Coordinate, f64),
    StopColor(usize, Color),
    StopOffset(usize, f64),
    AddStop,
    RemoveStop(usize),
    ReverseStops,
    ImageScale(core::ImageScale),
    ImageCrop(Rect),
}

#[derive(Clone, Copy)]
enum StackEdit {
    Add,
    Remove(usize),
    Duplicate(usize),
    Move(usize, isize),
    Paint(usize, PaintEdit),
}

impl EditorShell {
    fn displayed_fills<'a>(&self, editor: &'a Editor) -> Option<&'a [Fill]> {
        let id = *editor.selection.first()?;
        let node = editor.document.nodes.get(&id)?;
        if let Some((target, region)) = self.paint_region.get() {
            if target != id || editor.selection.len() != 1 { return None; }
            let region = node.vector_network.as_ref()?.regions.get(&region)?;
            Some(region.fills.as_deref().unwrap_or(&node.fills))
        } else {
            Some(&node.fills)
        }
    }

    pub(super) fn paints_dialog(self: &Rc<Self>) {
        if !self.flush_inspector_drafts() { return; }
        self.paint_region.set(None);
        self.show_paints_dialog();
    }

    pub(super) fn region_paints_dialog(self: &Rc<Self>) {
        if !self.flush_inspector_drafts() { return; }
        let Some(target) = self.canvas.active_network_region() else {
            self.toast("Use the paint bucket to select a closed vector region.");
            return;
        };
        self.paint_region.set(Some(target));
        self.show_paints_dialog();
    }

    fn show_paints_dialog(self: &Rc<Self>) {
        let snapshot = {
            let editor = self.editor.borrow();
            editor.selection.first().and_then(|id| {
                editor.document.nodes.get(id).and_then(|node| {
                    if let Some((target, region)) = self.paint_region.get() {
                        if target != *id || editor.selection.len() != 1 { return None; }
                        let region_value = node.vector_network.as_ref()?.regions.get(&region)?;
                        Some((format!("{} · Region {region}", node.name),
                            region_value.fills.as_ref().unwrap_or(&node.fills).clone(), 1))
                    } else {
                        Some((node.name.clone(), node.fills.clone(), editor.selection.len()))
                    }
                })
            })
        };
        let Some((name, fills, selected)) = snapshot else {
            self.toast("Select a layer to edit its fills.");
            return;
        };
        let _updating = UpdatingPaints(&self.updating, self.updating.replace(true));
        let description = if selected == 1 {
            format!("{name} · Front paint first. Changes apply immediately and support undo.")
        } else {
            format!(
                "Showing {name}. Edits apply to matching paint rows on {selected} selected layers."
            )
        };
        self.modal(if self.paint_region.get().is_some() { "Region paints" } else { "Fill paints" }, &description);
        let host = self.control::<StackPanel>("ModalContent");
        host.set_spacing(12.0);
        if fills.is_empty() {
            host.children().add(paint_text(
                "This layer has no fills. Add a paint to give it a color or gradient.",
                true,
            ));
        }
        // The first authored paint is topmost, matching native scene rendering.
        for (index, fill) in fills.iter().enumerate() {
            host.children()
                .add(self.paint_card(index, fill, fills.len()));
        }
        let actions = self.control::<StackPanel>("ModalActions");
        actions
            .children()
            .add(self.paint_button("Add fill", "PaintAdd", true, |shell| {
                shell.edit_paints("Add fill paint", StackEdit::Add, true);
            }));
        let done = self.paint_button("Done", "PaintDone", true, |shell| shell.close_modal());
        done.classes().add("primary");
        actions.children().add(done);
    }

    fn paint_card(self: &Rc<Self>, index: usize, fill: &Fill, count: usize) -> Ref<Border> {
        let card = Border::new();
        card.set_name(Some(format!("PaintCard{index}")));
        card.set_padding(Thickness::uniform(12.0));
        card.set_corner_radius(CornerRadius::uniform(8.0));
        card.set_background(Some(icons::brush("#F7F7F7")));
        card.set_border_brush(Some(icons::brush("#E5E5E5")));
        card.set_border_thickness(Thickness::uniform(1.0));
        let content = StackPanel::new();
        content.set_spacing(9.0);
        card.set_child(content.clone());

        let preview = Border::new();
        preview.set_height(34.0);
        preview.set_corner_radius(CornerRadius::uniform(4.0));
        preview.set_background(Some(paint_preview(fill)));

        let header = paint_grid("*,54,54,78,74", 5.0);
        let visible = CheckBox::new();
        visible.set_name(Some(format!("PaintVisible{index}")));
        visible.set_content(Some(Rc::new(format!("Paint {}", index + 1))));
        visible.set_is_checked(Some(fill.visible));
        let weak_shell = Rc::downgrade(self);
        let weak_check = visible.downgrade();
        let weak_preview = preview.downgrade();
        visible.is_checked_changed(move |_, _| {
            if let (Some(shell), Some(check)) = (weak_shell.upgrade(), weak_check.upgrade()) {
                if shell.updating.get() || !check.is_attached_to_visual_tree() { return; }
                let requested = check.is_checked() == Some(true);
                if shell.flush_inspector_drafts() {
                    shell.edit_paints(
                        "Toggle fill paint",
                        StackEdit::Paint(index, PaintEdit::Visible(requested)),
                        false,
                    );
                }
                // The control remains mounted across edits. Read the latest
                // committed target instead of restoring its initial value.
                let committed = shell.displayed_fills(&shell.editor.borrow())
                    .and_then(|fills| fills.get(index)).map(|fill| fill.visible);
                let _updating = UpdatingPaints(&shell.updating, shell.updating.replace(true));
                check.set_is_checked(committed);
                shell.refresh_paint_preview(index, &weak_preview);
            }
        });
        header.children().add(visible);
        for (column, label, suffix, enabled, edit) in [
            (1, "Up", "Up", index > 0, StackEdit::Move(index, -1)),
            (2, "Down", "Down", index + 1 < count, StackEdit::Move(index, 1)),
            (
                3,
                "Duplicate",
                "Duplicate",
                true,
                StackEdit::Duplicate(index),
            ),
            (4, "Remove", "Remove", true, StackEdit::Remove(index)),
        ] {
            let button = self.paint_button(
                label,
                &format!("Paint{suffix}{index}"),
                enabled,
                move |shell| {
                    shell.edit_paints("Edit fill stack", edit, true);
                },
            );
            Grid::set_column(&button, column);
            header.children().add(button);
        }
        content.children().add(header);
        content.children().add(preview.clone());

        let properties = paint_grid("*,132", 10.0);
        let kind = match fill.paint {
            Paint::Solid(_) => 0,
            Paint::LinearGradient { .. } => 1,
            Paint::RadialGradient { .. } => 2,
            Paint::AngularGradient { .. } => 3,
            Paint::Image { .. } => 4,
        };
        let kinds: &[&str] = if kind == 4 {
            &[
                "Solid",
                "Linear gradient",
                "Radial gradient",
                "Angular gradient",
                "Embedded image",
            ]
        } else {
            &["Solid", "Linear gradient", "Radial gradient", "Angular gradient"]
        };
        let choice = self.paint_choice(
            "Type",
            &format!("PaintType{index}"),
            kinds,
            kind,
            move |shell, kind| {
                shell.edit_paints(
                    "Change fill type",
                    StackEdit::Paint(index, PaintEdit::Kind(kind)),
                    true,
                );
            },
        );
        properties.children().add(choice);
        let weak_preview = preview.downgrade();
        let opacity = self.paint_field(
            "Opacity %",
            &format!("PaintOpacity{index}"),
            &(fill.opacity * 100.0).to_string(),
            move |shell, value| {
                let value = paint_percent(value)?;
                shell.edit_paints(
                    "Change fill opacity",
                    StackEdit::Paint(index, PaintEdit::Opacity(value)),
                    false,
                );
                shell.refresh_paint_preview(index, &weak_preview);
                Ok(())
            },
        );
        Grid::set_column(&opacity, 1);
        properties.children().add(opacity);
        content.children().add(properties);

        match &fill.paint {
            Paint::Solid(color) => {
                let weak_preview = preview.downgrade();
                content.children().add(self.paint_field(
                    "Hex color · RGB or RGBA",
                    &format!("PaintColor{index}"),
                    &color.hex(),
                    move |shell, value| {
                        let color = paint_color(value)?;
                        shell.edit_paints(
                            "Change fill color",
                            StackEdit::Paint(index, PaintEdit::Solid(color)),
                            false,
                        );
                        shell.refresh_paint_preview(index, &weak_preview);
                        Ok(())
                    },
                ));
            }
            Paint::LinearGradient { start, end, stops } => {
                self.paint_stops(&content, &preview, index, stops);
                content.children().add(paint_text(
                    "Gradient geometry · relative to layer bounds (0 to 1).",
                    true,
                ));
                let row = paint_grid("*,*,*,*", 8.0);
                for (column, label, coordinate, value) in [
                    (0, "Start X", Coordinate::StartX, start.x),
                    (1, "Start Y", Coordinate::StartY, start.y),
                    (2, "End X", Coordinate::EndX, end.x),
                    (3, "End Y", Coordinate::EndY, end.y),
                ] {
                    let field = self.paint_coordinate(index, label, coordinate, value, &preview);
                    Grid::set_column(&field, column);
                    row.children().add(field);
                }
                content.children().add(row);
            }
            Paint::RadialGradient {
                center,
                radius,
                stops,
            } => {
                self.paint_stops(&content, &preview, index, stops);
                content.children().add(paint_text(
                    "Gradient geometry · relative to layer bounds (0 to 1).",
                    true,
                ));
                let row = paint_grid("*,*,*", 8.0);
                for (column, label, coordinate, value) in [
                    (0, "Center X", Coordinate::CenterX, center.x),
                    (1, "Center Y", Coordinate::CenterY, center.y),
                    (2, "Radius", Coordinate::Radius, *radius),
                ] {
                    let field = self.paint_coordinate(index, label, coordinate, value, &preview);
                    Grid::set_column(&field, column);
                    row.children().add(field);
                }
                content.children().add(row);
            }
            Paint::AngularGradient { center, angle, stops } => {
                self.paint_stops(&content, &preview, index, stops);
                content.children().add(paint_text(
                    "Center is relative to layer bounds. Angle is clockwise in degrees; 0° points up.",
                    true,
                ));
                let row = paint_grid("*,*,*", 8.0);
                for (column, label, coordinate, value) in [
                    (0, "Center X", Coordinate::CenterX, center.x),
                    (1, "Center Y", Coordinate::CenterY, center.y),
                    (2, "Angle", Coordinate::Angle, *angle),
                ] {
                    let field = self.paint_coordinate(index, label, coordinate, value, &preview);
                    Grid::set_column(&field, column);
                    row.children().add(field);
                }
                content.children().add(row);
            }
            Paint::Image { scale, .. } => {
                content.children().add(paint_text(
                    "Image data stays embedded in the document. Changing this paint to a color or gradient can be undone.",
                    true,
                ));
                let selected = match scale {
                    core::ImageScale::Fill => 0,
                    core::ImageScale::Fit => 1,
                    core::ImageScale::Stretch => 2,
                    core::ImageScale::Tile => 3,
                };
                content.children().add(self.paint_choice(
                    "Image scaling",
                    &format!("PaintImageScale{index}"),
                    &["Fill", "Fit", "Stretch", "Tile"],
                    selected,
                    move |shell, scale| {
                        let scale = [
                            core::ImageScale::Fill,
                            core::ImageScale::Fit,
                            core::ImageScale::Stretch,
                            core::ImageScale::Tile,
                        ][scale];
                        shell.edit_paints(
                            "Change image scaling",
                            StackEdit::Paint(index, PaintEdit::ImageScale(scale)),
                            true,
                        );
                    },
                ));
                self.paint_crop_controls(&content, index, fill);
            }
        }
        card
    }

    fn paint_crop_controls(self: &Rc<Self>, content: &Ref<StackPanel>, index: usize, fill: &Fill) {
        let Some(crop) = fill.paint.image_crop() else { return; };
        content.children().add(paint_text(
            "Source window · percentages of the original image. Apply the complete window; image scaling stays unchanged.",
            true,
        ));
        let row = paint_grid("*,*,*,*", 8.0);
        let mut fields = Vec::new();
        for (column, label, suffix, value) in [
            (0, "Left %", "X", crop.x), (1, "Top %", "Y", crop.y),
            (2, "Width %", "Width", crop.width), (3, "Height %", "Height", crop.height),
        ] {
            let panel = StackPanel::new();
            panel.set_spacing(4.0);
            panel.children().add(paint_text(label, true));
            let field = TextBox::new();
            field.set_name(Some(format!("PaintCrop{suffix}{index}")));
            field.set_text(Some(&(value * 100.0).to_string()));
            fields.push(field.clone());
            panel.children().add(field);
            Grid::set_column(&panel, column);
            row.children().add(panel);
        }
        content.children().add(row);
        let error = paint_text("", true);
        error.set_name(Some(format!("PaintCropError{index}")));
        error.set_foreground(Some(icons::brush("#C8372B")));
        error.set_is_visible(false);
        content.children().add(error.clone());
        let weak_fields: Vec<_> = fields.iter().map(|field| field.downgrade()).collect();
        let weak_error = error.downgrade();
        let apply = Rc::new(move |shell: &Rc<Self>| {
            if shell.updating.get() { return; }
            let values: Option<Vec<_>> = weak_fields.iter().map(|field| field.upgrade()
                .map(|field| field.text().unwrap_or_default())).collect();
            let (Some(values), Some(error)) = (values, weak_error.upgrade()) else { return; };
            let crop = paint_crop_values(&values);
            match crop {
                Ok(crop) => {
                    shell.edit_paints("Crop image source", StackEdit::Paint(index, PaintEdit::ImageCrop(crop)), false);
                    error.set_is_visible(false);
                }
                Err(message) => {
                    error.set_text(Some(message));
                    error.set_is_visible(true);
                    error.bring_into_view();
                }
            }
        });
        for field in fields {
            let weak_shell = Rc::downgrade(self);
            let apply = apply.clone();
            field.add_handler(InputElement::key_down_event(), move |_, event| {
                if event.key == Key::Enter {
                    if let Some(shell) = weak_shell.upgrade() { apply(&shell); }
                    event.set_handled(true);
                }
            });
        }
        let actions = paint_grid("*,*,*", 6.0);
        actions.children().add(self.paint_button("Apply window", &format!("PaintCropApply{index}"), true,
            move |shell| apply(shell)));
        let reset = self.paint_button("Reset source", &format!("PaintCropReset{index}"), true,
            move |shell| {
                shell.edit_paints("Reset image source", StackEdit::Paint(index,
                    PaintEdit::ImageCrop(Rect::new(0.0, 0.0, 1.0, 1.0))), false);
                // Also discard unapplied percentage fields when the source was
                // already full-sized and the document edit was a no-op.
                shell.show_paints_dialog();
            });
        Grid::set_column(&reset, 1);
        actions.children().add(reset);
        let direct = self.paint_region.get().is_none()
            && matches!(fill.paint, Paint::Image { scale: core::ImageScale::Fill | core::ImageScale::Stretch, .. });
        let single = self.editor.borrow().selection.len() == 1;
        let button = self.paint_button("Crop on canvas", &format!("PaintCropCanvas{index}"), direct && single && fill.visible && fill.opacity > 0.0,
            move |shell| {
                shell.finish_inline_text(true);
                shell.close_modal();
                if !shell.canvas.edit_image_crop(Some(index)) {
                    shell.toast("Direct crop needs one unlocked rectangle, ellipse, polygon, star, or empty frame with a readable embedded image in Fill or Stretch mode.");
                }
            });
        Grid::set_column(&button, 2);
        actions.children().add(button);
        content.children().add(actions);
        content.children().add(paint_text(if direct {
            "On canvas: drag the image to pan, or drag a corner to change its window while retained pixels stay fixed. Enter finishes; Esc cancels the active drag. Flow layout children support panning."
        } else {
            "Fit and Tile support the source-window fields. Choose Fill to use direct canvas crop handles without stretching the image."
        }, true));
    }

    fn paint_stops(
        self: &Rc<Self>,
        content: &Ref<StackPanel>,
        preview: &Ref<Border>,
        index: usize,
        stops: &[GradientStop],
    ) {
        let heading = paint_text("Gradient stops", false);
        heading.set_font_weight(FontWeight::SemiBold);
        content.children().add(heading);
        for (stop, value) in stops.iter().enumerate() {
            let row = paint_grid("*,110,74", 8.0);
            let weak_preview = preview.downgrade();
            row.children().add(self.paint_field(
                &format!("Stop {} color", stop + 1),
                &format!("PaintStopColor{index}_{stop}"),
                &value.color.hex(),
                move |shell, value| {
                    let color = paint_color(value)?;
                    shell.edit_paints(
                        "Change gradient stop color",
                        StackEdit::Paint(index, PaintEdit::StopColor(stop, color)),
                        false,
                    );
                    shell.refresh_paint_preview(index, &weak_preview);
                    Ok(())
                },
            ));
            let weak_preview = preview.downgrade();
            let offset = self.paint_field(
                "Position %",
                &format!("PaintStopOffset{index}_{stop}"),
                &(value.offset * 100.0).to_string(),
                move |shell, value| {
                    let offset = paint_percent(value)?;
                    shell.edit_paints(
                        "Move gradient stop",
                        StackEdit::Paint(index, PaintEdit::StopOffset(stop, offset)),
                        false,
                    );
                    shell.refresh_paint_preview(index, &weak_preview);
                    Ok(())
                },
            );
            Grid::set_column(&offset, 1);
            row.children().add(offset);
            let remove = self.paint_button(
                "Remove",
                &format!("PaintStopRemove{index}_{stop}"),
                stops.len() > 2,
                move |shell| {
                    shell.edit_paints(
                        "Remove gradient stop",
                        StackEdit::Paint(index, PaintEdit::RemoveStop(stop)),
                        true,
                    );
                },
            );
            remove.set_vertical_alignment(VerticalAlignment::Bottom);
            Grid::set_column(&remove, 2);
            row.children().add(remove);
            content.children().add(row);
        }
        let buttons = StackPanel::new();
        buttons.set_orientation(Orientation::Horizontal);
        buttons.set_spacing(8.0);
        buttons.children().add(self.paint_button(
            "Add stop",
            &format!("PaintStopAdd{index}"),
            true,
            move |shell| {
                shell.edit_paints(
                    "Add gradient stop",
                    StackEdit::Paint(index, PaintEdit::AddStop),
                    true,
                );
            },
        ));
        buttons.children().add(self.paint_button(
            "Reverse gradient",
            &format!("PaintReverse{index}"),
            stops.len() > 1,
            move |shell| {
                shell.edit_paints(
                    "Reverse gradient",
                    StackEdit::Paint(index, PaintEdit::ReverseStops),
                    true,
                );
            },
        ));
        content.children().add(buttons);
    }

    fn paint_coordinate(
        self: &Rc<Self>,
        index: usize,
        label: &str,
        coordinate: Coordinate,
        value: f64,
        preview: &Ref<Border>,
    ) -> Ref<StackPanel> {
        let weak_preview = preview.downgrade();
        self.paint_field(
            label,
            &format!("Paint{}{}", label.replace(' ', ""), index),
            &value.to_string(),
            move |shell, text| {
                let value = text
                    .trim()
                    .parse::<f64>()
                    .map_err(|_| "Enter a finite number.")?;
                if !value.is_finite() || value.abs() > 1000.0 {
                    return Err("Enter a finite value between −1000 and 1000.");
                }
                if matches!(coordinate, Coordinate::Radius) && value <= 0.0 {
                    return Err("Radius must be greater than zero.");
                }
                shell.edit_paints(
                    "Change gradient geometry",
                    StackEdit::Paint(index, PaintEdit::Coordinate(coordinate, value)),
                    false,
                );
                shell.refresh_paint_preview(index, &weak_preview);
                Ok(())
            },
        )
    }

    fn paint_field(
        self: &Rc<Self>,
        label: &str,
        name: &str,
        value: &str,
        apply: impl Fn(&Rc<Self>, &str) -> Result<(), &'static str> + 'static,
    ) -> Ref<StackPanel> {
        let panel = StackPanel::new();
        panel.set_spacing(4.0);
        panel.children().add(paint_text(label, true));
        let field = TextBox::new();
        field.set_name(Some(name.to_string()));
        field.set_text(Some(value));
        panel.children().add(field.clone());
        let error = paint_text("", true);
        error.set_foreground(Some(icons::brush("#C8372B")));
        error.set_is_visible(false);
        panel.children().add(error.clone());
        let weak_shell = Rc::downgrade(self);
        let weak_field = field.downgrade();
        let weak_error = error.downgrade();
        let committed = Rc::new(RefCell::new(value.to_string()));
        let committed_value = committed.clone();
        let commit = Rc::new(move || {
            let (Some(shell), Some(field), Some(error)) = (
                weak_shell.upgrade(),
                weak_field.upgrade(),
                weak_error.upgrade(),
            ) else {
                return;
            };
            if shell.updating.get() || field.is_composing_text() {
                return;
            }
            let text = field.text().unwrap_or_default();
            // Enter followed by blur must not produce a second undo entry. This also
            // avoids silently rounding an untouched imported fractional coordinate.
            if *committed_value.borrow() == text {
                return;
            }
            match apply(&shell, &text) {
                Ok(()) => {
                    *committed_value.borrow_mut() = text;
                    error.set_is_visible(false);
                    field.set_border_brush(Some(icons::brush("#E5E5E5")));
                }
                Err(message) => {
                    error.set_text(Some(message));
                    error.set_is_visible(true);
                    field.set_border_brush(Some(icons::brush("#C8372B")));
                }
            }
        });
        let blur = commit.clone();
        field.add_handler(InputElement::lost_focus_event(), move |_, _| blur());
        let draft_commit = commit.clone();
        field.add_handler(InputElement::key_down_event(), move |_, event| {
            if event.key == Key::Enter {
                commit();
                event.set_handled(true);
            }
        });
        self.bind_inspector_draft(&field, committed, draft_commit, Some(&error));
        panel
    }

    fn paint_choice(
        self: &Rc<Self>,
        label: &str,
        name: &str,
        labels: &[&str],
        selected: usize,
        apply: impl Fn(&Rc<Self>, usize) + 'static,
    ) -> Ref<StackPanel> {
        let panel = StackPanel::new();
        panel.set_spacing(4.0);
        panel.children().add(paint_text(label, true));
        let choice = ComboBox::new();
        choice.set_name(Some(name.to_string()));
        for label in labels {
            choice.items().add(Some(Rc::new(label.to_string())));
        }
        choice.set_selected_index(selected as i32);
        let weak_shell = Rc::downgrade(self);
        let weak_choice = choice.downgrade();
        let count = labels.len();
        let previous = selected;
        choice.selection_changed(move |_, _| {
            if let (Some(shell), Some(choice)) = (weak_shell.upgrade(), weak_choice.upgrade()) {
                let selected = choice.selected_index();
                if !shell.updating.get() && selected >= 0 && (selected as usize) < count {
                    if !shell.flush_inspector_drafts() {
                        let _updating = UpdatingPaints(&shell.updating, shell.updating.replace(true));
                        choice.set_selected_index(previous as i32);
                        return;
                    }
                    apply(&shell, selected as usize);
                }
            }
        });
        panel.children().add(choice);
        panel
    }

    fn paint_button(
        self: &Rc<Self>,
        caption: &str,
        name: &str,
        enabled: bool,
        apply: impl Fn(&Rc<Self>) + 'static,
    ) -> Ref<Button> {
        let button = Button::new();
        button.set_name(Some(name.to_string()));
        button.set_content(Some(Rc::new(caption.to_string())));
        button.set_is_enabled(enabled);
        button.set_height(30.0);
        let weak_shell = Rc::downgrade(self);
        button.click(move |_, _| {
            if let Some(shell) = weak_shell.upgrade() {
                if !shell.updating.get() && shell.flush_inspector_drafts() {
                    apply(&shell);
                }
            }
        });
        button
    }

    fn edit_paints(self: &Rc<Self>, label: &str, edit: StackEdit, rebuild: bool) {
        if self.updating.get() {
            return;
        }
        if let Some((id, region)) = self.paint_region.get() {
            let fills = {
                let editor = self.editor.borrow();
                if editor.document.effective_locked(id) || editor.selection.as_slice() != [id] { return; }
                editor.document.nodes.get(&id).and_then(|node| {
                    let region = node.vector_network.as_ref()?.regions.get(&region)?;
                    Some(region.fills.as_ref().unwrap_or(&node.fills).clone())
                })
            };
            let Some(mut fills) = fills else { return; };
            if !apply_stack_edit(&mut fills, edit) { return; }
            let result = self.editor.borrow_mut().set_network_region_fills(id, region, Some(fills));
            match result {
                Ok(true) => {
                    self.changed();
                    if rebuild { self.show_paints_dialog(); }
                }
                Ok(false) => {}
                Err(error) => self.toast(&error.to_string()),
            }
            return;
        }
        let ids: Vec<_> = {
            let editor = self.editor.borrow();
            editor.selection.iter().copied()
                .filter(|id| !editor.document.effective_locked(*id)).collect()
        };
        let mut changed = false;
        self.editor.borrow_mut().update_nodes_with_layout(&ids, label, |node| {
            if apply_stack_edit(&mut node.fills, edit) {
                node.fill_style = None;
                node.variable_bindings.remove("fill");
                node.overridden_properties.insert("fills".into());
                changed = true;
            }
        });
        if changed {
            self.changed();
            if rebuild {
                self.show_paints_dialog();
            }
        }
    }

    fn refresh_paint_preview(&self, index: usize, preview: &WeakRef<Border>) {
        let Some(preview) = preview.upgrade() else {
            return;
        };
        let editor = self.editor.borrow();
        if let Some(fill) = self.displayed_fills(&editor).and_then(|fills| fills.get(index)) {
            preview.set_background(Some(paint_preview(fill)));
        }
    }
}

fn paint_text(value: &str, muted: bool) -> Ref<TextBlock> {
    let text = TextBlock::new();
    text.set_text(Some(value));
    text.set_font_size(if muted { 11.0 } else { 12.0 });
    text.set_foreground(Some(icons::brush(if muted {
        "#737373"
    } else {
        "#242424"
    })));
    text.set_text_wrapping(TextWrapping::Wrap);
    text
}

fn paint_grid(columns: &str, spacing: f64) -> Ref<Grid> {
    let grid = Grid::new();
    grid.set_column_definitions(ColumnDefinitions::parse(columns).expect("paint columns"));
    grid.set_column_spacing(spacing);
    grid
}

fn paint_color(value: &str) -> Result<Color, &'static str> {
    Color::from_hex(value).ok_or("Use 3, 4, 6, or 8 hexadecimal digits; alpha comes last.")
}

fn paint_percent(value: &str) -> Result<f64, &'static str> {
    let value = value
        .trim()
        .trim_end_matches('%')
        .trim()
        .parse::<f64>()
        .map_err(|_| "Enter a percentage from 0 to 100.")?;
    if value.is_finite() && (0.0..=100.0).contains(&value) {
        Ok(value / 100.0)
    } else {
        Err("Enter a percentage from 0 to 100.")
    }
}

fn paint_crop_values(values: &[String]) -> Result<Rect, &'static str> {
    if values.len() != 4 { return Err("Enter left, top, width, and height percentages."); }
    let crop = Rect::new(paint_percent(&values[0])?, paint_percent(&values[1])?,
        paint_percent(&values[2])?, paint_percent(&values[3])?);
    if core::valid_image_crop(crop) { Ok(crop) }
    else { Err("Width and height must be above 0%; the complete window must stay inside 100% of the source.") }
}

fn replace_paint_value<T: PartialEq>(target: &mut T, value: T) -> bool {
    if *target == value {
        false
    } else {
        *target = value;
        true
    }
}

fn apply_stack_edit(fills: &mut Vec<Fill>, edit: StackEdit) -> bool {
    match edit {
        StackEdit::Add => {
            fills.insert(0, Fill::solid(Color::rgb(217, 217, 217)));
            true
        }
        StackEdit::Remove(index) if index < fills.len() => {
            fills.remove(index);
            true
        }
        StackEdit::Duplicate(index) if index < fills.len() => {
            fills.insert(index, fills[index].clone());
            true
        }
        StackEdit::Move(index, offset) => {
            let Some(other) = index
                .checked_add_signed(offset)
                .filter(|other| *other < fills.len())
            else {
                return false;
            };
            if index >= fills.len() || index == other {
                return false;
            }
            fills.swap(index, other);
            true
        }
        StackEdit::Paint(index, edit) => fills
            .get_mut(index)
            .is_some_and(|fill| apply_paint_edit(fill, edit)),
        _ => false,
    }
}

fn apply_paint_edit(fill: &mut Fill, edit: PaintEdit) -> bool {
    match edit {
        PaintEdit::Visible(value) => replace_paint_value(&mut fill.visible, value),
        PaintEdit::Opacity(value) => replace_paint_value(&mut fill.opacity, value),
        PaintEdit::Kind(kind) => {
            if matches!(
                (&fill.paint, kind),
                (Paint::Solid(_), 0)
                    | (Paint::LinearGradient { .. }, 1)
                    | (Paint::RadialGradient { .. }, 2)
                    | (Paint::AngularGradient { .. }, 3)
                    | (Paint::Image { .. }, 4)
            ) {
                return false;
            }
            let color = fill.paint.representative_color();
            let stops = match &fill.paint {
                Paint::LinearGradient { stops, .. } | Paint::RadialGradient { stops, .. }
                | Paint::AngularGradient { stops, .. } => {
                    stops.clone()
                }
                _ => vec![
                    GradientStop { offset: 0.0, color },
                    GradientStop {
                        offset: 1.0,
                        color: color.lerp(Color::BLACK, 0.35),
                    },
                ],
            };
            let paint = match kind {
                0 => Paint::Solid(color),
                1 => Paint::LinearGradient {
                    start: Point::new(0.0, 0.0),
                    end: Point::new(1.0, 1.0),
                    stops,
                },
                2 => Paint::RadialGradient {
                    center: Point::new(0.5, 0.5),
                    radius: 0.5,
                    stops,
                },
                3 => Paint::AngularGradient {
                    center: Point::new(0.5, 0.5),
                    angle: 0.0,
                    stops,
                },
                // Creating an image requires imported bytes, never an empty href.
                _ => return false,
            };
            replace_paint_value(&mut fill.paint, paint)
        }
        PaintEdit::Solid(value) => match &mut fill.paint {
            Paint::Solid(color) => replace_paint_value(color, value),
            _ => false,
        },
        PaintEdit::ImageScale(value) => match &mut fill.paint {
            Paint::Image { scale, .. } => replace_paint_value(scale, value),
            _ => false,
        },
        PaintEdit::ImageCrop(value) => fill.paint.set_image_crop(value),
        PaintEdit::Coordinate(coordinate, value) => {
            if !value.is_finite() || value.abs() > 1000.0
                || matches!(coordinate, Coordinate::Radius) && value <= 0.0
            {
                return false;
            }
            let target = match (&mut fill.paint, coordinate) {
                (Paint::LinearGradient { start, .. }, Coordinate::StartX) => &mut start.x,
                (Paint::LinearGradient { start, .. }, Coordinate::StartY) => &mut start.y,
                (Paint::LinearGradient { end, .. }, Coordinate::EndX) => &mut end.x,
                (Paint::LinearGradient { end, .. }, Coordinate::EndY) => &mut end.y,
                (Paint::RadialGradient { center, .. } | Paint::AngularGradient { center, .. }, Coordinate::CenterX) => &mut center.x,
                (Paint::RadialGradient { center, .. } | Paint::AngularGradient { center, .. }, Coordinate::CenterY) => &mut center.y,
                (Paint::AngularGradient { angle, .. }, Coordinate::Angle) => angle,
                (Paint::RadialGradient { radius, .. }, Coordinate::Radius) => radius,
                _ => return false,
            };
            replace_paint_value(target, value)
        }
        edit => {
            let stops = match &mut fill.paint {
                Paint::LinearGradient { stops, .. } | Paint::RadialGradient { stops, .. }
                | Paint::AngularGradient { stops, .. } => stops,
                _ => return false,
            };
            match edit {
                PaintEdit::StopColor(index, value) => stops
                    .get_mut(index)
                    .is_some_and(|stop| replace_paint_value(&mut stop.color, value)),
                // Keep indices stable while a field has focus. The renderer sorts a
                // copy by offset; structural edits rebuild controls with new indices.
                PaintEdit::StopOffset(index, value) => stops
                    .get_mut(index)
                    .is_some_and(|stop| replace_paint_value(&mut stop.offset, value)),
                PaintEdit::AddStop => {
                    add_gradient_stop(stops);
                    true
                }
                PaintEdit::RemoveStop(index) if stops.len() > 2 && index < stops.len() => {
                    stops.remove(index);
                    true
                }
                PaintEdit::ReverseStops if stops.len() > 1 => {
                    for stop in stops.iter_mut() {
                        stop.offset = 1.0 - stop.offset;
                    }
                    stops.reverse();
                    true
                }
                _ => false,
            }
        }
    }
}

fn add_gradient_stop(stops: &mut Vec<GradientStop>) {
    stops.sort_by(|a, b| a.offset.total_cmp(&b.offset));
    if stops.is_empty() {
        stops.extend([
            GradientStop {
                offset: 0.0,
                color: Color::BLACK,
            },
            GradientStop {
                offset: 1.0,
                color: Color::WHITE,
            },
        ]);
        return;
    }
    let first = &stops[0];
    let last = &stops[stops.len() - 1];
    let mut gap = first.offset;
    let mut new = GradientStop {
        offset: first.offset / 2.0,
        color: first.color,
    };
    for pair in stops.windows(2) {
        let width = pair[1].offset - pair[0].offset;
        if width > gap {
            gap = width;
            new = GradientStop {
                offset: (pair[0].offset + pair[1].offset) / 2.0,
                color: pair[0].color.lerp(pair[1].color, 0.5),
            };
        }
    }
    if 1.0 - last.offset > gap {
        new = GradientStop {
            offset: (last.offset + 1.0) / 2.0,
            color: last.color,
        };
    }
    stops.push(new);
    stops.sort_by(|a, b| a.offset.total_cmp(&b.offset));
}

fn paint_preview(fill: &Fill) -> Rc<dyn IBrush> {
    let color =
        |color: Color| ferroui_base::media::Color::from_argb(color.a, color.r, color.g, color.b);
    let relative = |point: Point| RelativePoint::new(point.x, point.y, RelativeUnit::Relative);
    let opacity = if fill.visible {
        fill.opacity.clamp(0.0, 1.0)
    } else {
        0.15
    };
    let gradient = |stops: &[GradientStop]| {
        let mut result: Vec<_> = stops
            .iter()
            .map(|stop| ImmutableGradientStop::new(stop.offset, color(stop.color)))
            .collect();
        result.sort_by(|a, b| a.offset().total_cmp(&b.offset()));
        if result.is_empty() {
            result.push(ImmutableGradientStop::new(0.0, color(Color::TRANSPARENT)));
        }
        if result.len() == 1 {
            result.push(ImmutableGradientStop::new(1.0, result[0].color()));
        }
        result
    };
    match &fill.paint {
        Paint::Solid(value) => Rc::new(ImmutableSolidColorBrush::with_opacity(
            color(*value),
            opacity,
        )),
        Paint::LinearGradient { start, end, stops } => Rc::new(ImmutableLinearGradientBrush::new(
            &gradient(stops),
            opacity,
            None,
            None,
            GradientSpreadMethod::Pad,
            Some(relative(*start)),
            Some(relative(*end)),
            None,
        )),
        Paint::RadialGradient {
            center,
            radius,
            stops,
        } => Rc::new(ImmutableRadialGradientBrush::with_radius(
            &gradient(stops),
            opacity,
            None,
            None,
            GradientSpreadMethod::Pad,
            Some(relative(*center)),
            Some(relative(*center)),
            *radius,
        )),
        Paint::AngularGradient { center, angle, stops } => Rc::new(ImmutableConicGradientBrush::new(
            &gradient(stops), opacity, None, None, GradientSpreadMethod::Pad,
            Some(relative(*center)), angle.rem_euclid(360.0), None,
        )),
        Paint::Image { .. } => Rc::new(ImmutableSolidColorBrush::with_opacity(
            color(fill.paint.representative_color()),
            opacity,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paint_reorder_and_non_image_edits_preserve_embedded_bytes() {
        let image = Fill {
            paint: Paint::Image {
                href: "data:image/png;base64,exact-bytes".into(),
                scale: core::ImageScale::Fill,
                crop: None,
            },
            opacity: 1.0,
            visible: true,
        };
        let mut fills = vec![image, Fill::solid(Color::BLACK)];
        assert!(apply_stack_edit(&mut fills, StackEdit::Move(0, 1)));
        assert!(apply_stack_edit(
            &mut fills,
            StackEdit::Paint(1, PaintEdit::Opacity(0.4))
        ));
        assert!(apply_stack_edit(
            &mut fills,
            StackEdit::Paint(1, PaintEdit::ImageScale(core::ImageScale::Fit))
        ));
        assert!(
            matches!(&fills[1].paint, Paint::Image { href, scale: core::ImageScale::Fit, .. } if href == "data:image/png;base64,exact-bytes")
        );
        assert_eq!(fills[1].opacity, 0.4);
        assert!(!apply_stack_edit(
            &mut fills,
            StackEdit::Paint(0, PaintEdit::Kind(4))
        ));
        assert!(!apply_stack_edit(
            &mut fills,
            StackEdit::Paint(99, PaintEdit::Solid(Color::WHITE))
        ));
    }

    #[test]
    fn image_crop_percentages_are_atomic_and_preserve_scaling_and_embedded_data() {
        let values = |values: [&str; 4]| values.map(str::to_string).to_vec();
        let crop = paint_crop_values(&values(["10%", "20", "80", "60"])).unwrap();
        assert_eq!(crop, Rect::new(0.1, 0.2, 0.8, 0.6));
        for invalid in [["90", "0", "20", "100"], ["0", "0", "0", "100"], ["NaN", "0", "50", "50"]] {
            assert!(paint_crop_values(&values(invalid)).is_err());
        }
        let mut fill = Fill {
            paint: Paint::Image { href: "data:image/png;base64,exact".into(), scale: core::ImageScale::Fit, crop: None },
            ..Fill::solid(Color::WHITE)
        };
        assert!(apply_paint_edit(&mut fill, PaintEdit::ImageCrop(crop)));
        assert!(!apply_paint_edit(&mut fill, PaintEdit::ImageCrop(crop)));
        assert!(matches!(&fill.paint, Paint::Image { href, scale: core::ImageScale::Fit, crop: Some(value) }
            if href == "data:image/png;base64,exact" && *value == crop));
        assert!(apply_paint_edit(&mut fill, PaintEdit::ImageCrop(Rect::new(0.0, 0.0, 1.0, 1.0))));
        assert!(matches!(fill.paint, Paint::Image { scale: core::ImageScale::Fit, crop: None, .. }));
    }

    #[test]
    fn gradient_conversion_preserves_stops_and_live_offset_indices() {
        let stops = vec![
            GradientStop {
                offset: 0.0,
                color: Color::BLACK,
            },
            GradientStop {
                offset: 1.0,
                color: Color::WHITE,
            },
        ];
        let mut fill = Fill {
            paint: Paint::LinearGradient {
                start: Point::new(0.0, 0.0),
                end: Point::new(1.0, 1.0),
                stops: stops.clone(),
            },
            opacity: 0.75,
            visible: false,
        };
        assert!(apply_paint_edit(&mut fill, PaintEdit::Kind(2)));
        assert!(
            matches!(&fill.paint, Paint::RadialGradient { stops: actual, .. } if *actual == stops)
        );
        assert!(!apply_paint_edit(&mut fill, PaintEdit::RemoveStop(0)));
        assert!(apply_paint_edit(&mut fill, PaintEdit::StopOffset(0, 0.9)));
        assert!(apply_paint_edit(&mut fill, PaintEdit::StopOffset(1, 0.1)));
        assert!(apply_paint_edit(
            &mut fill,
            PaintEdit::StopColor(0, Color::rgb(1, 2, 3))
        ));
        assert!(
            matches!(&fill.paint, Paint::RadialGradient { stops, .. } if stops[0].color == Color::rgb(1, 2, 3) && stops[0].offset == 0.9)
        );
        assert_eq!(fill.opacity, 0.75);
        assert!(!fill.visible);
    }

    #[test]
    fn adding_a_stop_interpolates_the_largest_gap() {
        let mut stops = vec![
            GradientStop {
                offset: 1.0,
                color: Color::WHITE,
            },
            GradientStop {
                offset: 0.0,
                color: Color::BLACK,
            },
        ];
        add_gradient_stop(&mut stops);
        assert_eq!(stops.len(), 3);
        assert_eq!(stops[1].offset, 0.5);
        assert_eq!(stops[1].color, Color::rgb(128, 128, 128));
    }

    #[test]
    fn invalid_numbers_and_colors_never_enter_the_document() {
        for invalid in ["NaN", "inf", "-1", "100.01", "hello"] {
            assert!(paint_percent(invalid).is_err());
        }
        assert_eq!(paint_percent("25 %"), Ok(0.25));
        assert!(paint_color("💜💜💜").is_err());
        assert_eq!(paint_color("#1234"), Ok(Color::rgba(17, 34, 51, 68)));
    }
}

#[cfg(test)]
mod angular_tests {
    use super::*;

    #[test]
    fn angular_gradient_type_conversion_preserves_latest_colors_stops_and_alpha() {
        let mut fill = Fill::solid(Color::BLACK);
        assert!(apply_paint_edit(&mut fill, PaintEdit::Solid(Color::rgba(11, 22, 33, 44))));
        assert!(apply_paint_edit(&mut fill, PaintEdit::Kind(3)));
        let Paint::AngularGradient { stops, .. } = &fill.paint else { panic!("angular type must create an angular paint") };
        assert_eq!(stops[0].color, Color::rgba(11, 22, 33, 44));
        assert!(apply_paint_edit(&mut fill, PaintEdit::AddStop));
        assert!(apply_paint_edit(&mut fill, PaintEdit::StopColor(1, Color::rgb(1, 2, 3))));
        let Paint::AngularGradient { stops, .. } = &fill.paint else { unreachable!() };
        let expected = stops.clone();
        assert!(apply_paint_edit(&mut fill, PaintEdit::Kind(1)));
        assert!(apply_paint_edit(&mut fill, PaintEdit::Kind(3)));
        let Paint::AngularGradient { stops, .. } = &fill.paint else { unreachable!() };
        assert_eq!(*stops, expected);
        assert!(!apply_paint_edit(&mut fill, PaintEdit::Kind(3)), "same type leaves geometry unchanged");
    }

    #[test]
    fn angular_gradient_fields_reject_nonfinite_values_and_preserve_noops() {
        let mut fill = Fill::solid(Color::BLACK);
        apply_paint_edit(&mut fill, PaintEdit::Kind(3));
        assert!(apply_paint_edit(&mut fill, PaintEdit::Coordinate(Coordinate::Angle, -90.0)));
        assert!(apply_paint_edit(&mut fill, PaintEdit::Coordinate(Coordinate::CenterX, 0.25)));
        let before = fill.clone();
        for value in [f64::NAN, f64::INFINITY, -1001.0, 1001.0] {
            assert!(!apply_paint_edit(&mut fill, PaintEdit::Coordinate(Coordinate::Angle, value)));
            assert_eq!(fill, before);
        }
        assert!(!apply_paint_edit(&mut fill, PaintEdit::Coordinate(Coordinate::Angle, -90.0)));
        assert!(!apply_paint_edit(&mut fill, PaintEdit::Coordinate(Coordinate::Radius, 2.0)));
        assert_eq!(fill, before);
    }

    #[test]
    fn angular_gradient_css_reports_center_angle_all_colors_and_hard_stops() {
        let mut node = Node::new(NodeKind::Rectangle, "Angular CSS", 0.0, 0.0, 100.0, 100.0);
        node.fills = vec![Fill {
            paint: Paint::AngularGradient { center: Point::new(0.25, 0.75), angle: -90.0,
                stops: vec![GradientStop { offset: 0.0, color: Color::BLACK },
                    GradientStop { offset: 0.5, color: Color::WHITE },
                    GradientStop { offset: 0.5, color: Color::rgba(255, 0, 0, 128) }] },
            ..Fill::solid(Color::BLACK)
        }];
        let css = css(&node);
        assert!(css.contains("conic-gradient(from 270deg at 25% 75%"), "{css}");
        assert!(css.contains("#000000 0%, #FFFFFF 50%, rgba(255,0,0,0.50196078) 50%"), "{css}");
        assert!(!css.contains("approximated"), "Hard transitions need no interpolation samples");
    }

    #[test]
    fn angular_gradient_css_single_stop_is_a_valid_constant_with_fill_opacity() {
        let mut node = Node::new(NodeKind::Rectangle, "One stop", 0.0, 0.0, 100.0, 100.0);
        node.fills = vec![Fill {
            paint: Paint::AngularGradient { center: Point::new(0.2, 0.8), angle: 45.0,
                stops: vec![GradientStop { offset: 0.37, color: Color::rgba(12, 34, 56, 128) }] },
            opacity: 0.5, visible: true,
        }];
        let css = css(&node);
        assert!(css.contains("background: rgba(12,34,56,0.25098039);"), "{css}");
        assert!(!css.contains("conic-gradient"), "A CSS gradient with only one stop is invalid");
    }

    #[test]
    fn angular_gradient_css_samples_match_native_straight_alpha_between_stops() {
        let stops = vec![GradientStop { offset: 0.0, color: Color::rgba(255, 0, 0, 0) },
            GradientStop { offset: 1.0, color: Color::rgb(0, 0, 255) }];
        for opacity in [1.0, 0.7] {
            let (samples, approximate) = angular_css_stops(&stops, opacity);
            assert!(approximate);
            for t in [0.0003, 0.0209, 0.314159, 0.50123, 0.73219, 0.9987] {
                let pair = samples.windows(2).find(|pair| pair[0].0 <= t && t <= pair[1].0).unwrap();
                let weight = (t - pair[0].0) / (pair[1].0 - pair[0].0);
                let alpha = pair[0].1[3] * (1.0 - weight) + pair[1].1[3] * weight;
                let native_alpha = t * opacity;
                let native_color = [255.0 * (1.0 - t), 0.0, 255.0 * t];
                assert!((alpha - native_alpha).abs() < 1e-12);
                // Evaluate CSS's premultiplied interpolation independently of
                // the sampling helper, on both opaque white and black.
                for background in [0.0, 255.0] {
                    for channel in 0..3 {
                        let css = pair[0].1[channel] * pair[0].1[3] * (1.0 - weight)
                            + pair[1].1[channel] * pair[1].1[3] * weight
                            + background * (1.0 - alpha);
                        let native = native_color[channel] * native_alpha + background * (1.0 - native_alpha);
                        assert!((css - native).abs() <= 0.25,
                            "CSS must remain within a quarter channel: t={t}, opacity={opacity}, css={css}, native={native}");
                    }
                }
            }
        }
        let mut node = Node::new(NodeKind::Rectangle, "Transparent sweep", 0.0, 0.0, 100.0, 100.0);
        node.fills = vec![Fill { paint: Paint::AngularGradient { center: Point::new(0.5, 0.5), angle: 0.0, stops },
            ..Fill::solid(Color::BLACK) }];
        let css = css(&node);
        assert!(css.contains("approximated with up to 1024 additional native-interpolated stops"));
        assert!(css.contains("rgba(127.5,0,127.5,0.5) 50%"), "{css}");
    }

    #[test]
    fn angular_gradient_css_preserves_authored_hard_stops_and_bounds_extra_work() {
        let hard = vec![GradientStop { offset: 1.0, color: Color::rgb(0, 0, 255) },
            GradientStop { offset: 0.5, color: Color::rgba(255, 0, 0, 0) },
            GradientStop { offset: 0.0, color: Color::BLACK },
            GradientStop { offset: 0.5, color: Color::rgba(0, 255, 0, 200) }];
        let (samples, approximate) = angular_css_stops(&hard, 1.0);
        assert!(approximate);
        let boundary: Vec<_> = samples.iter().filter(|(offset, _)| *offset == 0.5).map(|(_, color)| *color).collect();
        assert_eq!(boundary, vec![[255.0, 0.0, 0.0, 0.0], [0.0, 255.0, 0.0, 200.0 / 255.0]],
            "Stable sorting and inserted samples must preserve both authored sides of the hard transition");
        let many: Vec<_> = (0..2050).map(|index| GradientStop { offset: index as f64 / 2049.0,
            color: if index % 2 == 0 { Color::rgba(255, 0, 0, 0) } else { Color::rgb(0, 0, 255) } }).collect();
        let (samples, approximate) = angular_css_stops(&many, 1.0);
        assert!(approximate);
        assert!(samples.len() <= many.len() + 1024, "Additional interpolation work has a fixed budget");
        for stop in &many {
            assert!(samples.iter().any(|(offset, color)| *offset == stop.offset
                && *color == [stop.color.r as f64, stop.color.g as f64, stop.color.b as f64, stop.color.a as f64 / 255.0]),
                "The work limit must never discard authored stops");
        }
        let opaque = vec![GradientStop { offset: 0.0, color: Color::BLACK }, GradientStop { offset: 1.0, color: Color::WHITE }];
        let (samples, approximate) = angular_css_stops(&opaque, 0.4);
        assert_eq!(samples.len(), 2, "Uniform alpha already matches CSS interpolation");
        assert!(!approximate);
    }
}

#[cfg(test)]
mod angular_xaml_tests {
    use super::*;

    #[test]
    fn angular_gradient_xaml_retains_public_brush_coordinates_alpha_and_stroke() {
        let mut document = core::Document::new("Angular XAML");
        let mut node = Node::new(NodeKind::Rectangle, "Angular", 4.0, 8.0, 100.0, 70.0);
        let paint = Paint::AngularGradient { center: Point::new(0.25, 0.75), angle: 35.0,
            stops: vec![GradientStop { offset: 0.0, color: Color::rgba(20, 40, 60, 128) },
                GradientStop { offset: 0.5, color: Color::WHITE },
                GradientStop { offset: 0.5, color: Color::BLACK }] };
        node.fills = vec![Fill { paint: paint.clone(), opacity: 0.6, visible: true }];
        node.strokes = vec![core::Stroke { paint, opacity: 0.4, ..core::Stroke::new(Color::BLACK, 3.0) }];
        let id = document.insert_node(1, node).unwrap();
        let xaml = export_xaml(&document, &[id]).unwrap();
        assert!(xaml.contains("<Rectangle.Fill>"));
        assert!(xaml.contains("<Rectangle.Stroke>"));
        assert_eq!(xaml.matches("<ConicGradientBrush ").count(), 2);
        assert!(xaml.contains("Center=\"25%,75%\" Angle=\"35\" Opacity=\"0.6\""));
        assert!(xaml.contains("Center=\"25%,75%\" Angle=\"35\" Opacity=\"0.4\""));
        assert_eq!(xaml.matches("Offset=\"0\" Color=\"#8014283C\"").count(), 2);
        assert_eq!(xaml.matches("Offset=\"0.5\"").count(), 4, "duplicate offsets preserve hard color stops");
    }
}
