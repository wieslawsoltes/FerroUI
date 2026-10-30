// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::controls::SelectionOverlayAdorner;
use ferrodock_model::core::{DockableControlMap, DockableObjectMap, IDockable, IFactory};
use ferroui_base::{
    data::core::ValueTypes, reactive::IDisposable, AnyValue, BoxedValue, Point, Rect, Ref,
    StyledElement,
};
use ferroui_controls::{primitives::AdornerLayer, Control};
use std::{cell::RefCell, rc::Rc};

pub(crate) struct SelectionOverlayHelper {
    adorner: RefCell<Option<Ref<SelectionOverlayAdorner>>>,
    root: RefCell<Option<Ref<Control>>>,
    target: RefCell<Option<Ref<Control>>>,
    layout_updated: RefCell<Option<Rc<dyn IDisposable>>>,
}
impl SelectionOverlayHelper {
    pub(crate) fn new() -> Rc<Self> {
        Rc::new(Self {
            adorner: RefCell::new(None),
            root: RefCell::new(None),
            target: RefCell::new(None),
            layout_updated: RefCell::new(None),
        })
    }
    pub(crate) fn attach_overlay(self: &Rc<Self>, root: Ref<Control>) {
        if self.root.borrow().as_ref() == Some(&root) {
            return;
        }
        self.remove_overlay();
        let Some(layer) = AdornerLayer::get_adorner_layer(&root) else {
            return;
        };
        self.root.replace(Some(root.clone()));
        let adorner = SelectionOverlayAdorner::new();
        AdornerLayer::set_adorned_element(&adorner, root.clone());
        self.adorner.replace(Some(adorner.clone()));
        adorner.set_parent(root.clone().upcast::<StyledElement>());
        layer.children().add(adorner.upcast::<Control>());
        let owner = self.clone();
        self.layout_updated
            .replace(Some(root.layout_updated(move || owner.on_layout_updated())));
    }
    pub(crate) fn remove_overlay(&self) {
        let root = self.root.borrow().clone();
        let adorner = self.adorner.borrow().clone();
        let (Some(root), Some(adorner)) = (root, adorner) else {
            return;
        };
        if let Some(layer) = AdornerLayer::get_adorner_layer(&root) {
            layer.children().remove(adorner.clone().upcast::<Control>());
        }
        // Re-read source fields after callbacks: a nested removal can clear them.
        let adorner = self
            .adorner
            .borrow()
            .clone()
            .expect("Object reference not set to an instance of an object.");
        adorner.set_parent(None::<Ref<StyledElement>>);
        let _root = self
            .root
            .borrow()
            .clone()
            .expect("Object reference not set to an instance of an object.");
        let subscription = self.layout_updated.borrow().clone();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
        self.layout_updated.replace(None);
        self.root.replace(None);
        self.adorner.replace(None);
        self.target.replace(None);
    }
    pub(crate) fn highlight(&self, target: Option<Ref<Control>>) {
        self.target.replace(target);
        self.update();
    }
    fn on_layout_updated(&self) {
        self.update();
    }
    fn update(&self) {
        let root = self.root.borrow().clone();
        let adorner = self.adorner.borrow().clone();
        let (Some(root), Some(adorner)) = (root, adorner) else {
            return;
        };
        let target = self.target.borrow().clone();
        let rect = target.and_then(|target| {
            target
                .translate_point(Point::default(), &root)
                .map(|position| Rect::from_position_size(position, target.bounds().size()))
        });
        adorner.set_highlight_rect(rect);
        adorner.invalidate_visual();
    }
    pub(crate) fn try_get_control(
        factory: &dyn IFactory,
        dockable: &Rc<dyn IDockable>,
    ) -> Option<(Ref<Control>, Option<Ref<Control>>)> {
        // Keep source short-circuit ordering: do not query later factory maps after a match.
        if let Some(pair) = Self::find_control(
            factory.visible_dockable_controls(),
            || factory.visible_root_controls(),
            dockable,
        ) {
            return Some(pair);
        }
        if let Some(pair) = Self::find_control(
            factory.pinned_dockable_controls(),
            || factory.pinned_root_controls(),
            dockable,
        ) {
            return Some(pair);
        }
        Self::find_control(
            factory.tab_dockable_controls(),
            || factory.tab_root_controls(),
            dockable,
        )
    }
    fn find_control(
        controls: DockableControlMap,
        roots: impl FnOnce() -> DockableObjectMap,
        dockable: &Rc<dyn IDockable>,
    ) -> Option<(Ref<Control>, Option<Ref<Control>>)> {
        let control = controls.borrow().get(dockable).cloned()?;
        let root = roots().borrow().get(dockable).cloned()?;
        let root = Self::control_value(&root)?;
        let control = Self::control_value(&control.boxed_reference());
        Some((root, control))
    }
    fn control_value(value: &ferrodock_model::BoxedValue) -> Option<Ref<Control>> {
        // Probe the actual reference through native assignability without wrapping
        // or replacing the control that is returned. Models use framework-neutral Any.
        let probe: BoxedValue = Rc::new(ReferenceProbe(value.clone()));
        ValueTypes::as_object(probe.as_ref())?.cast::<Control>()
    }
}

struct ReferenceProbe(ferrodock_model::BoxedValue);
impl AnyValue for ReferenceProbe {
    fn any_value_as_any(&self) -> &dyn std::any::Any {
        self.0.as_ref()
    }
    fn any_value_eq(&self, other: &dyn AnyValue) -> bool {
        std::ptr::addr_eq(self.0.as_ref(), other.any_value_as_any())
    }
    fn any_value_type_name(&self) -> &'static str {
        "source control projection"
    }
}
