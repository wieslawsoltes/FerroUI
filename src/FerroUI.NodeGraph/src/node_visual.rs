//! **Real FerroUI Visual-Tree Node Builder (`build_node_visual`)**.
//!
//! Every node on the Grasshopper canvas is a genuine FerroUI control tree (`Border`, `StackPanel`,
//! `DockPanel`, `Slider`, `TextBox`, `Button`, `TextBlock`, [`GraphMapperControl`], [`MdSliderControl`])
//! with real pointer-captured dragging, real port socket visuals registered for pixel-perfect
//! Bezier wires, reactive Grasshopper state tinting, and live microsecond Profiler pills!

use crate::canvas::{ActiveWireDrag, VisualGraphState};
use crate::types::{NodeId, NodeKind, NodeState, ParamWidget, PortDirection, PortSpec};
use crate::widgets::{GraphMapperControl, MdSliderControl};
use ferroui_base::input::{InputElement, KeyModifiers};
use ferroui_base::layout::{HorizontalAlignment, Orientation, VerticalAlignment};
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::IBrush;
use ferroui_base::{Point, Ref, Visual};
use ferroui_controls::{Border, Canvas, Dock};
use ferroui_view::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

const SWATCH_COLORS: [u32; 8] = [
    0xFF38BDF8, 0xFF34D399, 0xFFA78BFA, 0xFFF472B6, 0xFFFBBF24, 0xFFFB923C, 0xFFFB7185, 0xFF2DD4BF,
];

fn solid_brush(argb: u32) -> Option<Rc<dyn IBrush>> {
    Some(Rc::new(ImmutableSolidColorBrush::from_uint32(argb)) as Rc<dyn IBrush>)
}

/// Builds a complete, interactive FerroUI Visual tree for a single Grasshopper node `nid`.
pub fn build_node_visual(state: VisualGraphState, nid: NodeId) -> Element {
    let (type_id, init_pos, init_title, init_widget) = {
        let g = state.graph.borrow();
        let Some(n) = g.nodes.get(&nid) else {
            return Element::Empty;
        };
        (n.type_id.clone(), n.position, n.custom_title.clone(), n.widget.clone())
    };
    let Some(comp) = state.registry.get(&type_id).cloned() else {
        return Element::Empty;
    };

    let kind = comp.kind();
    let title = init_title.unwrap_or_else(|| comp.nickname().to_string());
    let accent_argb = comp.category().accent_argb();
    let inputs = comp.inputs();
    let outputs = comp.outputs();

    let node_ref = NodeRef::<Border>::new();
    let drag_state: Rc<RefCell<Option<(Point, Vec<(NodeId, [f64; 2])>)>>> = Rc::new(RefCell::new(None));

    // Build real FerroUI Port Socket rows on left (inputs) and right (outputs)
    let mut left_ports = stack_panel().spacing(5.0).vertical_alignment(VerticalAlignment::Center);
    for (pidx, spec) in inputs.iter().enumerate() {
        left_ports = left_ports.child(build_port_row(&state, nid, pidx, PortDirection::Input, spec));
    }

    let mut right_ports = stack_panel().spacing(5.0).vertical_alignment(VerticalAlignment::Center);
    for (pidx, spec) in outputs.iter().enumerate() {
        right_ports = right_ports.child(build_port_row(&state, nid, pidx, PortDirection::Output, spec));
    }

    // Center interactive real FerroUI control body for every Grasshopper NodeKind!
    let center_body = build_node_center(&state, nid, kind, &title, accent_argb, init_widget);

    // Assemble the node capsule interior
    let inner_layout: Element = match kind {
        NodeKind::Relay => stack_panel()
            .orientation(Orientation::Horizontal)
            .spacing(6.0)
            .vertical_alignment(VerticalAlignment::Center)
            .child(left_ports)
            .child(text_block().text("●").font_size(10.0).foreground("#7DD3FC"))
            .child(right_ports)
            .into(),
        NodeKind::Scribble => center_body,
        _ => {
            let mut row = dock_panel();
            if !inputs.is_empty() {
                row = row.child(left_ports.dock_panel_dock(Dock::Left).margin((0.0, 0.0, 6.0, 0.0)));
            }
            if !outputs.is_empty() {
                row = row.child(right_ports.dock_panel_dock(Dock::Right).margin((6.0, 0.0, 0.0, 0.0)));
            }
            row.child(center_body).into()
        }
    };

    // Reactive border brush & opacity reflecting Selection, Warning, Error, Disabled
    let st_bd = state.clone();
    let border_brush = move || -> &'static str {
        let _ = st_bd.selection_rev.get();
        let _ = st_bd.eval_rev.get();
        let g = st_bd.graph.borrow();
        if g.selected_nodes.contains(&nid) {
            return "#10B981";
        }
        match g.nodes.get(&nid).map(|n| &n.state) {
            Some(NodeState::Error(_)) => "#F43F5E",
            Some(NodeState::Warning(_)) => "#F59E0B",
            Some(NodeState::Disabled) => "#475569",
            _ => "#334155",
        }
    };

    let st_bg = state.clone();
    let bg_color = move || -> &'static str {
        let _ = st_bg.eval_rev.get();
        let g = st_bg.graph.borrow();
        match kind {
            NodeKind::Panel => "#1A2216",
            NodeKind::ParamViewer => "#141D2E",
            NodeKind::Scribble => "Transparent",
            _ => match g.nodes.get(&nid).map(|n| &n.state) {
                Some(NodeState::Error(_)) => "#2A121C",
                Some(NodeState::Warning(_)) => "#271F11",
                Some(NodeState::Disabled) => "#0E1422",
                _ => "#161F33",
            },
        }
    };

    let st_op = state.clone();
    let node_opacity = move || -> f64 {
        let _ = st_op.eval_rev.get();
        if st_op.graph.borrow().nodes.get(&nid).is_some_and(|n| n.disabled) {
            0.46
        } else {
            1.0
        }
    };

    // Live Grasshopper Profiler Pill beneath node!
    let st_prof = state.clone();
    let profiler_pill = border()
        .horizontal_alignment(HorizontalAlignment::Center)
        .background("#1E293B")
        .corner_radius(4.0)
        .padding((5.0, 1.0, 5.0, 1.0))
        .margin((0.0, 2.0, 0.0, 0.0))
        .is_visible(move || {
            let _ = st_prof.eval_rev.get();
            st_prof.graph.borrow().show_profiler && !matches!(kind, NodeKind::Relay | NodeKind::Scribble)
        })
        .child(
            text_block()
                .font_size(9.0)
                .foreground("#94A3B8")
                .text({
                    let st_pt = state.clone();
                    move || {
                        let _ = st_pt.eval_rev.get();
                        let us = st_pt.graph.borrow().nodes.get(&nid).map(|n| n.eval_us).unwrap_or(0);
                        if us >= 1000 {
                            format!("{:.2} ms", us as f64 / 1000.0)
                        } else {
                            format!("{us} µs")
                        }
                    }
                }),
        );

    // Header context menu trigger + status error/warning indicator
    let st_ctx = state.clone();
    let header_bar = border()
        .background("#0F1626")
        .border_brush(solid_brush(accent_argb))
        .border_thickness((0.0, 2.5, 0.0, 0.0))
        .corner_radius(6.0)
        .padding((6.0, 2.0, 4.0, 2.0))
        .margin((0.0, 0.0, 0.0, 4.0))
        .is_visible(!matches!(kind, NodeKind::Relay | NodeKind::Scribble))
        .child(
            dock_panel()
                .child(
                    button()
                        .dock_panel_dock(Dock::Right)
                        .padding((4.0, 0.0, 4.0, 0.0))
                        .corner_radius(4.0)
                        .background("Transparent")
                        .foreground("#94A3B8")
                        .font_size(11.0)
                        .content("⋯")
                        .on_click(move |_| {
                            let pt = st_ctx
                                .wire_layer
                                .borrow()
                                .as_ref()
                                .and_then(|w| w.upgrade())
                                .and_then(|layer| {
                                    node_ref.get_untracked().and_then(|b| {
                                        let vis: Ref<Visual> = b.upcast();
                                        vis.translate_point(Point::new(140.0, 24.0), &layer)
                                    })
                                })
                                .unwrap_or(Point::new(260.0, 140.0));
                            st_ctx.context_menu.set(Some((nid, pt)));
                        }),
                )
                .child(
                    text_block()
                        .text(title.clone())
                        .font_size(10.5)
                        .foreground("#CBD5E1"),
                ),
        );

    let st_m = state.clone();
    let st_p = state.clone();
    let st_mv = state.clone();
    let st_r = state.clone();
    let ds_p = drag_state.clone();
    let ds_m = drag_state.clone();
    let ds_r = drag_state.clone();

    let capsule = border()
        .background(bg_color)
        .border_brush(border_brush)
        .border_thickness(if kind == NodeKind::Scribble { 0.0 } else { 1.8 })
        .corner_radius(if kind == NodeKind::Relay { 14.0 } else { 9.0 })
        .padding(if kind == NodeKind::Relay { (6.0, 4.0, 6.0, 4.0) } else { (6.0, 5.0, 6.0, 6.0) })
        .opacity(node_opacity)
        .child(stack_panel().spacing(0.0).child(header_bar).child(inner_layout));

    // Outer positioned container on the GPU-transformed world Canvas
    border()
        .canvas_left(init_pos[0])
        .canvas_top(init_pos[1])
        .node_ref(node_ref)
        .with(move |b: &Ref<Border>| {
            let vis: Ref<Visual> = b.clone().upcast();
            st_m.node_visuals.borrow_mut().insert(nid, vis.downgrade());
            // Keep wire layer refreshed as soon as layout measures the real control bounds
            st_m.invalidate_wires();
        })
        .on_pointer_pressed(move |e| {
            if e.handled() {
                return;
            }
            e.set_handled(true);
            st_p.quick_search.set(None);

            // Right-click on node opens the floating Grasshopper Context Menu!
            if e.properties().is_right_button_pressed {
                let pt = st_p
                    .wire_layer
                    .borrow()
                    .as_ref()
                    .and_then(|w| w.upgrade())
                    .map(|layer| e.get_position(Some(&layer)))
                    .unwrap_or_else(|| e.get_position(None));
                {
                    let mut g = st_p.graph.borrow_mut();
                    if !g.selected_nodes.contains(&nid) {
                        g.selected_nodes.clear();
                        g.selected_nodes.insert(nid);
                    }
                }
                st_p.notify_selection_changed();
                st_p.context_menu.set(Some((nid, pt)));
                return;
            }

            st_p.context_menu.set(None);
            if let Some(b) = node_ref.get_untracked() {
                let input: Ref<InputElement> = b.upcast();
                e.pointer().capture(Some(&input));
            }
            let shift = e.key_modifiers().contains(KeyModifiers::SHIFT);
            let originals = {
                let mut g = st_p.graph.borrow_mut();
                g.selected_wires.clear();
                if shift {
                    if g.selected_nodes.contains(&nid) {
                        g.selected_nodes.remove(&nid);
                    } else {
                        g.selected_nodes.insert(nid);
                    }
                } else if !g.selected_nodes.contains(&nid) {
                    g.selected_nodes.clear();
                    g.selected_nodes.insert(nid);
                }
                g.push_undo_checkpoint();
                g.selected_nodes
                    .iter()
                    .filter_map(|id| g.nodes.get(id).map(|n| (*id, n.position)))
                    .collect::<Vec<_>>()
            };
            st_p.notify_selection_changed();
            let start_pt = e.get_position(None);
            *ds_p.borrow_mut() = Some((start_pt, originals));
        })
        .on_pointer_moved(move |e| {
            let Some((start_pt, ref originals)) = *ds_m.borrow() else {
                return;
            };
            e.set_handled(true);
            let cur = e.get_position(None);
            let z = st_mv.graph.borrow().camera_zoom.max(0.1);
            let dx = (cur.x - start_pt.x) / z;
            let dy = (cur.y - start_pt.y) / z;

            {
                let mut g = st_mv.graph.borrow_mut();
                let snap = g.snap_to_grid;
                let visuals = st_mv.node_visuals.borrow();
                for &(id, orig) in originals {
                    let nx = if snap { ((orig[0] + dx) / 20.0).round() * 20.0 } else { orig[0] + dx };
                    let ny = if snap { ((orig[1] + dy) / 20.0).round() * 20.0 } else { orig[1] + dy };
                    if let Some(n) = g.nodes.get_mut(&id) {
                        n.position = [nx, ny];
                    }
                    if let Some(weak) = visuals.get(&id) {
                        if let Some(vis) = weak.upgrade() {
                            Canvas::set_left(&vis, nx);
                            Canvas::set_top(&vis, ny);
                            let b = vis.bounds();
                            if b.width > 10.0 && b.height > 10.0 {
                                if let Some(n) = g.nodes.get_mut(&id) {
                                    n.size = [b.width, b.height];
                                }
                            }
                        }
                    }
                }
            }
            st_mv.invalidate_wires();
        })
        .on_pointer_released(move |e| {
            if ds_r.borrow_mut().take().is_some() {
                e.set_handled(true);
                e.pointer().capture(None);
                st_r.invalidate_wires();
            }
        })
        .child(stack_panel().spacing(0.0).child(capsule).child(profiler_pill))
        .into()
}

/// Builds a real FerroUI Port Socket visual (`Border` circle + nickname + Grasshopper `F/G/R/S` badge).
fn build_port_row(
    state: &VisualGraphState,
    nid: NodeId,
    pidx: usize,
    dir: PortDirection,
    spec: &PortSpec,
) -> Element {
    let sock_ref = NodeRef::<Border>::new();
    let sock_brush = solid_brush(spec.port_type.color_argb());

    let st_m = state.clone();
    let st_p = state.clone();
    let st_mv = state.clone();
    let st_r = state.clone();
    let st_en = state.clone();
    let st_ex = state.clone();

    let socket = border()
        .width(12.0)
        .height(12.0)
        .corner_radius(6.0)
        .background(sock_brush)
        .border_brush("#090D16")
        .border_thickness(1.6)
        .node_ref(sock_ref)
        .with(move |b: &Ref<Border>| {
            let vis: Ref<Visual> = b.clone().upcast();
            st_m.socket_visuals.borrow_mut().insert((nid, dir, pidx), vis.downgrade());
            st_m.invalidate_wires();
        })
        .on_pointer_entered(move |e| {
            let pt = st_en
                .wire_layer
                .borrow()
                .as_ref()
                .and_then(|w| w.upgrade())
                .map(|l| e.get_position(Some(&l)))
                .unwrap_or_else(|| e.get_position(None));
            *st_en.hover_port.borrow_mut() = Some((nid, dir, pidx, pt));
            st_en.invalidate_wires();
        })
        .on_pointer_exited(move |_| {
            *st_ex.hover_port.borrow_mut() = None;
            st_ex.invalidate_wires();
        })
        .on_pointer_pressed(move |e| {
            e.set_handled(true);
            // Right-click or Alt+click on an input socket disconnects its wires!
            if (e.properties().is_right_button_pressed || e.key_modifiers().contains(KeyModifiers::ALT))
                && dir == PortDirection::Input
            {
                let removed = {
                    let mut g = st_p.graph.borrow_mut();
                    g.push_undo_checkpoint();
                    g.disconnect_input_port(nid, pidx)
                };
                if removed > 0 {
                    st_p.reevaluate_and_notify();
                }
                return;
            }
            if let Some(b) = sock_ref.get_untracked() {
                let input: Ref<InputElement> = b.upcast();
                e.pointer().capture(Some(&input));
            }
            let spt = st_p
                .wire_layer
                .borrow()
                .as_ref()
                .and_then(|w| w.upgrade())
                .map(|l| e.get_position(Some(&l)))
                .unwrap_or_else(|| e.get_position(None));
            *st_p.active_wire.borrow_mut() = Some(ActiveWireDrag {
                from_node: nid,
                from_port: pidx,
                from_dir: dir,
                cursor_screen: spt,
                multi_wire: e.key_modifiers().contains(KeyModifiers::SHIFT),
                snap_target: None,
            });
            st_p.invalidate_wires();
        })
        .on_pointer_moved(move |e| {
            let has_drag = st_mv.active_wire.borrow().is_some();
            if !has_drag {
                return;
            }
            e.set_handled(true);
            let spt = st_mv
                .wire_layer
                .borrow()
                .as_ref()
                .and_then(|w| w.upgrade())
                .map(|l| e.get_position(Some(&l)))
                .unwrap_or_else(|| e.get_position(None));
            let snap = st_mv.find_snap_socket(spt, nid, pidx, dir);
            if let Some(drag) = st_mv.active_wire.borrow_mut().as_mut() {
                drag.cursor_screen = spt;
                drag.snap_target = snap;
            }
            st_mv.invalidate_wires();
        })
        .on_pointer_released(move |e| {
            if st_r.active_wire.borrow().is_some() {
                e.set_handled(true);
                e.pointer().capture(None);
                let spt = st_r
                    .wire_layer
                    .borrow()
                    .as_ref()
                    .and_then(|w| w.upgrade())
                    .map(|l| e.get_position(Some(&l)))
                    .unwrap_or_else(|| e.get_position(None));
                st_r.finish_wire_drag(spt, e.key_modifiers().contains(KeyModifiers::SHIFT));
            }
        });

    let nick = spec.nickname.clone();
    let label = text_block()
        .text({
            let st_l = state.clone();
            move || {
                let _ = st_l.eval_rev.get();
                if dir == PortDirection::Input {
                    let badge = st_l
                        .graph
                        .borrow()
                        .nodes
                        .get(&nid)
                        .and_then(|n| n.input_modifiers.get(pidx))
                        .map(|m| m.badge())
                        .unwrap_or_default();
                    if badge.is_empty() {
                        nick.clone()
                    } else {
                        format!("{nick} [{badge}]")
                    }
                } else {
                    nick.clone()
                }
            }
        })
        .font_size(10.0)
        .foreground("#94A3B8")
        .vertical_alignment(VerticalAlignment::Center);

    let mut row = stack_panel()
        .orientation(Orientation::Horizontal)
        .spacing(4.0)
        .vertical_alignment(VerticalAlignment::Center);
    match dir {
        PortDirection::Input => {
            row = row.child(socket).child(label);
        }
        PortDirection::Output => {
            row = row.horizontal_alignment(HorizontalAlignment::Right).child(label).child(socket);
        }
    }
    row.into()
}

/// Builds the real FerroUI interactive center control for every Grasshopper [`NodeKind`].
fn build_node_center(
    state: &VisualGraphState,
    nid: NodeId,
    kind: NodeKind,
    title: &str,
    accent_argb: u32,
    init_widget: ParamWidget,
) -> Element {
    match kind {
        NodeKind::NumberSlider => {
            let (init_v, min, max, step, integer) = match init_widget {
                ParamWidget::Slider { value, min, max, step, integer } => (value, min, max, step, integer),
                _ => (5.0, 0.0, 10.0, 0.1, false),
            };
            let val_sig = signal(init_v);
            let st_s = state.clone();
            stack_panel()
                .width(178.0)
                .spacing(3.0)
                .child(
                    dock_panel()
                        .child(
                            text_block()
                                .dock_panel_dock(Dock::Right)
                                .font_size(11.0)
                                .foreground("#38BDF8")
                                .text(move || {
                                    let v = val_sig.get();
                                    if integer { format!("{:.0}", v.round()) } else { format!("{v:.2}") }
                                }),
                        )
                        .child(
                            text_block()
                                .font_size(9.5)
                                .foreground("#64748B")
                                .text(format!("{min:.1} .. {max:.1}")),
                        ),
                )
                .child(
                    slider()
                        .minimum(min)
                        .maximum(max)
                        .value(val_sig.two_way())
                        .on_changed(ferroui_controls::primitives::RangeBase::value_property(), move |v| {
                            let mut val = *v;
                            if step > 0.0 {
                                val = ((val - min) / step).round() * step + min;
                            }
                            if integer {
                                val = val.round();
                            }
                            val = val.clamp(min, max);
                            let mut changed = false;
                            {
                                let mut g = st_s.graph.borrow_mut();
                                if let Some(n) = g.nodes.get_mut(&nid) {
                                    if let ParamWidget::Slider { ref mut value, .. } = n.widget {
                                        if (*value - val).abs() > 1e-6 {
                                            *value = val;
                                            changed = true;
                                        }
                                    }
                                }
                                if changed {
                                    g.mark_dirty(nid);
                                }
                            }
                            if changed {
                                st_s.reevaluate_and_notify();
                            }
                        }),
                )
                .into()
        }

        NodeKind::BoolToggle => {
            let init_b = matches!(init_widget, ParamWidget::Toggle { value: true });
            let sig = signal(init_b);
            let st_t = state.clone();
            button()
                .width(96.0)
                .padding((10.0, 4.0, 10.0, 4.0))
                .corner_radius(10.0)
                .background(move || if sig.get() { "#059669" } else { "#334155" })
                .foreground("#FFFFFF")
                .content(move || if sig.get() { "● True" } else { "○ False" })
                .on_click(move |_| {
                    let next = !sig.get_untracked();
                    sig.set(next);
                    {
                        let mut g = st_t.graph.borrow_mut();
                        g.push_undo_checkpoint();
                        if let Some(n) = g.nodes.get_mut(&nid) {
                            n.widget = ParamWidget::Toggle { value: next };
                        }
                        g.mark_dirty(nid);
                    }
                    st_t.reevaluate_and_notify();
                })
                .into()
        }

        NodeKind::MomentaryButton => {
            let st_b = state.clone();
            button()
                .width(96.0)
                .padding((10.0, 5.0, 10.0, 5.0))
                .corner_radius(7.0)
                .background("#0284C7")
                .foreground("#FFFFFF")
                .content("▶ Fire Pulse")
                .on_click(move |_| {
                    {
                        let mut g = st_b.graph.borrow_mut();
                        if let Some(n) = g.nodes.get_mut(&nid) {
                            n.widget = ParamWidget::Button { pressed: true };
                        }
                        g.mark_dirty(nid);
                    }
                    st_b.reevaluate_and_notify();
                })
                .into()
        }

        NodeKind::ColorSwatch => {
            let init_c = match init_widget {
                ParamWidget::Color { argb } => argb,
                _ => 0xFF38BDF8,
            };
            let col_sig = signal(init_c);
            let mut row = stack_panel().orientation(Orientation::Horizontal).spacing(4.0);
            for &c in &SWATCH_COLORS[..5] {
                let st_c = state.clone();
                row = row.child(
                    button()
                        .width(18.0)
                        .height(18.0)
                        .corner_radius(9.0)
                        .background(solid_brush(c))
                        .content("")
                        .on_click(move |_| {
                            col_sig.set(c);
                            {
                                let mut g = st_c.graph.borrow_mut();
                                g.push_undo_checkpoint();
                                if let Some(n) = g.nodes.get_mut(&nid) {
                                    n.widget = ParamWidget::Color { argb: c };
                                }
                                g.mark_dirty(nid);
                            }
                            st_c.reevaluate_and_notify();
                        }),
                );
            }
            stack_panel()
                .spacing(4.0)
                .child(
                    border()
                        .height(16.0)
                        .corner_radius(5.0)
                        .background(move || solid_brush(col_sig.get())),
                )
                .child(row)
                .into()
        }

        NodeKind::ExpressionBox => {
            let init_txt = match init_widget {
                ParamWidget::Text { text } => text,
                _ => "sqrt(x / max(y, 0.5))".into(),
            };
            let txt_sig = signal(init_txt);
            let st_e = state.clone();
            effect(move || {
                let expr = txt_sig.get();
                let mut changed = false;
                {
                    let mut g = st_e.graph.borrow_mut();
                    if let Some(n) = g.nodes.get_mut(&nid) {
                        let same = matches!(&n.widget, ParamWidget::Text { text } if *text == expr);
                        if !same {
                            n.widget = ParamWidget::Text { text: expr };
                            changed = true;
                        }
                    }
                    if changed {
                        g.mark_dirty(nid);
                    }
                }
                if changed {
                    st_e.reevaluate_and_notify();
                }
            });
            stack_panel()
                .width(154.0)
                .spacing(3.0)
                .child(text_block().text("f(x, y, z, t) =").font_size(9.5).foreground("#7DD3FC"))
                .child(text_box().text(txt_sig.two_way()).font_size(11.0))
                .into()
        }

        NodeKind::PathMapper => {
            let (init_s, init_t) = match init_widget {
                ParamWidget::PathMapper { source_mask, target_mask } => (source_mask, target_mask),
                _ => ("{A;B}(i)".into(), "{B;A}".into()),
            };
            let src_sig = signal(init_s);
            let tgt_sig = signal(init_t);
            let st_pm = state.clone();
            effect(move || {
                let s = src_sig.get();
                let t = tgt_sig.get();
                let mut changed = false;
                {
                    let mut g = st_pm.graph.borrow_mut();
                    if let Some(n) = g.nodes.get_mut(&nid) {
                        let same = matches!(&n.widget, ParamWidget::PathMapper { source_mask, target_mask } if *source_mask == s && *target_mask == t);
                        if !same {
                            n.widget = ParamWidget::PathMapper { source_mask: s, target_mask: t };
                            changed = true;
                        }
                    }
                    if changed {
                        g.mark_dirty(nid);
                    }
                }
                if changed {
                    st_pm.reevaluate_and_notify();
                }
            });
            stack_panel()
                .width(146.0)
                .spacing(4.0)
                .child(text_box().text(src_sig.two_way()).font_size(10.5))
                .child(text_block().text("↓  remap to").font_size(9.0).foreground("#A78BFA").horizontal_alignment(HorizontalAlignment::Center))
                .child(text_box().text(tgt_sig.two_way()).font_size(10.5))
                .into()
        }

        NodeKind::Panel => {
            let st_pn = state.clone();
            border()
                .width(164.0)
                .min_height(64.0)
                .background("#11170E")
                .border_brush("#4D7C0F")
                .border_thickness(1.0)
                .corner_radius(5.0)
                .padding(6.0)
                .child(
                    text_block()
                        .font_size(10.0)
                        .foreground("#D9F99D")
                        .text(move || {
                            let _ = st_pn.eval_rev.get();
                            st_pn
                                .graph
                                .borrow()
                                .nodes
                                .get(&nid)
                                .and_then(|n| n.output_tree(0))
                                .map(|t| t.format_panel(6))
                                .unwrap_or_else(|| "<empty>".into())
                        }),
                )
                .into()
        }

        NodeKind::ParamViewer => {
            let st_pv = state.clone();
            border()
                .width(156.0)
                .min_height(62.0)
                .background("#0C1322")
                .border_brush("#38BDF8")
                .border_thickness(1.0)
                .corner_radius(5.0)
                .padding(6.0)
                .child(
                    text_block()
                        .font_size(10.0)
                        .foreground("#BAE6FD")
                        .text(move || {
                            let _ = st_pv.eval_rev.get();
                            // Inspect incoming tree on input port 0
                            let g = st_pv.graph.borrow();
                            let mut merged = crate::data_tree::DataTree::new();
                            for w in g.wires.values() {
                                if w.to_node == nid && w.to_port == 0 {
                                    if let Some(src) = g.nodes.get(&w.from_node).and_then(|n| n.output_tree(w.from_port)) {
                                        merged.merge_from(src);
                                    }
                                }
                            }
                            merged.format_param_viewer(6)
                        }),
                )
                .into()
        }

        NodeKind::ValueList => {
            let (options, init_sel) = match init_widget {
                ParamWidget::ValueList { options, selected } => (options, selected),
                _ => (vec![("Low".into(), 3.0), ("Med".into(), 6.0), ("High".into(), 10.0)], 1),
            };
            let sel_sig = signal(init_sel);
            let mut row = stack_panel().orientation(Orientation::Horizontal).spacing(4.0);
            for (idx, (label, _val)) in options.into_iter().enumerate() {
                let st_v = state.clone();
                row = row.child(
                    button()
                        .padding((8.0, 4.0, 8.0, 4.0))
                        .corner_radius(5.0)
                        .background(move || if sel_sig.get() == idx { "#0284C7" } else { "#1E293B" })
                        .foreground("#FFFFFF")
                        .font_size(10.5)
                        .content(label)
                        .on_click(move |_| {
                            sel_sig.set(idx);
                            {
                                let mut g = st_v.graph.borrow_mut();
                                g.push_undo_checkpoint();
                                if let Some(n) = g.nodes.get_mut(&nid) {
                                    if let ParamWidget::ValueList { ref mut selected, .. } = n.widget {
                                        *selected = idx;
                                    }
                                }
                                g.mark_dirty(nid);
                            }
                            st_v.reevaluate_and_notify();
                        }),
                );
            }
            row.into()
        }

        NodeKind::GraphMapper => {
            let (init_curve, init_p1, init_p2) = match init_widget {
                ParamWidget::GraphMapper { curve, p1, p2 } => (curve, p1, p2),
                _ => (crate::types::GraphCurveKind::Bezier, [0.25, 0.80], [0.75, 0.20]),
            };
            let curve_sig = signal(init_curve);
            let gm_ref = NodeRef::<GraphMapperControl>::new();
            let st_gm = state.clone();
            let st_btn = state.clone();

            stack_panel()
                .spacing(4.0)
                .child(
                    El::<GraphMapperControl>::new()
                        .width(142.0)
                        .height(92.0)
                        .node_ref(gm_ref)
                        .with(move |ctrl| {
                            let st_cb = st_gm.clone();
                            ctrl.configure(
                                init_curve,
                                init_p1,
                                init_p2,
                                Rc::new(move |c, p1, p2| {
                                    {
                                        let mut g = st_cb.graph.borrow_mut();
                                        if let Some(n) = g.nodes.get_mut(&nid) {
                                            n.widget = ParamWidget::GraphMapper { curve: c, p1, p2 };
                                        }
                                        g.mark_dirty(nid);
                                    }
                                    st_cb.reevaluate_and_notify();
                                }),
                            );
                        }),
                )
                .child(
                    button()
                        .horizontal_alignment(HorizontalAlignment::Stretch)
                        .padding((6.0, 2.0, 6.0, 2.0))
                        .corner_radius(4.0)
                        .background("#1E293B")
                        .foreground("#7DD3FC")
                        .font_size(10.0)
                        .content(move || format!("Curve: {} ↻", curve_sig.get().label()))
                        .on_click(move |_| {
                            let next = curve_sig.get_untracked().next();
                            curve_sig.set(next);
                            if let Some(ctrl) = gm_ref.get_untracked() {
                                ctrl.set_curve(next);
                            }
                            {
                                let mut g = st_btn.graph.borrow_mut();
                                if let Some(n) = g.nodes.get_mut(&nid) {
                                    if let ParamWidget::GraphMapper { ref mut curve, .. } = n.widget {
                                        *curve = next;
                                    }
                                }
                                g.mark_dirty(nid);
                            }
                            st_btn.reevaluate_and_notify();
                        }),
                )
                .into()
        }

        NodeKind::MdSlider => {
            let (init_x, init_y) = match init_widget {
                ParamWidget::MdSlider { x, y } => (x, y),
                _ => (0.5, 0.5),
            };
            let xy_sig = signal((init_x, init_y));
            let st_md = state.clone();

            stack_panel()
                .spacing(3.0)
                .child(
                    El::<MdSliderControl>::new()
                        .width(114.0)
                        .height(94.0)
                        .with(move |ctrl| {
                            let st_cb = st_md.clone();
                            ctrl.configure(
                                init_x,
                                init_y,
                                Rc::new(move |x, y| {
                                    xy_sig.set((x, y));
                                    {
                                        let mut g = st_cb.graph.borrow_mut();
                                        if let Some(n) = g.nodes.get_mut(&nid) {
                                            n.widget = ParamWidget::MdSlider { x, y };
                                        }
                                        g.mark_dirty(nid);
                                    }
                                    st_cb.reevaluate_and_notify();
                                }),
                            );
                        }),
                )
                .child(
                    text_block()
                        .horizontal_alignment(HorizontalAlignment::Center)
                        .font_size(9.5)
                        .foreground("#7DD3FC")
                        .text(move || {
                            let (x, y) = xy_sig.get();
                            format!("({x:.2}, {y:.2})")
                        }),
                )
                .into()
        }

        NodeKind::Scribble => {
            let init_txt = match init_widget {
                ParamWidget::Text { text } => text,
                _ => "Parametric Note".into(),
            };
            let sig = signal(init_txt);
            let st_sc = state.clone();
            effect(move || {
                let txt = sig.get();
                let mut g = st_sc.graph.borrow_mut();
                if let Some(n) = g.nodes.get_mut(&nid) {
                    n.widget = ParamWidget::Text { text: txt };
                }
            });
            text_box().width(196.0).font_size(14.0).text(sig.two_way()).into()
        }

        NodeKind::Relay => Element::Empty,

        NodeKind::Component => {
            // Iconic Rhino Grasshopper dark metallic central badge!
            border()
                .min_width(58.0)
                .min_height(32.0)
                .background("#0D1424")
                .border_brush(solid_brush(accent_argb))
                .border_thickness((0.0, 2.0, 0.0, 0.0))
                .corner_radius(5.0)
                .padding((8.0, 6.0, 8.0, 6.0))
                .vertical_alignment(VerticalAlignment::Center)
                .child(
                    text_block()
                        .text(title.to_string())
                        .font_size(11.0)
                        .foreground("#F1F5F9")
                        .horizontal_alignment(HorizontalAlignment::Center)
                        .vertical_alignment(VerticalAlignment::Center),
                )
                .into()
        }
    }
}
