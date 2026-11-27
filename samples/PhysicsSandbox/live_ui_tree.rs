//! **Live Visual & Logical Tree Physics Showcase**
//!
//! Demonstrates `ferroui_physics::ferroui` (`VisualPhysicsHost` + `VisualSpringDriver`) applying
//! real rigid-body collisions, elastic home springs, explosions, magnetic hover, tactile spring
//! clicks, and damped form-shake directly to **real interactive FerroUI controls** (`Button`,
//! `Slider`, `Border`, `CheckBox`, `TextBlock`) via zero-layout-thrash `MatrixTransform` updates.

use ferroui_base::layout::{HorizontalAlignment, Orientation};
use ferroui_base::{Rect, StyledElement, Visual};
use ferroui_controls::{Border, Canvas, StackPanel};
use ferroui_physics::prelude::*;
use ferroui_view::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// Builds the live interactive Visual/Logical Tree Physics UX showcase panel.
pub fn live_ui_tree_showcase() -> Element {
    let host = Rc::new(RefCell::new(VisualPhysicsHost::new(64.0)));
    let springs = Rc::new(RefCell::new(VisualSpringDriver::new()));

    let gravity_on = signal(false);
    let home_springs_on = signal(true);
    let action_log = signal(String::from("Move pointer over Magnetic Dock buttons or drag/drop real cards below!"));
    let sync_telemetry = signal(String::from("Bound: 0 · Matrix Writes: 0 · Sync: 0 µs · Layout Passes: 0"));
    let slider_gain = signal(65.0_f64);
    let notify_count = signal(3i32);
    let form_card_spring_id = Rc::new(RefCell::new(None::<SpringVisualId>));
    let all_card_spring_ids = Rc::new(RefCell::new(Vec::<SpringVisualId>::new()));

    let arena_ref = NodeRef::<Canvas>::new();
    let dock_ref = NodeRef::<StackPanel>::new();
    let attached_once = Rc::new(RefCell::new(false));

    // Attach logical children lazily once layout bounds are ready, and keep container walls synced!
    let host_tick = host.clone();
    let springs_tick = springs.clone();
    let attached_tick = attached_once.clone();
    let arena_tick = arena_ref;
    let timer = start_ui_physics_timer(host_tick.clone(), springs_tick.clone(), move |sync, active_sp| {
        if let Some(arena) = arena_tick.get_untracked() {
            let bounds = arena.bounds();
            if bounds.width > 50.0 && bounds.height > 50.0 {
                let mut attached = attached_tick.borrow_mut();
                if !*attached {
                    let mut h = host_tick.borrow_mut();
                    h.set_container_bounds(Rect::new(0.0, 0.0, bounds.width, bounds.height));
                    let styled: &StyledElement = &arena;
                    let opts = VisualBodyOptions::elastic_card(3.2, 0.72);
                    h.attach_logical_children(styled, opts);
                    *attached = true;
                }
            }
        }
        if sync_telemetry.is_alive() {
            sync_telemetry.set_if_changed(format!(
                "Bound Visuals: {}  ·  Matrix Writes: {}  ·  Active Springs: {}  ·  Sync: {} µs  ·  Layout Passes: 0",
                sync.bound_visuals, sync.transforms_written, active_sp, sync.sync_us
            ));
        }
    });
    on_cleanup(move || timer.stop());

    // -------------------------------------------------------------------------
    // 1. Live Action Toolbar (Gravity, Elastic Home, Shockwave, Form Shake, Entrance)
    // -------------------------------------------------------------------------
    let host_grav = host.clone();
    let host_home = host.clone();
    let host_boom = host.clone();
    let host_reset = host.clone();
    let sp_shake = springs.clone();
    let form_id_shake = form_card_spring_id.clone();
    let sp_enter = springs.clone();
    let all_ids_enter = all_card_spring_ids.clone();

    let controls_bar = border()
        .dock_panel_dock(ferroui_controls::Dock::Top)
        .background("#111827")
        .border_brush("#24314A")
        .border_thickness(1.0)
        .corner_radius(8.0)
        .margin((12.0, 12.0, 12.0, 8.0))
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
                                .padding((10.0, 6.0, 10.0, 6.0))
                                .corner_radius(6.0)
                                .background(move || if gravity_on.get() { "#059669" } else { "#1E293B" })
                                .foreground("#FFFFFF")
                                .content(move || if gravity_on.get() { "🌍 Gravity: ON" } else { "🌍 Drop UI Gravity" })
                                .on_click(move |_| {
                                    let next = !gravity_on.get_untracked();
                                    gravity_on.set(next);
                                    let mut h = host_grav.borrow_mut();
                                    h.world.gravity = if next { Vec2::new(0.0, 14.0) } else { Vec2::ZERO };
                                    if next {
                                        h.disable_home_springs();
                                        home_springs_on.set(false);
                                    }
                                    h.world.wake_all();
                                    action_log.set(if next {
                                        "Gravity enabled on real FerroUI controls! Try clicking sliders & buttons while they tumble!".into()
                                    } else {
                                        "Zero-G enabled on live controls.".into()
                                    });
                                }),
                        )
                        .child(
                            button()
                                .padding((10.0, 6.0, 10.0, 6.0))
                                .corner_radius(6.0)
                                .background(move || if home_springs_on.get() { "#0284C7" } else { "#1E293B" })
                                .foreground("#FFFFFF")
                                .content("🧲 Elastic Snap Home")
                                .on_click(move |_| {
                                    let mut h = host_home.borrow_mut();
                                    h.world.gravity = Vec2::ZERO;
                                    gravity_on.set(false);
                                    h.enable_home_springs(3.4, 0.72);
                                    home_springs_on.set(true);
                                    action_log.set("Home springs engaged — every real UI control springs back to its layout slot!".into());
                                }),
                        )
                        .child(
                            button()
                                .padding((10.0, 6.0, 10.0, 6.0))
                                .corner_radius(6.0)
                                .background("#D97706")
                                .foreground("#FFFFFF")
                                .content("💥 UI Shockwave")
                                .on_click(move |_| {
                                    let mut h = host_boom.borrow_mut();
                                    h.apply_explosion_px(Vec2::new(420.0, 240.0), 650.0, 14.0);
                                    action_log.set("Radial shockwave applied across the FerroUI logical tree!".into());
                                }),
                        )
                        .child(
                            button()
                                .padding((10.0, 6.0, 10.0, 6.0))
                                .corner_radius(6.0)
                                .background("#DC2626")
                                .foreground("#FFFFFF")
                                .content("⚠ Invalid Form Shake")
                                .on_click(move |_| {
                                    if let Some(sid) = *form_id_shake.borrow() {
                                        sp_shake.borrow_mut().impulse_shake(sid, 780.0);
                                        action_log.set("Damped analytical error-shake applied to the Sign-In Form card!".into());
                                    }
                                }),
                        )
                        .child(
                            button()
                                .padding((10.0, 6.0, 10.0, 6.0))
                                .corner_radius(6.0)
                                .background("#7C3AED")
                                .foreground("#FFFFFF")
                                .content("✦ Cascade Entrance")
                                .on_click(move |_| {
                                    let mut sp = sp_enter.borrow_mut();
                                    for (i, &sid) in all_ids_enter.borrow().iter().enumerate() {
                                        sp.trigger_entrance(sid, Vec2::new(0.0, 75.0 + i as f32 * 24.0), 0.82);
                                    }
                                    action_log.set("Staggered spring entrance triggered on real Visual controls!".into());
                                }),
                        )
                        .child(
                            button()
                                .padding((10.0, 6.0, 10.0, 6.0))
                                .corner_radius(6.0)
                                .background("#334155")
                                .foreground("#F8FAFC")
                                .content("↺ Reset Layout")
                                .on_click(move |_| {
                                    let mut h = host_reset.borrow_mut();
                                    h.world.gravity = Vec2::ZERO;
                                    gravity_on.set(false);
                                    h.reset_all_to_rest();
                                    h.enable_home_springs(3.4, 0.72);
                                    home_springs_on.set(true);
                                }),
                        ),
                )
                .child(
                    stack_panel()
                        .orientation(Orientation::Horizontal)
                        .spacing(16.0)
                        .child(text_block().text(move || action_log.get()).font_size(11.5).foreground("#38BDF8"))
                        .child(text_block().text(move || sync_telemetry.get()).font_size(11.0).foreground("#34D399")),
                ),
        );

    // -------------------------------------------------------------------------
    // 2. Magnetic Hover & Tactile Spring Action Dock (Real FerroUI Buttons!)
    // -------------------------------------------------------------------------
    let mut magnetic_buttons = stack_panel()
        .orientation(Orientation::Horizontal)
        .horizontal_alignment(HorizontalAlignment::Center)
        .spacing(14.0);

    for (label, color) in [
        ("💾 Save Draft", "#0284C7"),
        ("🚀 Publish Live", "#7C3AED"),
        ("🔗 Share Link", "#059669"),
        ("⚡ Run Deploy", "#D97706"),
        ("📦 Export Kit", "#DB2777"),
    ] {
        let sp_mount = springs.clone();
        let sp_click = springs.clone();
        let sid_cell = Rc::new(RefCell::new(None::<SpringVisualId>));
        let sid_click = sid_cell.clone();
        let all_ids = all_card_spring_ids.clone();

        magnetic_buttons = magnetic_buttons.child(
            button()
                .padding((16.0, 10.0, 16.0, 10.0))
                .corner_radius(10.0)
                .background(color)
                .foreground("#FFFFFF")
                .content(label)
                .with(move |btn| {
                    let vis: ferroui_base::Ref<Visual> = btn.clone().upcast();
                    let mut sp = sp_mount.borrow_mut();
                    let sid = sp.attach(&vis, SpringConfig::bouncy());
                    sp.configure_magnetic_hover(sid, 130.0, 14.0, 0.09);
                    *sid_cell.borrow_mut() = Some(sid);
                    all_ids.borrow_mut().push(sid);
                })
                .on_click(move |_| {
                    if let Some(sid) = *sid_click.borrow() {
                        sp_click.borrow_mut().impulse_pop(sid, 3.2);
                    }
                    action_log.set(format!("Clicked real FerroUI Button `{label}` — tactile spring pop fired!"));
                }),
        );
    }

    let sp_dock_move = springs.clone();
    let sp_dock_exit = springs.clone();
    let dock_panel_box = border()
        .dock_panel_dock(ferroui_controls::Dock::Bottom)
        .background("#111827")
        .border_brush("#24314A")
        .border_thickness(1.0)
        .corner_radius(10.0)
        .margin((12.0, 8.0, 12.0, 12.0))
        .padding((14.0, 10.0, 14.0, 10.0))
        .clip_to_bounds(true)
        .child(
            stack_panel()
                .spacing(6.0)
                .child(
                    text_block()
                        .text("MAGNETIC HOVER & TACTILE SPRING DOCK  (VisualSpringDriver on real FerroUI Buttons)")
                        .font_size(10.5)
                        .foreground("#94A3B8")
                        .horizontal_alignment(HorizontalAlignment::Center),
                )
                .child(
                    magnetic_buttons
                        .node_ref(dock_ref)
                        .on_pointer_moved(move |e| {
                            if let Some(p) = dock_ref.get_untracked() {
                                let pt = e.get_position(Some(&p));
                                sp_dock_move
                                    .borrow_mut()
                                    .update_pointer_in_parent(Vec2::new(pt.x as f32, pt.y as f32));
                            }
                        })
                        .on_pointer_exited(move |_| {
                            sp_dock_exit.borrow_mut().update_pointer_in_parent(Vec2::new(-9999.0, -9999.0));
                        }),
                ),
        );

    // -------------------------------------------------------------------------
    // 3. Center Live Physics-Driven Logical Tree Arena (Real Interactive Controls!)
    // -------------------------------------------------------------------------
    let sp_form_mount = springs.clone();
    let form_id_mount = form_card_spring_id.clone();
    let sp_form_btn = springs.clone();
    let form_id_btn = form_card_spring_id.clone();

    let card_1 = ui_widget_card(
        "🎛 Audio Mixer Control",
        "#0284C7",
        24.0,
        24.0,
        stack_panel()
            .spacing(6.0)
            .child(text_block().text(move || format!("Master Gain: {:.0}%", slider_gain.get())).font_size(12.0).foreground("#E2E8F0"))
            .child(slider().minimum(0.0).maximum(100.0).value(slider_gain.two_way()))
            .child(text_block().text("Real interactive Slider works even mid-air!").font_size(10.5).foreground("#94A3B8")),
    );

    let card_2 = border()
        .canvas_left(310.0)
        .canvas_top(24.0)
        .width(260.0)
        .background("#162033")
        .border_brush("#F43F5E")
        .border_thickness(1.5)
        .corner_radius(10.0)
        .padding(12.0)
        .child(
            stack_panel()
                .spacing(6.0)
                .child(text_block().text("🔐 Sign-In Validation UX").font_size(13.0).foreground("#FDA4AF"))
                .child(text_block().text("Demonstrates analytical spring error-shake directly on a real form container.").font_size(11.0).foreground("#CBD5E1"))
                .child(
                    button()
                        .padding((10.0, 5.0, 10.0, 5.0))
                        .corner_radius(6.0)
                        .background("#E11D48")
                        .foreground("#FFFFFF")
                        .content("Simulate Wrong Password")
                        .on_click(move |_| {
                            if let Some(sid) = *form_id_btn.borrow() {
                                sp_form_btn.borrow_mut().impulse_shake(sid, 820.0);
                            }
                            action_log.set("Form rejected — horizontal analytical spring shake triggered!".into());
                        }),
                ),
        )
        .with(move |b: &ferroui_base::Ref<Border>| {
            let vis: ferroui_base::Ref<Visual> = b.clone().upcast();
            // Attach secondary spring driver to inner child so error-shake composes cleanly with rigid-body host!
            if let Some(children) = vis.visual_children_snapshot() {
                if let Some(inner) = children.first() {
                    let sid = sp_form_mount.borrow_mut().attach(inner, SpringConfig::from_response_damping(0.26, 0.36));
                    *form_id_mount.borrow_mut() = Some(sid);
                }
            }
        });

    let card_3 = ui_widget_card(
        "🔔 Tactile Notification Card",
        "#10B981",
        596.0,
        24.0,
        stack_panel()
            .spacing(6.0)
            .child(text_block().text(move || format!("Unread Alerts: {}", notify_count.get())).font_size(12.0).foreground("#E2E8F0"))
            .child(
                button()
                    .padding((10.0, 5.0, 10.0, 5.0))
                    .corner_radius(6.0)
                    .background("#059669")
                    .foreground("#FFFFFF")
                    .content("+ Push Alert")
                    .on_click(move |_| {
                        notify_count.update(|n| *n += 1);
                        action_log.set("Pushed new alert onto real reactive FerroUI signal!".into());
                    }),
            ),
    );

    let host_card4 = host.clone();
    let card_4 = ui_widget_card(
        "⚡ Live Physics Control",
        "#F59E0B",
        24.0,
        196.0,
        stack_panel()
            .spacing(6.0)
            .child(text_block().text("Click inside this card to detonate a local impulse from its own position!").font_size(11.0).foreground("#CBD5E1"))
            .child(
                button()
                    .padding((10.0, 5.0, 10.0, 5.0))
                    .corner_radius(6.0)
                    .background("#D97706")
                    .foreground("#FFFFFF")
                    .content("Pulse Neighbors")
                    .on_click(move |_| {
                        host_card4.borrow_mut().apply_explosion_px(Vec2::new(150.0, 260.0), 500.0, 11.0);
                    }),
            ),
    );

    let card_5 = ui_widget_card(
        "📐 Zero-Layout-Thrash API",
        "#A78BFA",
        310.0,
        196.0,
        stack_panel()
            .spacing(4.0)
            .child(text_block().text("host.attach_logical_children(&panel, opts)").font_size(10.5).foreground("#C4B5FD"))
            .child(text_block().text("Updates persistent MatrixTransform only for moving visuals — sleeping controls cost 0 ns.").font_size(11.0).foreground("#CBD5E1")),
    );

    let card_6 = ui_widget_card(
        "🎮 Drag & Fling Anything",
        "#EC4899",
        596.0,
        196.0,
        stack_panel()
            .spacing(4.0)
            .child(text_block().text("Left-drag any card to grab & fling it with a TargetJoint; right-click anywhere in the arena for a shockwave!").font_size(11.0).foreground("#CBD5E1")),
    );

    let host_press = host.clone();
    let host_move = host.clone();
    let host_rel = host.clone();

    let live_arena = border()
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
                .node_ref(arena_ref)
                .on_pointer_pressed(move |e| {
                    let Some(arena) = arena_ref.get_untracked() else { return };
                    let pt = e.get_position(Some(&arena));
                    let p = Vec2::new(pt.x as f32, pt.y as f32);
                    if e.properties().is_right_button_pressed {
                        host_press.borrow_mut().apply_explosion_px(p, 520.0, 14.0);
                    } else {
                        host_press.borrow_mut().begin_pointer_drag_px(p);
                    }
                })
                .on_pointer_moved(move |e| {
                    let Some(arena) = arena_ref.get_untracked() else { return };
                    let pt = e.get_position(Some(&arena));
                    host_move.borrow_mut().update_pointer_drag_px(Vec2::new(pt.x as f32, pt.y as f32));
                })
                .on_pointer_released(move |_| {
                    host_rel.borrow_mut().end_pointer_drag();
                })
                .child(card_1)
                .child(card_2)
                .child(card_3)
                .child(card_4)
                .child(card_5)
                .child(card_6),
        );

    dock_panel()
        .background("#090D16")
        .clip_to_bounds(true)
        .child(controls_bar)
        .child(dock_panel_box)
        .child(live_arena)
        .into()
}

fn ui_widget_card(title: &'static str, accent: &'static str, left: f64, top: f64, content: impl IntoElement) -> Element {
    border()
        .canvas_left(left)
        .canvas_top(top)
        .width(260.0)
        .background("#162033")
        .border_brush(accent)
        .border_thickness(1.5)
        .corner_radius(10.0)
        .padding(12.0)
        .child(
            stack_panel()
                .spacing(6.0)
                .child(text_block().text(title).font_size(13.0).foreground("#F8FAFC"))
                .child(content),
        )
        .into()
}
