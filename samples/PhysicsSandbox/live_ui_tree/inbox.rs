use super::*;

const ROW_COUNT: usize = 5;
const ROW_HEIGHT: f32 = 92.0;
const TASKS: [(&str, &str); ROW_COUNT] = [
    (
        "Review the navigation flow",
        "Design review · Ready for feedback",
    ),
    ("Polish the empty state", "Workspace · Due today"),
    ("Check keyboard navigation", "Accessibility · Follow-up"),
    ("Prepare the release notes", "Documentation · Draft"),
    ("Schedule a usability session", "Research · Next week"),
];

fn row_offsets(archived: &[bool; ROW_COUNT]) -> [f32; ROW_COUNT] {
    let mut removed = 0;
    std::array::from_fn(|i| {
        let offset = -(removed as f32) * ROW_HEIGHT;
        if archived[i] {
            removed += 1;
        }
        offset
    })
}

pub(super) fn build() -> Element {
    let archived = signal([false; ROW_COUNT]);
    let hidden = signal([false; ROW_COUNT]);
    let history = Rc::new(RefCell::new(Vec::<usize>::new()));
    let xs = Rc::new(RefCell::new(std::array::from_fn::<_, ROW_COUNT, _>(|_| {
        Spring1D::new(0.0, SpringConfig::snappy())
    })));
    let ys = Rc::new(RefCell::new(std::array::from_fn::<_, ROW_COUNT, _>(|_| {
        Spring1D::new(0.0, SpringConfig::gentle())
    })));
    let transforms = Rc::new(RefCell::new(std::array::from_fn::<_, ROW_COUNT, _>(|_| {
        None::<Ref<MatrixTransform>>
    })));
    let width = signal(700.0_f64);
    let dragging = Rc::new(Cell::new(None::<(usize, f32, f32)>));
    let tracker = Rc::new(RefCell::new(GestureTracker::default()));
    let epoch = Instant::now();
    let status = signal(String::from(
        "Swipe a task left to archive it, or use its Archive button. Undo restores the last task.",
    ));

    let archive: Rc<dyn Fn(usize)> = {
        let xs = xs.clone();
        let ys = ys.clone();
        let history = history.clone();
        Rc::new(move |index| {
            let mut next = archived.get_untracked();
            if next[index] {
                return;
            }
            next[index] = true;
            history.borrow_mut().push(index);
            archived.set(next);
            xs.borrow_mut()[index].set_target(-(width.get_untracked() as f32 + 48.0));
            let offsets = row_offsets(&next);
            let mut springs = ys.borrow_mut();
            for i in 0..ROW_COUNT {
                if !next[i] {
                    springs[i].set_target(offsets[i]);
                }
            }
            status.set(format!("Archived “{}”. Undo is available.", TASKS[index].0));
        })
    };

    let timer = DispatcherTimer::new();
    timer.set_interval(Duration::from_secs_f64(1.0 / 60.0));
    {
        let xs = xs.clone();
        let ys = ys.clone();
        let transforms = transforms.clone();
        let last = Cell::new(Instant::now());
        timer.tick(move |_| {
            let dt = last
                .replace(Instant::now())
                .elapsed()
                .as_secs_f32()
                .clamp(0.001, 0.033);
            let mut x = xs.borrow_mut();
            let mut y = ys.borrow_mut();
            let transforms = transforms.borrow();
            let dismissed = archived.get_untracked();
            let mut invisible = hidden.get_untracked();
            let mut changed = false;
            for i in 0..ROW_COUNT {
                if !x[i].is_settled() || !y[i].is_settled() {
                    let next_x = x[i].step(dt);
                    let next_y = y[i].step(dt);
                    if let Some(transform) = &transforms[i] {
                        transform
                            .set_matrix(Matrix::create_translation(next_x as f64, next_y as f64));
                    }
                }
                if dismissed[i] && x[i].is_settled() && !invisible[i] {
                    invisible[i] = true;
                    changed = true;
                }
            }
            if changed {
                hidden.set(invisible);
            }
        });
    }
    let timer_start = timer.clone();
    let timer_stop = timer.clone();
    on_cleanup(move || timer.stop());

    let mut rows = canvas()
        .height(ROW_HEIGHT as f64 * ROW_COUNT as f64 + 32.0)
        .clip_to_bounds(true);
    for (index, (title, detail)) in TASKS.into_iter().enumerate() {
        let row_ref = NodeRef::<Border>::new();
        let transforms = transforms.clone();
        let press_x = xs.clone();
        let move_x = xs.clone();
        let release_x = xs.clone();
        let cancel_x = xs.clone();
        let press_drag = dragging.clone();
        let move_drag = dragging.clone();
        let release_drag = dragging.clone();
        let cancel_drag = dragging.clone();
        let press_tracker = tracker.clone();
        let move_tracker = tracker.clone();
        let release_tracker = tracker.clone();
        let archive_release = archive.clone();
        let archive_click = archive.clone();
        let row = surface()
            .canvas_left(16.0)
            .canvas_top(16.0 + index as f64 * ROW_HEIGHT as f64)
            .width(move || width.get())
            .height(80.0)
            .padding((16.0, 12.0, 16.0, 12.0))
            .is_visible(move || !hidden.get()[index])
            .is_hit_test_visible(move || !archived.get()[index])
            .node_ref(row_ref)
            .with(move |b: &Ref<Border>| {
                let transform = MatrixTransform::with_matrix(Matrix::IDENTITY);
                b.set_render_transform(Some((&transform).into()));
                transforms.borrow_mut()[index] = Some(transform);
            })
            .on_pointer_pressed(move |e| {
                if e.handled() {
                    return;
                }
                if let Some(row) = row_ref.get_untracked() {
                    let input: Ref<InputElement> = row.upcast();
                    e.pointer().capture(Some(&input));
                }
                let x = e.get_position(None).x as f32;
                press_drag.set(Some((index, x, press_x.borrow()[index].value)));
                let mut tracker = press_tracker.borrow_mut();
                tracker.clear();
                tracker.push(epoch.elapsed().as_secs_f64(), Vec2::new(x, 0.0));
                e.set_handled(true);
            })
            .on_pointer_moved(move |e| {
                let Some((active, start, offset)) = move_drag.get() else {
                    return;
                };
                if active != index {
                    return;
                }
                let x = e.get_position(None).x as f32;
                move_tracker
                    .borrow_mut()
                    .push(epoch.elapsed().as_secs_f64(), Vec2::new(x, 0.0));
                let raw = offset + x - start;
                let position = if raw > 0.0 {
                    InertiaScroller::rubber_band(raw, 110.0, 0.55)
                } else {
                    raw
                };
                move_x.borrow_mut()[index].set_target(position);
                e.set_handled(true);
            })
            .on_pointer_released(move |e| {
                if release_drag.get().map(|d| d.0) != Some(index) {
                    return;
                }
                release_drag.set(None);
                let velocity = release_tracker.borrow().velocity().x;
                let position = release_x.borrow()[index].target;
                if position < -64.0 && position + velocity * 0.12 < -112.0 {
                    archive_release(index);
                } else {
                    release_x.borrow_mut()[index].set_target(0.0);
                }
                e.pointer().capture(None);
                e.set_handled(true);
            })
            .on_pointer_capture_lost(move |_| {
                if cancel_drag.get().map(|d| d.0) == Some(index) {
                    cancel_drag.set(None);
                    cancel_x.borrow_mut()[index].set_target(0.0);
                }
            })
            .child(
                dock_panel()
                    .child(
                        button()
                            .dock_panel_dock(Dock::Right)
                            .vertical_alignment(VerticalAlignment::Center)
                            .margin((16.0, 0.0, 0.0, 0.0))
                            .content("Archive")
                            .on_click(move |_| archive_click(index)),
                    )
                    .child(
                        stack_panel()
                            .spacing(4.0)
                            .child(
                                text_block()
                                    .text(title)
                                    .font_size(14.0)
                                    .font_weight(FontWeight::SemiBold),
                            )
                            .child(subtle_text().text(detail).font_size(12.0)),
                    ),
            );
        rows = rows.child(row);
    }

    let undo_x = xs.clone();
    let undo_y = ys.clone();
    let undo_history = history.clone();
    let restore_x = xs.clone();
    let restore_y = ys.clone();
    let restore_history = history.clone();
    let toolbar = wrap_panel()
        .item_spacing(12.0)
        .line_spacing(12.0)
        .child(
            text_block()
                .text(move || {
                    format!(
                        "{} tasks in your inbox",
                        archived.get().iter().filter(|&&a| !a).count()
                    )
                })
                .font_size(16.0)
                .font_weight(FontWeight::SemiBold)
                .vertical_alignment(VerticalAlignment::Center),
        )
        .child(
            button()
                .content("Undo archive")
                .is_enabled(move || archived.get().iter().any(|&a| a))
                .on_click(move |_| {
                    if let Some(index) = undo_history.borrow_mut().pop() {
                        archived.update(|a| a[index] = false);
                        hidden.update(|a| a[index] = false);
                        undo_x.borrow_mut()[index].set_target(0.0);
                        let offsets = row_offsets(&archived.get_untracked());
                        for (spring, target) in undo_y.borrow_mut().iter_mut().zip(offsets) {
                            spring.set_target(target);
                        }
                        status.set(format!("Restored “{}”.", TASKS[index].0));
                    }
                }),
        )
        .child(
            button()
                .content("Restore all")
                .is_enabled(move || archived.get().iter().any(|&a| a))
                .on_click(move |_| {
                    archived.set([false; ROW_COUNT]);
                    hidden.set([false; ROW_COUNT]);
                    restore_history.borrow_mut().clear();
                    for spring in restore_x.borrow_mut().iter_mut() {
                        spring.set_target(0.0);
                    }
                    for spring in restore_y.borrow_mut().iter_mut() {
                        spring.set_target(0.0);
                    }
                    status.set("All tasks restored.".into());
                }),
        );

    dock_panel()
        .margin(24.0)
        .on_loaded(move |_| timer_start.start())
        .on_unloaded(move |_| timer_stop.stop())
        .child(
            stack_panel()
                .dock_panel_dock(Dock::Top)
                .spacing(12.0)
                .margin((0.0, 0.0, 0.0, 16.0))
                .child(toolbar)
                .child(subtle_text().text(move || status.get())),
        )
        .child(
            surface()
                .clip_to_bounds(true)
                .on_size_changed(move |e| {
                    width.set_if_changed((e.new_size().width - 32.0).max(160.0))
                })
                .child(
                    scroll_viewer().content(
                        grid().child(rows).child(
                            stack_panel()
                                .is_visible(move || archived.get().iter().all(|&a| a))
                                .horizontal_alignment(HorizontalAlignment::Center)
                                .vertical_alignment(VerticalAlignment::Center)
                                .spacing(12.0)
                                .child(
                                    text_block()
                                        .text("You’re all caught up")
                                        .font_size(24.0)
                                        .font_weight(FontWeight::SemiBold),
                                )
                                .child(
                                    subtle_text().text("Your archived tasks are one Undo away."),
                                ),
                        ),
                    ),
                ),
        )
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archiving_compacts_only_the_rows_below_it() {
        assert_eq!(
            row_offsets(&[false, true, false, true, false]),
            [0.0, 0.0, -92.0, -92.0, -184.0]
        );
        assert_eq!(row_offsets(&[false; ROW_COUNT]), [0.0; ROW_COUNT]);
    }
}
