//! Port of `Diagnostics/IDevToolsTopLevelGroup.cs`.

use ferroui_base::collections::FerroList;
use ferroui_base::Ref;
use ferroui_controls::application_lifetimes::{IApplicationLifetime, IClassicDesktopStyleApplicationLifetime};
use ferroui_controls::TopLevel;
use std::any::Any;
use std::rc::Rc;

/// The top levels DevTools inspects together: the windows of an application, or one top level.
pub trait IDevToolsTopLevelGroup {
    /// The top levels of the group.
    fn items(&self) -> Vec<Ref<TopLevel>>;

    /// `Items is INotifyCollectionChanged`: the observable collection of the top levels when the
    /// group changes, `None` when it does not.
    fn observable_items(&self) -> Option<FerroList<Ref<TopLevel>>> {
        None
    }

    /// `Equals(object)`: whether `other` is the same group. By default a group is only equal to
    /// itself.
    fn equals(&self, other: &dyn IDevToolsTopLevelGroup) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const dyn IDevToolsTopLevelGroup)
    }

    /// The group as its concrete type (`is`/`as`).
    fn as_any(&self) -> &dyn Any;
}

impl PartialEq for dyn IDevToolsTopLevelGroup {
    fn eq(&self, other: &Self) -> bool {
        self.equals(other)
    }
}

/// The windows of an application with a classic desktop lifetime.
pub struct ClassicDesktopStyleApplicationLifetimeTopLevelGroup {
    lifetime: Rc<dyn IApplicationLifetime>,
}

impl ClassicDesktopStyleApplicationLifetimeTopLevelGroup {
    /// The group of the windows of `lifetime`.
    ///
    /// # Panics
    /// When `lifetime` is not an `IClassicDesktopStyleApplicationLifetime` (the original takes
    /// one and throws on null).
    pub fn new(lifetime: Rc<dyn IApplicationLifetime>) -> Self {
        assert!(
            lifetime.as_classic_desktop_style_application_lifetime().is_some(),
            "Value cannot be null. (Parameter 'lifetime')"
        );
        Self { lifetime }
    }

    /// The lifetime of the group.
    pub fn lifetime(&self) -> &dyn IClassicDesktopStyleApplicationLifetime {
        self.lifetime
            .as_classic_desktop_style_application_lifetime()
            .expect("the lifetime of the group is a classic desktop lifetime")
    }
}

impl IDevToolsTopLevelGroup for ClassicDesktopStyleApplicationLifetimeTopLevelGroup {
    fn items(&self) -> Vec<Ref<TopLevel>> {
        self.lifetime().windows().into_iter().map(|window| window.upcast()).collect()
    }

    fn equals(&self, other: &dyn IDevToolsTopLevelGroup) -> bool {
        other
            .as_any()
            .downcast_ref::<Self>()
            .is_some_and(|g| std::ptr::addr_eq(Rc::as_ptr(&g.lifetime), Rc::as_ptr(&self.lifetime)))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// One top level.
pub struct SingleViewTopLevelGroup {
    top_level: Ref<TopLevel>,
}

impl SingleViewTopLevelGroup {
    pub fn new(top_level: Ref<TopLevel>) -> Self {
        Self { top_level }
    }
}

impl IDevToolsTopLevelGroup for SingleViewTopLevelGroup {
    fn items(&self) -> Vec<Ref<TopLevel>> {
        vec![self.top_level.clone()]
    }

    fn equals(&self, other: &dyn IDevToolsTopLevelGroup) -> bool {
        other.as_any().downcast_ref::<Self>().is_some_and(|g| g.top_level == self.top_level)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
