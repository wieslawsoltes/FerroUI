//! The map in the automation tree: the control, and its elements as children with a name, bounds
//! and Invoke. The peers exist only once an automation client has asked for them.

use crate::map_control::MapControl;
use crate::map_elements::{MapElement, MapElementsLayer};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, ObjectType,
    Rect, Ref, WeakRef,
};
use ferroui_controls::automation::peers::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, ControlAutomationPeer,
    ControlAutomationPeerImpl,
};
use ferroui_controls::automation::provider::{IFocusedDescendantProvider, IInvokeProvider};
use ferroui_controls::automation::ElementNotEnabledError;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// The handle through which a peer of this crate implements a provider contract.
struct ProviderAdapter<T: ObjectType>(Ref<T>);

impl ProviderAdapter<MapControlAutomationPeer> {
    fn as_focused_descendant_provider(
        peer: Ref<MapControlAutomationPeer>,
    ) -> Rc<dyn IFocusedDescendantProvider> {
        Rc::new(Self(peer))
    }
}

impl ProviderAdapter<MapElementAutomationPeer> {
    fn as_invoke_provider(peer: Ref<MapElementAutomationPeer>) -> Rc<dyn IInvokeProvider> {
        Rc::new(Self(peer))
    }
}

/// The automation peer of a map: a pane whose children are its elements.
#[repr(C)]
pub struct MapControlAutomationPeer {
    base: ControlAutomationPeer,
    /// Kept between reads, so that an element keeps its peer while it stays where it is.
    element_peers: RefCell<Vec<Ref<MapElementAutomationPeer>>>,
    focus_changed: HandlerList<dyn Fn()>,
}

ferro_class!(MapControlAutomationPeer: ControlAutomationPeer);
ferro_class_info!(MapControlAutomationPeer {
    interfaces: [Rc<dyn IFocusedDescendantProvider> => ProviderAdapter::as_focused_descendant_provider]
});
ferro_impl_classes!(MapControlAutomationPeer: FerroObjectImpl);

impl AutomationPeerImpl for MapControlAutomationPeer {
    fn get_automation_control_type_core(_: &Self) -> AutomationControlType {
        AutomationControlType::Pane
    }

    fn get_class_name_core(_: &Self) -> String {
        "MapControl".into()
    }
}

impl ControlAutomationPeerImpl for MapControlAutomationPeer {
    fn get_children_core(this: &Self) -> Option<Vec<Ref<AutomationPeer>>> {
        let map = this.map()?;
        let this_peer: Ref<AutomationPeer> = this.to_ref().upcast();
        let previous = std::mem::take(&mut *this.element_peers.borrow_mut());
        let mut peers = Vec::new();
        for layer in map.layers() {
            for index in 0..layer.len() {
                if !layer.can_focus(index) {
                    continue;
                }
                let known = previous
                    .iter()
                    .find(|peer| peer.is_at(&layer, index))
                    .cloned();
                peers.push(known.unwrap_or_else(|| {
                    MapElementAutomationPeer::new(&map, &this_peer, &layer, index)
                }));
            }
        }
        let children = peers
            .iter()
            .map(|peer| peer.clone().upcast())
            .collect::<Vec<_>>();
        *this.element_peers.borrow_mut() = peers;
        (!children.is_empty()).then_some(children)
    }
}

impl IFocusedDescendantProvider for ProviderAdapter<MapControlAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }

    fn focused_descendant(&self) -> Option<Ref<AutomationPeer>> {
        let map = self.0.map()?;
        let (layer, index) = map.focused_element()?;
        self.0.get_children();
        let peers = self.0.element_peers.borrow();
        peers
            .iter()
            .find(|peer| peer.is_at(&layer, index))
            .map(|peer| peer.clone().upcast())
    }

    fn focused_descendant_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        let token = self.0.focus_changed.add(handler);
        let weak = self.0.downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.focus_changed.remove(token);
            }
        })
    }
}

impl MapControlAutomationPeer {
    pub fn new(owner: &MapControl) -> Ref<Self> {
        instantiate(Self {
            base: ControlAutomationPeer::construct(owner),
            element_peers: RefCell::new(Vec::new()),
            focus_changed: HandlerList::new(),
        })
    }

    fn map(&self) -> Option<Ref<MapControl>> {
        self.try_owner()
            .and_then(|owner| owner.cast::<MapControl>())
    }

    /// Tells the root of the tree that the focus moved inside the map.
    pub(crate) fn raise_focused_element_changed(&self) {
        for (_, handler) in self.focus_changed.snapshot().iter() {
            handler();
        }
    }
}

/// The automation peer of one element of a map.
#[repr(C)]
pub struct MapElementAutomationPeer {
    base: AutomationPeer,
    map: WeakRef<MapControl>,
    container: WeakRef<AutomationPeer>,
    layer: Weak<MapElementsLayer>,
    index: usize,
    parent: RefCell<Option<WeakRef<AutomationPeer>>>,
}

ferro_class!(MapElementAutomationPeer: AutomationPeer);
ferro_class_info!(MapElementAutomationPeer {
    interfaces: [Rc<dyn IInvokeProvider> => ProviderAdapter::as_invoke_provider]
});

impl FerroObjectImpl for MapElementAutomationPeer {}

impl AutomationPeerImpl for MapElementAutomationPeer {
    fn is_element_available_core(this: &Self) -> bool {
        this.parts().is_some()
    }

    fn bring_into_view_core(this: &Self) {
        if let Some((map, layer, _)) = this.parts() {
            map.reveal_element(&layer, this.index);
        }
    }

    fn get_accelerator_key_core(_: &Self) -> Option<String> {
        None
    }

    fn get_access_key_core(_: &Self) -> Option<String> {
        None
    }

    fn get_automation_control_type_core(_: &Self) -> AutomationControlType {
        // An element does one thing when it is activated.
        AutomationControlType::Button
    }

    fn get_automation_id_core(_: &Self) -> Option<String> {
        None
    }

    fn get_bounding_rectangle_core(this: &Self) -> Rect {
        let Some((map, _, element)) = this.parts() else {
            return Rect::default();
        };
        let view = map.view();
        let Some((left, top, right, bottom)) = element.view_bounds(&view) else {
            return Rect::default();
        };
        // What is outside the map is clipped by it.
        let (left, top) = (left.max(0.0), top.max(0.0));
        let (right, bottom) = (right.min(view.width), bottom.min(view.height));
        if right <= left || bottom <= top {
            return Rect::default();
        }
        let Some(root) = map.visual_root() else {
            return Rect::default();
        };
        let Some(transform) = map.transform_to_visual(&root) else {
            return Rect::default();
        };
        Rect::new(left, top, right - left, bottom - top).transform_to_aabb(transform)
    }

    fn get_or_create_children_core(_: &Self) -> Rc<Vec<Ref<AutomationPeer>>> {
        Rc::new(Vec::new())
    }

    fn get_class_name_core(this: &Self) -> String {
        this.parts().map_or_else(String::new, |(_, _, element)| {
            element.class_name().to_owned()
        })
    }

    fn get_labeled_by_core(_: &Self) -> Option<Ref<AutomationPeer>> {
        None
    }

    fn get_name_core(this: &Self) -> Option<String> {
        this.parts()
            .map(|(_, _, element)| element.accessible_name())
    }

    fn get_parent_core(this: &Self) -> Option<Ref<AutomationPeer>> {
        this.parent.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    fn has_keyboard_focus_core(this: &Self) -> bool {
        this.parts().is_some_and(|(map, layer, _)| {
            map.is_focused()
                && map
                    .focused_element()
                    .is_some_and(|(held, index)| Rc::ptr_eq(&held, &layer) && index == this.index)
        })
    }

    fn is_content_element_core(_: &Self) -> bool {
        true
    }

    fn is_control_element_core(_: &Self) -> bool {
        true
    }

    fn is_enabled_core(this: &Self) -> bool {
        this.parts()
            .is_some_and(|(map, _, _)| map.is_effectively_enabled())
    }

    fn is_keyboard_focusable_core(this: &Self) -> bool {
        this.is_enabled()
    }

    fn is_offscreen_core(this: &Self) -> bool {
        this.get_bounding_rectangle().is_empty()
    }

    fn set_focus_core(this: &Self) {
        if !this.is_keyboard_focusable() {
            return;
        }
        if let Some((map, layer, _)) = this.parts() {
            map.focus();
            map.set_focused_element(Some((layer, this.index)));
        }
    }

    fn show_context_menu_core(this: &Self) -> bool {
        this.container
            .upgrade()
            .is_some_and(|container| container.show_context_menu())
    }

    fn get_automation_root_core(this: &Self) -> Option<Ref<AutomationPeer>> {
        this.container
            .upgrade()
            .and_then(|container| container.get_automation_root())
    }

    fn get_visual_root_core(this: &Self) -> Option<Ref<AutomationPeer>> {
        this.container
            .upgrade()
            .and_then(|container| container.get_visual_root())
    }

    fn to_screen_core(this: &Self, rect: Rect) -> Option<Rect> {
        this.container
            .upgrade()
            .and_then(|container| container.to_screen(rect))
    }

    fn try_set_parent(this: &Self, parent: Option<Ref<AutomationPeer>>) -> bool {
        *this.parent.borrow_mut() = parent.map(|peer| peer.downgrade());
        true
    }
}

impl IInvokeProvider for ProviderAdapter<MapElementAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }

    fn invoke(&self) -> Result<(), ElementNotEnabledError> {
        if !self.0.is_enabled() {
            return Err(ElementNotEnabledError::new());
        }
        if let Some((map, layer, _)) = self.0.parts() {
            map.activate_element(&layer, self.0.index);
        }
        Ok(())
    }
}

impl MapElementAutomationPeer {
    fn new(
        map: &Ref<MapControl>,
        container: &Ref<AutomationPeer>,
        layer: &Rc<MapElementsLayer>,
        index: usize,
    ) -> Ref<Self> {
        #[cfg(test)]
        crate::tests::count_element_peer();
        instantiate(Self {
            base: AutomationPeer::construct(),
            map: map.downgrade(),
            container: container.downgrade(),
            layer: Rc::downgrade(layer),
            index,
            parent: RefCell::new(Some(container.downgrade())),
        })
    }

    fn is_at(&self, layer: &Rc<MapElementsLayer>, index: usize) -> bool {
        self.index == index && std::ptr::eq(self.layer.as_ptr(), Rc::as_ptr(layer))
    }

    /// The map, the layer and the element, while all of them still exist.
    fn parts(&self) -> Option<(Ref<MapControl>, Rc<MapElementsLayer>, MapElement)> {
        let map = self.map.upgrade()?;
        let layer = self.layer.upgrade()?;
        let element = layer.element(self.index)?;
        Some((map, layer, element))
    }
}
