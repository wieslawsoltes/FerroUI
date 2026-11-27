//! **Rhino Grasshopper Real FerroUI Visual-Tree Node Capsules (`build_node_visual`)**.
//!
//! Styled cleanly with FerroUI's **stock current application theme** (no custom theme!):
//! - Standard components are compact **3-part horizontal capsules** (`[Inputs | Center Pill | Outputs]`)
//!   with protruding port nubs.
//! - Stock FerroUI controls (`Slider`, `Button`, `TextBox`, `TextBlock`, `Border`) inherit the
//!   host application's active theme automatically, adapting to Light or Dark `ThemeVariant`.

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

/// Builds a complete, authentic Rhino Grasshopper FerroUI Visual tree for node `nid`.
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

    // Left Input Port Nub Column
    let mut left_ports = stack_panel().spacing(4.0).vertical_alignment(VerticalAlignment::Center);
    for (pidx, spec) in inputs.iter().enumerate() {
        left_ports = left_ports.child(build_port_row(&state, nid, pidx, PortDirection::Input, spec));
    }

    // Right Output Port Nub Column
    let mut right_ports = stack_panel().spacing(4.0).vertical_alignment(VerticalAlignment::Center);
    for (pidx, spec) in outputs.iter().enumerate() {
        right_ports = right_ports.child(build_port_row(&state, nid, pidx, PortDirection::Output, spec));
    }

    // Center Real FerroUI Body
    let center_body = build_node_center(&state, nid, kind, &title, accent_argb, init_widget);

    // Authentic Grasshopper compact horizontal 3-part capsule layout!
    let inner_layout: Element = match kind {
        NodeKind::Relay => stack_panel()
            .orientation(Orientation::Horizontal)
            .spacing(6.0)
            .vertical_alignment(VerticalAlignment::Center)
            .child(left_ports)
            .child(text_block().text("●").font_size(9.0).foreground("#28B838"))
            .child(right_ports)
            .into(),
        NodeKind::Scribble => center_body,
        _ => {
            let mut row = dock_panel();
            if !inputs.is_empty() {
                row = row.child(left_ports.dock_panel_dock(Dock::Left).margin((-10.0, 0.0, 5.0, 0.0)));
            }
            if !outputs.is_empty() {
                row = row.child(right_ports.dock_panel_dock(Dock::Right).margin((5.0, 0.0, -10.0, 0.0)));
            }
            row.child(center_body).into()
        }
    };

    // Stock-theme adaptive capsule surfaces (follows host Light/Dark ThemeVariant with semantic states)
    let st_bg = state.clone();
    let capsule_bg = move || -> Option<Rc<dyn IBrush>> {
        let _ = st_bg.selection_rev.get();
        let _ = st_bg.eval_rev.get();
        let dark = st_bg.is_dark_theme.get();
        let g = st_bg.graph.borrow();
        let sel = g.selected_nodes.contains(&nid);
        let st = g.nodes.get(&nid).map(|n| n.state.clone()).unwrap_or_default();

        if kind == NodeKind::Scribble {
            return None;
        }
        let argb = if dark {
            if sel {
                0xFF1F3628
            } else {
                match st {
                    NodeState::Error(_) => 0xFF3A1D22,
                    NodeState::Warning(_) => 0xFF362A16,
                    NodeState::Disabled => 0xFF202020,
                    NodeState::Normal => 0xFF2B2B2B,
                }
            }
        } else if sel {
            0xFFD9F3DC
        } else {
            match st {
                NodeState::Error(_) => 0xFFFDE2E2,
                NodeState::Warning(_) => 0xFFFEF3C7,
                NodeState::Disabled => 0xFFE5E5E5,
                NodeState::Normal => 0xFFF6F6F6,
            }
        };
        solid_brush(argb)
    };

    let st_bd = state.clone();
    let capsule_border = move || -> Option<Rc<dyn IBrush>> {
        let _ = st_bd.selection_rev.get();
        let _ = st_bd.eval_rev.get();
        let dark = st_bd.is_dark_theme.get();
        let g = st_bd.graph.borrow();
        let sel = g.selected_nodes.contains(&nid);
        let st = g.nodes.get(&nid).map(|n| n.state.clone()).unwrap_or_default();
        let argb = if sel {
            0xFF22C55E
        } else {
            match st {
                NodeState::Error(_) => 0xFFDC2626,
                NodeState::Warning(_) => 0xFFD97706,
                _ => if dark { 0x33FFFFFF } else { 0x33000000 },
            }
        };
        solid_brush(argb)
    };

    let st_op = state.clone();
    let node_opacity = move || -> f64 {
        let _ = st_op.eval_rev.get();
        if st_op.graph.borrow().nodes.get(&nid).is_some_and(|n| n.disabled) { 0.45 } else { 1.0 }
    };

    // Compact Profiler Pill directly beneath the capsule
    let st_prof = state.clone();
    let profiler_pill = border()
        .horizontal_alignment(HorizontalAlignment::Center)
        .background(move || {
            if st_prof.is_dark_theme.get() {
                solid_brush(0xCC202020)
            } else {
                solid_brush(0xCCECECEC)
            }
        })
        .corner_radius(3.0)
        .padding((5.0, 1.0, 5.0, 1.0))
        .margin((0.0, 2.0, 0.0, 0.0))
        .is_visible({
            let st_pv = state.clone();
            move || {
                let _ = st_pv.eval_rev.get();
                st_pv.graph.borrow().show_profiler && !matches!(kind, NodeKind::Relay | NodeKind::Scribble)
            }
        })
        .child(
            text_block()
                .font_size(8.5)
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

    let st_m = state.clone();
    let st_p = state.clone();
    let st_mv = state.clone();
    let st_r = state.clone();
    let ds_p = drag_state.clone();
    let ds_m = drag_state.clone();
    let ds_r = drag_state.clone();

    let capsule = border()
        .background(capsule_bg)
        .border_brush(capsule_border)
        .border_thickness(if kind == NodeKind::Scribble { 0.0 } else { 1.5 })
        .corner_radius(if kind == NodeKind::Relay { 13.0 } else { 6.0 })
        .padding(match kind {
            NodeKind::Relay => (6.0, 3.0, 6.0, 3.0),
            NodeKind::NumberSlider => (8.0, 4.0, 8.0, 4.0),
            NodeKind::Panel => (8.0, 5.0, 8.0, 6.0),
            _ => (6.0, 4.0, 6.0, 4.0),
        })
        .opacity(node_opacity)
        .child(inner_layout);

    border()
        .canvas_left(init_pos[0])
        .canvas_top(init_pos[1])
        .node_ref(node_ref)
        .with(move |b: &Ref<Border>| {
            let vis: Ref<Visual> = b.clone().upcast();
            st_m.node_visuals.borrow_mut().insert(nid, vis.downgrade());
            st_m.invalidate_wires();
        })
        .on_pointer_pressed(move |e| {
            if e.handled() {
                return;
            }
            e.set_handled(true);
            st_p.quick_search.set(None);

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

/// Protruding Rhino Grasshopper half-round port nub socket + crisp port nickname + F/G/R/S badge.
fn build_port_row(
    state: &VisualGraphState,
    nid: NodeId,
    pidx: usize,
    dir: PortDirection,
    spec: &PortSpec,
) -> Element {
    let sock_ref = NodeRef::<Border>::new();
    let st_m = state.clone();
    let st_p = state.clone();
    let st_mv = state.clone();
    let st_r = state.clone();
    let st_en = state.clone();
    let st_ex = state.clone();
    let st_sk = state.clone();
    let port_color = spec.port_type.color_argb();

    // Port socket bead: semantic port color (or selection green) with stock-adaptive border ring
    let socket = border()
        .width(11.0)
        .height(11.0)
        .corner_radius(5.5)
        .background(move || {
            let _ = st_sk.selection_rev.get();
            if st_sk.graph.borrow().selected_nodes.contains(&nid) {
                solid_brush(0xFF22C55E)
            } else {
                solid_brush(port_color)
            }
        })
        .border_brush({
            let st_sb = state.clone();
            move || {
                if st_sb.is_dark_theme.get() {
                    solid_brush(0xFF181818)
                } else {
                    solid_brush(0xFF2C2C2C)
                }
            }
        })
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
            if st_mv.active_wire.borrow().is_none() {
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
                drag.multi_wire = drag.multi_wire || e.key_modifiers().contains(KeyModifiers::SHIFT);
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
    let st_l = state.clone();
    let label = text_block()
        .text(move || {
            let _ = st_l.eval_rev.get();
            if dir == PortDirection::Input {
                let badge = st_l
                    .graph
                    .borrow()
                    .nodes
                    .get(&nid)
                    .and_then(|n| n.input_modifiers.get(pidx))
                    .map(|m| {
                        let mut s = String::new();
                        if m.flatten { s.push('↓'); }
                        if m.graft { s.push('↑'); }
                        if m.reverse { s.push('⇄'); }
                        if m.simplify { s.push('∗'); }
                        s
                    })
                    .unwrap_or_default();
                if badge.is_empty() { nick.clone() } else { format!("{nick}{badge}") }
            } else {
                nick.clone()
            }
        })
        .font_size(10.0)
        .vertical_alignment(VerticalAlignment::Center);

    let mut row = stack_panel()
        .orientation(Orientation::Horizontal)
        .spacing(4.0)
        .vertical_alignment(VerticalAlignment::Center);
    match dir {
        PortDirection::Input => row = row.child(socket).child(label),
        PortDirection::Output => row = row.horizontal_alignment(HorizontalAlignment::Right).child(label).child(socket),
    }
    row.into()
}

/// Builds the center FerroUI control for every [`NodeKind`] using the stock current theme.
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
            let st_vb = state.clone();
            stack_panel()
                .orientation(Orientation::Horizontal)
                .spacing(7.0)
                .vertical_alignment(VerticalAlignment::Center)
                .child(
                    text_block()
                        .text(title.to_string())
                        .font_size(10.5)
                        .vertical_alignment(VerticalAlignment::Center),
                )
                .child(
                    slider()
                        .width(106.0)
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
                .child(
                    border()
                        .background(move || {
                            if st_vb.is_dark_theme.get() {
                                solid_brush(0x28000000)
                            } else {
                                solid_brush(0x14000000)
                            }
                        })
                        .border_brush(solid_brush(0x40888888))
                        .border_thickness(1.0)
                        .corner_radius(3.0)
                        .padding((5.0, 1.0, 5.0, 1.0))
                        .child(
                            text_block()
                                .font_size(10.5)
                                .text(move || {
                                    let v = val_sig.get();
                                    if integer { format!("{:.0}", v.round()) } else { format!("{v:.2}") }
                                }),
                        ),
                )
                .into()
        }

        NodeKind::BoolToggle => {
            let init_b = matches!(init_widget, ParamWidget::Toggle { value: true });
            let sig = signal(init_b);
            let st_t = state.clone();
            stack_panel()
                .orientation(Orientation::Horizontal)
                .spacing(6.0)
                .vertical_alignment(VerticalAlignment::Center)
                .child(text_block().text(title.to_string()).font_size(10.5).vertical_alignment(VerticalAlignment::Center))
                .child(
                    button()
                        .padding((8.0, 2.0, 8.0, 2.0))
                        .font_size(10.5)
                        .content(move || if sig.get() { "True" } else { "False" })
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
                        }),
                )
                .into()
        }

        NodeKind::MomentaryButton => {
            let st_b = state.clone();
            button()
                .padding((10.0, 4.0, 10.0, 4.0))
                .font_size(10.5)
                .content(title.to_string())
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
            let mut row = stack_panel().orientation(Orientation::Horizontal).spacing(3.0);
            for &c in &SWATCH_COLORS[..5] {
                let st_c = state.clone();
                row = row.child(
                    button()
                        .width(16.0)
                        .height(16.0)
                        .corner_radius(8.0)
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
                .child(border().height(16.0).corner_radius(4.0).background(move || solid_brush(col_sig.get())))
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
                .width(152.0)
                .spacing(3.0)
                .child(text_block().text(title.to_string()).font_size(10.0))
                .child(text_box().text(txt_sig.two_way()).font_size(10.5))
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
                .width(142.0)
                .spacing(3.0)
                .child(text_box().text(src_sig.two_way()).font_size(10.0))
                .child(text_block().text("↓  Path Map").font_size(9.0).horizontal_alignment(HorizontalAlignment::Center))
                .child(text_box().text(tgt_sig.two_way()).font_size(10.0))
                .into()
        }

        NodeKind::Panel => {
            let st_pn = state.clone();
            stack_panel()
                .width(172.0)
                .spacing(3.0)
                .child(text_block().text(title.to_string()).font_size(10.0))
                .child(
                    text_block()
                        .min_height(56.0)
                        .font_size(10.0)
                        .text(move || {
                            let _ = st_pn.eval_rev.get();
                            st_pn
                                .graph
                                .borrow()
                                .nodes
                                .get(&nid)
                                .and_then(|n| n.output_tree(0))
                                .map(|t| t.format_panel(6))
                                .unwrap_or_else(|| "Empty parameter".into())
                        }),
                )
                .into()
        }

        NodeKind::ParamViewer => {
            let st_pv = state.clone();
            stack_panel()
                .width(156.0)
                .spacing(3.0)
                .child(text_block().text("Param Viewer").font_size(10.0))
                .child(
                    text_block()
                        .min_height(52.0)
                        .font_size(9.5)
                        .text(move || {
                            let _ = st_pv.eval_rev.get();
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
            let mut row = stack_panel().orientation(Orientation::Horizontal).spacing(3.0);
            for (idx, (label, _val)) in options.into_iter().enumerate() {
                let st_v = state.clone();
                row = row.child(
                    button()
                        .padding((7.0, 3.0, 7.0, 3.0))
                        .border_brush(move || if sel_sig.get() == idx { solid_brush(0xFF22C55E) } else { None })
                        .border_thickness(move || if sel_sig.get() == idx { 1.5 } else { 1.0 })
                        .font_size(10.0)
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
                        .width(138.0)
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
                        .font_size(9.5)
                        .content(move || format!("{} ↻", curve_sig.get().label()))
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
            let md_ref = NodeRef::<MdSliderControl>::new();
            let st_md = state.clone();

            stack_panel()
                .spacing(3.0)
                .child(
                    El::<MdSliderControl>::new()
                        .width(108.0)
                        .height(92.0)
                        .node_ref(md_ref)
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
            text_box().width(190.0).font_size(14.0).text(sig.two_way()).into()
        }

        NodeKind::Relay => Element::Empty,

        NodeKind::Component => {
            let st_cc = state.clone();
            border()
                .min_width(54.0)
                .min_height(34.0)
                .background(move || {
                    let _ = st_cc.selection_rev.get();
                    let dark = st_cc.is_dark_theme.get();
                    if st_cc.graph.borrow().selected_nodes.contains(&nid) {
                        solid_brush(0x2822C55E)
                    } else if dark {
                        solid_brush(0x28000000)
                    } else {
                        solid_brush(0x14000000)
                    }
                })
                .border_brush(solid_brush(accent_argb))
                .border_thickness((0.0, 2.5, 0.0, 0.0))
                .corner_radius(4.0)
                .padding((8.0, 5.0, 8.0, 5.0))
                .vertical_alignment(VerticalAlignment::Stretch)
                .child(
                    text_block()
                        .text(title.to_string())
                        .font_size(10.5)
                        .horizontal_alignment(HorizontalAlignment::Center)
                        .vertical_alignment(VerticalAlignment::Center),
                )
                .into()
        }
    }
}
