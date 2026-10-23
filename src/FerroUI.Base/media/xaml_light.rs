//! `XamlLight` of WinUI (`Microsoft.UI.Xaml.Media.XamlLight`): a light of the
//! object model that an element holds in its `Lights`, which lights the
//! elements and the brushes that target its id in the subtree of that
//! element (winui-materials.md).

use crate::collections::{FerroList, NotifyCollectionChangedEventArgs};
use crate::media::{IBrush, XamlCompositionBrushBase};
use crate::reactive::{Disposable, IDisposable};
use crate::rendering::composition::{CompositionLight, CompositionVisual, DistantLight, PointLight, SpotLight};
use crate::{ferro_class, FerroObject, FerroObjectImpl, Ref, Visual, WeakRef};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default)]
struct XamlLightState {
    /// `CompositionLight`: the composition light the subclass set.
    composition_light: Option<Rc<CompositionLight>>,
    /// The element whose `Lights` holds the light, while it is connected.
    host: Option<WeakRef<Visual>>,
}

/// A light of the object model (WinUI `Microsoft.UI.Xaml.Media.XamlLight`).
///
/// A subclass gives the light an id ([`get_id`](XamlLightImpl::get_id)) and
/// creates its composition light when the light is connected to the element
/// that holds it ([`on_connected`](XamlLightImpl::on_connected)), setting it
/// with [`set_composition_light`](Self::set_composition_light). The light
/// then lights, in the subtree of that element:
///
/// - the elements that target its id ([`add_target_element`](Self::add_target_element)):
///   their composition visuals are targets of the composition light, which
///   lights their content;
/// - the XAML brushes that target its id ([`add_target_brush`](Self::add_target_brush)):
///   the scene lighting effects of those brushes receive the light.
///
/// The composition light is in the coordinates of the element that holds
/// it (its `CoordinateSpace` is the composition visual of that element).
#[repr(C)]
pub struct XamlLight {
    base: FerroObject,
    state: RefCell<XamlLightState>,
}

ferro_class! {
    XamlLight: FerroObject, virtuals XamlLightImpl: FerroObjectImpl {
        /// `GetId`: the id the elements and brushes the light lights target.
        fn get_id(this) -> String;

        /// `OnConnected`: the light is in the `Lights` of `new_element`, which
        /// is in a visual tree.
        fn on_connected(this, new_element: &Visual);

        /// `OnDisconnected`: the light left `old_element`, or the element
        /// left its visual tree.
        fn on_disconnected(this, old_element: &Visual);
    }
}
crate::ferro_class_info!(XamlLight {
    markup: {
        namespace: "FerroUI.Media",
    },
});

crate::ferro_impl_classes!(XamlLight: FerroObjectImpl);

impl XamlLightImpl for XamlLight {
    fn get_id(_this: &Self) -> String {
        String::new()
    }

    fn on_connected(_this: &Self, _new_element: &Visual) {}

    fn on_disconnected(_this: &Self, _old_element: &Visual) {}
}

impl XamlLight {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: FerroObject::construct(), state: RefCell::new(XamlLightState::default()) }
    }

    /// The id of the light (`GetId`).
    pub fn id(&self) -> String {
        self.get_id()
    }

    /// `CompositionLight`.
    pub fn composition_light(&self) -> Option<Rc<CompositionLight>> {
        self.state.borrow().composition_light.clone()
    }

    /// Sets the composition light (`None` removes it). While the light is
    /// connected, the composition light is bound to the element that holds
    /// it: its coordinate space is that element, and it lights the targets
    /// of the id of the light in its subtree.
    pub fn set_composition_light(&self, light: Option<Rc<CompositionLight>>) {
        let old = {
            let mut state = self.state.borrow_mut();
            let same = match (&state.composition_light, &light) {
                (Some(a), Some(b)) => Rc::ptr_eq(a, b),
                (None, None) => true,
                _ => false,
            };
            if same {
                return;
            }
            std::mem::replace(&mut state.composition_light, light)
        };
        if let Some(old) = old {
            unbind(&old);
        }
        self.bind();
    }

    /// The element whose `Lights` holds the light, while it is connected.
    pub fn host(&self) -> Option<Ref<Visual>> {
        self.state.borrow().host.as_ref().and_then(WeakRef::upgrade)
    }

    /// Whether the light is connected to an element.
    pub fn is_connected(&self) -> bool {
        self.host().is_some()
    }

    fn connect(&self, host: &Visual) {
        if self.host().is_some_and(|existing| std::ptr::eq::<Visual>(&*existing, host)) {
            return;
        }
        if let Some(previous) = self.host() {
            self.disconnect(&previous);
        }
        self.state.borrow_mut().host = Some(host.to_ref().downgrade());
        self.on_connected(host);
        self.bind();
    }

    fn disconnect(&self, host: &Visual) {
        if !self.host().is_some_and(|existing| std::ptr::eq::<Visual>(&*existing, host)) {
            return;
        }
        if let Some(light) = self.composition_light() {
            unbind(&light);
        }
        self.on_disconnected(host);
        self.state.borrow_mut().host = None;
    }

    /// Binds the composition light to the element that holds the light:
    /// its XAML id and scope, its coordinate space, and the elements of the
    /// subtree that target the id.
    fn bind(&self) {
        let (Some(light), Some(host)) = (self.composition_light(), self.host()) else { return };
        let Some(scope) = host.composition_visual().map(|visual| (*visual).clone()) else { return };
        let id = self.id();
        light.set_xaml_binding(Some(id.clone()), Some(scope.clone()));
        set_coordinate_space(&light, Some(scope));
        let mut stack = vec![host.clone()];
        while let Some(visual) = stack.pop() {
            if visual.light_target_ids().iter().any(|existing| *existing == id) {
                if let Some(target) = visual.composition_visual() {
                    light.targets().add((*target).clone());
                }
            }
            stack.extend(visual.visual_children().to_vec());
        }
    }

    /// `AddTargetElement(id, element)`: the XAML lights of id `id` light
    /// `element` (its content and its subtree) where it is in the subtree of
    /// the element that holds them.
    pub fn add_target_element(id: &str, element: &Visual) {
        if element.add_light_target_id(id) {
            offer_to_lights(element);
        }
    }

    /// `RemoveTargetElement(id, element)`.
    pub fn remove_target_element(id: &str, element: &Visual) {
        if !element.remove_light_target_id(id) {
            return;
        }
        let Some(target) = element.composition_visual().map(|visual| (*visual).clone()) else { return };
        for light in lights_above(element) {
            if light.id() == id {
                if let Some(composition_light) = light.composition_light() {
                    composition_light.targets().remove(&target);
                }
            }
        }
    }

    /// `AddTargetBrush(id, brush)`: the XAML lights of id `id` light `brush`
    /// where it is drawn in the subtree of the element that holds them. Only
    /// a XAML composition brush can be lit (its scene lighting effects
    /// receive the light); other brushes are left as they are.
    pub fn add_target_brush(id: &str, brush: &Rc<dyn IBrush>) {
        if let Some(brush) = XamlCompositionBrushBase::of(&**brush) {
            brush.add_light_target_id(id);
        }
    }

    /// `RemoveTargetBrush(id, brush)`.
    pub fn remove_target_brush(id: &str, brush: &Rc<dyn IBrush>) {
        if let Some(brush) = XamlCompositionBrushBase::of(&**brush) {
            brush.remove_light_target_id(id);
        }
    }
}

fn set_coordinate_space(light: &Rc<CompositionLight>, space: Option<Rc<CompositionVisual>>) {
    if let Some(spot) = SpotLight::from_light(light) {
        spot.set_coordinate_space(space);
    } else if let Some(point) = PointLight::from_light(light) {
        point.set_coordinate_space(space);
    } else if let Some(distant) = DistantLight::from_light(light) {
        distant.set_coordinate_space(space);
    }
}

fn unbind(light: &Rc<CompositionLight>) {
    light.set_xaml_binding(None, None);
    set_coordinate_space(light, None);
    light.targets().remove_all();
}

/// The connected XAML lights of the element and its ancestors.
fn lights_above(element: &Visual) -> Vec<Ref<XamlLight>> {
    let mut lights = Vec::new();
    let mut current = Some(element.to_ref());
    while let Some(visual) = current {
        if visual.has_lights() {
            lights.extend(visual.lights().to_vec().into_iter().filter(|light| light.is_connected()));
        }
        current = visual.visual_parent();
    }
    lights
}

/// Adds `element` to the targets of the connected lights above it whose id
/// it targets.
fn offer_to_lights(element: &Visual) {
    let Some(target) = element.composition_visual().map(|visual| (*visual).clone()) else { return };
    let ids = element.light_target_ids();
    for light in lights_above(element) {
        if ids.iter().any(|id| *id == light.id()) {
            if let Some(composition_light) = light.composition_light() {
                composition_light.targets().add(target.clone());
            }
        }
    }
}

/// Connects the lights of `lights` to `element` while it is in a visual
/// tree, and follows the changes of the collection.
pub(crate) fn track_lights(element: &Visual, lights: &FerroList<Ref<XamlLight>>) -> Vec<Rc<dyn IDisposable>> {
    let connected: Rc<RefCell<Vec<Ref<XamlLight>>>> = Rc::new(RefCell::new(Vec::new()));
    let sync = {
        let weak = element.to_ref().downgrade();
        let lights = lights.clone();
        let connected = connected.clone();
        // `attached`: whether the element is in a visual tree (`None`: as it says). The detached
        // event is raised while the element still reports itself attached.
        Rc::new(move |attached: Option<bool>| {
            let Some(element) = weak.upgrade() else { return };
            let attached = attached.unwrap_or_else(|| element.is_attached_to_visual_tree());
            let wanted = if attached { lights.to_vec() } else { Vec::new() };
            let previous = std::mem::take(&mut *connected.borrow_mut());
            for light in previous.iter().filter(|light| !wanted.contains(light)) {
                light.disconnect(&element);
            }
            for light in &wanted {
                light.connect(&element);
            }
            *connected.borrow_mut() = wanted;
        })
    };
    let token = {
        let sync = sync.clone();
        lights.add_collection_changed(Rc::new(move |_: &NotifyCollectionChangedEventArgs<'_, Ref<XamlLight>>| sync(None)))
    };
    let collection = {
        let lights = lights.clone();
        Disposable::create(move || {
            lights.remove_collection_changed(token);
        })
    };
    let attached = {
        let sync = sync.clone();
        element.attached_to_visual_tree(move |_| sync(Some(true)))
    };
    let detached = {
        let sync = sync.clone();
        element.detached_from_visual_tree(move |_| sync(Some(false)))
    };
    sync(None);
    vec![collection, attached, detached]
}

/// Offers `element` to the lights above it each time it enters a visual
/// tree.
pub(crate) fn track_light_target(element: &Visual) -> Rc<dyn IDisposable> {
    let weak = element.to_ref().downgrade();
    element.attached_to_visual_tree(move |_| {
        if let Some(element) = weak.upgrade() {
            offer_to_lights(&element);
        }
    })
}
