//! Complete **Rhino Grasshopper Visual-Tree Editor (`node_graph_editor`)**:
//!
//! Authentic Rhino Grasshopper chrome & UX with one-click **Classic GH Parchment** / **Dark GH** toggle:
//! - Top Rhino Grasshopper Ribbon (`Params`, `Math`, `Logic`, `Sets`, `Vector`, `Geometry`, `Physics`)
//! - Auto-Fit framing on mount so every graph definition opens perfectly centered
//! - Layer 0: `WireBackdropLayer` (warm parchment drafting grid, pastel groups, Fancy Bezier Wires,
//!   L→R Window vs R→L Crossing marquee, minimap, DataTree hover tooltips)
//! - Layer 1: GPU-transformed `Canvas` (`MatrixTransform`) hosting keyed real FerroUI node visuals
//! - Layer 2: Floating **Multi-Select Align & Distribute Bar**
//! - Layer 3: Floating **Double-Click / Spacebar Quick-Search Popup**
//! - Layer 4: Floating **Right-Click Grasshopper Context Menu**

use crate::canvas::{VisualGraphState, WireBackdropLayer};
use crate::eval::evaluate_graph;
use crate::graph::NodeGraph;
use crate::node::NodeRegistry;
use crate::node_visual::build_node_visual;
use crate::serialization::graph_to_json;
use crate::types::{AlignMode, CanvasTheme, DataMatching, NodeCategory, NodeId, WireDisplay};
use ferroui_base::input::Key;
use ferroui_base::layout::{HorizontalAlignment, Orientation, VerticalAlignment};
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{IBrush, MatrixTransform};
use ferroui_base::threading::DispatcherTimer;
use ferroui_base::{Matrix, Ref, RelativePoint};
use ferroui_controls::{Canvas, Dock};
use ferroui_view::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

fn solid(argb: u32) -> Option<Rc<dyn IBrush>> {
    Some(Rc::new(ImmutableSolidColorBrush::from_uint32(argb)) as Rc<dyn IBrush>)
}

/// Builds a complete, Rhino Grasshopper-styled visual node graph editor `Element`.
pub fn node_graph_editor(
    graph: Rc<RefCell<NodeGraph>>,
    registry: Rc<NodeRegistry>,
    on_evaluated: Rc<dyn Fn(&NodeGraph)>,
) -> Element {
    {
        let mut g = graph.borrow_mut();
        let _ = evaluate_graph(&mut g, &registry, 0.0);
    }

    let state = VisualGraphState::new(graph.clone(), registry.clone(), on_evaluated);
    let categories = registry.categories();
    let active_cat = signal(categories.first().cloned().unwrap_or(NodeCategory::Params));
    let search_query = signal(String::new());
    let quick_query = signal(String::new());
    let fancy_wires = signal(graph.borrow().fancy_wires);
    let show_profiler = signal(graph.borrow().show_profiler);
    let solution_locked = signal(graph.borrow().solution_locked);

    // Post-layout sync + auto-fit on first mount so every graph opens perfectly framed & wires lock pixel-tight!
    {
        let st_init = state.clone();
        let ticks = std::cell::Cell::new(0u8);
        let timer = DispatcherTimer::new();
        timer.set_interval(Duration::from_millis(55));
        let timer_c = timer.clone();
        timer.tick(move |_| {
            let n = ticks.get() + 1;
            ticks.set(n);
            st_init.sync_measured_node_sizes();
            if n == 2 {
                st_init.fit_view();
            } else {
                st_init.sync_camera_transform();
            }
            if n >= 6 {
                timer_c.stop();
            }
        });
        timer.start();
        on_cleanup(move || timer.stop());
    }

    let is_light = {
        let th = state.theme;
        move || th.get() == CanvasTheme::ClassicGrasshopper
    };

    // -------------------------------------------------------------------------
    // 1. Top Rhino Grasshopper Ribbon & Canvas Toolbar
    // -------------------------------------------------------------------------
    let mut cat_tabs = stack_panel().orientation(Orientation::Horizontal).spacing(3.0);
    for cat in &categories {
        let c = cat.clone();
        let c_bg = c.clone();
        let c_fg = c.clone();
        let c_cl = c.clone();
        let th_bg = state.theme;
        let th_fg = state.theme;
        cat_tabs = cat_tabs.child(
            button()
                .padding((10.0, 4.0, 10.0, 4.0))
                .corner_radius(4.0)
                .background(move || {
                    let active = active_cat.get() == c_bg;
                    if th_bg.get() == CanvasTheme::ClassicGrasshopper {
                        if active { solid(0xFF28A745) } else { solid(0xFFD8D7CE) }
                    } else if active {
                        solid(0xFF0284C7)
                    } else {
                        solid(0xFF182235)
                    }
                })
                .foreground(move || {
                    let active = active_cat.get() == c_fg;
                    if th_fg.get() == CanvasTheme::ClassicGrasshopper && !active {
                        solid(0xFF242422)
                    } else {
                        solid(0xFFFFFFFF)
                    }
                })
                .font_size(11.0)
                .content(c.label().to_string())
                .on_click(move |_| {
                    search_query.set(String::new());
                    active_cat.set(c_cl.clone());
                }),
        );
    }

    let tool_btn_bg = {
        let th = state.theme;
        move || if th.get() == CanvasTheme::ClassicGrasshopper { solid(0xFFD8D7CE) } else { solid(0xFF1E293B) }
    };
    let tool_btn_fg = {
        let th = state.theme;
        move || if th.get() == CanvasTheme::ClassicGrasshopper { solid(0xFF1F1F1D) } else { solid(0xFFE2E8F0) }
    };

    let st_undo = state.clone();
    let st_redo = state.clone();
    let st_layout = state.clone();
    let st_fit = state.clone();
    let st_grp = state.clone();
    let st_dup = state.clone();
    let st_del = state.clone();
    let st_lock = state.clone();
    let st_fancy = state.clone();
    let st_prof = state.clone();
    let st_theme = state.clone();
    let st_json = state.clone();

    let action_bar = stack_panel()
        .orientation(Orientation::Horizontal)
        .spacing(3.0)
        .child(
            button()
                .padding((7.0, 4.0, 7.0, 4.0))
                .corner_radius(4.0)
                .background(tool_btn_bg)
                .foreground(tool_btn_fg)
                .font_size(10.5)
                .content("↶")
                .on_click(move |_| {
                    if st_undo.graph.borrow_mut().undo() {
                        st_undo.notify_topology_changed();
                        st_undo.sync_all_node_positions();
                    }
                }),
        )
        .child(
            button()
                .padding((7.0, 4.0, 7.0, 4.0))
                .corner_radius(4.0)
                .background(tool_btn_bg)
                .foreground(tool_btn_fg)
                .font_size(10.5)
                .content("↷")
                .on_click(move |_| {
                    if st_redo.graph.borrow_mut().redo() {
                        st_redo.notify_topology_changed();
                        st_redo.sync_all_node_positions();
                    }
                }),
        )
        .child(
            button()
                .padding((7.0, 4.0, 7.0, 4.0))
                .corner_radius(4.0)
                .background(tool_btn_bg)
                .foreground(tool_btn_fg)
                .font_size(10.5)
                .content("⊞ Layout")
                .on_click(move |_| {
                    st_layout.sync_measured_node_sizes();
                    st_layout.graph.borrow_mut().auto_layout();
                    st_layout.sync_all_node_positions();
                    st_layout.fit_view();
                }),
        )
        .child(
            button()
                .padding((7.0, 4.0, 7.0, 4.0))
                .corner_radius(4.0)
                .background(tool_btn_bg)
                .foreground(tool_btn_fg)
                .font_size(10.5)
                .content("⊡ Fit")
                .on_click(move |_| st_fit.fit_view()),
        )
        .child(
            button()
                .padding((7.0, 4.0, 7.0, 4.0))
                .corner_radius(4.0)
                .background(tool_btn_bg)
                .foreground(tool_btn_fg)
                .font_size(10.5)
                .content("▢ Group")
                .on_click(move |_| {
                    if st_grp.graph.borrow_mut().group_selection("Group", 0x337C3AED).is_some() {
                        st_grp.invalidate_wires();
                    }
                }),
        )
        .child(
            button()
                .padding((7.0, 4.0, 7.0, 4.0))
                .corner_radius(4.0)
                .background(tool_btn_bg)
                .foreground(tool_btn_fg)
                .font_size(10.5)
                .content("⧉ Dup")
                .on_click(move |_| {
                    if !st_dup.graph.borrow_mut().duplicate_selection().is_empty() {
                        st_dup.notify_topology_changed();
                        st_dup.notify_selection_changed();
                    }
                }),
        )
        .child(
            button()
                .padding((7.0, 4.0, 7.0, 4.0))
                .corner_radius(4.0)
                .background("#B91C1C")
                .foreground("#FFFFFF")
                .font_size(10.5)
                .content("✕")
                .on_click(move |_| {
                    if st_del.graph.borrow_mut().delete_selection() {
                        st_del.notify_topology_changed();
                        st_del.notify_selection_changed();
                    }
                }),
        )
        .child(
            button()
                .padding((7.0, 4.0, 7.0, 4.0))
                .corner_radius(4.0)
                .background(move || if solution_locked.get() { solid(0xFFD97706) } else { tool_btn_bg() })
                .foreground(move || if solution_locked.get() { solid(0xFFFFFFFF) } else { tool_btn_fg() })
                .font_size(10.5)
                .content(move || if solution_locked.get() { "🔒 Locked" } else { "🔓 Live" })
                .on_click(move |_| {
                    let next = !solution_locked.get_untracked();
                    st_lock.graph.borrow_mut().solution_locked = next;
                    solution_locked.set(next);
                    if !next {
                        st_lock.reevaluate_and_notify();
                    }
                }),
        )
        .child(
            button()
                .padding((7.0, 4.0, 7.0, 4.0))
                .corner_radius(4.0)
                .background(move || if fancy_wires.get() { solid(0xFF28A745) } else { tool_btn_bg() })
                .foreground(move || if fancy_wires.get() { solid(0xFFFFFFFF) } else { tool_btn_fg() })
                .font_size(10.5)
                .content("Fancy")
                .on_click(move |_| {
                    let next = !fancy_wires.get_untracked();
                    st_fancy.graph.borrow_mut().fancy_wires = next;
                    fancy_wires.set(next);
                    st_fancy.invalidate_wires();
                }),
        )
        .child(
            button()
                .padding((7.0, 4.0, 7.0, 4.0))
                .corner_radius(4.0)
                .background(move || if show_profiler.get() { solid(0xFF28A745) } else { tool_btn_bg() })
                .foreground(move || if show_profiler.get() { solid(0xFFFFFFFF) } else { tool_btn_fg() })
                .font_size(10.5)
                .content("Prof")
                .on_click(move |_| {
                    let next = !show_profiler.get_untracked();
                    st_prof.graph.borrow_mut().show_profiler = next;
                    show_profiler.set(next);
                    st_prof.eval_rev.update(|v| *v = v.wrapping_add(1));
                }),
        )
        .child(
            button()
                .padding((7.0, 4.0, 7.0, 4.0))
                .corner_radius(4.0)
                .background("#2B2B29")
                .foreground("#FFFFFF")
                .font_size(10.5)
                .content({
                    let th = state.theme;
                    move || th.get().label()
                })
                .on_click(move |_| {
                    let next = st_theme.theme.get_untracked().toggle();
                    st_theme.graph.borrow_mut().theme = next;
                    st_theme.theme.set(next);
                    st_theme.invalidate_wires();
                }),
        )
        .child(
            button()
                .padding((7.0, 4.0, 7.0, 4.0))
                .corner_radius(4.0)
                .background(tool_btn_bg)
                .foreground(tool_btn_fg)
                .font_size(10.5)
                .content("{ }")
                .on_click(move |_| {
                    let json = graph_to_json(&st_json.graph.borrow());
                    st_json.status_msg.set(format!(
                        "Exported Grasshopper JSON ({} bytes · {} nodes · {} wires)",
                        json.len(),
                        st_json.graph.borrow().nodes.len(),
                        st_json.graph.borrow().wires.len(),
                    ));
                }),
        );

    let st_shelf = state.clone();
    let shelf = dynamic(move || {
        let q = search_query.get();
        let cat = active_cat.get();
        let light = st_shelf.theme.get() == CanvasTheme::ClassicGrasshopper;
        let items = if q.trim().is_empty() {
            st_shelf.registry.by_category(&cat)
        } else {
            st_shelf.registry.search(&q)
        };
        let mut row = stack_panel().orientation(Orientation::Horizontal).spacing(4.0);
        for meta in items.into_iter().take(13) {
            let tid = meta.type_id.clone();
            let disp = meta.display_name.clone();
            let st_add = st_shelf.clone();
            row = row.child(
                button()
                    .padding((8.0, 3.0, 8.0, 3.0))
                    .corner_radius(4.0)
                    .background(if light { solid(0xFFF3F2EC) } else { solid(0xFF141C2F) })
                    .foreground(if light { solid(0xFF1F1F1D) } else { solid(0xFFE2E8F0) })
                    .border_brush(if light { solid(0xFFB5B3A8) } else { solid(0xFF28354D) })
                    .border_thickness(1.0)
                    .font_size(10.5)
                    .content(meta.nickname)
                    .on_click(move |_| {
                        let added = {
                            let mut g = st_add.graph.borrow_mut();
                            g.push_undo_checkpoint();
                            let cx = (220.0 - g.camera_pan[0]) / g.camera_zoom + (g.nodes.len() % 5) as f64 * 28.0;
                            let cy = (150.0 - g.camera_pan[1]) / g.camera_zoom + (g.nodes.len() % 4) as f64 * 24.0;
                            let id = g.add_node(&st_add.registry, &tid, cx, cy);
                            if let Some(nid) = id {
                                g.selected_nodes.clear();
                                g.selected_nodes.insert(nid);
                            }
                            id
                        };
                        if added.is_some() {
                            st_add.notify_topology_changed();
                            st_add.notify_selection_changed();
                            st_add.status_msg.set(format!("Placed `{disp}` component on canvas."));
                        }
                    }),
            );
        }
        row.into_element()
    });

    let ribbon = border()
        .dock_panel_dock(Dock::Top)
        .background(move || if is_light() { solid(0xFFE8E7E0) } else { solid(0xFF0D1321) })
        .border_brush(move || if is_light() { solid(0xFFB5B3A7) } else { solid(0xFF1E293B) })
        .border_thickness((0.0, 0.0, 0.0, 1.0))
        .padding((10.0, 6.0, 10.0, 6.0))
        .clip_to_bounds(true)
        .child(
            stack_panel()
                .spacing(5.0)
                .child(dock_panel().child(action_bar.dock_panel_dock(Dock::Right)).child(cat_tabs))
                .child(
                    dock_panel()
                        .child(
                            text_box()
                                .dock_panel_dock(Dock::Left)
                                .width(154.0)
                                .margin((0.0, 0.0, 8.0, 0.0))
                                .placeholder_text("🔍 Search 65+ nodes…")
                                .text(search_query.two_way()),
                        )
                        .child(shelf),
                ),
        );

    // -------------------------------------------------------------------------
    // 2. Center Layered Grasshopper Viewport
    // -------------------------------------------------------------------------
    let st_wire = state.clone();
    let wire_layer = El::<WireBackdropLayer>::new()
        .with(move |ctrl| ctrl.bind(st_wire.clone()))
        .into_element();

    let st_nodes = state.clone();
    let st_build = state.clone();
    let st_tf = state.clone();
    let world_canvas = canvas()
        .with(move |c: &Ref<Canvas>| {
            let (pan, z) = {
                let g = st_tf.graph.borrow();
                (g.camera_pan, g.camera_zoom)
            };
            let tf = MatrixTransform::with_matrix(
                Matrix::create_scale(z, z) * Matrix::create_translation(pan[0], pan[1]),
            );
            c.set_render_transform_origin(RelativePoint::TOP_LEFT);
            c.set_render_transform(Some((&tf).into()));
            *st_tf.world_transform.borrow_mut() = Some(tf);
        })
        .child(for_each(
            move || {
                let _ = st_nodes.topology_rev.get();
                let ids: Vec<NodeId> = st_nodes.graph.borrow().nodes.keys().copied().collect();
                ids
            },
            |nid| nid.0 as i64,
            move |nid| build_node_visual(st_build.clone(), nid),
        ));

    let align_overlay = build_align_overlay(&state);
    let quick_search_overlay = build_quick_search_overlay(&state, quick_query);
    let context_menu_overlay = build_context_menu_overlay(&state);

    let st_wheel = state.clone();
    let viewport_grid = grid()
        .clip_to_bounds(true)
        .on_pointer_wheel_changed(move |e| {
            if !e.handled() {
                if let Some(layer) = st_wheel.wire_layer.borrow().as_ref().and_then(|w| w.upgrade()) {
                    st_wheel.zoom_at(e.get_position(Some(&layer)), e.delta().y);
                    e.set_handled(true);
                }
            }
        })
        .child(wire_layer)
        .child(world_canvas)
        .child(align_overlay)
        .child(quick_search_overlay)
        .child(context_menu_overlay);

    // -------------------------------------------------------------------------
    // 3. Bottom Grasshopper Status & Per-Port Modifier Bar
    // -------------------------------------------------------------------------
    let inspector_bar = build_bottom_inspector(&state);

    dock_panel()
        .background(move || if is_light() { solid(0xFFD5D3C8) } else { solid(0xFF0A0F1A) })
        .clip_to_bounds(true)
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .child(ribbon)
        .child(inspector_bar)
        .child(viewport_grid)
        .into()
}

fn build_align_overlay(state: &VisualGraphState) -> Element {
    let st_vis = state.clone();
    let make_align_btn = |label: &'static str, mode: AlignMode| {
        let st_a = state.clone();
        button()
            .padding((7.0, 3.0, 7.0, 3.0))
            .corner_radius(4.0)
            .background("#2B2B29")
            .foreground("#FFFFFF")
            .font_size(10.0)
            .content(label)
            .on_click(move |_| {
                st_a.sync_measured_node_sizes();
                if st_a.graph.borrow_mut().align_selection(mode) {
                    st_a.sync_all_node_positions();
                }
            })
    };

    border()
        .horizontal_alignment(HorizontalAlignment::Center)
        .vertical_alignment(VerticalAlignment::Top)
        .margin((0.0, 10.0, 0.0, 0.0))
        .background("#F4F3EC")
        .border_brush("#28A745")
        .border_thickness(1.5)
        .corner_radius(7.0)
        .padding((8.0, 4.0, 8.0, 4.0))
        .is_visible(move || {
            let _ = st_vis.selection_rev.get();
            st_vis.graph.borrow().selected_nodes.len() >= 2
        })
        .child(
            stack_panel()
                .orientation(Orientation::Horizontal)
                .spacing(4.0)
                .child(text_block().text("ALIGN").font_size(10.0).foreground("#1E7E24").vertical_alignment(VerticalAlignment::Center))
                .child(make_align_btn("⇤ Left", AlignMode::Left))
                .child(make_align_btn("⇥ Right", AlignMode::Right))
                .child(make_align_btn("⤒ Top", AlignMode::Top))
                .child(make_align_btn("⤓ Bot", AlignMode::Bottom))
                .child(make_align_btn("⇔ Dist H", AlignMode::DistributeHorizontal))
                .child(make_align_btn("⇕ Dist V", AlignMode::DistributeVertical)),
        )
        .into()
}

fn build_quick_search_overlay(state: &VisualGraphState, quick_query: Signal<String>) -> Element {
    let st_qs = state.clone();
    canvas()
        .is_visible(move || st_qs.quick_search.get().is_some())
        .child(dynamic({
            let st_in = state.clone();
            move || -> Element {
                let Some((screen_pt, canvas_pt)) = st_in.quick_search.get() else {
                    return Element::Empty;
                };
                let q = quick_query.get();
                let matches = st_in.registry.search(&q);
                let first_match = matches.first().cloned();

                let mut results_col = stack_panel().spacing(2.0);
                for (i, meta) in matches.into_iter().take(8).enumerate() {
                    let tid = meta.type_id.clone();
                    let disp = meta.display_name.clone();
                    let cat = meta.category.label().to_string();
                    let st_pick = st_in.clone();
                    results_col = results_col.child(
                        button()
                            .horizontal_alignment(HorizontalAlignment::Stretch)
                            .padding((8.0, 4.0, 8.0, 4.0))
                            .corner_radius(4.0)
                            .background(if i == 0 { "#28A745" } else { "#ECEBE4" })
                            .foreground(if i == 0 { "#FFFFFF" } else { "#1E1E1C" })
                            .font_size(11.0)
                            .content(format!("{}  [{}]  ·  {}", meta.display_name, meta.nickname, cat))
                            .on_click(move |_| {
                                let added = {
                                    let mut g = st_pick.graph.borrow_mut();
                                    g.push_undo_checkpoint();
                                    let id = g.add_node(&st_pick.registry, &tid, canvas_pt[0], canvas_pt[1]);
                                    if let Some(nid) = id {
                                        g.selected_nodes.clear();
                                        g.selected_nodes.insert(nid);
                                    }
                                    id
                                };
                                st_pick.quick_search.set(None);
                                quick_query.set(String::new());
                                if added.is_some() {
                                    st_pick.notify_topology_changed();
                                    st_pick.notify_selection_changed();
                                    st_pick.status_msg.set(format!("Placed `{disp}` via Canvas Search."));
                                }
                            }),
                    );
                }

                let st_close = st_in.clone();
                let st_enter = st_in.clone();
                border()
                    .canvas_left(screen_pt.x.max(12.0))
                    .canvas_top(screen_pt.y.max(12.0))
                    .width(268.0)
                    .background("#F7F6F0")
                    .border_brush("#28A745")
                    .border_thickness(1.8)
                    .corner_radius(8.0)
                    .padding(9.0)
                    .child(
                        stack_panel()
                            .spacing(5.0)
                            .child(
                                dock_panel()
                                    .child(
                                        button()
                                            .dock_panel_dock(Dock::Right)
                                            .padding((5.0, 1.0, 5.0, 1.0))
                                            .corner_radius(3.0)
                                            .background("#D8D7CE")
                                            .foreground("#222220")
                                            .content("✕")
                                            .on_click(move |_| st_close.quick_search.set(None)),
                                    )
                                    .child(
                                        text_block()
                                            .text("Enter a search keyword…")
                                            .font_size(10.5)
                                            .foreground("#1E7E24"),
                                    ),
                            )
                            .child(
                                text_box()
                                    .placeholder_text("slider, grid, mapper, attr, body…")
                                    .text(quick_query.two_way())
                                    .on_key_down(move |e| {
                                        if e.key == Key::Return {
                                            if let Some(ref m) = first_match {
                                                let added = {
                                                    let mut g = st_enter.graph.borrow_mut();
                                                    g.push_undo_checkpoint();
                                                    let id = g.add_node(&st_enter.registry, &m.type_id, canvas_pt[0], canvas_pt[1]);
                                                    if let Some(nid) = id {
                                                        g.selected_nodes.clear();
                                                        g.selected_nodes.insert(nid);
                                                    }
                                                    id
                                                };
                                                st_enter.quick_search.set(None);
                                                quick_query.set(String::new());
                                                if added.is_some() {
                                                    st_enter.notify_topology_changed();
                                                    st_enter.notify_selection_changed();
                                                }
                                                e.set_handled(true);
                                            }
                                        }
                                    }),
                            )
                            .child(results_col),
                    )
                    .into()
            }
        }))
        .into()
}

fn build_context_menu_overlay(state: &VisualGraphState) -> Element {
    let st_cm = state.clone();
    canvas()
        .is_visible(move || st_cm.context_menu.get().is_some())
        .child(dynamic({
            let st_in = state.clone();
            move || -> Element {
                let Some((nid, screen_pt)) = st_in.context_menu.get() else {
                    return Element::Empty;
                };
                let (title, disabled, matching) = {
                    let g = st_in.graph.borrow();
                    let Some(n) = g.nodes.get(&nid) else { return Element::Empty };
                    let t = n
                        .custom_title
                        .clone()
                        .or_else(|| st_in.registry.get(&n.type_id).map(|c| c.display_name().to_string()))
                        .unwrap_or_else(|| n.type_id.clone());
                    (t, n.disabled, n.matching)
                };

                let st_dis = st_in.clone();
                let st_lac = st_in.clone();
                let st_wdef = st_in.clone();
                let st_wfnt = st_in.clone();
                let st_whid = st_in.clone();
                let st_dup = st_in.clone();
                let st_del = st_in.clone();
                let st_cls = st_in.clone();

                let menu_btn = |text: String, bg: &'static str, fg: &'static str| {
                    button()
                        .horizontal_alignment(HorizontalAlignment::Stretch)
                        .padding((8.0, 4.0, 8.0, 4.0))
                        .corner_radius(4.0)
                        .background(bg)
                        .foreground(fg)
                        .font_size(11.0)
                        .content(text)
                };

                border()
                    .canvas_left(screen_pt.x.max(12.0))
                    .canvas_top(screen_pt.y.max(12.0))
                    .width(216.0)
                    .background("#F7F6F0")
                    .border_brush("#4A4A46")
                    .border_thickness(1.4)
                    .corner_radius(7.0)
                    .padding(7.0)
                    .child(
                        stack_panel()
                            .spacing(3.0)
                            .child(
                                dock_panel()
                                    .child(
                                        button()
                                            .dock_panel_dock(Dock::Right)
                                            .padding((5.0, 1.0, 5.0, 1.0))
                                            .corner_radius(3.0)
                                            .background("#D8D7CE")
                                            .foreground("#222220")
                                            .content("✕")
                                            .on_click(move |_| st_cls.context_menu.set(None)),
                                    )
                                    .child(text_block().text(title).font_size(11.0).foreground("#1E7E24")),
                            )
                            .child(
                                menu_btn(
                                    if disabled { "▶ Enable (Ctrl+E)".into() } else { "⏸ Disable (Ctrl+E)".into() },
                                    "#E8E7E0",
                                    "#1E1E1C",
                                )
                                .on_click(move |_| {
                                    {
                                        let mut g = st_dis.graph.borrow_mut();
                                        g.push_undo_checkpoint();
                                        if let Some(n) = g.nodes.get_mut(&nid) {
                                            n.disabled = !n.disabled;
                                        }
                                        g.mark_dirty(nid);
                                    }
                                    st_dis.context_menu.set(None);
                                    st_dis.reevaluate_and_notify();
                                }),
                            )
                            .child(
                                menu_btn(format!("Lacing: {matching:?} ↻"), "#E8E7E0", "#1E1E1C").on_click(move |_| {
                                    {
                                        let mut g = st_lac.graph.borrow_mut();
                                        if let Some(n) = g.nodes.get_mut(&nid) {
                                            n.matching = match n.matching {
                                                DataMatching::LongestList => DataMatching::ShortestList,
                                                DataMatching::ShortestList => DataMatching::CrossReference,
                                                DataMatching::CrossReference => DataMatching::LongestList,
                                            };
                                        }
                                        g.mark_dirty(nid);
                                    }
                                    st_lac.context_menu.set(None);
                                    st_lac.reevaluate_and_notify();
                                }),
                            )
                            .child(
                                stack_panel()
                                    .orientation(Orientation::Horizontal)
                                    .spacing(3.0)
                                    .child(
                                        button()
                                            .padding((6.0, 3.0, 6.0, 3.0))
                                            .corner_radius(3.0)
                                            .background("#2B2B29")
                                            .foreground("#FFFFFF")
                                            .font_size(10.0)
                                            .content("Wire: Def")
                                            .on_click(move |_| {
                                                st_wdef.graph.borrow_mut().set_node_wire_display(nid, WireDisplay::Default);
                                                st_wdef.context_menu.set(None);
                                                st_wdef.invalidate_wires();
                                            }),
                                    )
                                    .child(
                                        button()
                                            .padding((6.0, 3.0, 6.0, 3.0))
                                            .corner_radius(3.0)
                                            .background("#D8D7CE")
                                            .foreground("#1E1E1C")
                                            .font_size(10.0)
                                            .content("Faint")
                                            .on_click(move |_| {
                                                st_wfnt.graph.borrow_mut().set_node_wire_display(nid, WireDisplay::Faint);
                                                st_wfnt.context_menu.set(None);
                                                st_wfnt.invalidate_wires();
                                            }),
                                    )
                                    .child(
                                        button()
                                            .padding((6.0, 3.0, 6.0, 3.0))
                                            .corner_radius(3.0)
                                            .background("#D8D7CE")
                                            .foreground("#1E1E1C")
                                            .font_size(10.0)
                                            .content("Hidden")
                                            .on_click(move |_| {
                                                st_whid.graph.borrow_mut().set_node_wire_display(nid, WireDisplay::Hidden);
                                                st_whid.context_menu.set(None);
                                                st_whid.invalidate_wires();
                                            }),
                                    ),
                            )
                            .child(menu_btn("⧉ Duplicate (Ctrl+D)".into(), "#E8E7E0", "#1E1E1C").on_click(move |_| {
                                {
                                    let mut g = st_dup.graph.borrow_mut();
                                    g.selected_nodes.clear();
                                    g.selected_nodes.insert(nid);
                                    g.duplicate_selection();
                                }
                                st_dup.context_menu.set(None);
                                st_dup.notify_topology_changed();
                                st_dup.notify_selection_changed();
                            }))
                            .child(menu_btn("✕ Delete (Del)".into(), "#DC2626", "#FFFFFF").on_click(move |_| {
                                {
                                    let mut g = st_del.graph.borrow_mut();
                                    g.push_undo_checkpoint();
                                    g.remove_node(nid);
                                }
                                st_del.context_menu.set(None);
                                st_del.notify_topology_changed();
                                st_del.notify_selection_changed();
                            })),
                    )
                    .into()
            }
        }))
        .into()
}

fn build_bottom_inspector(state: &VisualGraphState) -> Element {
    let st_i = state.clone();
    let th_bg = state.theme;
    let th_bd = state.theme;
    border()
        .dock_panel_dock(Dock::Bottom)
        .background(move || if th_bg.get() == CanvasTheme::ClassicGrasshopper { solid(0xFFE8E7E0) } else { solid(0xFF0B101D) })
        .border_brush(move || if th_bd.get() == CanvasTheme::ClassicGrasshopper { solid(0xFFB5B3A7) } else { solid(0xFF1E293B) })
        .border_thickness((0.0, 1.0, 0.0, 0.0))
        .padding((10.0, 5.0, 10.0, 5.0))
        .clip_to_bounds(true)
        .child(dynamic(move || -> Element {
            let _ = st_i.selection_rev.get();
            let _ = st_i.eval_rev.get();
            let light = st_i.theme.get() == CanvasTheme::ClassicGrasshopper;
            let sel_id = st_i.graph.borrow().selected_nodes.iter().next().copied();
            let Some(nid) = sel_id else {
                let msg = st_i.status_msg;
                return text_block()
                    .text(move || msg.get())
                    .font_size(10.5)
                    .foreground(if light { solid(0xFF3E3E3A) } else { solid(0xFF7DD3FC) })
                    .into();
            };

            let (title, out_summary, cur_match, in0_mod, has_in) = {
                let g = st_i.graph.borrow();
                let Some(n) = g.nodes.get(&nid) else { return text_block().text("").into() };
                let comp = st_i.registry.get(&n.type_id);
                let title = n
                    .custom_title
                    .clone()
                    .or_else(|| comp.map(|c| c.display_name().to_string()))
                    .unwrap_or_else(|| n.type_id.clone());
                let out_s = n
                    .output_tree(0)
                    .map(|t| format!("Out[0]: {} br · {} items · {}", t.branch_count(), t.item_count(), t.format_panel(1)))
                    .unwrap_or_else(|| "Empty parameter".into());
                let m0 = n.input_modifiers.first().copied().unwrap_or_default();
                (title, out_s, n.matching, m0, !n.input_modifiers.is_empty())
            };

            let st_m = st_i.clone();
            let mut bar = stack_panel()
                .orientation(Orientation::Horizontal)
                .spacing(8.0)
                .child(text_block().text(title).font_size(11.0).foreground(if light { solid(0xFF1E7E24) } else { solid(0xFF34D399) }))
                .child(text_block().text(out_summary).font_size(10.5).foreground(if light { solid(0xFF262624) } else { solid(0xFFCBD5E1) }))
                .child(
                    button()
                        .padding((7.0, 2.0, 7.0, 2.0))
                        .corner_radius(4.0)
                        .background("#2B2B29")
                        .foreground("#FFFFFF")
                        .font_size(10.0)
                        .content(format!("Lacing: {cur_match:?}"))
                        .on_click(move |_| {
                            {
                                let mut g = st_m.graph.borrow_mut();
                                if let Some(n) = g.nodes.get_mut(&nid) {
                                    n.matching = match n.matching {
                                        DataMatching::LongestList => DataMatching::ShortestList,
                                        DataMatching::ShortestList => DataMatching::CrossReference,
                                        DataMatching::CrossReference => DataMatching::LongestList,
                                    };
                                }
                                g.mark_dirty(nid);
                            }
                            st_m.reevaluate_and_notify();
                        }),
                );

            if has_in {
                let make_mod = |lbl: &'static str, active: bool, col: u32, which: u8| {
                    let st_x = st_i.clone();
                    button()
                        .padding((6.0, 2.0, 6.0, 2.0))
                        .corner_radius(4.0)
                        .background(if active { solid(col) } else if light { solid(0xFFD8D7CE) } else { solid(0xFF1E293B) })
                        .foreground(if active || !light { solid(0xFFFFFFFF) } else { solid(0xFF222220) })
                        .font_size(10.0)
                        .content(lbl)
                        .on_click(move |_| {
                            {
                                let mut g = st_x.graph.borrow_mut();
                                if let Some(n) = g.nodes.get_mut(&nid) {
                                    if let Some(m) = n.input_modifiers.get_mut(0) {
                                        match which {
                                            0 => m.flatten = !m.flatten,
                                            1 => m.graft = !m.graft,
                                            2 => m.reverse = !m.reverse,
                                            _ => m.simplify = !m.simplify,
                                        }
                                    }
                                }
                                g.mark_dirty(nid);
                            }
                            st_x.reevaluate_and_notify();
                        })
                };
                bar = bar
                    .child(make_mod("↓ Flatten", in0_mod.flatten, 0xFF0284C7, 0))
                    .child(make_mod("↑ Graft", in0_mod.graft, 0xFF7C3AED, 1))
                    .child(make_mod("⇄ Reverse", in0_mod.reverse, 0xFFD97706, 2))
                    .child(make_mod("∗ Simplify", in0_mod.simplify, 0xFF28A745, 3));
            }
            bar.into()
        }))
        .into()
}
