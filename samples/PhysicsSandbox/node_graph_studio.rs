//! Parametric workspace: edit a graph and inspect its live physics result.
//!
//! Preview updates are coalesced while sliders move. Clean solutions retain the
//! same output payload and therefore do not reset a running simulation.

use crate::icons::{icon_label, Icons};
use crate::scenes::SandboxEngine;
use crate::showcase_style::{page_header, selection_button, subtle_text, surface};
use crate::viewport::PhysicsViewport;
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::FontWeight;
use ferroui_base::threading::DispatcherTimer;
use ferroui_controls::{Dock, GridResizeBehavior, GridResizeDirection};
use ferroui_nodegraph::prelude::*;
use ferroui_physics::Vec2;
use ferroui_view::prelude::*;
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

type ScenePayload = Arc<dyn Any + Send + Sync>;

#[derive(Default)]
struct PreviewQueue {
    pending: Option<ScenePayload>,
    applied: Option<ScenePayload>,
}

impl PreviewQueue {
    fn stage(&mut self, payload: ScenePayload) -> bool {
        if self
            .applied
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, &payload))
        {
            self.pending = None;
            false
        } else {
            self.pending = Some(payload);
            true
        }
    }

    fn apply(&mut self, engine: &mut SandboxEngine, paused: bool) -> bool {
        let Some(payload) = self.pending.take() else {
            return false;
        };
        let Some(blueprint) = payload.downcast_ref::<CompiledPhysicsBlueprint>() else {
            return false;
        };
        engine.load_from_blueprint(blueprint);
        engine.paused = paused;
        self.applied = Some(payload);
        true
    }
}

fn scene_payload(graph: &NodeGraph) -> Option<ScenePayload> {
    graph
        .nodes
        .values()
        .filter(|node| node.type_id == "physics.scene_output" && !node.disabled)
        .find_map(|node| match node.output_tree(0)?.first_item()? {
            Value::Custom(payload) if payload.data.is::<CompiledPhysicsBlueprint>() => {
                Some(payload.data.clone())
            }
            _ => None,
        })
}

fn template_name(template: PhysicsGraphTemplate) -> &'static str {
    match template {
        PhysicsGraphTemplate::AttractorGraphMapper => "Attractor field",
        PhysicsGraphTemplate::PyramidPendulum => "Pyramid & pendulum",
        PhysicsGraphTemplate::KeplerianSolarSystem => "Orbital system",
        PhysicsGraphTemplate::SuspensionBridgeRain => "Suspension bridge",
        PhysicsGraphTemplate::ClothJellyWindTunnel => "Wind tunnel",
    }
}

pub fn node_graph_physics_studio() -> Element {
    let registry = Rc::new(NodeRegistry::with_standard_library());
    let active_template = signal(PhysicsGraphTemplate::AttractorGraphMapper);
    let live = signal(Some(true));
    let paused = signal(false);
    let pending = signal(false);
    let summary = signal(String::from("Preparing preview…"));
    let status = signal(String::from(
        "Edit a parameter or connect two ports. The preview follows valid graph changes.",
    ));
    let editor_version = signal(0u64);
    let wide = signal(true);
    let compact_preview = signal(std::env::var("FERROUI_GRAPH_VIEW").as_deref() == Ok("preview"));
    let graph = Rc::new(RefCell::new(
        active_template.get_untracked().build(&registry),
    ));
    // Parametric templates use rigid bodies, cloth and soft bodies; they do
    // not need a second GPU particle device for the preview.
    let engine = Rc::new(RefCell::new(SandboxEngine::new_cpu()));
    let queue = Rc::new(RefCell::new(PreviewQueue::default()));

    let on_evaluated: Rc<dyn Fn(&NodeGraph)> = {
        let queue = queue.clone();
        Rc::new(move |graph| {
            let errors = graph
                .nodes
                .values()
                .filter(|node| matches!(node.state, NodeState::Error(_)))
                .count();
            if errors != 0 {
                queue.borrow_mut().pending = None;
                pending.set(false);
                status.set(format!(
                    "{errors} graph error(s). The preview keeps the last valid scene."
                ));
                return;
            }
            let Some(payload) = scene_payload(graph) else {
                queue.borrow_mut().pending = None;
                pending.set(false);
                status.set("Connect a Scene Output node to generate a physics preview.".into());
                return;
            };
            if let Some(blueprint) = payload.downcast_ref::<CompiledPhysicsBlueprint>() {
                summary.set_if_changed(format!(
                    "{} bodies · {} joints · {} fields · {} soft bodies · {} cloths",
                    blueprint.bodies.len(),
                    blueprint.joints.len(),
                    blueprint.force_fields.len(),
                    blueprint.soft_bodies.len(),
                    blueprint.cloths.len()
                ));
            }
            let changed = queue.borrow_mut().stage(payload);
            pending.set(changed);
            if changed {
                status.set(if live.get_untracked() == Some(true) {
                    "Updating the preview…".into()
                } else {
                    "Changes are ready. Choose Apply changes to rebuild the preview.".into()
                });
            } else {
                status.set("Preview is up to date.".into());
            }
        })
    };
    {
        let mut graph = graph.borrow_mut();
        evaluate_graph(&mut graph, &registry, 0.0);
        on_evaluated(&graph);
        queue
            .borrow_mut()
            .apply(&mut engine.borrow_mut(), paused.get_untracked());
        pending.set(false);
        status.set(
            "Edit a parameter or connect two ports. The preview follows valid graph changes."
                .into(),
        );
    }

    // At most one world rebuild per 50 ms. The latest evaluated output wins,
    // and turning Live preview off retains the pending output for Apply.
    let timer = DispatcherTimer::new();
    timer.set_interval(Duration::from_millis(50));
    {
        let queue = queue.clone();
        let engine = engine.clone();
        timer.tick(move |_| {
            if live.get_untracked() != Some(true) || !pending.get_untracked() {
                return;
            }
            if queue
                .borrow_mut()
                .apply(&mut engine.borrow_mut(), paused.get_untracked())
            {
                pending.set(false);
                status.set("Preview is up to date.".into());
            }
        });
    }
    let start = timer.clone();
    let stop = timer.clone();
    on_cleanup(move || timer.stop());

    let mut templates = wrap_panel().item_spacing(8.0).line_spacing(8.0);
    for template in PhysicsGraphTemplate::ALL {
        let graph = graph.clone();
        let registry = registry.clone();
        let engine = engine.clone();
        let queue = queue.clone();
        let evaluated = on_evaluated.clone();
        templates = templates.child(
            selection_button(active_template, template, move || {
                let mut next = template.build(&registry);
                evaluate_graph(&mut next, &registry, 0.0);
                evaluated(&next);
                queue
                    .borrow_mut()
                    .apply(&mut engine.borrow_mut(), paused.get_untracked());
                pending.set(false);
                status.set(format!(
                    "{} loaded. Edit its parameters to explore the result.",
                    template_name(template)
                ));
                *graph.borrow_mut() = next;
                editor_version.update(|version| *version = version.wrapping_add(1));
            })
            .content(template_name(template)),
        );
    }
    let apply_queue = queue.clone();
    let apply_engine = engine.clone();
    let reset_queue = queue.clone();
    let reset_engine = engine.clone();
    let pause_engine = engine.clone();
    let blast_engine = engine.clone();
    let controls = wrap_panel()
        .item_spacing(12.0)
        .line_spacing(8.0)
        .child(
            check_box()
                .content("Live preview")
                .is_checked(live.two_way())
                .vertical_alignment(VerticalAlignment::Center),
        )
        .child(
            button()
                .content("Apply changes")
                .is_enabled(move || pending.get())
                .on_click(move |_| {
                    if apply_queue
                        .borrow_mut()
                        .apply(&mut apply_engine.borrow_mut(), paused.get_untracked())
                    {
                        pending.set(false);
                        status.set("Changes applied to the preview.".into());
                    }
                }),
        )
        .child(
            button()
                .content(icon_label(Icons::RESET, "Restart scene", 16.0))
                .on_click(move |_| {
                    let mut queue = reset_queue.borrow_mut();
                    if let Some(payload) = queue.applied.clone() {
                        let pending_payload = queue.pending.take();
                        queue.pending = Some(payload);
                        queue.apply(&mut reset_engine.borrow_mut(), paused.get_untracked());
                        queue.pending = pending_payload;
                        status.set("Preview restarted with its last applied parameters.".into());
                    }
                }),
        )
        .child(
            button()
                .content(dynamic(move || {
                    if paused.get() {
                        icon_label(Icons::PLAY, "Resume", 16.0)
                    } else {
                        icon_label(Icons::PAUSE, "Pause", 16.0)
                    }
                }))
                .on_click(move |_| {
                    let next = !paused.get_untracked();
                    pause_engine.borrow_mut().paused = next;
                    paused.set(next);
                }),
        )
        .child(
            button()
                .content(icon_label(Icons::TARGET, "Impulse", 16.0))
                .on_click(move |_| {
                    blast_engine
                        .borrow_mut()
                        .world
                        .apply_explosion(Vec2::new(8.0, 5.5), 6.5, 16.0);
                }),
        )
        .child(
            selection_button(compact_preview, false, || {})
                .is_visible(move || !wide.get())
                .content(icon_label(Icons::NODE_GRAPH, "Edit graph", 16.0)),
        )
        .child(
            selection_button(compact_preview, true, || {})
                .is_visible(move || !wide.get())
                .content(icon_label(Icons::CUBE, "View simulation", 16.0)),
        );

    let preview_engine = engine.clone();
    let preview = surface()
        .grid_column(move || if wide.get() { 2 } else { 0 })
        .grid_row(0)
        .is_visible(move || wide.get() || compact_preview.get())
        .clip_to_bounds(true)
        .child(
            dock_panel()
                .child(
                    stack_panel()
                        .dock_panel_dock(Dock::Top)
                        .spacing(4.0)
                        .margin(16.0)
                        .child(
                            text_block()
                                .text("Live preview")
                                .font_size(16.0)
                                .font_weight(FontWeight::SemiBold),
                        )
                        .child(subtle_text().text(move || summary.get()).font_size(12.0)),
                )
                .child(El::<PhysicsViewport>::new().with(move |viewport| {
                    viewport.bind_engine(preview_engine.clone(), Rc::new(|_| {}));
                })),
        );
    let editor_graph = graph.clone();
    let editor_registry = registry.clone();
    let editor_callback = on_evaluated.clone();
    let editor = surface()
        .grid_column(0)
        .grid_row(0)
        .is_visible(move || wide.get() || !compact_preview.get())
        .clip_to_bounds(true)
        .child(dynamic(move || {
            let _ = editor_version.get();
            node_graph_editor(
                editor_graph.clone(),
                editor_registry.clone(),
                editor_callback.clone(),
            )
        }));
    let split = grid()
        .set(GridDefinitionsText::columns_property(), move || {
            if wide.get() {
                "3*,8,2*"
            } else {
                "*"
            }
        })
        .set(GridDefinitionsText::rows_property(), "*")
        .child(editor)
        .child(
            grid_splitter()
                .grid_column(move || if wide.get() { 1 } else { 0 })
                .grid_row(0)
                .is_visible(move || wide.get())
                .horizontal_alignment(HorizontalAlignment::Stretch)
                .vertical_alignment(VerticalAlignment::Stretch)
                .resize_direction(GridResizeDirection::Columns)
                .resize_behavior(GridResizeBehavior::PreviousAndNext),
        )
        .child(preview);

    dock_panel().margin(24.0).clip_to_bounds(true)
        .on_loaded(move |_| start.start()).on_unloaded(move |_| stop.stop())
        .on_size_changed(move |e| wide.set_if_changed(e.new_size().width >= 980.0))
        .child(stack_panel().dock_panel_dock(Dock::Top).spacing(12.0).margin((0.0, 0.0, 0.0, 16.0))
            .child(page_header("Parametric workspace", "Build a scene from connected components. Start with a template, edit its inputs, and inspect the live simulation."))
            .child(templates).child(controls))
        .child(subtle_text().dock_panel_dock(Dock::Bottom).margin((0.0, 12.0, 0.0, 0.0)).text(move || status.get()))
        .child(split)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unchanged_output_does_not_restart_the_preview() {
        let payload: ScenePayload = Arc::new(CompiledPhysicsBlueprint::default());
        let mut queue = PreviewQueue {
            applied: Some(payload.clone()),
            pending: None,
        };
        assert!(!queue.stage(payload));
        assert!(queue.pending.is_none());
        assert!(queue.stage(Arc::new(CompiledPhysicsBlueprint::default())));
        let mut engine = SandboxEngine::new_cpu();
        assert!(queue.apply(&mut engine, false));
        for _ in 0..3 {
            engine.step(1.0 / 60.0);
        }
        assert!(
            engine.history.is_empty(),
            "graph previews must not allocate hidden playback snapshots"
        );
    }
}
