//! **Real-Life UI/UX Physics Showcase on Live FerroUI Controls**
//!
//! Applies `ferroui_physics::ferroui` (`VisualPhysicsHost`, `VisualSpringDriver`, `DraggableVisual`)
//! directly to **real interactive FerroUI visual & logical tree controls** with zero layout passes:
//! 1. **iOS / macOS Picture-in-Picture (PiP) Floating Player**: Drag & fling anywhere — velocity
//!    projection (`MagneticSnap`) snaps it smoothly into the 4 corners with velocity-coupled tilt!
//! 2. **Swipe-to-Action Inbox Notifications**: Drag real notification rows left/right with
//!    rubber-band resistance, archive snap, or spring back; push new alerts with spring entrance!
//! 3. **3-Detent Interactive Bottom Sheet (Apple Maps / Spotify UX)**: Smooth analytical `Spring1D`
//!    detent transitions (Collapsed / Half / Expanded) with live interactive controls inside!
//! 4. **macOS Passcode / Biometric Validation Card**: Wrong PIN triggers damped horizontal
//!    error-shake (`impulse_shake`); correct PIN triggers emerald celebratory spring pop (`impulse_pop`)!
//! 5. **macOS Magnetic Hover Dock + Live UI Chaos Gravity**: Real buttons magnetically pull & tilt
//!    toward the cursor, plus a one-click toggle to drop the entire logical tree under rigid-body gravity!

use ferroui_base::layout::{HorizontalAlignment, Orientation};
use ferroui_base::media::MatrixTransform;
use ferroui_base::threading::DispatcherTimer;
use ferroui_base::{Matrix, Rect, Ref, RelativePoint, StyledElement, Visual};
use ferroui_controls::{Border, Canvas, StackPanel};
use ferroui_physics::prelude::*;
use ferroui_view::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SheetDetent {
    Collapsed,
    Half,
    Expanded,
}

impl SheetDetent {
    fn offset_y(self) -> f32 {
        match self {
            Self::Collapsed => 112.0,
            Self::Half => 46.0,
            Self::Expanded => 0.0,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Collapsed => "Collapsed",
            Self::Half => "Half",
            Self::Expanded => "Expanded",
        }
    }
}

/// Builds the real-life FerroUI Visual/Logical Tree Physics UX showcase.
pub fn live_ui_tree_showcase() -> Element {
    let host = Rc::new(RefCell::new(VisualPhysicsHost::new(64.0)));
    let springs = Rc::new(RefCell::new(VisualSpringDriver::new()));

    // PiP draggable video player controller
    let pip_ctrl: Rc<RefCell<Option<DraggableVisual>>> = Rc::new(RefCell::new(None));
    let pip_playing = signal(true);
    let pip_muted = signal(false);

    // 3-Detent Bottom Sheet analytical spring
    let sheet_detent = signal(SheetDetent::Half);
    let sheet_spring = Rc::new(RefCell::new(Spring1D::new(
        SheetDetent::Half.offset_y(),
        SpringConfig::from_response_damping(0.36, 0.82),
    )));
    let sheet_tf: Rc<RefCell<Option<Ref<MatrixTransform>>>> = Rc::new(RefCell::new(None));

    // Swipe-to-Archive Inbox rows (3 real interactive rows driven by Spring1D!)
    let swipe_offsets: Rc<RefCell<[Spring1D; 3]>> = Rc::new(RefCell::new([
        Spring1D::new(0.0, SpringConfig::snappy()),
        Spring1D::new(0.0, SpringConfig::snappy()),
        Spring1D::new(0.0, SpringConfig::snappy()),
    ]));
    let swipe_tfs: Rc<RefCell<[Option<Ref<MatrixTransform>>; 3]>> = Rc::new(RefCell::new([None, None, None]));
    let swipe_drag: Rc<RefCell<Option<(usize, f32, f32)>>> = Rc::new(RefCell::new(None));
    let archived = [signal(false), signal(false), signal(false)];

    // Passcode UX state
    let pin_status = signal(String::from("Enter PIN: try Wrong (shake) or 4242 (unlock)"));
    let pin_unlocked = signal(false);
    let pin_card_sid = Rc::new(RefCell::new(None::<SpringVisualId>));

    // Rigid-body Chaos Gravity toggle on logical tree cards
    let gravity_on = signal(false);
    let audio_vol = signal(72.0_f64);
    let ux_log = signal(String::from("Fling the PiP player to any corner, swipe inbox rows, toggle Bottom Sheet detents, or test PIN shake!"));
    let perf_chip = signal(String::from("Zero Layout Passes · Persistent MatrixTransform Sync"));

    let stage_ref = NodeRef::<Canvas>::new();
    let dock_ref = NodeRef::<StackPanel>::new();
    let rigid_attached = Rc::new(RefCell::new(false));
    let dock_sids = Rc::new(RefCell::new(Vec::<SpringVisualId>::new()));

    // Unified 60Hz compositor-transform physics ticker
    let timer = DispatcherTimer::new();
    timer.set_interval(Duration::from_secs_f64(1.0 / 60.0));
    {
        let host_t = host.clone();
        let springs_t = springs.clone();
        let pip_t = pip_ctrl.clone();
        let sheet_sp_t = sheet_spring.clone();
        let sheet_tf_t = sheet_tf.clone();
        let sw_sp_t = swipe_offsets.clone();
        let sw_tf_t = swipe_tfs.clone();
        let attached_t = rigid_attached.clone();
        let stage_t = stage_ref;
        let last = std::cell::Cell::new(Instant::now());

        timer.tick(move |_| {
            let now = Instant::now();
            let dt = now.duration_since(last.replace(now)).as_secs_f32().clamp(0.001, 0.033);
            let t0 = Instant::now();

            // Update PiP corner snap bounds dynamically to stage size
            if let Some(stage) = stage_t.get_untracked() {
                let b = stage.bounds();
                if b.width > 200.0 && b.height > 200.0 {
                    if let Some(pip) = pip_t.borrow_mut().as_mut() {
                        let right = (b.width as f32 - 280.0).max(0.0);
                        let bottom = (b.height as f32 - 190.0).max(0.0);
                        pip.snap.anchors = vec![
                            Vec2::new(0.0, 0.0),
                            Vec2::new(right, 0.0),
                            Vec2::new(0.0, bottom),
                            Vec2::new(right, bottom),
                        ];
                    }
                    if *attached_t.borrow() {
                        host_t.borrow_mut().set_container_bounds(Rect::new(0.0, 0.0, b.width, b.height));
                    }
                }
            }

            let mut active_writes = 0usize;
            if *attached_t.borrow() {
                let st = host_t.borrow_mut().step_and_sync(dt);
                active_writes += st.transforms_written;
            }
            active_writes += springs_t.borrow_mut().step(dt);

            if let Some(pip) = pip_t.borrow_mut().as_mut() {
                if !pip.card.position.is_settled() || pip.card.is_dragging || !pip.card.tilt_radians.is_settled() {
                    pip.step(dt);
                    active_writes += 1;
                }
            }

            // Bottom Sheet spring
            {
                let mut sp = sheet_sp_t.borrow_mut();
                if !sp.is_settled() {
                    let y = sp.step(dt);
                    if let Some(tf) = sheet_tf_t.borrow().as_ref() {
                        tf.set_matrix(Matrix::create_translation(0.0, y as f64));
                        active_writes += 1;
                    }
                }
            }

            // Swipe rows springs
            {
                let mut rows = sw_sp_t.borrow_mut();
                let tfs = sw_tf_t.borrow();
                for i in 0..3 {
                    if !rows[i].is_settled() {
                        let x = rows[i].step(dt);
                        if let Some(tf) = tfs[i].as_ref() {
                            tf.set_matrix(Matrix::create_translation(x as f64, 0.0));
                            active_writes += 1;
                        }
                    }
                }
            }

            let us = t0.elapsed().as_micros();
            if perf_chip.is_alive() {
                perf_chip.set_if_changed(format!(
                    "Live Matrix Writes: {active_writes}  ·  Sync: {us} µs  ·  Layout Passes: 0"
                ));
            }
        });
    }
    timer.start();
    on_cleanup(move || timer.stop());

    // -------------------------------------------------------------------------
    // 1. Top UX Action Bar
    // -------------------------------------------------------------------------
    let host_grav = host.clone();
    let attached_grav = rigid_attached.clone();
    let stage_grav = stage_ref;
    let host_home = host.clone();
    let sp_dock_enter = springs.clone();
    let dock_sids_enter = dock_sids.clone();
    let sw_reset = swipe_offsets.clone();

    let top_strip = border()
        .dock_panel_dock(ferroui_controls::Dock::Top)
        .background("#111827")
        .border_brush("#24314A")
        .border_thickness(1.0)
        .corner_radius(8.0)
        .margin((12.0, 10.0, 12.0, 8.0))
        .padding((12.0, 8.0, 12.0, 8.0))
        .child(
            stack_panel()
                .spacing(6.0)
                .child(
                    stack_panel()
                        .orientation(Orientation::Horizontal)
                        .spacing(8.0)
                        .child(
                            button()
                                .padding((10.0, 5.0, 10.0, 5.0))
                                .corner_radius(6.0)
                                .background(move || if gravity_on.get() { "#059669" } else { "#1E293B" })
                                .foreground("#FFFFFF")
                                .content(move || if gravity_on.get() { "🌍 UI Gravity: ON" } else { "🌍 Drop UI Cards" })
                                .on_click(move |_| {
                                    let next = !gravity_on.get_untracked();
                                    gravity_on.set(next);
                                    if next {
                                        if let Some(stage) = stage_grav.get_untracked() {
                                            let mut att = attached_grav.borrow_mut();
                                            let mut h = host_grav.borrow_mut();
                                            let b = stage.bounds();
                                            h.set_container_bounds(Rect::new(0.0, 0.0, b.width, b.height));
                                            if !*att {
                                                let styled: &StyledElement = &stage;
                                                h.attach_logical_children(styled, VisualBodyOptions::dynamic_card());
                                                *att = true;
                                            }
                                            h.disable_home_springs();
                                            h.world.gravity = Vec2::new(0.0, 14.0);
                                            h.world.wake_all();
                                        }
                                        ux_log.set("Rigid-body gravity enabled across real FerroUI controls!".into());
                                    } else {
                                        let mut h = host_grav.borrow_mut();
                                        h.world.gravity = Vec2::ZERO;
                                        h.enable_home_springs(3.6, 0.78);
                                        ux_log.set("Home springs engaged — controls springing smoothly back to layout!".into());
                                    }
                                }),
                        )
                        .child(
                            button()
                                .padding((10.0, 5.0, 10.0, 5.0))
                                .corner_radius(6.0)
                                .background("#0284C7")
                                .foreground("#FFFFFF")
                                .content("🧲 Snap All Home")
                                .on_click(move |_| {
                                    gravity_on.set(false);
                                    let mut h = host_home.borrow_mut();
                                    h.world.gravity = Vec2::ZERO;
                                    h.enable_home_springs(3.8, 0.8);
                                    for i in 0..3 {
                                        archived[i].set(false);
                                        sw_reset.borrow_mut()[i].set_target(0.0);
                                    }
                                    ux_log.set("Restored inbox rows and snapped all live UI controls home!".into());
                                }),
                        )
                        .child(
                            button()
                                .padding((10.0, 5.0, 10.0, 5.0))
                                .corner_radius(6.0)
                                .background("#7C3AED")
                                .foreground("#FFFFFF")
                                .content("✦ Spring Cascade")
                                .on_click(move |_| {
                                    let mut sp = sp_dock_enter.borrow_mut();
                                    for (i, &sid) in dock_sids_enter.borrow().iter().enumerate() {
                                        sp.trigger_entrance(sid, Vec2::new(0.0, 48.0 + i as f32 * 14.0), 0.78);
                                    }
                                    ux_log.set("Triggered staggered analytical spring entrance on Dock controls!".into());
                                }),
                        )
                        .child(text_block().text(move || perf_chip.get()).font_size(11.0).foreground("#34D399")),
                )
                .child(text_block().text(move || ux_log.get()).font_size(11.5).foreground("#7DD3FC")),
        );

    // -------------------------------------------------------------------------
    // 2. macOS Magnetic Hover Action Dock (Bottom)
    // -------------------------------------------------------------------------
    let mut dock_row = stack_panel()
        .orientation(Orientation::Horizontal)
        .horizontal_alignment(HorizontalAlignment::Center)
        .spacing(12.0);

    for (label, color) in [
        ("🔍 Search", "#0284C7"),
        ("📁 Projects", "#7C3AED"),
        ("⚡ Profiler", "#059669"),
        ("🎨 Themes", "#DB2777"),
        ("⚙ Settings", "#D97706"),
    ] {
        let sp_m = springs.clone();
        let sp_c = springs.clone();
        let sids_all = dock_sids.clone();
        let sid_cell = Rc::new(RefCell::new(None::<SpringVisualId>));
        let sid_c = sid_cell.clone();

        dock_row = dock_row.child(
            button()
                .padding((14.0, 8.0, 14.0, 8.0))
                .corner_radius(10.0)
                .background(color)
                .foreground("#FFFFFF")
                .content(label)
                .with(move |btn| {
                    let vis: Ref<Visual> = btn.clone().upcast();
                    let mut sp = sp_m.borrow_mut();
                    let sid = sp.attach(&vis, SpringConfig::bouncy());
                    sp.configure_magnetic_hover(sid, 125.0, 12.0, 0.08);
                    *sid_cell.borrow_mut() = Some(sid);
                    sids_all.borrow_mut().push(sid);
                })
                .on_click(move |_| {
                    if let Some(sid) = *sid_c.borrow() {
                        sp_c.borrow_mut().impulse_pop(sid, 3.0);
                    }
                    ux_log.set(format!("Dock action `{label}` — tactile spring bounce fired with zero layout pass!"));
                }),
        );
    }

    let sp_dm = springs.clone();
    let sp_de = springs.clone();
    let magnetic_dock = border()
        .dock_panel_dock(ferroui_controls::Dock::Bottom)
        .background("#111827")
        .border_brush("#24314A")
        .border_thickness(1.0)
        .corner_radius(10.0)
        .margin((12.0, 8.0, 12.0, 10.0))
        .padding((12.0, 8.0, 12.0, 8.0))
        .clip_to_bounds(true)
        .child(
            dock_row
                .node_ref(dock_ref)
                .on_pointer_moved(move |e| {
                    if let Some(p) = dock_ref.get_untracked() {
                        let pt = e.get_position(Some(&p));
                        sp_dm.borrow_mut().update_pointer_in_parent(Vec2::new(pt.x as f32, pt.y as f32));
                    }
                })
                .on_pointer_exited(move |_| {
                    sp_de.borrow_mut().update_pointer_in_parent(Vec2::new(-9999.0, -9999.0));
                }),
        );

    // -------------------------------------------------------------------------
    // 3. Real-Life Interactive UX Stage
    // -------------------------------------------------------------------------

    // 3A. Swipe-to-Action Notification Inbox (Top-Left)
    let mut inbox_stack = stack_panel().spacing(8.0).child(
        text_block()
            .text("📬 SWIPE-TO-ARCHIVE INBOX  (Drag rows horizontally)")
            .font_size(11.0)
            .foreground("#94A3B8"),
    );

    let items = [
        ("PR #1488 Checks Passed", "19/19 physics tests green · 2m ago", "#0284C7"),
        ("Metal Compute Ready", "Apple M3 Pro · 8,192 particles @ 60fps", "#059669"),
        ("Design Review Mention", "Check spring overshoot curve · 12m ago", "#7C3AED"),
    ];
    for (idx, (title, sub, accent)) in items.into_iter().enumerate() {
        let tf_mount = swipe_tfs.clone();
        let drag_p = swipe_drag.clone();
        let drag_m = swipe_drag.clone();
        let drag_r = swipe_drag.clone();
        let sp_p = swipe_offsets.clone();
        let sp_m = swipe_offsets.clone();
        let sp_r = swipe_offsets.clone();
        let sp_btn = swipe_offsets.clone();
        let arch_sig = archived[idx];

        let row_card = border()
            .background("#172135")
            .border_brush(accent)
            .border_thickness((3.0, 1.0, 1.0, 1.0))
            .corner_radius(8.0)
            .padding((10.0, 8.0, 10.0, 8.0))
            .with(move |b: &Ref<Border>| {
                let tf = MatrixTransform::with_matrix(Matrix::IDENTITY);
                b.set_render_transform_origin(RelativePoint::CENTER);
                b.set_render_transform(Some((&tf).into()));
                tf_mount.borrow_mut()[idx] = Some(tf);
            })
            .on_pointer_pressed(move |e| {
                let pt = e.get_position(None);
                let cur_x = sp_p.borrow()[idx].value;
                *drag_p.borrow_mut() = Some((idx, pt.x as f32, cur_x));
            })
            .on_pointer_moved(move |e| {
                if let Some((active_i, start_x, start_off)) = *drag_m.borrow() {
                    if active_i == idx {
                        let pt = e.get_position(None);
                        let raw = start_off + (pt.x as f32 - start_x);
                        // Rubber-band resistance when swiping right past 0
                        let damped = if raw > 0.0 {
                            InertiaScroller::rubber_band(raw, 90.0, 0.55)
                        } else {
                            raw.max(-130.0)
                        };
                        sp_m.borrow_mut()[idx].set_target(damped);
                    }
                }
            })
            .on_pointer_released(move |_| {
                let active = *drag_r.borrow();
                if let Some((active_i, _, _)) = active {
                    if active_i == idx {
                        *drag_r.borrow_mut() = None;
                        let open = {
                            let mut rows = sp_r.borrow_mut();
                            if rows[idx].value < -62.0 {
                                rows[idx].set_target(-92.0);
                                true
                            } else {
                                rows[idx].set_target(0.0);
                                false
                            }
                        };
                        arch_sig.set(open);
                        if open {
                            ux_log.set(format!("Swiped `{title}` open — Archive action revealed!"));
                        }
                    }
                }
            })
            .child(
                dock_panel()
                    .child(
                        button()
                            .dock_panel_dock(ferroui_controls::Dock::Right)
                            .padding((8.0, 4.0, 8.0, 4.0))
                            .corner_radius(5.0)
                            .background(move || if arch_sig.get() { "#059669" } else { "#1E293B" })
                            .foreground("#F8FAFC")
                            .content(move || if arch_sig.get() { "Undo" } else { "Swipe ◀" })
                            .on_click(move |_| {
                                let next = !arch_sig.get_untracked();
                                arch_sig.set(next);
                                sp_btn.borrow_mut()[idx].set_target(if next { -84.0 } else { 0.0 });
                            }),
                    )
                    .child(
                        stack_panel()
                            .spacing(2.0)
                            .child(text_block().text(title).font_size(12.5).foreground("#F8FAFC"))
                            .child(text_block().text(sub).font_size(10.5).foreground("#94A3B8")),
                    ),
            );

        inbox_stack = inbox_stack.child(row_card);
    }

    let inbox_panel = border()
        .canvas_left(20.0)
        .canvas_top(18.0)
        .width(340.0)
        .background("#101726")
        .border_brush("#24314A")
        .border_thickness(1.0)
        .corner_radius(10.0)
        .padding(12.0)
        .clip_to_bounds(true)
        .child(inbox_stack);

    // 3B. macOS Tactile Passcode / Biometric Card (Top-Center)
    let sp_pin_mount = springs.clone();
    let pin_sid_m = pin_card_sid.clone();
    let sp_wrong = springs.clone();
    let pin_sid_w = pin_card_sid.clone();
    let sp_ok = springs.clone();
    let pin_sid_ok = pin_card_sid.clone();

    let passcode_card = border()
        .canvas_left(384.0)
        .canvas_top(18.0)
        .width(280.0)
        .background("#101726")
        .border_brush(move || if pin_unlocked.get() { "#10B981" } else { "#24314A" })
        .border_thickness(1.5)
        .corner_radius(10.0)
        .padding(12.0)
        .with(move |b: &Ref<Border>| {
            let vis: Ref<Visual> = b.clone().upcast();
            let sid = sp_pin_mount
                .borrow_mut()
                .attach(&vis, SpringConfig::from_response_damping(0.28, 0.38));
            *pin_sid_m.borrow_mut() = Some(sid);
        })
        .child(
            stack_panel()
                .spacing(8.0)
                .child(text_block().text("🔐 MACOS PASSCODE FEEDBACK").font_size(11.0).foreground("#94A3B8"))
                .child(
                    text_block()
                        .text(move || pin_status.get())
                        .font_size(11.5)
                        .foreground(move || if pin_unlocked.get() { "#34D399" } else { "#F8FAFC" }),
                )
                .child(
                    stack_panel()
                        .orientation(Orientation::Horizontal)
                        .spacing(8.0)
                        .child(
                            button()
                                .padding((10.0, 6.0, 10.0, 6.0))
                                .corner_radius(6.0)
                                .background("#E11D48")
                                .foreground("#FFFFFF")
                                .content("Wrong PIN")
                                .on_click(move |_| {
                                    pin_unlocked.set(false);
                                    pin_status.set("Incorrect PIN — try again".into());
                                    if let Some(sid) = *pin_sid_w.borrow() {
                                        sp_wrong.borrow_mut().impulse_shake(sid, 780.0);
                                    }
                                    ux_log.set("Triggered macOS damped horizontal error-shake via VisualSpringDriver!".into());
                                }),
                        )
                        .child(
                            button()
                                .padding((10.0, 6.0, 10.0, 6.0))
                                .corner_radius(6.0)
                                .background("#059669")
                                .foreground("#FFFFFF")
                                .content("PIN 4242 ✓")
                                .on_click(move |_| {
                                    pin_unlocked.set(true);
                                    pin_status.set("Unlocked with tactile spring pop!".into());
                                    if let Some(sid) = *pin_sid_ok.borrow() {
                                        sp_ok.borrow_mut().impulse_pop(sid, 2.8);
                                    }
                                    ux_log.set("PIN accepted — celebratory spring scale pop applied!".into());
                                }),
                        ),
                ),
        );

    // 3C. Interactive 3-Detent Bottom Sheet (Apple Maps / Spotify UX)
    let sh_tf_mount = sheet_tf.clone();
    let mut detent_buttons = stack_panel().orientation(Orientation::Horizontal).spacing(6.0);
    for det in [SheetDetent::Collapsed, SheetDetent::Half, SheetDetent::Expanded] {
        let sh_sp = sheet_spring.clone();
        detent_buttons = detent_buttons.child(
            button()
                .padding((10.0, 4.0, 10.0, 4.0))
                .corner_radius(5.0)
                .background(move || if sheet_detent.get() == det { "#0284C7" } else { "#1E293B" })
                .foreground("#FFFFFF")
                .content(det.label())
                .on_click(move |_| {
                    sheet_detent.set(det);
                    sh_sp.borrow_mut().set_target(det.offset_y());
                    ux_log.set(format!("Bottom Sheet smoothly animating to `{}` detent via Spring1D!", det.label()));
                }),
        );
    }

    let bottom_sheet = border()
        .canvas_left(20.0)
        .canvas_top(300.0)
        .width(644.0)
        .height(175.0)
        .background("#131C30")
        .border_brush("#38BDF8")
        .border_thickness((1.0, 2.0, 1.0, 0.0))
        .corner_radius(14.0)
        .padding((14.0, 10.0, 14.0, 10.0))
        .with(move |b: &Ref<Border>| {
            let tf = MatrixTransform::with_matrix(Matrix::create_translation(0.0, SheetDetent::Half.offset_y() as f64));
            b.set_render_transform_origin(RelativePoint::CENTER);
            b.set_render_transform(Some((&tf).into()));
            *sh_tf_mount.borrow_mut() = Some(tf);
        })
        .child(
            stack_panel()
                .spacing(8.0)
                .child(
                    dock_panel()
                        .child(
                            text_block()
                                .dock_panel_dock(ferroui_controls::Dock::Left)
                                .text("▤ 3-DETENT BOTTOM SHEET  (Apple Maps / Spotify UX)")
                                .font_size(11.5)
                                .foreground("#7DD3FC"),
                        )
                        .child(detent_buttons.dock_panel_dock(ferroui_controls::Dock::Right)),
                )
                .child(text_block().text(move || format!("Studio Audio Output Gain: {:.0}%", audio_vol.get())).font_size(12.0).foreground("#E2E8F0"))
                .child(slider().minimum(0.0).maximum(100.0).value(audio_vol.two_way()))
                .child(text_block().text("Real FerroUI Slider & Buttons inside an analytically spring-animated drawer (0 layout passes).").font_size(10.5).foreground("#94A3B8")),
        );

    // 3D. iOS / macOS Corner-Snapping Picture-in-Picture (PiP) Video Player!
    let pip_mount = pip_ctrl.clone();
    let pip_press = pip_ctrl.clone();
    let pip_move = pip_ctrl.clone();
    let pip_rel = pip_ctrl.clone();

    let pip_player = border()
        .canvas_left(20.0)
        .canvas_top(18.0)
        .width(248.0)
        .background("#0F172A")
        .border_brush("#38BDF8")
        .border_thickness(2.0)
        .corner_radius(12.0)
        .padding(10.0)
        .with(move |b: &Ref<Border>| {
            let vis: Ref<Visual> = b.clone().upcast();
            let corners = vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(660.0, 0.0),
                Vec2::new(0.0, 300.0),
                Vec2::new(660.0, 300.0),
            ];
            let mut dv = DraggableVisual::attach(&vis, SpringConfig::bouncy(), corners);
            // Start docked in top-right corner
            dv.card.position.snap(Vec2::new(660.0, 0.0));
            dv.step(1.0 / 60.0);
            *pip_mount.borrow_mut() = Some(dv);
        })
        .on_pointer_pressed(move |e| {
            let pt = e.get_position(None);
            if let Some(pip) = pip_press.borrow_mut().as_mut() {
                pip.begin_drag(Vec2::new(pt.x as f32, pt.y as f32));
            }
        })
        .on_pointer_moved(move |e| {
            let pt = e.get_position(None);
            if let Some(pip) = pip_move.borrow_mut().as_mut() {
                pip.drag_to(Vec2::new(pt.x as f32, pt.y as f32), 1.0 / 60.0);
            }
        })
        .on_pointer_released(move |_| {
            if let Some(pip) = pip_rel.borrow_mut().as_mut() {
                pip.end_drag();
                ux_log.set("PiP Player flung! Velocity projected into nearest magnetic corner with spring bounce!".into());
            }
        })
        .child(
            stack_panel()
                .spacing(6.0)
                .child(text_block().text("📺 PICTURE-IN-PICTURE (DRAG & FLING!)").font_size(10.0).foreground("#38BDF8"))
                .child(
                    border()
                        .background("#1E293B")
                        .corner_radius(6.0)
                        .padding((10.0, 14.0, 10.0, 14.0))
                        .child(
                            text_block()
                                .text(move || {
                                    if pip_playing.get() {
                                        "▶  FerroUI Live · 4K 60fps"
                                    } else {
                                        "⏸  Stream Paused"
                                    }
                                })
                                .font_size(12.0)
                                .foreground("#F8FAFC")
                                .horizontal_alignment(HorizontalAlignment::Center),
                        ),
                )
                .child(
                    stack_panel()
                        .orientation(Orientation::Horizontal)
                        .horizontal_alignment(HorizontalAlignment::Center)
                        .spacing(8.0)
                        .child(
                            button()
                                .padding((10.0, 4.0, 10.0, 4.0))
                                .corner_radius(5.0)
                                .background("#0284C7")
                                .foreground("#FFFFFF")
                                .content(move || if pip_playing.get() { "⏸ Pause" } else { "▶ Play" })
                                .on_click(move |_| pip_playing.update(|v| *v = !*v)),
                        )
                        .child(
                            button()
                                .padding((10.0, 4.0, 10.0, 4.0))
                                .corner_radius(5.0)
                                .background("#334155")
                                .foreground("#FFFFFF")
                                .content(move || if pip_muted.get() { "🔇 Muted" } else { "🔊 Sound" })
                                .on_click(move |_| pip_muted.update(|v| *v = !*v)),
                        ),
                ),
        );

    let stage = border()
        .background("#0A0F1C")
        .border_brush("#24314A")
        .border_thickness(1.0)
        .corner_radius(10.0)
        .margin((12.0, 0.0, 12.0, 0.0))
        .clip_to_bounds(true)
        .child(
            canvas()
                .background("#0A0F1C")
                .clip_to_bounds(true)
                .node_ref(stage_ref)
                .child(inbox_panel)
                .child(passcode_card)
                .child(bottom_sheet)
                .child(pip_player),
        );

    dock_panel()
        .background("#090D16")
        .clip_to_bounds(true)
        .child(top_strip)
        .child(magnetic_dock)
        .child(stage)
        .into()
}
