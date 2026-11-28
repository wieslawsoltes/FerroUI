use super::*;

const PANEL_WIDTH: f32 = 280.0;
const PANEL_HEIGHT: f32 = 300.0;
const INSET: f32 = 24.0;

/// Render-transform offsets that keep the scaled panel inside its workspace.
fn corner_offsets(width: f32, height: f32, scale: f32) -> [Vec2; 4] {
    let extra_x = PANEL_WIDTH * (scale - 1.0) * 0.5;
    let extra_y = PANEL_HEIGHT * (scale - 1.0) * 0.5;
    let left = extra_x.max(0.0);
    let top = extra_y.max(0.0);
    let right = (width - INSET * 2.0 - PANEL_WIDTH - extra_x).max(left);
    let bottom = (height - INSET * 2.0 - PANEL_HEIGHT - extra_y).max(top);
    [
        Vec2::new(left, top),
        Vec2::new(right, top),
        Vec2::new(left, bottom),
        Vec2::new(right, bottom),
    ]
}

pub(super) fn build() -> Element {
    let controller = Rc::new(RefCell::new(None::<DraggableVisual>));
    let size_spring = Rc::new(RefCell::new(Spring1D::new(1.0, SpringConfig::gentle())));
    let extent = Rc::new(Cell::new((900.0_f32, 480.0_f32)));
    let drag_handle = NodeRef::<Border>::new();
    let selected_corner = signal(1usize);
    let size_mode = signal(1u8);
    let name = signal(Some(String::from("Project overview")));
    let opacity = signal(100.0_f64);
    let radius = signal(8.0_f64);
    let status = signal(String::from(
        "Drag the inspector header to a corner. Its controls continue to edit the preview.",
    ));

    let timer = DispatcherTimer::new();
    timer.set_interval(Duration::from_secs_f64(1.0 / 60.0));
    {
        let controller = controller.clone();
        let size_spring = size_spring.clone();
        let extent = extent.clone();
        let last = Cell::new(Instant::now());
        timer.tick(move |_| {
            let dt = last
                .replace(Instant::now())
                .elapsed()
                .as_secs_f32()
                .clamp(0.001, 0.033);
            let scale = size_spring.borrow_mut().step(dt);
            let (width, height) = extent.get();
            let scale = scale
                .min(((width - 48.0) / PANEL_WIDTH).max(0.25))
                .min(((height - 48.0) / PANEL_HEIGHT).max(0.25));
            if let Some(p) = controller.borrow_mut().as_mut() {
                let anchors = corner_offsets(width, height, scale);
                p.snap.anchors.copy_from_slice(&anchors);
                if !p.card.is_dragging {
                    p.card
                        .position
                        .set_target(anchors[selected_corner.get_untracked()]);
                }
                p.card.scale.set_target(scale);
                p.step(dt);
            }
        });
    }
    let timer_start = timer.clone();
    let timer_stop = timer.clone();
    on_cleanup(move || timer.stop());

    let mut position_buttons = wrap_panel().item_spacing(8.0).line_spacing(8.0);
    for (index, label) in ["Top left", "Top right", "Bottom left", "Bottom right"]
        .into_iter()
        .enumerate()
    {
        position_buttons = position_buttons.child(
            toggle_button()
                .content(label)
                .is_checked(move || Some(selected_corner.get() == index))
                .on_click(move |_| {
                    selected_corner.set(index);
                    status.set(format!("Inspector moved to {}.", label.to_lowercase()));
                }),
        );
    }
    let mut size_buttons = stack_panel()
        .orientation(Orientation::Horizontal)
        .spacing(8.0);
    for (index, label, scale) in [(0, "Compact", 0.88), (1, "Normal", 1.0), (2, "Large", 1.12)] {
        let spring = size_spring.clone();
        size_buttons = size_buttons.child(
            toggle_button()
                .content(label)
                .is_checked(move || Some(size_mode.get() == index))
                .on_click(move |_| {
                    size_mode.set(index);
                    spring.borrow_mut().set_target(scale);
                }),
        );
    }

    let down = controller.clone();
    let moved = controller.clone();
    let up = controller.clone();
    let lost = controller.clone();
    let last_move = Cell::new(Instant::now());
    let header = surface()
        .padding((12.0, 8.0, 12.0, 8.0))
        .node_ref(drag_handle)
        .on_pointer_pressed(move |e| {
            if e.handled() {
                return;
            }
            if let Some(handle) = drag_handle.get_untracked() {
                let input: Ref<InputElement> = handle.upcast();
                e.pointer().capture(Some(&input));
            }
            let pt = e.get_position(None);
            if let Some(p) = down.borrow_mut().as_mut() {
                p.begin_drag(Vec2::new(pt.x as f32, pt.y as f32));
            }
            e.set_handled(true);
        })
        .on_pointer_moved(move |e| {
            let pt = e.get_position(None);
            if let Some(p) = moved.borrow_mut().as_mut() {
                if !p.card.is_dragging {
                    return;
                }
                let dt = last_move
                    .replace(Instant::now())
                    .elapsed()
                    .as_secs_f32()
                    .clamp(0.001, 0.05);
                p.drag_to(Vec2::new(pt.x as f32, pt.y as f32), dt);
                if let Some((index, _)) = p
                    .snap
                    .choose_anchor(p.card.position.value(), p.card.position.velocity())
                {
                    selected_corner.set_if_changed(index);
                }
                e.set_handled(true);
            }
        })
        .on_pointer_released(move |e| {
            if let Some(p) = up.borrow_mut().as_mut() {
                p.end_drag();
            }
            e.pointer().capture(None);
            e.set_handled(true);
            status.set("Inspector snapped into place. You can also use the corner buttons.".into());
        })
        .on_pointer_capture_lost(move |_| {
            if let Some(p) = lost.borrow_mut().as_mut() {
                p.end_drag();
            }
        })
        .child(icon_label(Icons::SLIDER, "Appearance inspector", 16.0));

    let mount = controller.clone();
    let panel = surface()
        // A floating inspector can cover the document; keep its controls opaque
        // while ordinary surfaces retain the theme's translucent card treatment.
        .dynamic_resource(
            Border::background_property(),
            "SolidBackgroundFillColorQuarternaryBrush",
        )
        .canvas_left(INSET as f64)
        .canvas_top(INSET as f64)
        .width(PANEL_WIDTH as f64)
        .height(PANEL_HEIGHT as f64)
        .padding(16.0)
        .with(move |b: &Ref<Border>| {
            let visual: Ref<Visual> = b.clone().upcast();
            let mut p =
                DraggableVisual::attach(&visual, SpringConfig::gentle(), vec![Vec2::ZERO; 4]);
            p.snap.velocity_projection = 0.18;
            p.card.position.snap(Vec2::new(560.0, 0.0));
            *mount.borrow_mut() = Some(p);
        })
        .child(
            stack_panel()
                .spacing(12.0)
                .child(header)
                .child(text_box().text(name.two_way()))
                .child(subtle_text().text(move || format!("Opacity · {:.0}%", opacity.get())))
                .child(
                    slider()
                        .minimum(20.0)
                        .maximum(100.0)
                        .value(opacity.two_way()),
                )
                .child(
                    subtle_text().text(move || format!("Corner radius · {:.0} px", radius.get())),
                )
                .child(slider().minimum(0.0).maximum(28.0).value(radius.two_way()))
                .child(size_buttons),
        );

    let extent_changed = extent.clone();
    let preview = surface().padding(24.0).max_width(420.0)
        .horizontal_alignment(HorizontalAlignment::Center).vertical_alignment(VerticalAlignment::Center)
        .opacity(move || opacity.get() / 100.0).corner_radius(move || radius.get())
        .child(stack_panel().spacing(12.0)
            .child(text_block().text(move || name.get().unwrap_or_default()).font_size(24.0).font_weight(FontWeight::SemiBold).text_wrapping(TextWrapping::Wrap))
            .child(subtle_text().text("A live document preview. Edit its title, opacity, and corners using the floating inspector."))
            .child(progress_bar().minimum(0.0).maximum(100.0).value(72.0))
            .child(subtle_text().text("Review progress · 72%")));

    dock_panel()
        .margin(24.0)
        .on_loaded(move |_| timer_start.start())
        .on_unloaded(move |_| timer_stop.stop())
        .child(
            stack_panel()
                .dock_panel_dock(Dock::Top)
                .spacing(12.0)
                .margin((0.0, 0.0, 0.0, 16.0))
                .child(position_buttons)
                .child(subtle_text().text(move || status.get())),
        )
        .child(
            surface()
                .clip_to_bounds(true)
                .on_size_changed(move |e| {
                    let s = e.new_size();
                    extent_changed.set((s.width as f32, s.height as f32));
                })
                .child(
                    grid()
                        .child(preview)
                        .child(canvas().clip_to_bounds(true).child(panel)),
                ),
        )
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaled_inspector_keeps_its_corners_inside_workspace() {
        let anchors = corner_offsets(900.0, 600.0, 1.12);
        for a in anchors {
            assert!(a.x >= 0.0 && a.y >= 0.0);
            assert!(a.x + PANEL_WIDTH * 1.06 + INSET <= 900.0);
            assert!(a.y + PANEL_HEIGHT * 1.06 + INSET <= 600.0);
        }
    }
}
