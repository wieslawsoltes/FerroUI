//! **Real-Life FerroUI Visual & Logical Tree Physics UX Showcase**
//!
//! Organized into 4 uncluttered, pointer-captured real-world product UX demos built entirely
//! from **real interactive FerroUI controls** (`Button`, `Border`, `Slider`, `TextBlock`, `Canvas`,
//! `StackPanel`) animated at 60 FPS via persistent `MatrixTransform`s (zero layout passes):
//!
//! 1. **📺 PiP Player** — iOS/macOS corner-snapping Picture-in-Picture floating video player
//!    with full pointer capture, velocity-projected corner ghost highlight, spring resize `S/M/L`,
//!    and velocity-coupled tilt!
//! 2. **📬 Swipe Inbox** — iOS Mail / Reminders pointer-captured horizontal swipe rows with
//!    Apple rubber-band resistance, archive snap, smooth spring dismiss, and animated row reflow!
//! 3. **▤ Bottom Sheet** — Apple Maps / Spotify pointer-captured draggable bottom drawer with
//!    rubber-band overscroll resistance, velocity-projected 3-detent snap (`Collapsed`/`Half`/`Expanded`),
//!    and a spring-driven Pull-to-Refresh banner!
//! 4. **🔐 Keypad & Chaos UI** — Real 4-digit macOS Passcode keypad (tactile key pops, wrong-PIN
//!    damped shake, `1234` unlock pop), macOS Magnetic Hover Dock, and Live Logical-Tree Gravity!

use crate::icons::{icon_label, Icons};
use ferroui_base::input::InputElement;
use ferroui_base::layout::{HorizontalAlignment, Orientation};
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{FontWeight, IBrush, MatrixTransform};
use ferroui_base::threading::DispatcherTimer;
use ferroui_base::{Matrix, Rect, Ref, RelativePoint, StyledElement, Visual};
use ferroui_controls::{Border, Canvas, StackPanel};
use ferroui_physics::prelude::*;
use ferroui_view::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

fn solid(argb: u32) -> Option<Rc<dyn IBrush>> {
    Some(Rc::new(ImmutableSolidColorBrush::from_uint32(argb)) as Rc<dyn IBrush>)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UxDemoTab {
    PipPlayer,
    SwipeInbox,
    BottomSheet,
    KeypadDockChaos,
}

impl UxDemoTab {
    const ALL: [Self; 4] = [Self::PipPlayer, Self::SwipeInbox, Self::BottomSheet, Self::KeypadDockChaos];

    fn title(self) -> &'static str {
        match self {
            Self::PipPlayer => "PiP Window",
            Self::SwipeInbox => "Swipe Inbox",
            Self::BottomSheet => "Bottom Sheet",
            Self::KeypadDockChaos => "Keypad, Dock & Gravity",
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Self::PipPlayer => Icons::UI_MOTION,
            Self::SwipeInbox => Icons::LAYERS,
            Self::BottomSheet => Icons::SLIDER,
            Self::KeypadDockChaos => Icons::CUBE,
        }
    }
}

/// Builds the 4-tab real-life FerroUI Visual/Logical Tree Physics UX showcase.
pub fn live_ui_tree_showcase() -> Element {
    let active_tab = signal(UxDemoTab::PipPlayer);

    let mut sub_tabs = stack_panel().orientation(Orientation::Horizontal).spacing(6.0);
    for tab in UxDemoTab::ALL {
        sub_tabs = sub_tabs.child(
            button()
                .padding((11.0, 6.0, 11.0, 6.0))
                .border_brush(move || if active_tab.get() == tab { solid(0xFF0078D4) } else { None })
                .border_thickness(move || if active_tab.get() == tab { 1.5 } else { 1.0 })
                .content(icon_label(tab.icon(), tab.title(), 14.0))
                .on_click(move |_| active_tab.set(tab)),
        );
    }

    let top_nav = border()
        .dock_panel_dock(ferroui_controls::Dock::Top)
        .border_brush(solid(0x22808080))
        .border_thickness((0.0, 0.0, 0.0, 1.0))
        .padding((22.0, 14.0, 22.0, 12.0))
        .child(
            stack_panel()
                .spacing(10.0)
                .child(
                    stack_panel()
                        .spacing(3.0)
                        .child(
                            text_block()
                                .text("Live UI Tree Physics")
                                .font_size(22.0)
                                .font_weight(FontWeight::SemiBold),
                        )
                        .child(
                            text_block()
                                .text("Interactive FerroUI controls animated at 60 FPS via persistent MatrixTransform bindings with zero layout passes.")
                                .font_size(12.5)
                                .opacity(0.75),
                        ),
                )
                .child(sub_tabs),
        );

    let body = dynamic(move || match active_tab.get() {
        UxDemoTab::PipPlayer => demo_pip_player(),
        UxDemoTab::SwipeInbox => demo_swipe_inbox(),
        UxDemoTab::BottomSheet => demo_bottom_sheet(),
        UxDemoTab::KeypadDockChaos => demo_keypad_dock_chaos(),
    });

    dock_panel()
        .clip_to_bounds(true)
        .child(top_nav)
        .child(body)
        .into()
}

// =============================================================================
// UX DEMO 1: iOS / macOS Picture-in-Picture (PiP) Corner-Snapping Player
// =============================================================================
fn demo_pip_player() -> Element {
    let pip_ctrl: Rc<RefCell<Option<DraggableVisual>>> = Rc::new(RefCell::new(None));
    let pip_size_spring = Rc::new(RefCell::new(Spring1D::new(1.0, SpringConfig::bouncy())));
    let stage_ref = NodeRef::<Canvas>::new();
    let pip_border_ref = NodeRef::<Border>::new();

    let playing = signal(true);
    let muted = signal(false);
    let size_mode = signal(1u8); // 0=S, 1=M, 2=L
    let projected_corner = signal(Some(1usize)); // 0=TL, 1=TR, 2=BL, 3=BR
    let right_slot_x = signal(584.0_f64);
    let bottom_slot_y = signal(344.0_f64);
    let status = signal(String::from("Grab the PiP player anywhere and FLING it! Velocity projection snaps to the predicted corner."));

    let timer = DispatcherTimer::new();
    timer.set_interval(Duration::from_secs_f64(1.0 / 60.0));
    {
        let pip_t = pip_ctrl.clone();
        let sz_t = pip_size_spring.clone();
        let stage_t = stage_ref;
        let last = Cell::new(Instant::now());
        timer.tick(move |_| {
            let now = Instant::now();
            let dt = now.duration_since(last.replace(now)).as_secs_f32().clamp(0.001, 0.033);

            if let Some(stage) = stage_t.get_untracked() {
                let b = stage.bounds();
                if b.width > 340.0 && b.height > 240.0 {
                    let right = (b.width as f32 - 316.0).max(0.0);
                    let bottom = (b.height as f32 - 205.0).max(0.0);
                    if right_slot_x.is_alive() {
                        right_slot_x.set_if_changed(24.0 + right as f64);
                        bottom_slot_y.set_if_changed(24.0 + bottom as f64);
                    }
                    if let Some(pip) = pip_t.borrow_mut().as_mut() {
                        pip.snap.anchors = vec![
                            Vec2::new(0.0, 0.0),
                            Vec2::new(right, 0.0),
                            Vec2::new(0.0, bottom),
                            Vec2::new(right, bottom),
                        ];
                    }
                }
            }

            let target_scale = sz_t.borrow_mut().step(dt);
            if let Some(pip) = pip_t.borrow_mut().as_mut() {
                pip.card.scale.set_target(if pip.card.is_dragging { target_scale * 1.05 } else { target_scale });
                pip.step(dt);
            }
        });
    }
    timer.start();
    on_cleanup(move || timer.stop());

    let corner_slot = move |idx: usize, label: &'static str, is_right: bool, is_bottom: bool| {
        border()
            .canvas_left(move || if is_right { right_slot_x.get() } else { 24.0 })
            .canvas_top(move || if is_bottom { bottom_slot_y.get() } else { 24.0 })
            .width(268.0)
            .height(152.0)
            .corner_radius(12.0)
            .background(move || if projected_corner.get() == Some(idx) { "#102A43" } else { "#0D1322" })
            .border_brush(move || if projected_corner.get() == Some(idx) { "#38BDF8" } else { "#1E293B" })
            .border_thickness(move || if projected_corner.get() == Some(idx) { 2.0 } else { 1.0 })
            .padding(12.0)
            .child(
                text_block()
                    .text(label)
                    .font_size(11.0)
                    .foreground(move || if projected_corner.get() == Some(idx) { "#7DD3FC" } else { "#475569" }),
            )
    };

    let pip_m = pip_ctrl.clone();
    let pip_p = pip_ctrl.clone();
    let pip_mv = pip_ctrl.clone();
    let pip_r = pip_ctrl.clone();
    let sz_s = pip_size_spring.clone();
    let sz_m = pip_size_spring.clone();
    let sz_l = pip_size_spring.clone();

    let pip_card = border()
        .canvas_left(24.0)
        .canvas_top(24.0)
        .width(268.0)
        .background("#0F172A")
        .border_brush("#38BDF8")
        .border_thickness(2.0)
        .corner_radius(14.0)
        .padding(12.0)
        .node_ref(pip_border_ref)
        .with(move |b: &Ref<Border>| {
            let vis: Ref<Visual> = b.clone().upcast();
            let corners = vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(560.0, 0.0),
                Vec2::new(0.0, 320.0),
                Vec2::new(560.0, 320.0),
            ];
            let mut dv = DraggableVisual::attach(&vis, SpringConfig::bouncy(), corners);
            dv.snap.velocity_projection = 0.18;
            dv.card.position.snap(Vec2::new(560.0, 0.0));
            dv.step(1.0 / 60.0);
            *pip_m.borrow_mut() = Some(dv);
        })
        .on_pointer_pressed(move |e| {
            if e.handled() {
                return;
            }
            if let Some(b) = pip_border_ref.get_untracked() {
                let input: Ref<InputElement> = b.upcast();
                e.pointer().capture(Some(&input));
            }
            let pt = e.get_position(None);
            if let Some(pip) = pip_p.borrow_mut().as_mut() {
                pip.begin_drag(Vec2::new(pt.x as f32, pt.y as f32));
            }
        })
        .on_pointer_moved(move |e| {
            let pt = e.get_position(None);
            let pred = {
                let mut guard = pip_mv.borrow_mut();
                if let Some(pip) = guard.as_mut() {
                    if pip.card.is_dragging {
                        pip.drag_to(Vec2::new(pt.x as f32, pt.y as f32), 1.0 / 60.0);
                        pip.snap.choose_anchor(pip.card.position.value(), pip.card.position.velocity()).map(|(i, _)| i)
                    } else {
                        None
                    }
                } else {
                    None
                }
            };
            if pred.is_some() {
                projected_corner.set_if_changed(pred);
            }
        })
        .on_pointer_released(move |e| {
            e.pointer().capture(None);
            let landed = {
                let mut guard = pip_r.borrow_mut();
                if let Some(pip) = guard.as_mut() {
                    pip.end_drag();
                    pip.snap.choose_anchor(pip.card.position.target(), Vec2::ZERO).map(|(i, _)| i)
                } else {
                    None
                }
            };
            if let Some(idx) = landed {
                projected_corner.set(Some(idx));
                let names = ["Top-Left", "Top-Right", "Bottom-Left", "Bottom-Right"];
                status.set(format!("Projected throw landed in {} corner with bouncy spring settle!", names[idx.min(3)]));
            }
        })
        .child(
            stack_panel()
                .spacing(8.0)
                .child(text_block().text("📺 PICTURE-IN-PICTURE · DRAG & FLING").font_size(10.0).foreground("#38BDF8"))
                .child(
                    border()
                        .background("#1E293B")
                        .corner_radius(8.0)
                        .padding((12.0, 18.0, 12.0, 18.0))
                        .child(
                            text_block()
                                .text(move || if playing.get() { "▶  FerroUI Keynote · 4K 60fps" } else { "⏸  Playback Paused" })
                                .font_size(12.5)
                                .foreground("#F8FAFC")
                                .horizontal_alignment(HorizontalAlignment::Center),
                        ),
                )
                .child(
                    stack_panel()
                        .orientation(Orientation::Horizontal)
                        .horizontal_alignment(HorizontalAlignment::Center)
                        .spacing(6.0)
                        .child(
                            button()
                                .padding((9.0, 4.0, 9.0, 4.0))
                                .corner_radius(5.0)
                                .background("#0284C7")
                                .foreground("#FFFFFF")
                                .content(move || if playing.get() { "⏸" } else { "▶" })
                                .on_click(move |_| playing.update(|v| *v = !*v)),
                        )
                        .child(
                            button()
                                .padding((9.0, 4.0, 9.0, 4.0))
                                .corner_radius(5.0)
                                .background("#334155")
                                .foreground("#FFFFFF")
                                .content(move || if muted.get() { "🔇" } else { "🔊" })
                                .on_click(move |_| muted.update(|v| *v = !*v)),
                        )
                        .child(
                            button()
                                .padding((8.0, 4.0, 8.0, 4.0))
                                .corner_radius(5.0)
                                .background(move || if size_mode.get() == 0 { "#7C3AED" } else { "#1E293B" })
                                .foreground("#FFFFFF")
                                .content("S")
                                .on_click(move |_| {
                                    size_mode.set(0);
                                    sz_s.borrow_mut().set_target(0.84);
                                }),
                        )
                        .child(
                            button()
                                .padding((8.0, 4.0, 8.0, 4.0))
                                .corner_radius(5.0)
                                .background(move || if size_mode.get() == 1 { "#7C3AED" } else { "#1E293B" })
                                .foreground("#FFFFFF")
                                .content("M")
                                .on_click(move |_| {
                                    size_mode.set(1);
                                    sz_m.borrow_mut().set_target(1.0);
                                }),
                        )
                        .child(
                            button()
                                .padding((8.0, 4.0, 8.0, 4.0))
                                .corner_radius(5.0)
                                .background(move || if size_mode.get() == 2 { "#7C3AED" } else { "#1E293B" })
                                .foreground("#FFFFFF")
                                .content("L")
                                .on_click(move |_| {
                                    size_mode.set(2);
                                    sz_l.borrow_mut().set_target(1.18);
                                }),
                        ),
                ),
        );

    dock_panel()
        .margin(14.0)
        .child(
            border()
                .dock_panel_dock(ferroui_controls::Dock::Bottom)
                .background("#111827")
                .border_brush("#24314A")
                .border_thickness(1.0)
                .corner_radius(8.0)
                .margin((0.0, 10.0, 0.0, 0.0))
                .padding((12.0, 8.0, 12.0, 8.0))
                .child(text_block().text(move || status.get()).font_size(11.5).foreground("#7DD3FC")),
        )
        .child(
            border()
                .background("#0A0F1C")
                .border_brush("#24314A")
                .border_thickness(1.0)
                .corner_radius(12.0)
                .clip_to_bounds(true)
                .child(
                    canvas()
                        .background("#0A0F1C")
                        .clip_to_bounds(true)
                        .node_ref(stage_ref)
                        .child(corner_slot(0, "TOP-LEFT MAGNET", false, false))
                        .child(corner_slot(1, "TOP-RIGHT MAGNET", true, false))
                        .child(corner_slot(2, "BOTTOM-LEFT MAGNET", false, true))
                        .child(corner_slot(3, "BOTTOM-RIGHT MAGNET", true, true))
                        .child(pip_card),
                ),
        )
        .into()
}

// =============================================================================
// UX DEMO 2: Pointer-Captured Swipe-to-Archive Inbox with Spring Reflow
// =============================================================================
fn demo_swipe_inbox() -> Element {
    const ROW_COUNT: usize = 5;
    let x_springs: Rc<RefCell<[Spring1D; ROW_COUNT]>> = Rc::new(RefCell::new([
        Spring1D::new(0.0, SpringConfig::snappy()),
        Spring1D::new(0.0, SpringConfig::snappy()),
        Spring1D::new(0.0, SpringConfig::snappy()),
        Spring1D::new(0.0, SpringConfig::snappy()),
        Spring1D::new(0.0, SpringConfig::snappy()),
    ]));
    let y_springs: Rc<RefCell<[Spring1D; ROW_COUNT]>> = Rc::new(RefCell::new([
        Spring1D::new(0.0, SpringConfig::bouncy()),
        Spring1D::new(0.0, SpringConfig::bouncy()),
        Spring1D::new(0.0, SpringConfig::bouncy()),
        Spring1D::new(0.0, SpringConfig::bouncy()),
        Spring1D::new(0.0, SpringConfig::bouncy()),
    ]));
    let tfs: Rc<RefCell<[Option<Ref<MatrixTransform>>; ROW_COUNT]>> =
        Rc::new(RefCell::new([None, None, None, None, None]));
    let drag_state: Rc<Cell<Option<(usize, f32, f32)>>> = Rc::new(Cell::new(None));
    let archived = [signal(false), signal(false), signal(false), signal(false), signal(false)];
    let status = signal(String::from("Drag any inbox row LEFT to reveal Archive or fling far left to dismiss with spring reflow; drag RIGHT for rubber-band resistance!"));

    let timer = DispatcherTimer::new();
    timer.set_interval(Duration::from_secs_f64(1.0 / 60.0));
    {
        let xs = x_springs.clone();
        let ys = y_springs.clone();
        let tfs_t = tfs.clone();
        let last = Cell::new(Instant::now());
        timer.tick(move |_| {
            let now = Instant::now();
            let dt = now.duration_since(last.replace(now)).as_secs_f32().clamp(0.001, 0.033);
            let mut x_arr = xs.borrow_mut();
            let mut y_arr = ys.borrow_mut();
            let tf_arr = tfs_t.borrow();
            for i in 0..ROW_COUNT {
                if !x_arr[i].is_settled() || !y_arr[i].is_settled() {
                    let x = x_arr[i].step(dt);
                    let y = y_arr[i].step(dt);
                    if let Some(tf) = tf_arr[i].as_ref() {
                        tf.set_matrix(Matrix::create_translation(x as f64, y as f64));
                    }
                }
            }
        });
    }
    timer.start();
    on_cleanup(move || timer.stop());

    let msgs = [
        ("PR #1488 · Metal Compute Verified", "All 19 physics tests green on Apple M3 Pro · 1m ago", "#0284C7"),
        ("Design Review · Spring Overshoot", "Approved bouncy preset ζ=0.55 for PiP player · 8m ago", "#7C3AED"),
        ("CI Pipeline · Release Metadata", "126 published crates verified cleanly · 15m ago", "#059669"),
        ("Telemetry Alert · 0 Layout Passes", "MatrixTransform render sync averaging 14 µs · 22m ago", "#D97706"),
        ("Team Update · XPBD Cloth & SoftBody", "Stress heatmap & tearing shipped in Studio · 41m ago", "#DB2777"),
    ];

    let mut rows_stack = stack_panel().spacing(10.0);
    for (idx, (title, sub, accent)) in msgs.into_iter().enumerate() {
        let row_ref = NodeRef::<Border>::new();
        let tf_m = tfs.clone();
        let xs_p = x_springs.clone();
        let xs_mv = x_springs.clone();
        let xs_r = x_springs.clone();
        let xs_btn = x_springs.clone();
        let ys_r = y_springs.clone();
        let ds_p = drag_state.clone();
        let ds_m = drag_state.clone();
        let ds_r = drag_state.clone();
        let arch = archived[idx];

        let row = border()
            .background("#141C2E")
            .border_brush(accent)
            .border_thickness((4.0, 1.0, 1.0, 1.0))
            .corner_radius(10.0)
            .padding((14.0, 11.0, 14.0, 11.0))
            .node_ref(row_ref)
            .with(move |b: &Ref<Border>| {
                let tf = MatrixTransform::with_matrix(Matrix::IDENTITY);
                b.set_render_transform_origin(RelativePoint::CENTER);
                b.set_render_transform(Some((&tf).into()));
                tf_m.borrow_mut()[idx] = Some(tf);
            })
            .on_pointer_pressed(move |e| {
                if e.handled() {
                    return;
                }
                if let Some(b) = row_ref.get_untracked() {
                    let input: Ref<InputElement> = b.upcast();
                    e.pointer().capture(Some(&input));
                }
                let pt = e.get_position(None);
                let cur_x = xs_p.borrow()[idx].value;
                ds_p.set(Some((idx, pt.x as f32, cur_x)));
            })
            .on_pointer_moved(move |e| {
                if let Some((active_i, start_x, start_off)) = ds_m.get() {
                    if active_i == idx {
                        let pt = e.get_position(None);
                        let raw = start_off + (pt.x as f32 - start_x);
                        let damped = if raw > 0.0 {
                            InertiaScroller::rubber_band(raw, 110.0, 0.55)
                        } else if raw < -140.0 {
                            -140.0 + InertiaScroller::rubber_band(raw + 140.0, 110.0, 0.55)
                        } else {
                            raw
                        };
                        xs_mv.borrow_mut()[idx].set_target(damped);
                    }
                }
            })
            .on_pointer_released(move |e| {
                e.pointer().capture(None);
                if let Some((active_i, _, _)) = ds_r.get() {
                    if active_i == idx {
                        ds_r.set(None);
                        let mode = {
                            let mut xs = xs_r.borrow_mut();
                            let v = xs[idx].value;
                            if v < -125.0 {
                                xs[idx].set_target(-900.0);
                                2u8
                            } else if v < -52.0 {
                                xs[idx].set_target(-100.0);
                                1u8
                            } else {
                                xs[idx].set_target(0.0);
                                0u8
                            }
                        };
                        if mode == 2 {
                            arch.set(true);
                            // Gentle spring bounce ripple on remaining rows below!
                            let mut ys = ys_r.borrow_mut();
                            for j in (idx + 1)..ROW_COUNT {
                                ys[j].impulse(-140.0);
                            }
                            status.set(format!("Dismissed `{title}` with spring fly-out + list ripple!"));
                        } else if mode == 1 {
                            arch.set(true);
                            status.set(format!("Revealed Archive action on `{title}`."));
                        } else {
                            arch.set(false);
                        }
                    }
                }
            })
            .child(
                dock_panel()
                    .child(
                        button()
                            .dock_panel_dock(ferroui_controls::Dock::Right)
                            .padding((10.0, 5.0, 10.0, 5.0))
                            .corner_radius(6.0)
                            .background(move || if arch.get() { "#059669" } else { "#1E293B" })
                            .foreground("#FFFFFF")
                            .content(move || if arch.get() { "Restore" } else { "Archive ◀" })
                            .on_click(move |_| {
                                let next = !arch.get_untracked();
                                arch.set(next);
                                xs_btn.borrow_mut()[idx].set_target(if next { -100.0 } else { 0.0 });
                            }),
                    )
                    .child(
                        stack_panel()
                            .spacing(3.0)
                            .child(text_block().text(title).font_size(13.0).foreground("#F8FAFC"))
                            .child(text_block().text(sub).font_size(11.0).foreground("#94A3B8")),
                    ),
            );
        rows_stack = rows_stack.child(row);
    }

    let xs_all = x_springs.clone();
    let ys_all = y_springs.clone();

    dock_panel()
        .margin(16.0)
        .child(
            stack_panel()
                .dock_panel_dock(ferroui_controls::Dock::Top)
                .orientation(Orientation::Horizontal)
                .spacing(10.0)
                .margin((0.0, 0.0, 0.0, 12.0))
                .child(
                    button()
                        .padding((12.0, 6.0, 12.0, 6.0))
                        .corner_radius(6.0)
                        .background("#0284C7")
                        .foreground("#FFFFFF")
                        .content("↺ Restore & Cascade All Rows")
                        .on_click(move |_| {
                            let mut xs = xs_all.borrow_mut();
                            let mut ys = ys_all.borrow_mut();
                            for i in 0..ROW_COUNT {
                                archived[i].set(false);
                                xs[i].value = 0.0;
                                xs[i].set_target(0.0);
                                ys[i].value = 40.0 + i as f32 * 18.0;
                                ys[i].set_target(0.0);
                            }
                            status.set("Restored all rows with staggered analytical spring reflow!".into());
                        }),
                )
                .child(text_block().text(move || status.get()).font_size(11.5).foreground("#7DD3FC")),
        )
        .child(
            border()
                .background("#0B101D")
                .border_brush("#24314A")
                .border_thickness(1.0)
                .corner_radius(12.0)
                .padding(18.0)
                .clip_to_bounds(true)
                .child(rows_stack),
        )
        .into()
}

// =============================================================================
// UX DEMO 3: Pointer-Captured 3-Detent Draggable Bottom Sheet + Rubber-Band
// =============================================================================
fn demo_bottom_sheet() -> Element {
    // 3 detents: Expanded = 80px, Half = 250px, Collapsed = 420px from top
    let scroller = Rc::new(RefCell::new({
        let mut sc = InertiaScroller::new(80.0, 420.0, 180.0);
        sc.detents = vec![80.0, 250.0, 420.0];
        sc.position = 250.0;
        sc.snap_spring = SpringConfig::from_response_damping(0.36, 0.80);
        sc
    }));
    let sheet_tf: Rc<RefCell<Option<Ref<MatrixTransform>>>> = Rc::new(RefCell::new(None));
    let tracker = Rc::new(RefCell::new(GestureTracker::default()));
    let dragging = Rc::new(Cell::new(false));
    let last_y = Rc::new(Cell::new(0.0_f32));
    let sim_t = Rc::new(Cell::new(0.0_f64));

    let sheet_ref = NodeRef::<Border>::new();
    let gain = signal(70.0_f64);
    let bass = signal(55.0_f64);
    let detent_label = signal(String::from("Detent: Half (drag the blue sheet up/down or flick!)"));

    let timer = DispatcherTimer::new();
    timer.set_interval(Duration::from_secs_f64(1.0 / 60.0));
    {
        let sc_t = scroller.clone();
        let tf_t = sheet_tf.clone();
        let sim_t_tick = sim_t.clone();
        let last = Cell::new(Instant::now());
        timer.tick(move |_| {
            let now = Instant::now();
            let dt = now.duration_since(last.replace(now)).as_secs_f32().clamp(0.001, 0.033);
            sim_t_tick.set(sim_t_tick.get() + dt as f64);
            let y = sc_t.borrow_mut().step(dt);
            if let Some(tf) = tf_t.borrow().as_ref() {
                tf.set_matrix(Matrix::create_translation(0.0, y as f64));
            }
        });
    }
    timer.start();
    on_cleanup(move || timer.stop());

    let mut detent_btns = stack_panel().orientation(Orientation::Horizontal).spacing(8.0);
    for (label, pos) in [("Expanded", 80.0_f32), ("Half", 250.0_f32), ("Collapsed", 420.0_f32)] {
        let sc_b = scroller.clone();
        detent_btns = detent_btns.child(
            button()
                .padding((12.0, 5.0, 12.0, 5.0))
                .corner_radius(6.0)
                .background("#1E293B")
                .foreground("#F8FAFC")
                .content(label)
                .on_click(move |_| {
                    let mut sc = sc_b.borrow_mut();
                    sc.begin_drag();
                    let delta = pos - sc.position;
                    sc.drag_by(delta * 0.15);
                    sc.end_drag(delta * 6.0);
                    detent_label.set(format!("Springing smoothly to `{label}` detent!"));
                }),
        );
    }

    let tf_m = sheet_tf.clone();
    let sc_p = scroller.clone();
    let sc_mv = scroller.clone();
    let sc_r = scroller.clone();
    let tr_p = tracker.clone();
    let tr_m = tracker.clone();
    let tr_r = tracker.clone();
    let dr_p = dragging.clone();
    let dr_m = dragging.clone();
    let dr_r = dragging.clone();
    let ly_p = last_y.clone();
    let ly_m = last_y.clone();
    let st_p = sim_t.clone();
    let st_m = sim_t.clone();

    let sheet = border()
        .canvas_left(40.0)
        .canvas_top(0.0)
        .width(740.0)
        .height(480.0)
        .background("#131C30")
        .border_brush("#38BDF8")
        .border_thickness(2.0)
        .corner_radius(16.0)
        .padding(16.0)
        .node_ref(sheet_ref)
        .with(move |b: &Ref<Border>| {
            let tf = MatrixTransform::with_matrix(Matrix::create_translation(0.0, 250.0));
            b.set_render_transform_origin(RelativePoint::CENTER);
            b.set_render_transform(Some((&tf).into()));
            *tf_m.borrow_mut() = Some(tf);
        })
        .on_pointer_pressed(move |e| {
            if e.handled() {
                return;
            }
            if let Some(b) = sheet_ref.get_untracked() {
                let input: Ref<InputElement> = b.upcast();
                e.pointer().capture(Some(&input));
            }
            let pt = e.get_position(None);
            dr_p.set(true);
            ly_p.set(pt.y as f32);
            tr_p.borrow_mut().clear();
            tr_p.borrow_mut().push(st_p.get(), Vec2::new(0.0, pt.y as f32));
            sc_p.borrow_mut().begin_drag();
        })
        .on_pointer_moved(move |e| {
            if dr_m.get() {
                let pt = e.get_position(None);
                let dy = pt.y as f32 - ly_m.get();
                ly_m.set(pt.y as f32);
                tr_m.borrow_mut().push(st_m.get(), Vec2::new(0.0, pt.y as f32));
                sc_mv.borrow_mut().drag_by(dy);
            }
        })
        .on_pointer_released(move |e| {
            e.pointer().capture(None);
            if dr_r.get() {
                dr_r.set(false);
                let vy = tr_r.borrow().velocity().y;
                sc_r.borrow_mut().end_drag(vy);
                let target_name = if vy < -180.0 {
                    "Expanded (flicked up!)"
                } else if vy > 180.0 {
                    "Collapsed (flicked down!)"
                } else {
                    "Nearest magnetic detent"
                };
                detent_label.set(format!("Released at vy={vy:.0} px/s → snapping to {target_name}"));
            }
        })
        .child(
            stack_panel()
                .spacing(12.0)
                .child(
                    border()
                        .horizontal_alignment(HorizontalAlignment::Center)
                        .width(54.0)
                        .height(5.0)
                        .corner_radius(3.0)
                        .background("#64748B"),
                )
                .child(text_block().text("▤ DRAG OR FLICK THIS BOTTOM SHEET UP / DOWN").font_size(12.0).foreground("#38BDF8"))
                .child(text_block().text("Pull past top/bottom limits to feel native Apple UIScrollView rubber-band overscroll resistance!").font_size(11.0).foreground("#94A3B8"))
                .child(text_block().text(move || format!("Master Gain: {:.0}%", gain.get())).font_size(12.0).foreground("#F8FAFC"))
                .child(slider().minimum(0.0).maximum(100.0).value(gain.two_way()))
                .child(text_block().text(move || format!("Sub-Bass Boost: {:.0}%", bass.get())).font_size(12.0).foreground("#F8FAFC"))
                .child(slider().minimum(0.0).maximum(100.0).value(bass.two_way())),
        );

    dock_panel()
        .margin(14.0)
        .child(
            stack_panel()
                .dock_panel_dock(ferroui_controls::Dock::Top)
                .orientation(Orientation::Horizontal)
                .spacing(12.0)
                .margin((0.0, 0.0, 0.0, 10.0))
                .child(detent_btns)
                .child(text_block().text(move || detent_label.get()).font_size(11.5).foreground("#7DD3FC")),
        )
        .child(
            border()
                .background("#0A0F1C")
                .border_brush("#24314A")
                .border_thickness(1.0)
                .corner_radius(12.0)
                .clip_to_bounds(true)
                .child(
                    canvas()
                        .background("#0A0F1C")
                        .clip_to_bounds(true)
                        .child(
                            text_block()
                                .canvas_left(44.0)
                                .canvas_top(26.0)
                                .text("Background Map / Content Viewport — grab the blue sheet below and flick up or down!")
                                .font_size(12.0)
                                .foreground("#64748B"),
                        )
                        .child(sheet),
                ),
        )
        .into()
}

// =============================================================================
// UX DEMO 4: Real 4-Digit macOS Passcode Keypad + Magnetic Dock + Live UI Gravity
// =============================================================================
fn demo_keypad_dock_chaos() -> Element {
    let host = Rc::new(RefCell::new(VisualPhysicsHost::new(64.0)));
    let springs = Rc::new(RefCell::new(VisualSpringDriver::new()));

    let pin = signal(String::new());
    let pin_state = signal(0i8); // 0=neutral, 1=unlocked, -1=wrong
    let gravity_on = signal(false);
    let pin_card_sid = Rc::new(RefCell::new(None::<SpringVisualId>));
    let dock_ref = NodeRef::<StackPanel>::new();
    let arena_ref = NodeRef::<Canvas>::new();
    let attached = Rc::new(Cell::new(false));
    let status = signal(String::from("Try PIN 1234 (emerald spring unlock) or any other 4 digits (macOS damped error-shake!), hover Dock, or Drop UI!"));

    let timer = start_ui_physics_timer(host.clone(), springs.clone(), move |_sync, _sp| {});
    on_cleanup(move || timer.stop());

    // Build 3x4 real tactile keypad
    let mut keypad = stack_panel().spacing(8.0);
    for row_digits in [["1", "2", "3"], ["4", "5", "6"], ["7", "8", "9"], ["C", "0", "⌫"]] {
        let mut row = stack_panel()
            .orientation(Orientation::Horizontal)
            .horizontal_alignment(HorizontalAlignment::Center)
            .spacing(8.0);
        for digit in row_digits {
            let sp_m = springs.clone();
            let sp_c = springs.clone();
            let sid_cell = Rc::new(Cell::new(None::<SpringVisualId>));
            let sid_c = sid_cell.clone();
            let pin_card_s = pin_card_sid.clone();
            let sp_card = springs.clone();

            row = row.child(
                button()
                    .width(56.0)
                    .height(42.0)
                    .corner_radius(10.0)
                    .background("#1E293B")
                    .foreground("#F8FAFC")
                    .content(digit)
                    .with(move |btn| {
                        let vis: Ref<Visual> = btn.clone().upcast();
                        let sid = sp_m.borrow_mut().attach(&vis, SpringConfig::snappy());
                        sid_cell.set(Some(sid));
                    })
                    .on_click(move |_| {
                        if let Some(sid) = sid_c.get() {
                            sp_c.borrow_mut().impulse_pop(sid, -2.4);
                        }
                        let mut cur = pin.get_untracked();
                        match digit {
                            "C" => {
                                cur.clear();
                                pin_state.set(0);
                            }
                            "⌫" => {
                                cur.pop();
                                pin_state.set(0);
                            }
                            d => {
                                if cur.len() < 4 {
                                    cur.push_str(d);
                                }
                                if cur.len() == 4 {
                                    if cur == "1234" {
                                        pin_state.set(1);
                                        if let Some(csid) = *pin_card_s.borrow() {
                                            sp_card.borrow_mut().impulse_pop(csid, 2.8);
                                        }
                                        status.set("PIN 1234 accepted — emerald tactile spring pop!".into());
                                    } else {
                                        pin_state.set(-1);
                                        if let Some(csid) = *pin_card_s.borrow() {
                                            sp_card.borrow_mut().impulse_shake(csid, 840.0);
                                        }
                                        status.set(format!("Wrong PIN `{cur}` — macOS damped horizontal error-shake fired!"));
                                        cur.clear();
                                    }
                                }
                            }
                        }
                        pin.set(cur);
                    }),
            );
        }
        keypad = keypad.child(row);
    }

    let sp_pin_m = springs.clone();
    let pin_sid_m = pin_card_sid.clone();
    let keypad_card = border()
        .canvas_left(36.0)
        .canvas_top(24.0)
        .width(260.0)
        .background("#131C30")
        .border_brush(move || match pin_state.get() {
            1 => "#10B981",
            -1 => "#F43F5E",
            _ => "#38BDF8",
        })
        .border_thickness(2.0)
        .corner_radius(14.0)
        .padding(14.0)
        .with(move |b: &Ref<Border>| {
            let vis: Ref<Visual> = b.clone().upcast();
            let sid = sp_pin_m
                .borrow_mut()
                .attach(&vis, SpringConfig::from_response_damping(0.28, 0.36));
            *pin_sid_m.borrow_mut() = Some(sid);
        })
        .child(
            stack_panel()
                .spacing(10.0)
                .child(text_block().text("🔐 MACOS PASSCODE (1234)").font_size(11.0).foreground("#7DD3FC").horizontal_alignment(HorizontalAlignment::Center))
                .child(
                    text_block()
                        .text(move || {
                            let p = pin.get();
                            let dots: String = (0..4).map(|i| if i < p.len() { '●' } else { '○' }).collect();
                            if pin_state.get() == 1 { "✓ UNLOCKED".into() } else { dots }
                        })
                        .font_size(20.0)
                        .foreground(move || if pin_state.get() == 1 { "#34D399" } else { "#F8FAFC" })
                        .horizontal_alignment(HorizontalAlignment::Center),
                )
                .child(keypad),
        );

    // Side card showcasing VisualPhysicsHost rigid-body drop & elastic home recovery on real UI!
    let host_g = host.clone();
    let host_h = host.clone();
    let host_b = host.clone();
    let att_g = attached.clone();
    let att_b = attached.clone();
    let arena_g = arena_ref;
    let arena_b = arena_ref;

    let chaos_card = border()
        .canvas_left(336.0)
        .canvas_top(24.0)
        .width(340.0)
        .background("#131C30")
        .border_brush("#A78BFA")
        .border_thickness(1.5)
        .corner_radius(14.0)
        .padding(16.0)
        .child(
            stack_panel()
                .spacing(10.0)
                .child(text_block().text("⚡ LIVE LOGICAL-TREE RIGID PHYSICS").font_size(12.0).foreground("#C4B5FD"))
                .child(text_block().text("Click below to attach real rigid bodies to these live FerroUI cards via VisualPhysicsHost::attach_logical_children — even while tumbling, the keypad & buttons still work!").font_size(11.0).foreground("#CBD5E1"))
                .child(
                    stack_panel()
                        .orientation(Orientation::Horizontal)
                        .spacing(8.0)
                        .child(
                            button()
                                .padding((10.0, 6.0, 10.0, 6.0))
                                .corner_radius(6.0)
                                .background(move || if gravity_on.get() { "#059669" } else { "#7C3AED" })
                                .foreground("#FFFFFF")
                                .content(move || if gravity_on.get() { "🌍 Gravity: ON" } else { "🌍 Drop UI" })
                                .on_click(move |_| {
                                    let next = !gravity_on.get_untracked();
                                    gravity_on.set(next);
                                    if let Some(arena) = arena_g.get_untracked() {
                                        let mut h = host_g.borrow_mut();
                                        let b = arena.bounds();
                                        h.set_container_bounds(Rect::new(0.0, 0.0, b.width, b.height));
                                        if !att_g.get() {
                                            let styled: &StyledElement = &arena;
                                            h.attach_logical_children(styled, VisualBodyOptions::dynamic_card());
                                            att_g.set(true);
                                        }
                                        if next {
                                            h.disable_home_springs();
                                            h.world.gravity = Vec2::new(0.0, 14.0);
                                        } else {
                                            h.world.gravity = Vec2::ZERO;
                                            h.enable_home_springs(3.6, 0.78);
                                        }
                                        h.world.wake_all();
                                    }
                                }),
                        )
                        .child(
                            button()
                                .padding((10.0, 6.0, 10.0, 6.0))
                                .corner_radius(6.0)
                                .background("#0284C7")
                                .foreground("#FFFFFF")
                                .content("🧲 Snap Home")
                                .on_click(move |_| {
                                    gravity_on.set(false);
                                    let mut h = host_h.borrow_mut();
                                    h.world.gravity = Vec2::ZERO;
                                    h.enable_home_springs(3.8, 0.8);
                                }),
                        )
                        .child(
                            button()
                                .padding((10.0, 6.0, 10.0, 6.0))
                                .corner_radius(6.0)
                                .background("#D97706")
                                .foreground("#FFFFFF")
                                .content("💥 Blast")
                                .on_click(move |_| {
                                    if let Some(arena) = arena_b.get_untracked() {
                                        let mut h = host_b.borrow_mut();
                                        let b = arena.bounds();
                                        h.set_container_bounds(Rect::new(0.0, 0.0, b.width, b.height));
                                        if !att_b.get() {
                                            let styled: &StyledElement = &arena;
                                            h.attach_logical_children(styled, VisualBodyOptions::dynamic_card());
                                            att_b.set(true);
                                        }
                                        h.apply_explosion_px(Vec2::new(340.0, 180.0), 600.0, 14.0);
                                        h.world.wake_all();
                                    }
                                }),
                        ),
                ),
        );

    // Magnetic Hover Dock at bottom
    let mut dock_row = stack_panel()
        .orientation(Orientation::Horizontal)
        .horizontal_alignment(HorizontalAlignment::Center)
        .spacing(12.0);
    for (label, color) in [
        ("🔍 Finder", "#0284C7"),
        ("⚡ Physics", "#7C3AED"),
        ("🎨 Studio", "#059669"),
        ("📬 Mail", "#DB2777"),
        ("⚙ Prefs", "#D97706"),
    ] {
        let sp_m = springs.clone();
        let sp_c = springs.clone();
        let sid_cell = Rc::new(Cell::new(None::<SpringVisualId>));
        let sid_c = sid_cell.clone();
        dock_row = dock_row.child(
            button()
                .padding((14.0, 9.0, 14.0, 9.0))
                .corner_radius(10.0)
                .background(color)
                .foreground("#FFFFFF")
                .content(label)
                .with(move |btn| {
                    let vis: Ref<Visual> = btn.clone().upcast();
                    let mut sp = sp_m.borrow_mut();
                    let sid = sp.attach(&vis, SpringConfig::bouncy());
                    sp.configure_magnetic_hover(sid, 130.0, 14.0, 0.09);
                    sid_cell.set(Some(sid));
                })
                .on_click(move |_| {
                    if let Some(sid) = sid_c.get() {
                        sp_c.borrow_mut().impulse_pop(sid, 3.0);
                    }
                    status.set(format!("Magnetic Dock `{label}` clicked — tactile spring bounce!"));
                }),
        );
    }

    let sp_dm = springs.clone();
    let sp_de = springs.clone();
    let dock_box = border()
        .dock_panel_dock(ferroui_controls::Dock::Bottom)
        .background("#111827")
        .border_brush("#24314A")
        .border_thickness(1.0)
        .corner_radius(10.0)
        .margin((0.0, 10.0, 0.0, 0.0))
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

    dock_panel()
        .margin(14.0)
        .child(
            text_block()
                .dock_panel_dock(ferroui_controls::Dock::Top)
                .margin((0.0, 0.0, 0.0, 8.0))
                .text(move || status.get())
                .font_size(11.5)
                .foreground("#7DD3FC"),
        )
        .child(dock_box)
        .child(
            border()
                .background("#0A0F1C")
                .border_brush("#24314A")
                .border_thickness(1.0)
                .corner_radius(12.0)
                .clip_to_bounds(true)
                .child(
                    canvas()
                        .background("#0A0F1C")
                        .clip_to_bounds(true)
                        .node_ref(arena_ref)
                        .child(keypad_card)
                        .child(chaos_card),
                ),
        )
        .into()
}
