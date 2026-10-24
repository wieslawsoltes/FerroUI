//! A control that hosts an element: a component in XAML or in any tree of
//! controls ([`ComponentHost`]), and the components that XAML names
//! ([`register_component`]).

use crate::element::Element;
use crate::mount::MountedChild;
use crate::reactive::Owner;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_property, instantiate,
    FerroObjectImpl, FerroProperty, Ref, StyledElementImpl, StyledProperty, VisualImpl,
    VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::{Control, ControlImpl, Decorator};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

/// What makes the element a host shows.
pub type View = Rc<dyn Fn() -> Element>;

ferroui_base::ferro_thread_local! {
    static COMPONENTS: RefCell<HashMap<String, View>> = RefCell::new(HashMap::new());
}

/// Registers the element `view` makes under `name`, for the `Component`
/// property of a [`ComponentHost`] (`<view:ComponentHost Component="Counter"/>`
/// in XAML).
pub fn register_component<E: crate::IntoElement>(name: &str, view: impl Fn() -> E + 'static) {
    let view: View = Rc::new(move || view().into_element());
    COMPONENTS.with(|components| components.borrow_mut().insert(name.to_string(), view));
}

fn registered(name: &str) -> Option<View> {
    COMPONENTS.with(|components| components.borrow().get(name).cloned())
}

#[derive(Default)]
struct HostState {
    view: Option<View>,
    /// The owner the content is mounted under: the owner that was current
    /// when the view was given, so that contexts reach it.
    parent: Option<Owner>,
    mounted: Option<(Owner, MountedChild)>,
    /// The pool of the template that made the host: a detached host goes
    /// there, mounted, instead of ending its component.
    pool: Option<Weak<HostPool>>,
    /// What the inspector calls the tree of the host.
    origin: Option<&'static str>,
}

/// Hosts that a data template keeps for reuse (see [`template`](crate::template)).
pub(crate) type HostPool = RefCell<Vec<Ref<ComponentHost>>>;

/// A control that shows an element: a component hosted in XAML, an item of
/// a data template ([`template`](crate::template)), or any element placed in
/// a tree of controls made otherwise.
///
/// The element is mounted when the host is given it and when the host is
/// attached to the visual tree again; it is disposed (its effects, hooks and
/// handlers end) when the host is detached from the visual tree. Components
/// inside read the `DataContext` of the host with
/// [`use_data_context`](crate::hooks::use_data_context).
///
/// In XAML, the `Component` property names a component registered with
/// [`register_component`].
#[repr(C)]
pub struct ComponentHost {
    base: Decorator,
    state: RefCell<HostState>,
}

ferro_class!(ComponentHost: Decorator);
ferro_class_info!(ComponentHost {
    new: ComponentHost::new
});
ferro_impl_classes!(
    ComponentHost: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl VisualImpl for ComponentHost {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        if let Some(pool) = this.pool() {
            pool.borrow_mut().retain(|host| !std::ptr::eq(&**host, this));
        }
        this.mount();
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);
        match this.pool() {
            Some(pool) => pool.borrow_mut().push(this.to_ref()),
            None => this.unmount(),
        }
    }
}

ferroui_base::ferro_properties! { impl ComponentHost {
    ferro_property!(
        /// Defines the `Component` property: the name of a component
        /// registered with [`register_component`].
        pub fn component_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<ComponentHost, _>("Component", None)
        }
    );
} }

impl ComponentHost {
    fn static_constructor() {
        Self::component_property()
            .changed()
            .add_class_handler::<ComponentHost>(|host, _| {
                let view = host.component().and_then(|name| {
                    let view = registered(&name);
                    if view.is_none() {
                        eprintln!("ComponentHost: no component is registered as `{name}`");
                    }
                    view
                });
                host.set_view_rc(view);
            });
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: Decorator::construct(),
            state: RefCell::new(HostState::default()),
        }
    }

    /// A host with nothing to show.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// A host that shows the element `view` makes.
    pub fn with_view<E: crate::IntoElement>(view: impl Fn() -> E + 'static) -> Ref<Self> {
        let host = Self::new();
        host.set_view(view);
        host
    }

    /// The name of the registered component the host shows.
    pub fn component(&self) -> Option<String> {
        self.get_value(Self::component_property())
    }

    /// Shows the registered component named `name`.
    pub fn set_component(&self, name: Option<String>) {
        self.set_value(Self::component_property(), name)
    }

    /// Shows the element `view` makes, in place of what the host showed.
    pub fn set_view<E: crate::IntoElement>(&self, view: impl Fn() -> E + 'static) {
        self.set_view_rc(Some(Rc::new(move || view().into_element())));
    }

    fn set_view_rc(&self, view: Option<View>) {
        self.unmount();
        {
            let mut state = self.state.borrow_mut();
            state.view = view;
            if state.parent.is_none() {
                state.parent = Owner::current();
            }
        }
        self.mount();
    }

    /// Mounts the content under `parent` (an owner whose contexts it sees)
    /// instead of the owner that was current when the view was given.
    pub fn set_parent_owner(&self, parent: Option<Owner>) {
        self.state.borrow_mut().parent = parent;
    }

    fn pool(&self) -> Option<Rc<HostPool>> {
        self.state.borrow().pool.as_ref().and_then(Weak::upgrade)
    }

    /// Makes the host one of a template's: detached, it waits in `pool`
    /// with its component mounted, for the template to give it another item.
    /// What the inspector calls the tree of the host.
    pub(crate) fn set_origin(&self, origin: &'static str) {
        self.state.borrow_mut().origin = Some(origin);
    }

    pub(crate) fn set_pool(&self, pool: &Rc<HostPool>) {
        self.state.borrow_mut().pool = Some(Rc::downgrade(pool));
    }

    fn mount(&self) {
        let (view, parent) = {
            let state = self.state.borrow();
            if state.mounted.is_some() {
                return;
            }
            let Some(view) = state.view.clone() else {
                return;
            };
            (view, state.parent.filter(Owner::is_alive))
        };
        let owner = parent.map_or_else(Owner::new_root, |parent| parent.with(Owner::new));
        let control: Ref<Control> = self.to_ref().upcast();
        let origin = self.state.borrow().origin.unwrap_or("ComponentHost");
        let mounted = owner.with(|| MountedChild::mount(&control, view(), origin));
        self.state.borrow_mut().mounted = Some((owner, mounted));
    }

    pub(crate) fn unmount(&self) {
        let mounted = self.state.borrow_mut().mounted.take();
        if let Some((owner, child)) = mounted {
            owner.with(|| child.unmount());
            owner.dispose();
        }
    }
}
