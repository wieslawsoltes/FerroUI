use super::*;

fn detents(height: f32) -> [f32; 3] {
    let bottom = (height - 88.0).max(24.0);
    [
        24.0_f32.min(bottom),
        (height * 0.46).clamp(24.0, bottom),
        bottom,
    ]
}

pub(super) fn build() -> Element {
    let scroll = Rc::new(RefCell::new(InertiaScroller::new(24.0, 400.0, 500.0)));
    let transform = Rc::new(RefCell::new(None::<Ref<MatrixTransform>>));
    let tracker = Rc::new(RefCell::new(GestureTracker::default()));
    let dragging = Rc::new(Cell::new(false));
    let last_y = Rc::new(Cell::new(0.0_f32));
    let selected = Rc::new(Cell::new(1usize));
    let extent = signal((780.0_f64, 520.0_f64));
    let handle_ref = NodeRef::<Border>::new();
    let title = signal(Some(String::from("Review the new workspace")));
    let notes = signal(Some(String::from(
        "Check the navigation, empty states, and keyboard flow.",
    )));
    let estimate = signal(4.0_f64);
    let progress = signal(40.0_f64);
    let completed = signal(Some(false));
    let status = signal(String::from(
        "Drag the drawer handle or choose a position. Edit the task while the drawer is moving.",
    ));
    let epoch = Instant::now();

    {
        let mut scroll = scroll.borrow_mut();
        scroll.detents = detents(520.0).to_vec();
        scroll.snap_spring = SpringConfig::gentle();
        let initial = scroll.detents[1];
        scroll.jump_to(initial);
    }

    let timer = DispatcherTimer::new();
    timer.set_interval(Duration::from_secs_f64(1.0 / 60.0));
    {
        let scroll = scroll.clone();
        let transform = transform.clone();
        let last = Cell::new(Instant::now());
        let previous = Cell::new(f32::NAN);
        timer.tick(move |_| {
            let dt = last
                .replace(Instant::now())
                .elapsed()
                .as_secs_f32()
                .clamp(0.001, 0.033);
            let y = scroll.borrow_mut().step(dt);
            if previous.get() != y {
                previous.set(y);
                if let Some(transform) = transform.borrow().as_ref() {
                    transform.set_matrix(Matrix::create_translation(0.0, y as f64));
                }
            }
        });
    }
    let timer_start = timer.clone();
    let timer_stop = timer.clone();
    on_cleanup(move || timer.stop());

    let mut positions = wrap_panel().item_spacing(8.0).line_spacing(8.0);
    for (index, label) in ["Expanded", "Half", "Collapsed"].into_iter().enumerate() {
        let scroll = scroll.clone();
        let selected = selected.clone();
        positions = positions.child(button().content(label).on_click(move |_| {
            let mut scroll = scroll.borrow_mut();
            let target = scroll.detents[index];
            scroll.animate_to(target);
            selected.set(index);
            status.set(format!("Drawer {}.", label.to_lowercase()));
        }));
    }

    let press = scroll.clone();
    let moved = scroll.clone();
    let released = scroll.clone();
    let cancelled = scroll.clone();
    let drag_p = dragging.clone();
    let drag_m = dragging.clone();
    let drag_r = dragging.clone();
    let drag_c = dragging.clone();
    let ly_p = last_y.clone();
    let ly_m = last_y.clone();
    let tr_p = tracker.clone();
    let tr_m = tracker.clone();
    let tr_r = tracker.clone();
    let selected_r = selected.clone();
    let header = surface()
        .padding((16.0, 12.0, 16.0, 12.0))
        .node_ref(handle_ref)
        .on_pointer_pressed(move |e| {
            if e.handled() {
                return;
            }
            if let Some(handle) = handle_ref.get_untracked() {
                let input: Ref<InputElement> = handle.upcast();
                e.pointer().capture(Some(&input));
            }
            let y = e.get_position(None).y as f32;
            ly_p.set(y);
            drag_p.set(true);
            let mut tracker = tr_p.borrow_mut();
            tracker.clear();
            tracker.push(epoch.elapsed().as_secs_f64(), Vec2::new(0.0, y));
            press.borrow_mut().begin_drag();
            e.set_handled(true);
        })
        .on_pointer_moved(move |e| {
            if !drag_m.get() {
                return;
            }
            let y = e.get_position(None).y as f32;
            let delta = y - ly_m.replace(y);
            tr_m.borrow_mut()
                .push(epoch.elapsed().as_secs_f64(), Vec2::new(0.0, y));
            moved.borrow_mut().drag_by(delta);
            e.set_handled(true);
        })
        .on_pointer_released(move |e| {
            if !drag_r.replace(false) {
                return;
            }
            let velocity = tr_r.borrow().velocity().y;
            let mut scroll = released.borrow_mut();
            scroll.end_drag(velocity);
            let target = scroll.projected_rest_position();
            if let Some((index, _)) = scroll
                .detents
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| (**a - target).abs().total_cmp(&(**b - target).abs()))
            {
                selected_r.set(index);
            }
            drop(scroll);
            e.pointer().capture(None);
            e.set_handled(true);
        })
        .on_pointer_capture_lost(move |_| {
            if drag_c.replace(false) {
                cancelled.borrow_mut().end_drag(0.0);
            }
        })
        .child(
            stack_panel()
                .spacing(8.0)
                .child(
                    border()
                        .height(4.0)
                        .width(40.0)
                        .corner_radius(2.0)
                        .opacity(0.35)
                        .horizontal_alignment(HorizontalAlignment::Center)
                        .background("#808080"),
                )
                .child(icon_label(Icons::SLIDER, "Task details", 16.0)),
        );

    let transform_mount = transform.clone();
    let collapse = scroll.clone();
    let collapsed_selection = selected.clone();
    let sheet = surface()
        // The moving sheet overlaps live content; a translucent card fill would show
        // the task overview through its labels and controls while expanded or dragged.
        .dynamic_resource(
            Border::background_property(),
            "SolidBackgroundFillColorQuarternaryBrush",
        )
        .canvas_left(16.0)
        .canvas_top(0.0)
        .width(move || (extent.get().0 - 32.0).max(240.0))
        .height(move || (extent.get().1 - 24.0).max(260.0))
        .padding(16.0)
        .with(move |b: &Ref<Border>| {
            let matrix = MatrixTransform::with_matrix(Matrix::IDENTITY);
            b.set_render_transform(Some((&matrix).into()));
            *transform_mount.borrow_mut() = Some(matrix);
        })
        .child(
            dock_panel().child(header.dock_panel_dock(Dock::Top)).child(
                scroll_viewer().content(
                    stack_panel()
                        .spacing(12.0)
                        .margin((0.0, 12.0, 0.0, 0.0))
                        .child(subtle_text().text("Task name"))
                        .child(text_box().text(title.two_way()))
                        .child(
                            check_box()
                                .content("Mark as complete")
                                .is_checked(completed.two_way()),
                        )
                        .child(
                            subtle_text()
                                .text(move || format!("Progress · {:.0}%", progress.get())),
                        )
                        .child(
                            slider()
                                .minimum(0.0)
                                .maximum(100.0)
                                .value(progress.two_way()),
                        )
                        .child(
                            subtle_text()
                                .text(move || format!("Estimate · {:.0} hours", estimate.get())),
                        )
                        .child(
                            slider()
                                .minimum(1.0)
                                .maximum(16.0)
                                .value(estimate.two_way()),
                        )
                        .child(subtle_text().text("Notes"))
                        .child(
                            text_box()
                                .text(notes.two_way())
                                .accepts_return(true)
                                .min_height(64.0),
                        )
                        .child(
                            button()
                                .content("Done")
                                .horizontal_alignment(HorizontalAlignment::Left)
                                .on_click(move |_| {
                                    let mut scroll = collapse.borrow_mut();
                                    let target = scroll.detents[2];
                                    scroll.animate_to(target);
                                    collapsed_selection.set(2);
                                    status.set("Task details updated in this workspace.".into());
                                }),
                        ),
                ),
            ),
        );

    let resize_scroll = scroll.clone();
    let resize_selected = selected.clone();
    let overview = stack_panel()
        .spacing(12.0)
        .margin(32.0)
        .max_width(460.0)
        .horizontal_alignment(HorizontalAlignment::Left)
        .vertical_alignment(VerticalAlignment::Top)
        .child(subtle_text().text("CURRENT TASK"))
        .child(
            text_block()
                .text(move || title.get().unwrap_or_default())
                .font_size(24.0)
                .font_weight(FontWeight::SemiBold)
                .text_wrapping(TextWrapping::Wrap),
        )
        .child(subtle_text().text(move || {
            if completed.get() == Some(true) {
                "Completed".into()
            } else {
                format!("In progress · {:.0} hours estimated", estimate.get())
            }
        }))
        .child(progress_bar().minimum(0.0).maximum(100.0).value(move || {
            if completed.get() == Some(true) {
                100.0
            } else {
                progress.get()
            }
        }))
        .child(subtle_text().text(move || notes.get().unwrap_or_default()));

    dock_panel()
        .margin(24.0)
        .on_loaded(move |_| timer_start.start())
        .on_unloaded(move |_| timer_stop.stop())
        .child(
            stack_panel()
                .dock_panel_dock(Dock::Top)
                .spacing(12.0)
                .margin((0.0, 0.0, 0.0, 16.0))
                .child(positions)
                .child(subtle_text().text(move || status.get())),
        )
        .child(
            surface()
                .clip_to_bounds(true)
                .on_size_changed(move |e| {
                    let size = e.new_size();
                    extent.set_if_changed((size.width, size.height));
                    let stops = detents(size.height as f32);
                    let mut scroll = resize_scroll.borrow_mut();
                    scroll.min_bound = stops[0];
                    scroll.max_bound = stops[2];
                    scroll.viewport_extent = size.height as f32;
                    scroll.detents.copy_from_slice(&stops);
                    scroll.animate_to(stops[resize_selected.get()]);
                })
                .child(
                    grid()
                        .child(overview)
                        .child(canvas().clip_to_bounds(true).child(sheet)),
                ),
        )
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drawer_detents_are_ordered_for_short_and_tall_workspaces() {
        for height in [20.0, 100.0, 320.0, 600.0, 1200.0] {
            let stops = detents(height);
            assert!(stops[0] <= stops[1] && stops[1] <= stops[2]);
            assert!(stops.iter().all(|s| s.is_finite()));
        }
    }
}
