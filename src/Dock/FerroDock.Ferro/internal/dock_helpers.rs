// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Original hit testing and model helpers; concrete host/external-surface members follow with DockControl.
use ferrodock_model::{
    controls::IProportionalDock,
    core::{DockFloatingWindowHostMode, DockPoint, IDock, IDockable},
};
use ferrodock_settings::{DockLogger, DockProperties, DockSettings};
use ferroui_base::{input::InputElement, Point, Ref, StyledProperty, Visual};
use ferroui_controls::{Control, TopLevel};
use std::{
    any::Any,
    panic::{catch_unwind, AssertUnwindSafe},
    rc::Rc,
};
macro_rules! log_drop_search {
    ($message:expr $(,)?) => {{
        #[cfg(debug_assertions)]
        DockHelpers::log_drop_search($message);
    }};
}
pub(crate) struct DockHelpers;
impl DockHelpers {
    pub(crate) fn resolve_floating_window_host_mode_for_dockable(
        dockable: Option<Rc<dyn IDockable>>,
    ) -> DockFloatingWindowHostMode {
        if let Some(root) = dockable.clone().and_then(|d| d.as_root_dock()) {
            return DockSettings::resolve_floating_window_host_mode(Some(&*root));
        }
        if let Some(root) = dockable
            .as_ref()
            .and_then(|d| d.owner())
            .and_then(|o| o.as_root_dock())
        {
            return DockSettings::resolve_floating_window_host_mode(Some(&*root));
        }
        if let Some(factory) = dockable
            .as_ref()
            .and_then(|d| d.owner())
            .and_then(|o| o.factory())
        {
            let root = factory.find_root(
                Some(dockable.expect("source nonnull dockable with owner")),
                Some(Rc::new(|_| true)),
            );
            return DockSettings::resolve_floating_window_host_mode(root.as_deref());
        }
        DockSettings::resolve_floating_window_host_mode(None)
    }
    pub(crate) fn is_managed_window_hosting_enabled_for_dockable(
        dockable: Option<Rc<dyn IDockable>>,
    ) -> bool {
        Self::resolve_floating_window_host_mode_for_dockable(dockable)
            == DockFloatingWindowHostMode::Managed
    }
    pub(crate) fn get_screen_point(visual: &Visual, point: Point) -> Point {
        let top = TopLevel::get_top_level(Some(visual));
        let scaling = top
            .as_ref()
            .and_then(|t| t.screens())
            .and_then(|s| s.screen_from_visual(visual))
            .map(|s| s.scaling())
            .or_else(|| top.as_ref().map(|t| t.render_scaling()))
            .unwrap_or(1.0);
        visual.point_to_screen(point).to_point(scaling)
    }
    fn is_hit_test_visible(visual: &Visual) -> bool {
        let Some(input) = visual.to_ref().cast::<InputElement>() else {
            return false;
        };
        visual.is_visible()
            && input.is_hit_test_visible()
            && input.is_effectively_enabled()
            && visual.visual_root().is_some()
    }
    fn log_drop_search(message: &str) {
        #[cfg(debug_assertions)]
        DockLogger::log_debug("DropSearch", message);
        #[cfg(not(debug_assertions))]
        let _ = message;
    }
    pub(crate) fn log_drop_areas(root: Option<&Visual>, context: &str) {
        let _ = context;
        if !DockSettings::enable_diagnostics_logging() {
            return;
        }
        let Some(root) = root else {
            log_drop_search!(&format!("[{context}] Root visual is null."));
            return;
        };
        let mut areas = Vec::new();
        for visual in root.get_visual_descendants() {
            if let Some(control) = visual.cast::<Control>() {
                if control.is_set(DockProperties::is_drop_area_property()) {
                    areas.push(control);
                }
            }
        }
        log_drop_search!(&format!(
            "[{context}] Found {} drop area controls under {}.",
            areas.len(),
            root.get_type().name()
        ));
        for control in areas {
            let _value = control.get_value(DockProperties::is_drop_area_property());
            log_drop_search!(&format!("  DropArea {}: Value={}, Bounds={}, IsVisible={}, IsHitTestVisible={}, IsEffectivelyEnabled={}",control.get_type().name(),boolean(_value),control.bounds(),boolean(control.is_visible()),boolean(control.is_hit_test_visible()),boolean(control.is_effectively_enabled())));
        }
    }
    pub(crate) fn get_control(
        input: Option<&Visual>,
        point: Point,
        property: &'static StyledProperty<bool>,
    ) -> Option<Ref<Control>> {
        let input = input.filter(|v| v.is_effectively_visible())?;
        let mut raw = None;
        let mut visible = None;
        if let Err(error) = catch_unwind(AssertUnwindSafe(|| {
            raw = Some(input.get_visuals_at(point));
            visible = Some(input.get_visuals_at_filtered(point, &Self::is_hit_test_visible));
        })) {
            Self::print_panic(&*error);
        }
        #[cfg(debug_assertions)]
        if let Some(raw) = &raw {
            log_drop_search!(&format!(
                "Raw hits ({}) at point {} on {}: {}",
                raw.len(),
                point,
                input.get_type().name(),
                raw.iter()
                    .map(|v| v.get_type().name())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        let mut controls = visible.map(|vs| {
            vs.into_iter()
                .filter_map(|v| v.cast::<Control>())
                .collect::<Vec<_>>()
        });
        let root = input.visual_root();
        if controls.as_ref().is_none_or(Vec::is_empty)
            && raw.as_ref().is_some_and(|r| !r.is_empty())
        {
            log_drop_search!(
                "Hit-test-visible search returned none; falling back to raw visual hits.",
            );
            controls = raw.map(|vs| vs.into_iter().filter_map(|v| v.cast::<Control>()).collect());
        }
        let Some(controls) = controls.filter(|c| !c.is_empty()) else {
            log_drop_search!(&format!(
                "No visuals hit for point {} on {}.",
                point,
                input.get_type().name()
            ));
            return None;
        };
        log_drop_search!(&format!(
            "Hit {} visuals at point {} on {}.",
            controls.len(),
            point,
            input.get_type().name()
        ));
        for control in controls {
            if !control.is_effectively_visible() {
                continue;
            }
            if root.is_some() && control.visual_root() != root {
                log_drop_search!(&format!(
                    "  Candidate {}: skipped due to different visual root ({}).",
                    control.get_type().name(),
                    control
                        .visual_root()
                        .map_or("null", |v| v.get_type().name())
                ));
                continue;
            }
            let is_set = control.is_set(property);
            let value = is_set && control.get_value(property);
            log_drop_search!(&format!(
                "  Candidate {}: IsSet={}, Value={}",
                control.get_type().name(),
                boolean(is_set),
                boolean(value)
            ));
            if value {
                log_drop_search!(&format!(
                    "Selected drop control {}.",
                    control.get_type().name()
                ));
                return Some(control);
            }
        }
        log_drop_search!("No control satisfied drop area requirement.");
        None
    }
    fn print_panic(error: &(dyn Any + Send)) {
        // Native hit testers unwind with a payload rather than CLR exception graphs.
        // Message transport is retained; stack/inner-exception transport remains open.
        let message = error
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| error.downcast_ref::<&str>().copied())
            .unwrap_or("Native hit-test failure");
        DockLogger::log("Exception", message);
        DockLogger::log("Exception", "");
    }
    pub(crate) fn to_dock_point(point: Point) -> DockPoint {
        DockPoint::new(point.x, point.y)
    }
    pub(crate) fn show_windows(dockable: Rc<dyn IDockable>) {
        if let Some(dock) = dockable.owner().and_then(|o| o.as_dock()) {
            if let Some(factory) = dock.factory() {
                if let Some(active) = factory
                    .find_root(Some(dock), Some(Rc::new(|_| true)))
                    .and_then(|r| r.active_dockable())
                    .and_then(|d| d.as_root_dock())
                {
                    if active.show_windows().can_execute(None) {
                        active.show_windows().execute(None);
                    }
                }
            }
        }
    }
    pub(crate) fn find_proportional_dock(dock: Rc<dyn IDock>) -> Option<Rc<dyn IProportionalDock>> {
        let factory = dock.factory()?;
        factory
            .find_dockable(
                Some(dock),
                Some(Rc::new(|d| {
                    d.and_then(|item| item.as_proportional_dock()).is_some()
                })),
            )?
            .as_proportional_dock()
    }
}
fn boolean(value: bool) -> &'static str {
    if value {
        "True"
    } else {
        "False"
    }
}
#[cfg(test)]
#[path = "dock_helpers_tests.rs"]
mod tests;

mod concrete_hosts {
    // Copyright (c) Wiesław Šoltés. All rights reserved.
    // Licensed under the MIT license. See LICENSE.TXT in the workspace root.
    use super::DockHelpers;
    use crate::{
        contract::IExternalDockSurface,
        controls::{DockControl, MdiDocumentWindow},
        model_projection,
    };
    use ferrodock_model::{
        collections::ListHandle,
        core::{DockFloatingWindowHostMode, IDockControl},
    };
    use ferrodock_settings::{DockProperties, DockSettings};
    use ferroui_base::{
        data::core::{ValueType, ValueTypes},
        BoxedValue, Point, Rect, Ref, StyledProperty, Visual,
    };
    use ferroui_controls::{Control, TopLevel, Window};
    use std::{any::TypeId, rc::Rc};
    impl DockHelpers {
        pub(crate) fn resolve_floating_window_host_mode(
            visual: Option<&Visual>,
        ) -> DockFloatingWindowHostMode {
            let Some(visual) = visual else {
                return DockSettings::resolve_floating_window_host_mode(None);
            };
            let dock = visual
                .to_ref()
                .cast::<DockControl>()
                .or_else(|| visual.find_ancestor_of_type::<DockControl>(false));
            let root = dock.and_then(|d| d.layout()).and_then(|l| l.as_root_dock());
            DockSettings::resolve_floating_window_host_mode(root.as_deref())
        }
        pub(crate) fn is_managed_window_hosting_enabled(visual: Option<&Visual>) -> bool {
            Self::resolve_floating_window_host_mode(visual) == DockFloatingWindowHostMode::Managed
        }
        pub(crate) fn resolve_dock_control(control: &Control) -> Option<Ref<DockControl>> {
            // Original source first checks ancestors, excluding the supplied control itself.
            if let Some(dock) = control.find_ancestor_of_type::<DockControl>(false) {
                return Some(dock);
            }
            let mut current = Some(control.to_ref().upcast::<Visual>());
            while let Some(visual) = current {
                let boxed: BoxedValue = Rc::new(visual.clone());
                if let Some(surface) =
                    ValueTypes::try_cast(&boxed, ValueType::of::<Rc<dyn IExternalDockSurface>>())
                        .and_then(|v| v.downcast_ref::<Rc<dyn IExternalDockSurface>>().cloned())
                {
                    if let Some(owner) = surface.dock_control() {
                        return Some(owner);
                    }
                }
                current = visual.visual_parent();
            }
            None
        }
        pub(crate) fn get_control_including_external(
            dock: &DockControl,
            point: Point,
            property: &'static StyledProperty<bool>,
        ) -> Option<Ref<Control>> {
            let local = Self::get_control(Some(dock), point, property);
            if local.is_some() {
                return local;
            }
            Self::get_external_control(dock, point, property)
        }
        pub(crate) fn get_external_control(
            dock: &DockControl,
            point: Point,
            property: &'static StyledProperty<bool>,
        ) -> Option<Ref<Control>> {
            if !dock.is_effectively_visible() {
                return None;
            }
            let root = dock.visual_root()?;
            let screen = dock.point_to_screen(point);
            for external in dock.enumerate_external_dock_surface_controls() {
                if !external.is_effectively_visible() {
                    continue;
                }
                let Some(external_root) = external.visual_root() else {
                    continue;
                };
                if !external_root.ptr_eq(&root) {
                    continue;
                }
                if !Self::resolve_dock_control(&external).is_some_and(|d| d.ptr_eq(&dock.to_ref()))
                {
                    continue;
                }
                let external_point = external.point_to_client(screen);
                let mut hit = Self::get_control(Some(&external), external_point, property);
                if hit.is_none()
                    && Rect::from_size(external.bounds().size()).contains(external_point)
                    && external.get_value(property)
                {
                    hit = Some(external.clone());
                }
                let Some(mut hit) = hit else {
                    continue;
                };
                if property.as_property() == DockProperties::is_drop_area_property().as_property()
                    && !hit.get_value(DockProperties::is_dock_target_property())
                {
                    let mut target = Self::get_control(
                        Some(&external),
                        external_point,
                        DockProperties::is_dock_target_property(),
                    );
                    if target.is_none()
                        && Rect::from_size(external.bounds().size()).contains(external_point)
                        && external.get_value(DockProperties::is_dock_target_property())
                    {
                        target = Some(external.clone());
                    }
                    if let Some(target) =
                        target.filter(|t| t.get_value(DockProperties::is_drop_area_property()))
                    {
                        hit = target;
                    }
                }
                if Self::resolve_dock_control(&hit).is_some_and(|d| d.ptr_eq(&dock.to_ref())) {
                    return Some(hit);
                }
            }
            None
        }
        fn index_of(windows: &[Ref<Window>], window: Option<Ref<Window>>) -> i32 {
            window
                .and_then(|w| windows.iter().position(|v| v.ptr_eq(&w)))
                .map_or(-1, |i| i as i32)
        }
        fn get_managed_window_z_order(dock: &DockControl) -> i32 {
            dock.find_ancestor_of_type::<MdiDocumentWindow>(false)
                .and_then(|w| model_projection::mdi_document(w.data_context().as_ref()))
                .map_or(i32::MIN, |d| d.mdi_z_index())
        }
        pub(crate) fn get_z_ordered_dock_controls(
            controls: ListHandle<Rc<dyn IDockControl>>,
        ) -> ZOrderedDockControls {
            let mut windows = Vec::new();
            for control in controls.iter().filter_map(native_dock_control) {
                if let Some(window) =
                    TopLevel::get_top_level(Some(&control)).and_then(|t| t.cast::<Window>())
                {
                    if !windows.contains(&window) {
                        windows.push(window);
                    }
                }
            }
            Window::sort_windows_by_z_order(&mut windows);
            ZOrderedDockControls {
                controls,
                windows: Rc::new(windows),
            }
        }
    }
    fn native_dock_control(control: Rc<dyn IDockControl>) -> Option<Ref<DockControl>> {
        control
            .boxed_interface(TypeId::of::<Ref<DockControl>>())?
            .downcast_ref::<Ref<DockControl>>()
            .cloned()
    }
    // The source snapshots/sorts native windows at method call, but keeps the dock query
    // repeatable and deferred. Each enumeration reads the original live collection again.
    #[derive(Clone)]
    pub(crate) struct ZOrderedDockControls {
        controls: ListHandle<Rc<dyn IDockControl>>,
        windows: Rc<Vec<Ref<Window>>>,
    }
    type ZEntry = (Ref<DockControl>, i32, i32);
    type ZIterator = std::iter::Map<std::vec::IntoIter<ZEntry>, fn(ZEntry) -> Ref<DockControl>>;
    fn select_dock(entry: ZEntry) -> Ref<DockControl> {
        entry.0
    }
    impl ZOrderedDockControls {
        pub(crate) fn iter(&self) -> ZIterator {
            let mut entries = Vec::new();
            for dock in self.controls.iter().filter_map(native_dock_control) {
                let order = DockHelpers::index_of(
                    &self.windows,
                    TopLevel::get_top_level(Some(&dock)).and_then(|t| t.cast::<Window>()),
                );
                let managed = DockHelpers::get_managed_window_z_order(&dock);
                entries.push((dock, order, managed));
            }
            entries.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| b.2.cmp(&a.2)));
            entries
                .into_iter()
                .map(select_dock as fn(ZEntry) -> Ref<DockControl>)
        }
    }
    impl IntoIterator for ZOrderedDockControls {
        type Item = Ref<DockControl>;
        type IntoIter = ZIterator;
        fn into_iter(self) -> Self::IntoIter {
            self.iter()
        }
    }
    impl IntoIterator for &ZOrderedDockControls {
        type Item = Ref<DockControl>;
        type IntoIter = ZIterator;
        fn into_iter(self) -> Self::IntoIter {
            self.iter()
        }
    }
}
