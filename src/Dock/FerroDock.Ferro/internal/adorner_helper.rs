// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::controls::{
    DockAdornerWindow, DockTarget, DockTargetBase, GlobalDockTarget, ManagedWindowLayer,
};
use crate::internal::dock_helpers::DockHelpers;
use ferroui_base::{
    data::core::ValueTypes, reactive::IDisposable, ObjectType, Point, Rect, Ref, Size,
    StyledElement, Thickness, Upcast, Visual,
};
use ferroui_controls::{primitives::AdornerLayer, Control, SizeToContent, TopLevel, Window};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
// The source's Control + IDockTarget + new() constraints, expressed on native class data.
pub(crate) trait AdornerTarget: ObjectType + Upcast<Control> {
    fn create() -> Ref<Self>;
    fn reset_target(&self);
}
impl AdornerTarget for DockTarget {
    fn create() -> Ref<Self> {
        Self::new()
    }
    fn reset_target(&self) {
        self.to_ref().upcast::<DockTargetBase>().reset();
    }
}
impl AdornerTarget for GlobalDockTarget {
    fn create() -> Ref<Self> {
        Self::new()
    }
    fn reset_target(&self) {
        self.to_ref().upcast::<DockTargetBase>().reset();
    }
}
pub(crate) struct AdornerHelper<T: AdornerTarget> {
    cached: Ref<T>,
    pub(crate) adorner: RefCell<Option<Ref<Control>>>,
    use_floating_dock_adorner: bool,
    window: RefCell<Option<Ref<DockAdornerWindow>>>,
    layer: RefCell<Option<Ref<AdornerLayer>>>,
    managed_layer: RefCell<Option<Ref<ManagedWindowLayer>>>,
    geometry_surface: RefCell<Option<Ref<Visual>>>,
    geometry_host: RefCell<Option<Ref<Visual>>>,
    geometry_window: RefCell<Option<Ref<Window>>>,
    managed_bounds: Cell<Option<Rect>>,
    updating_geometry: Cell<bool>,
    layout_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    detached_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    position_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}
impl<T: AdornerTarget> AdornerHelper<T> {
    pub(crate) fn new(use_floating_dock_adorner: bool) -> Rc<Self> {
        Rc::new(Self {
            cached: T::create(),
            adorner: RefCell::new(None),
            use_floating_dock_adorner,
            window: RefCell::new(None),
            layer: RefCell::new(None),
            managed_layer: RefCell::new(None),
            geometry_surface: RefCell::new(None),
            geometry_host: RefCell::new(None),
            geometry_window: RefCell::new(None),
            managed_bounds: Cell::new(None),
            updating_geometry: Cell::new(false),
            layout_subscription: RefCell::new(None),
            detached_subscription: RefCell::new(None),
            position_subscription: RefCell::new(None),
        })
    }
    fn overlay_key(&self) -> &'static str {
        if self.cached.cast::<GlobalDockTarget>().is_some() {
            "GlobalDockAdorner"
        } else {
            "DockAdorner"
        }
    }
    fn control(&self) -> Ref<Control> {
        self.cached.clone().upcast()
    }
    pub(crate) fn add_adorner(
        self: &Rc<Self>,
        visual: &Visual,
        indicators_only: bool,
        horizontal: bool,
        vertical: bool,
    ) {
        self.clear_geometry();
        if let Some(target) = self.cached.cast::<DockTarget>() {
            if let Some(theme) = DockTarget::get_target_theme(visual) {
                target.set_theme(Some(theme));
            } else {
                target.clear_value(ferroui_base::StyledElement::theme_property());
            }
            if let Some(host) = DockTarget::get_adorner_host_provider(visual)
                .and_then(|p| p.get_adorner_host(visual))
            {
                if !host.ptr_eq(&visual.to_ref()) && host.is_visual_ancestor_of(visual) {
                    self.geometry_surface.replace(Some(visual.to_ref()));
                    self.geometry_host.replace(Some(host.upcast()));
                }
            }
        }
        if self.use_floating_dock_adorner {
            self.add_floating_adorner(visual, indicators_only, horizontal, vertical);
        } else {
            self.add_regular_adorner(visual, indicators_only, horizontal, vertical);
        }
        let host = self
            .geometry_host
            .borrow()
            .clone()
            .and_then(|h| h.cast::<Control>());
        if let Some(host) = host.filter(|_| {
            self.window.borrow().is_some()
                || self.layer.borrow().is_some()
                || self.managed_layer.borrow().is_some()
        }) {
            let owner = self.clone();
            self.layout_subscription
                .replace(Some(host.layout_updated(move || owner.update_geometry())));
            let surface = self
                .geometry_surface
                .borrow()
                .clone()
                .expect("source geometry surface");
            let owner = self.clone();
            self.detached_subscription.replace(Some(
                surface.detached_from_visual_tree(move |_| owner.on_geometry_detached()),
            ));
            self.geometry_window
                .replace(TopLevel::get_top_level(Some(visual)).and_then(|t| t.cast::<Window>()));
            let window = self.geometry_window.borrow().clone();
            if let Some(window) = window {
                let owner = self.clone();
                self.position_subscription.replace(Some(
                    window.position_changed(move |_| owner.update_geometry()),
                ));
            }
            self.update_geometry();
        } else {
            self.clear_geometry();
        }
    }
    fn apply_flags(&self, control: &Control, indicators: bool, horizontal: bool, vertical: bool) {
        if let Some(target) = control.to_ref().cast::<DockTarget>() {
            target.set_show_indicators_only(indicators);
            target.set_show_horizontal_targets(horizontal);
            target.set_show_vertical_targets(vertical);
        } else if let Some(target) = control.to_ref().cast::<GlobalDockTarget>() {
            target.set_show_indicators_only(indicators);
            target.set_show_horizontal_targets(horizontal);
            target.set_show_vertical_targets(vertical);
        }
    }
    fn window(&self) -> Option<Ref<DockAdornerWindow>> {
        self.window.borrow().clone()
    }
    fn add_floating_adorner(
        &self,
        visual: &Visual,
        indicators: bool,
        horizontal: bool,
        vertical: bool,
    ) {
        if DockHelpers::is_managed_window_hosting_enabled(Some(visual)) {
            self.add_managed_adorner(visual, indicators, horizontal, vertical);
            return;
        }
        if self.window().is_some() {
            if self
                .window()
                .expect("source window")
                .content()
                .is_some_and(|v| {
                    ValueTypes::as_object(&*v).is_some_and(|o| o.ptr_eq(&self.control()))
                })
            {
                self.window().expect("source window").set_content(None);
            }
            self.window()
                .expect("source window after content change")
                .close();
            self.window.replace(None);
        }
        self.adorner.replace(Some(self.control()));
        let current = self.adorner.borrow().clone();
        if let Some(current) = current {
            self.apply_flags(&current, indicators, horizontal, vertical);
        }
        let Some(root) = TopLevel::get_top_level(Some(visual)).and_then(|r| r.cast::<Window>())
        else {
            return;
        };
        let position = visual.point_to_screen(Point::default());
        let width = visual.bounds().width;
        let height = visual.bounds().height;
        let window = DockAdornerWindow::get_factory(visual)
            .and_then(|f| f.create_window(visual, &self.control()))
            .unwrap_or_else(DockAdornerWindow::new);
        self.window.replace(Some(window));
        self.window()
            .expect("source assigned window")
            .set_width(width);
        self.window()
            .expect("source assigned window")
            .set_height(height);
        let current = self.adorner.borrow().clone();
        self.window()
            .expect("source assigned window")
            .set_content(current.map(Control::boxed));
        self.window()
            .expect("source assigned window")
            .set_position(position);
        self.window()
            .expect("source assigned window")
            .set_size_to_content(SizeToContent::MANUAL);
        self.window()
            .expect("source assigned window")
            .set_is_hit_test_visible(true);
        let current = self.adorner.borrow().clone();
        if let Some(current) = current {
            current.set_width(width);
            current.set_height(height);
        }
        self.update_geometry();
        self.window()
            .expect("source assigned window")
            .show_with_owner(&root);
    }
    fn add_regular_adorner(
        &self,
        visual: &Visual,
        indicators: bool,
        horizontal: bool,
        vertical: bool,
    ) {
        self.remove_regular_adorner();
        let Some(layer) = AdornerLayer::get_adorner_layer(visual) else {
            return;
        };
        self.adorner.replace(Some(self.control()));
        let current = self
            .adorner
            .borrow()
            .clone()
            .expect("source assigned adorner");
        let host = self
            .geometry_host
            .borrow()
            .clone()
            .unwrap_or_else(|| visual.to_ref());
        AdornerLayer::set_adorned_element(&current, host);
        let current = self.adorner.borrow().clone();
        if let Some(current) = current {
            self.apply_flags(&current, indicators, horizontal, vertical);
        }
        let current = self.adorner.borrow().clone().expect("source adorner");
        current.set_parent(visual.to_ref().upcast::<StyledElement>());
        let children = layer.children();
        let current = self.adorner.borrow().clone().expect("source adorner");
        children.add(current);
        self.layer.replace(Some(layer));
    }
    pub(crate) fn set_global_dock_availability(&self, value: bool) {
        if let Some(target) = self.cached.cast::<DockTargetBase>() {
            target.set_is_global_dock_available(value);
        }
        let current = self.adorner.borrow().clone();
        if let Some(current) = current.and_then(|c| c.cast::<DockTargetBase>()) {
            current.set_is_global_dock_available(value);
        }
    }
    pub(crate) fn set_global_dock_active(&self, value: bool) {
        if let Some(target) = self.cached.cast::<DockTargetBase>() {
            target.set_is_global_dock_active(value);
        }
        let current = self.adorner.borrow().clone();
        if let Some(current) = current.and_then(|c| c.cast::<DockTargetBase>()) {
            current.set_is_global_dock_active(value);
        }
    }
    pub(crate) fn remove_adorner(&self, _visual: Option<&Visual>) {
        self.clear_geometry();
        if self.use_floating_dock_adorner {
            self.remove_floating_adorner();
        } else {
            self.remove_regular_adorner();
        }
        if let Some(target) = self.cached.cast::<DockTarget>() {
            target.clear_value(ferroui_base::StyledElement::theme_property());
        }
    }
    fn remove_floating_adorner(&self) {
        if self.managed_layer.borrow().is_some() {
            self.remove_managed_adorner();
            return;
        }
        if self.window().is_some() {
            if self
                .window()
                .expect("source window")
                .content()
                .is_some_and(|v| {
                    ValueTypes::as_object(&*v).is_some_and(|o| o.ptr_eq(&self.control()))
                })
            {
                self.window().expect("source window").set_content(None);
            }
            self.window()
                .expect("source window after content change")
                .close();
            self.window.replace(None);
        }
        self.adorner.replace(None);
        self.cached.reset_target();
    }
    fn add_managed_adorner(
        &self,
        visual: &Visual,
        indicators: bool,
        horizontal: bool,
        vertical: bool,
    ) {
        let Some(layer) = ManagedWindowLayer::try_get_layer(Some(visual)) else {
            return;
        };
        self.managed_layer.replace(Some(layer.clone()));
        self.adorner.replace(Some(self.control()));
        let current = self.adorner.borrow().clone();
        if let Some(current) = current {
            self.apply_flags(&current, indicators, horizontal, vertical);
        }
        let position = visual.point_to_screen(Point::default());
        let bounds = Rect::from_size(visual.bounds().size());
        let bounds = Self::managed_bounds_from_screen(&layer, position, bounds.size());
        layer.show_overlay(self.overlay_key(), self.control(), bounds, true);
    }
    fn remove_managed_adorner(&self) {
        let layer = self.managed_layer.borrow().clone();
        if let Some(layer) = layer {
            layer.hide_overlay(self.overlay_key());
            self.managed_layer.replace(None);
            self.managed_bounds.set(None);
        }
        self.adorner.replace(None);
        self.cached.reset_target();
    }
    fn managed_bounds_from_screen(
        layer: &ManagedWindowLayer,
        point: ferroui_base::PixelPoint,
        size: Size,
    ) -> Rect {
        let Some(top) = TopLevel::get_top_level(Some(layer)) else {
            return Rect::from_size(size);
        };
        let client = top.point_to_client(point);
        let origin = layer
            .translate_point(Point::default(), &top)
            .unwrap_or_default();
        Rect::new(
            client.x - origin.x,
            client.y - origin.y,
            size.width,
            size.height,
        )
    }
    fn remove_regular_adorner(&self) {
        let layer = self.layer.borrow().clone();
        let current = self.adorner.borrow().clone();
        if let (Some(layer), Some(current)) = (layer, current) {
            layer.children().remove(&current);
            let current = self
                .adorner
                .borrow()
                .clone()
                .expect("source adorner after removal");
            current.set_parent(None::<Ref<StyledElement>>);
            let current = self
                .adorner
                .borrow()
                .clone()
                .expect("source adorner after parent change");
            AdornerLayer::set_adorned_element(&current, None::<Ref<Visual>>);
        }
        AdornerLayer::set_adorned_element(&self.control(), None::<Ref<Visual>>);
        self.adorner.replace(None);
        self.layer.replace(None);
        self.cached.reset_target();
    }
    pub(crate) fn update_geometry(&self) {
        if self.updating_geometry.get() {
            return;
        }
        let Some(surface) = self.geometry_surface.borrow().clone() else {
            return;
        };
        let Some(host) = self.geometry_host.borrow().clone() else {
            return;
        };
        let Some(target) = self.cached.cast::<DockTarget>() else {
            return;
        };
        let relative = self
            .managed_layer
            .borrow()
            .clone()
            .map(|l| l.upcast::<Visual>())
            .or_else(|| {
                if self.use_floating_dock_adorner {
                    TopLevel::get_top_level(Some(&surface)).map(|t| t.upcast::<Visual>())
                } else {
                    Some(host.clone())
                }
            });
        let Some(relative) = relative else { return };
        let Some(host_transform) = host.transform_to_visual(&relative) else {
            return;
        };
        let Some(surface_transform) = surface.transform_to_visual(&relative) else {
            return;
        };
        self.updating_geometry.set(true);
        let _guard = GeometryUpdateGuard(&self.updating_geometry);
        let bounds = Rect::from_size(host.bounds().size()).transform_to_aabb(host_transform);
        let local = Rect::from_size(surface.bounds().size()).transform_to_aabb(surface_transform);
        target.set_target_margin(Thickness::new(
            local.left() - bounds.left(),
            local.top() - bounds.top(),
            bounds.right() - local.right(),
            bounds.bottom() - local.bottom(),
        ));
        if self.window().is_some() {
            self.window()
                .expect("source window")
                .set_position(relative.point_to_screen(bounds.top_left()));
            let window = self.window().expect("source width assignment receiver");
            target.set_width(bounds.width);
            window.set_width(bounds.width);
            let window = self.window().expect("source height assignment receiver");
            target.set_height(bounds.height);
            window.set_height(bounds.height);
        } else if self.managed_layer.borrow().is_some() && self.managed_bounds.get() != Some(bounds)
        {
            self.managed_bounds.set(Some(bounds));
            let layer = self
                .managed_layer
                .borrow()
                .clone()
                .expect("source managed layer");
            layer.show_overlay(self.overlay_key(), self.control(), bounds, true);
        }
    }
    fn on_geometry_detached(&self) {
        let surface = self.geometry_surface.borrow().clone();
        if let Some(surface) = surface {
            self.remove_adorner(Some(&surface));
        }
    }
    fn clear_geometry(&self) {
        let sub = self.layout_subscription.borrow_mut().take();
        if let Some(sub) = sub {
            sub.dispose();
        }
        let sub = self.detached_subscription.borrow_mut().take();
        if let Some(sub) = sub {
            sub.dispose();
        }
        let sub = self.position_subscription.borrow_mut().take();
        if let Some(sub) = sub {
            sub.dispose();
        }
        self.geometry_surface.replace(None);
        self.geometry_host.replace(None);
        self.geometry_window.replace(None);
        self.managed_bounds.set(None);
        if let Some(target) = self.cached.cast::<DockTarget>() {
            target.set_target_margin(Thickness::default());
        }
    }
}
struct GeometryUpdateGuard<'a>(&'a Cell<bool>);
impl Drop for GeometryUpdateGuard<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}
