use super::*;

pub(super) fn build() -> Element {
    let host = Rc::new(RefCell::new(VisualPhysicsHost::new(64.0)));
    let springs = Rc::new(RefCell::new(VisualSpringDriver::new()));
    let dock_springs = Rc::new(RefCell::new(VisualSpringDriver::new()));
    let approval_id = Rc::new(Cell::new(None::<SpringVisualId>));
    let canvas_ref = NodeRef::<Canvas>::new();
    let dock_ref = NodeRef::<StackPanel>::new();
    let attached = Rc::new(Cell::new(false));
    let width = signal(760.0_f64);
    let pin = signal(String::new());
    let approved = signal(false);
    let invalid = signal(false);
    let gravity = signal(false);
    let active_command = signal("Overview");
    let status = signal(String::from("Approve the sample review with code 1234. Other codes demonstrate accessible error feedback."));

    let timer = DispatcherTimer::new();
    timer.set_interval(Duration::from_secs_f64(1.0 / 60.0));
    {
        let host = host.clone();
        let springs = springs.clone();
        let dock_springs = dock_springs.clone();
        let previous = Cell::new(Instant::now());
        let last_size = Cell::new((0.0, 0.0));
        timer.tick(move |_| {
            let dt = previous
                .replace(Instant::now())
                .elapsed()
                .as_secs_f32()
                .clamp(0.001, 0.033);
            let mut host = host.borrow_mut();
            if let Some(canvas) = canvas_ref.get_untracked() {
                let bounds = canvas.bounds();
                if last_size.replace((bounds.width, bounds.height)) != (bounds.width, bounds.height)
                {
                    host.set_container_bounds(Rect::new(0.0, 0.0, bounds.width, bounds.height));
                    host.refresh_rest_layout();
                    host.reset_all_to_rest();
                }
            }
            host.step_and_sync(dt);
            springs.borrow_mut().step(dt);
            dock_springs.borrow_mut().step(dt);
        });
    }
    let timer_start = timer.clone();
    let timer_stop = timer.clone();
    on_cleanup(move || timer.stop());

    let mut keypad = stack_panel().spacing(8.0);
    for digits in [
        ["1", "2", "3"],
        ["4", "5", "6"],
        ["7", "8", "9"],
        ["Clear", "0", "Back"],
    ] {
        let mut row = stack_panel()
            .orientation(Orientation::Horizontal)
            .spacing(8.0);
        for digit in digits {
            let mount = springs.clone();
            let animate = springs.clone();
            let id = Rc::new(Cell::new(None::<SpringVisualId>));
            let press_id = id.clone();
            let form_id = approval_id.clone();
            row = row.child(button().width(72.0).height(40.0).content(digit)
                .with(move |button| {
                    let visual: Ref<Visual> = button.clone().upcast();
                    id.set(Some(mount.borrow_mut().attach(&visual, SpringConfig::snappy())));
                })
                .on_click(move |_| {
                    let mut driver = animate.borrow_mut();
                    if let Some(id) = press_id.get() { driver.impulse_pop(id, -1.5); }
                    let mut code = pin.get_untracked();
                    if approved.get_untracked() { approved.set(false); code.clear(); }
                    invalid.set(false);
                    match digit {
                        "Clear" => code.clear(),
                        "Back" => { code.pop(); }
                        _ => {
                            if code.len() < 4 { code.push_str(digit); }
                            if code.len() == 4 {
                                if code == "1234" {
                                    approved.set(true);
                                    status.set("Review approved. The activity summary has been updated.".into());
                                    if let Some(id) = form_id.get() { driver.impulse_pop(id, 1.8); }
                                } else {
                                    invalid.set(true);
                                    status.set("Code not recognized. Use the sample code 1234 to approve.".into());
                                    if let Some(id) = form_id.get() { driver.impulse_shake(id, 620.0); }
                                    code.clear();
                                }
                            }
                        }
                    }
                    pin.set(code);
                }));
        }
        keypad = keypad.child(row);
    }

    let form_mount = springs.clone();
    let form_id_mount = approval_id.clone();
    let form = surface()
        .canvas_left(16.0)
        .canvas_top(16.0)
        .width(288.0)
        .height(392.0)
        .padding(16.0)
        // Physics owns this wrapper's transform. The validation spring owns the
        // inner form, so the two animation systems cannot overwrite each other.
        .child(
            border()
                .with(move |b: &Ref<Border>| {
                    let visual: Ref<Visual> = b.clone().upcast();
                    form_id_mount.set(Some(
                        form_mount
                            .borrow_mut()
                            .attach(&visual, SpringConfig::snappy()),
                    ));
                })
                .child(
                    stack_panel()
                        .spacing(12.0)
                        .child(
                            text_block()
                                .text("Approve review")
                                .font_size(20.0)
                                .font_weight(FontWeight::SemiBold),
                        )
                        .child(subtle_text().text("Sample approval code: 1234"))
                        .child(
                            text_block()
                                .text(move || {
                                    if approved.get() {
                                        "Approved".to_owned()
                                    } else {
                                        (0..4)
                                            .map(|i| if i < pin.get().len() { '●' } else { '○' })
                                            .collect::<String>()
                                    }
                                })
                                .font_size(24.0),
                        )
                        .child(keypad)
                        .child(
                            subtle_text()
                                .text(move || {
                                    if invalid.get() {
                                        "Try again with 1234."
                                    } else {
                                        "Local interaction sample"
                                    }
                                })
                                .font_size(12.0),
                        ),
                ),
        );

    let summary = surface()
        .canvas_left(move || if width.get() >= 650.0 { 320.0 } else { 16.0 })
        .canvas_top(move || if width.get() >= 650.0 { 16.0 } else { 424.0 })
        .width(move || {
            if width.get() >= 650.0 {
                (width.get() - 336.0).max(280.0)
            } else {
                (width.get() - 32.0).max(280.0)
            }
        })
        .height(184.0)
        .padding(20.0)
        .child(
            stack_panel()
                .spacing(12.0)
                .child(
                    text_block()
                        .text("Review package")
                        .font_size(20.0)
                        .font_weight(FontWeight::SemiBold),
                )
                .child(subtle_text().text("Navigation changes, keyboard flow, and visual polish."))
                .child(progress_bar().minimum(0.0).maximum(100.0).value(move || {
                    if approved.get() {
                        100.0
                    } else {
                        75.0
                    }
                }))
                .child(subtle_text().text(move || {
                    if approved.get() {
                        "All review steps complete"
                    } else {
                        "3 of 4 review steps complete"
                    }
                })),
        );
    let activity = surface().canvas_left(move || if width.get() >= 650.0 { 320.0 } else { 16.0 })
        .canvas_top(move || if width.get() >= 650.0 { 216.0 } else { 624.0 })
        .width(move || if width.get() >= 650.0 { (width.get() - 336.0).max(280.0) } else { (width.get() - 32.0).max(280.0) })
        .height(192.0).padding(20.0)
        .child(stack_panel().spacing(12.0)
            .child(text_block().text(move || active_command.get()).font_size(20.0).font_weight(FontWeight::SemiBold))
            .child(subtle_text().text(move || match active_command.get() {
                "Activity" => if approved.get() { "Review approved. All checks are complete." } else { "Waiting for the final review approval." },
                "Notes" => "Use clear labels, visible focus, and motion that explains a state change.",
                "Settings" => "This workspace follows the application theme. Gravity is an optional interaction experiment.",
                _ => "Use the magnetic command bar below to switch this panel. Its controls remain usable while cards move.",
            })));

    let attach: Rc<dyn Fn()> = {
        let host = host.clone();
        let attached = attached.clone();
        Rc::new(move || {
            if attached.get() {
                return;
            }
            if let Some(canvas) = canvas_ref.get_untracked() {
                let mut host = host.borrow_mut();
                let bounds = canvas.bounds();
                host.set_container_bounds(Rect::new(0.0, 0.0, bounds.width, bounds.height));
                let styled: &StyledElement = &canvas;
                host.attach_logical_children(styled, VisualBodyOptions::dynamic_card());
                attached.set(true);
            }
        })
    };
    let gravity_host = host.clone();
    let gravity_attach = attach.clone();
    let blast_host = host.clone();
    let blast_attach = attach.clone();
    let reset_host = host.clone();
    let toolbar = wrap_panel()
        .item_spacing(8.0)
        .line_spacing(8.0)
        .child(
            toggle_button()
                .content("Gravity experiment")
                .is_checked(move || Some(gravity.get()))
                .on_click(move |_| {
                    gravity_attach();
                    let enabled = !gravity.get_untracked();
                    gravity.set(enabled);
                    let mut host = gravity_host.borrow_mut();
                    host.world.gravity = if enabled {
                        Vec2::new(0.0, 10.0)
                    } else {
                        Vec2::ZERO
                    };
                    if enabled {
                        host.disable_home_springs();
                    } else {
                        host.enable_home_springs(3.8, 0.8);
                    }
                    host.world.wake_all();
                }),
        )
        .child(button().content("Impulse").on_click(move |_| {
            blast_attach();
            let mut host = blast_host.borrow_mut();
            host.apply_explosion_px(Vec2::new(320.0, 200.0), 600.0, 9.0);
            host.world.wake_all();
        }))
        .child(button().content("Restore layout").on_click(move |_| {
            gravity.set(false);
            let mut host = reset_host.borrow_mut();
            host.world.gravity = Vec2::ZERO;
            host.enable_home_springs(3.8, 0.8);
            host.reset_all_to_rest();
        }));

    let mut commands = stack_panel()
        .orientation(Orientation::Horizontal)
        .spacing(12.0)
        .horizontal_alignment(HorizontalAlignment::Center);
    for label in ["Overview", "Activity", "Notes", "Settings"] {
        let mount = dock_springs.clone();
        let animate = dock_springs.clone();
        let id = Rc::new(Cell::new(None::<SpringVisualId>));
        let clicked_id = id.clone();
        commands = commands.child(
            toggle_button()
                .content(label)
                .is_checked(move || Some(active_command.get() == label))
                .with(move |button| {
                    let visual: Ref<Visual> = button.clone().upcast();
                    let mut driver = mount.borrow_mut();
                    let sid = driver.attach(&visual, SpringConfig::gentle());
                    driver.configure_magnetic_hover(sid, 100.0, 5.0, 0.04);
                    id.set(Some(sid));
                })
                .on_click(move |_| {
                    active_command.set(label);
                    if let Some(id) = clicked_id.get() {
                        animate.borrow_mut().impulse_pop(id, 1.8);
                    }
                }),
        );
    }
    let hover = dock_springs.clone();
    let leave = dock_springs.clone();
    let command_bar = surface()
        .dock_panel_dock(Dock::Bottom)
        .padding(16.0)
        .margin((0.0, 12.0, 0.0, 0.0))
        .child(
            commands
                .node_ref(dock_ref)
                .on_pointer_moved(move |e| {
                    if let Some(parent) = dock_ref.get_untracked() {
                        let point = e.get_position(Some(&parent));
                        hover
                            .borrow_mut()
                            .update_pointer_in_parent(Vec2::new(point.x as f32, point.y as f32));
                    }
                })
                .on_pointer_exited(move |_| {
                    leave
                        .borrow_mut()
                        .update_pointer_in_parent(Vec2::new(-10000.0, -10000.0))
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
        .child(command_bar)
        .child(
            surface()
                .clip_to_bounds(true)
                .on_size_changed(move |e| width.set_if_changed(e.new_size().width))
                .child(
                    scroll_viewer().content(
                        canvas()
                            .node_ref(canvas_ref)
                            .height(move || if width.get() >= 650.0 { 440.0 } else { 848.0 })
                            .width(move || width.get().max(320.0))
                            .clip_to_bounds(true)
                            .child(form)
                            .child(summary)
                            .child(activity),
                    ),
                ),
        )
        .into()
}
