//! **FerroUI Motion Studio** — purposeful control interactions, composition,
//! parametric workflows, and simulation in a shared WinUI 3 navigation shell.

mod catalog;
pub mod compositor_showcase;
pub mod icons;
pub mod live_ui_tree;
pub mod node_graph_studio;
pub mod scenes;
mod showcase_style;
pub mod viewport;
mod workspace;

use catalog::{build_home_page, StudioPage};

use crate::icons::{icon_label, icon_view, nav_icon, Icons};
use crate::scenes::{SandboxEngine, SceneKind};
use ferroui_base::layout::{HorizontalAlignment, Orientation, VerticalAlignment};
use ferroui_base::media::FontWeight;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{BoxedValue, Ref, Thickness};
use ferroui_controls::{
    Application, Control, NavigationView, NavigationViewBackButtonVisible, NavigationViewItem,
    NavigationViewItemHeader, NavigationViewPaneDisplayMode,
};
use ferroui_view::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// Builds the adaptive WinUI 3 studio shell and its interactive workspaces.
pub fn physics_sandbox() -> Element {
    let engine = Rc::new(RefCell::new(SandboxEngine::new()));
    let gpu_available = engine.borrow().gpu_available();

    let initial_page =
        StudioPage::from_key(&std::env::var("FERROUI_START_PAGE").unwrap_or_default());
    if let StudioPage::Scene(kind) = initial_page {
        engine.borrow_mut().load_scene(kind);
    }
    let active_page = signal(initial_page);
    let pane_open = signal(true);
    let is_dark_theme = signal(
        Application::current()
            .and_then(|a| a.actual_theme_variant())
            .is_none_or(|v| v == ThemeVariant::dark()),
    );

    // Track NavigationView + menu item handles so gallery Home cards sync sidebar selection!
    let nav_ref = NodeRef::<NavigationView>::new();
    let nav_entries: Rc<RefCell<Vec<(StudioPage, Ref<NavigationViewItem>)>>> =
        Rc::new(RefCell::new(Vec::new()));
    let syncing_nav = Rc::new(std::cell::Cell::new(false));

    let navigate_to: Rc<dyn Fn(StudioPage)> = {
        let eng = engine.clone();
        let nav_entries = nav_entries.clone();
        let syncing_nav = syncing_nav.clone();
        Rc::new(move |page: StudioPage| {
            if active_page.get_untracked() == page {
                return;
            }
            if let StudioPage::Scene(kind) = page {
                {
                    let mut sim = eng.borrow_mut();
                    sim.blueprint_driven = false;
                    sim.load_scene(kind);
                }
            }
            active_page.set_if_changed(page);

            if let Some(nav) = nav_ref.get_untracked() {
                if let Some((_, item)) = nav_entries.borrow().iter().find(|(p, _)| *p == page) {
                    syncing_nav.set(true);
                    nav.set_selected_item(Some(Control::boxed(item)));
                    syncing_nav.set(false);
                }
            }
        })
    };

    // -------------------------------------------------------------------------
    // Adaptive WinUI 3 navigation shell
    // -------------------------------------------------------------------------
    let nav_entries_setup = nav_entries.clone();
    let nav_entries_sel = nav_entries.clone();
    let syncing_sel = syncing_nav.clone();
    let nav_to_sel = navigate_to.clone();
    let nav_to_home = navigate_to.clone();

    let pane_header = border().padding((8.0, 6.0, 8.0, 6.0)).child(
        stack_panel()
            .orientation(Orientation::Horizontal)
            .spacing(10.0)
            .vertical_alignment(VerticalAlignment::Center)
            .child(
                border()
                    .width(28.0)
                    .height(28.0)
                    .corner_radius(7.0)
                    .dynamic_resource(
                        ferroui_controls::Border::background_property(),
                        "AccentFillColorDefaultBrush",
                    )
                    .child(
                        border()
                            .horizontal_alignment(HorizontalAlignment::Center)
                            .vertical_alignment(VerticalAlignment::Center)
                            .child(icon_view(Icons::LIGHTNING, 16.0)),
                    ),
            )
            .child(
                stack_panel()
                    .spacing(0.0)
                    .child(
                        text_block()
                            .text("Motion Studio")
                            .font_size(13.5)
                            .font_weight(FontWeight::SemiBold),
                    )
                    .child(
                        text_block()
                            .text("Physics · Interfaces · Workflows")
                            .font_size(10.0)
                            .opacity(0.68),
                    ),
            ),
    );

    let pane_footer = border()
        .is_visible(move || pane_open.get())
        .padding((8.0, 6.0, 8.0, 8.0))
        .child(
            button()
                .horizontal_alignment(HorizontalAlignment::Stretch)
                .padding((10.0, 6.0, 10.0, 6.0))
                .content(dynamic(move || {
                    if is_dark_theme.get() {
                        icon_label(Icons::THEME, "Switch to Light Theme", 14.0)
                    } else {
                        icon_label(Icons::THEME, "Switch to Dark Theme", 14.0)
                    }
                }))
                .on_click(move |_| {
                    let next_dark = !is_dark_theme.get_untracked();
                    is_dark_theme.set(next_dark);
                    if let Some(app) = Application::current() {
                        app.set_requested_theme_variant(Some(if next_dark {
                            ThemeVariant::dark()
                        } else {
                            ThemeVariant::light()
                        }));
                    }
                }),
        );

    let page_content = border()
        .dynamic_resource(
            ferroui_controls::Border::background_property(),
            "LayerFillColorDefaultBrush",
        )
        .clip_to_bounds(true)
        .child(dynamic(move || match active_page.get() {
            StudioPage::Home => build_home_page(gpu_available, nav_to_home.clone()),
            StudioPage::CompositorShaders => compositor_showcase::compositor_shader_showcase(),
            StudioPage::LiveUiTree => live_ui_tree::live_ui_tree_showcase(),
            StudioPage::NodeGraph => node_graph_studio::node_graph_physics_studio(),
            StudioPage::Scene(_) => workspace::scene_workspace(engine.clone()),
        }));

    navigation_view()
        .node_ref(nav_ref)
        .pane_display_mode(NavigationViewPaneDisplayMode::Auto)
        .compact_mode_threshold_width(680.0)
        .expanded_mode_threshold_width(980.0)
        .open_pane_length(248.0)
        .is_settings_visible(false)
        .is_back_button_visible(NavigationViewBackButtonVisible::Collapsed)
        .always_show_header(false)
        .pane_header(pane_header)
        .pane_footer(pane_footer)
        .content(page_content)
        .on_changed(NavigationView::is_pane_open_property(), move |open| {
            pane_open.set_if_changed(*open);
        })
        .with(move |nav: &Ref<NavigationView>| {
            pane_open.set_if_changed(nav.is_pane_open());
            // Leave the labels beyond the icon-only compact pane. Its stock
            // tooltip and automation name still use the item's full content.
            nav.resources().add(
                "NavigationViewCompactItemContentPresenterMargin",
                Some(Rc::new(Thickness::new(8.0, 0.0, 0.0, 0.0)) as BoxedValue),
            );
            let mut entries = nav_entries_setup.borrow_mut();
            let add_header = |title: &str| {
                let hdr = NavigationViewItemHeader::new();
                hdr.set_content(Some(Rc::new(title.to_string()) as BoxedValue));
                nav.menu_items().add(Some(Control::boxed(&hdr)));
            };
            let mut add_item = |page: StudioPage| -> Ref<NavigationViewItem> {
                let item = NavigationViewItem::new();
                item.set_content(Some(Rc::new(page.title().to_string()) as BoxedValue));
                if let Some(ic) = nav_icon(page.icon()) {
                    item.set_icon(Some(ic));
                }
                nav.menu_items().add(Some(Control::boxed(&item)));
                entries.push((page, item.clone()));
                item
            };

            let home_item = add_item(StudioPage::Home);

            add_header("Interface patterns");
            let _ = add_item(StudioPage::CompositorShaders);
            let _ = add_item(StudioPage::LiveUiTree);

            add_header("Visual programming");
            let _ = add_item(StudioPage::NodeGraph);

            add_header("Simulation lab");
            for kind in SceneKind::ALL {
                add_item(StudioPage::Scene(kind));
            }
            drop(entries);

            let initial_item = nav_entries_setup
                .borrow()
                .iter()
                .find(|(page, _)| *page == initial_page)
                .map(|(_, item)| item.clone())
                .unwrap_or(home_item);
            syncing_sel.set(true);
            nav.set_selected_item(Some(Control::boxed(&initial_item)));
            syncing_sel.set(false);

            let entries_cb = nav_entries_sel.clone();
            let syncing_cb = syncing_sel.clone();
            let nav_to = nav_to_sel.clone();
            nav.selection_changed(move |args| {
                if syncing_cb.get() {
                    return;
                }
                if let Some(container) = args
                    .selected_item_container()
                    .and_then(|c| c.cast::<NavigationViewItem>())
                {
                    let found = entries_cb
                        .borrow()
                        .iter()
                        .find(|(_, item)| *item == container)
                        .map(|(page, _)| *page);
                    if let Some(page) = found {
                        nav_to(page);
                    }
                }
            })
            .detach();
        })
        .into()
}

#[cfg(test)]
mod tests {
    use super::scenes::{SandboxEngine, SceneKind, ToolMode};
    use ferroui_physics::Vec2;

    #[test]
    fn all_eight_studio_scenes_simulate_cleanly_with_interactions() {
        let mut engine = SandboxEngine::new_cpu();
        for kind in SceneKind::ALL {
            engine.load_scene(kind);
            for frame in 0..45 {
                if frame == 10 {
                    engine.on_pointer_pressed(Vec2::new(6.0, 4.0), false);
                } else if frame == 18 {
                    engine.on_pointer_moved(Vec2::new(8.0, 3.5));
                } else if frame == 25 {
                    engine.on_pointer_released();
                } else if frame == 30 {
                    engine.tool = ToolMode::Explosion;
                    engine.on_pointer_pressed(Vec2::new(7.5, 5.0), true);
                    engine.on_pointer_released();
                    engine.tool = ToolMode::Drag;
                }
                engine.step(1.0 / 60.0);
            }

            for b in engine.world.bodies() {
                assert!(b.position.is_finite(), "{kind:?}: non-finite body position");
                assert!(
                    b.linear_velocity.is_finite(),
                    "{kind:?}: non-finite body velocity"
                );
            }
            for p in &engine.gpu_particles_view {
                assert!(
                    p.pos[0].is_finite() && p.pos[1].is_finite(),
                    "{kind:?}: non-finite particle"
                );
            }
            for sb in &engine.soft_bodies {
                assert!(sb.center().is_finite(), "{kind:?}: non-finite softbody");
            }
            if let Some(cloth) = engine.cloth.as_ref() {
                for n in &cloth.nodes {
                    assert!(n.pos.is_finite(), "{kind:?}: non-finite cloth node");
                }
            }
            assert!(
                !engine.telemetry.backend_label.is_empty(),
                "{kind:?}: missing telemetry"
            );
            assert_eq!(
                !engine.history.is_empty(),
                kind.supports_history(),
                "{kind:?}: only scenes with playback controls should record history"
            );
        }
    }

    #[test]
    fn laser_gravity_orbits_remain_stable_and_bounded() {
        let mut engine = SandboxEngine::new_cpu();
        engine.load_scene(SceneKind::LaserGravity);

        let star_center = Vec2::new(16.0 * 0.54, 10.0 * 0.5);
        let initial_radii: Vec<(ferroui_physics::BodyHandle, f32)> = engine
            .world
            .bodies()
            .filter(|b| b.is_dynamic())
            .map(|b| (b.handle, (b.position - star_center).length()))
            .collect();
        assert_eq!(initial_radii.len(), 5, "expected 5 orbital bodies");

        for _ in 0..300 {
            engine.step(1.0 / 60.0);
        }

        for (handle, r0) in initial_radii {
            let body = engine.world.body(handle).expect("orbital body must exist");
            let r_now = (body.position - star_center).length();
            assert!(
                (r_now - r0).abs() < 0.35,
                "planet drifted out of stable Keplerian orbit: r0={r0:.2}, r_now={r_now:.2}"
            );
            assert!(
                body.position.x > 0.5
                    && body.position.x < 15.5
                    && body.position.y > 0.5
                    && body.position.y < 9.5,
                "planet escaped arena bounds: {:?}",
                body.position
            );
        }
        assert!(
            !engine.laser_segments.is_empty() && engine.laser_segments.len() <= 6,
            "laser should trace clean bounded segments, got {}",
            engine.laser_segments.len()
        );
    }

    #[test]
    fn fixed_frame_updates_match_across_display_rates() {
        let mut slow = SandboxEngine::new_cpu();
        let mut fast = SandboxEngine::new_cpu();
        for _ in 0..120 {
            slow.advance_frame(1.0 / 30.0);
        }
        for _ in 0..480 {
            fast.advance_frame(1.0 / 120.0);
        }
        assert!((slow.sim_time - fast.sim_time).abs() < 1e-5);
        for (a, b) in slow.world.bodies().zip(fast.world.bodies()) {
            assert!((a.position - b.position).length() < 1e-4);
            assert!((a.linear_velocity - b.linear_velocity).length() < 1e-4);
        }
    }

    #[test]
    fn reset_resumes_and_clears_scene_specific_state() {
        let mut engine = SandboxEngine::new_cpu();
        engine.load_scene(SceneKind::LaserGravity);
        engine.step(1.0 / 60.0);
        engine.on_pointer_pressed(Vec2::new(8.0, 5.0), false);
        engine.paused = true;
        engine.blueprint_driven = true;
        engine.load_scene(SceneKind::Playground);
        assert!(!engine.paused);
        assert!(!engine.blueprint_driven);
        assert_eq!(engine.sim_time, 0.0);
        assert!(engine.orbit_trails.is_empty());
        assert!(engine.step_ms_history.is_empty());
        assert_eq!(engine.world.gravity, Vec2::new(0.0, engine.gravity_y));
        assert!(engine.advance_frame(1.0 / 60.0));
    }

    #[test]
    fn all_scene_startup_keys_are_unique_and_round_trip() {
        use super::catalog::StudioPage;
        let keys: std::collections::HashSet<_> =
            SceneKind::ALL.iter().map(|scene| scene.key()).collect();
        assert_eq!(keys.len(), SceneKind::ALL.len());
        for scene in SceneKind::ALL {
            assert_eq!(StudioPage::from_key(scene.key()), StudioPage::Scene(scene));
        }
        assert_eq!(StudioPage::from_key("missing"), StudioPage::Home);
    }

    #[test]
    fn spawned_jelly_is_simulated_in_rigid_scenes() {
        let mut engine = SandboxEngine::new_cpu();
        // Isolate free fall from the Playground's overlapping bridge and rigid obstacles.
        engine.world.clear();
        engine.tool = ToolMode::SpawnJelly;
        engine.on_pointer_pressed(Vec2::new(13.0, 2.0), false);
        engine.on_pointer_released();
        let initial = engine.soft_bodies[0].center();
        for _ in 0..20 {
            engine.step(1.0 / 60.0);
        }
        let final_center = engine.soft_bodies[0].center();
        assert!(
            final_center.y > initial.y + 0.05,
            "jelly should descend in a rigid scene: initial={initial:?}, final={final_center:?}"
        );
    }

    #[test]
    fn character_elevator_moves_once_per_step() {
        let mut engine = SandboxEngine::new_cpu();
        engine.load_scene(SceneKind::CharacterDemo);
        for _ in 0..60 {
            engine.step(1.0 / 60.0);
            let platform = engine
                .world
                .bodies()
                .find(|body| body.is_kinematic())
                .unwrap();
            let expected_y = 5.4 + (engine.sim_time * 1.4).sin() * 1.4;
            assert!((platform.position.y - expected_y).abs() < 1e-4);
        }
    }

    #[test]
    fn history_restore_cancels_pointer_joints() {
        use ferroui_physics::JointKind;
        let mut engine = SandboxEngine::new_cpu();
        let center = engine
            .world
            .bodies()
            .find(|body| body.is_dynamic())
            .unwrap()
            .position;
        engine.on_pointer_pressed(center, false);
        engine.step(1.0 / 60.0);
        engine.on_pointer_released();
        engine.step(1.0 / 60.0);
        engine.step_back_history();
        assert!(!engine
            .world
            .all_joints()
            .any(|joint| matches!(joint.kind, JointKind::Target { .. })));
    }

    #[test]
    fn solver_preset_matches_actual_config_and_survives_reset() {
        use super::scenes::SolverProfile;
        let mut engine = SandboxEngine::new_cpu();
        for scene in SceneKind::ALL {
            engine.load_scene(scene);
            if matches!(scene, SceneKind::GpuParticles | SceneKind::UiSprings) {
                continue;
            }
            assert_eq!(
                engine.world.config.substeps,
                engine.solver_profile.config().substeps
            );
            engine.set_solver_profile(SolverProfile::UiFast);
            engine.load_scene(scene);
            assert_eq!(
                engine.world.config.substeps,
                SolverProfile::UiFast.config().substeps
            );
        }
    }

    #[test]
    fn selected_solver_survives_back_forward_scrub_and_resume() {
        use super::scenes::SolverProfile;
        let mut engine = SandboxEngine::new_cpu();
        for _ in 0..3 {
            engine.step(1.0 / 60.0);
        }
        engine.set_solver_profile(SolverProfile::Precision);

        let assert_precision = |engine: &SandboxEngine| {
            let expected = SolverProfile::Precision.config();
            assert_eq!(engine.solver_profile, SolverProfile::Precision);
            assert_eq!(engine.world.config.substeps, expected.substeps);
            assert_eq!(
                engine.world.config.velocity_iterations,
                expected.velocity_iterations
            );
            assert_eq!(
                engine.world.config.position_iterations,
                expected.position_iterations
            );
        };
        engine.step_back_history();
        assert_precision(&engine);
        engine.step_forward_one();
        assert_precision(&engine);
        engine.scrub_history(0.0);
        assert_precision(&engine);

        engine.set_solver_profile(SolverProfile::UiFast);
        engine.step_forward_one();
        assert_eq!(engine.solver_profile, SolverProfile::UiFast);
        assert_eq!(
            engine.world.config.substeps,
            SolverProfile::UiFast.config().substeps
        );
        assert!(!engine.world.config.enable_ccd);
        engine.paused = false;
        engine.step(1.0 / 60.0);
        assert_eq!(
            engine.world.config.substeps,
            SolverProfile::UiFast.config().substeps
        );
        assert!(!engine.world.config.enable_ccd);
    }

    #[test]
    fn destruction_crate_fractures_again_after_history_restore() {
        use ferroui_physics::{BodyBuilder, Shape2D};
        let mut engine = SandboxEngine::new_cpu();
        engine.load_scene(SceneKind::RagdollWrecking);
        let crate_handle = engine
            .world
            .bodies()
            .find(|body| body.user_data == 0xFF38BDF8 && body.shape == Shape2D::rect(0.95, 0.95))
            .expect("Destruction includes a shatterable crate")
            .handle;
        // Keep an original scene crate (and its fracture eligibility), isolating one collision.
        let others: Vec<_> = engine
            .world
            .bodies()
            .filter(|body| body.handle != crate_handle)
            .map(|body| body.handle)
            .collect();
        for handle in others {
            engine.world.remove_body(handle);
        }
        engine.gravity_y = 0.0;
        engine.world.gravity = Vec2::ZERO;
        let body = engine.world.body_mut(crate_handle).unwrap();
        body.set_position(Vec2::new(8.0, 4.0));
        body.linear_velocity = Vec2::new(20.0, 0.0);
        engine
            .world
            .add_body(BodyBuilder::static_body(Shape2D::rect(0.2, 3.0)).position((9.0, 4.0)));
        engine.history.clear();
        engine.history.record(&engine.world);

        for replay in 0..2 {
            for _ in 0..12 {
                engine.step(1.0 / 60.0);
                if engine.world.body(crate_handle).is_none() {
                    break;
                }
            }
            assert!(
                engine.world.body(crate_handle).is_none(),
                "original crate must fracture on collision in replay {replay}"
            );
            if replay == 0 {
                engine.scrub_history(0.0);
                assert!(
                    engine.world.body(crate_handle).is_some(),
                    "rewind must restore the original crate"
                );
                engine.paused = false;
            }
        }
    }

    #[test]
    fn scene_changes_normalize_tools_and_keep_jelly_in_cloth_workspace() {
        for scene in SceneKind::ALL {
            assert_eq!(
                scene.supports_tool(ToolMode::SpawnJelly),
                scene == SceneKind::ClothSoftBody
            );
        }

        let mut engine = SandboxEngine::new_cpu();
        engine.tool = ToolMode::Shatter;
        engine.load_scene(SceneKind::ClothSoftBody);
        assert_eq!(engine.tool, ToolMode::Drag);
        engine.tool = ToolMode::SpawnJelly;
        engine.load_scene(SceneKind::ClothSoftBody);
        assert_eq!(
            engine.tool,
            ToolMode::SpawnJelly,
            "reset preserves a supported tool"
        );

        for scene in SceneKind::ALL {
            if scene == SceneKind::ClothSoftBody {
                continue;
            }
            engine.load_scene(SceneKind::ClothSoftBody);
            engine.tool = ToolMode::SpawnJelly;
            engine.load_scene(scene);
            assert_eq!(
                engine.tool,
                ToolMode::Drag,
                "unsupported jelly tool in {scene:?}"
            );
            assert!(scene.supports_tool(engine.tool));
        }

        engine.load_scene(SceneKind::Playground);
        engine.tool = ToolMode::SpawnBox;
        engine.load_scene(SceneKind::VehicleTerrain);
        assert_eq!(
            engine.tool,
            ToolMode::SpawnBox,
            "a supported rigid tool survives navigation"
        );
    }
}
