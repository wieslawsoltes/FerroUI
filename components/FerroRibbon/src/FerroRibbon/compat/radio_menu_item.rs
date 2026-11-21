// Copyright (c) Microsoft Corporation, MIT. Pinned Uno HAS_UNO branches retained.
//! The selected platform has two checked property slots and a thread-wide weak
//! selection map, not the native menu handler's per-menu radio group manager.
use super::{MenuFlyoutItemImpl, MenuFlyoutSubItem, ToggleMenuFlyoutItem};
use ferroui_base::{
    AttachedProperty, FerroObject, FerroObjectImpl, FerroObjectImplExt, FerroProperty, Ref,
    StyledElementImpl, StyledProperty, VisualImpl, WeakRef, ferro_class, ferro_class_info,
    ferro_impl_classes, ferro_properties, instantiate,
};
use ferroui_base::{
    input::InputElementImpl, interactivity::InteractiveImpl, layout::LayoutableImpl,
};
use ferroui_controls::{
    ControlImpl, ItemsControlImpl, MenuItem, MenuItemImpl,
    primitives::{
        HeaderedSelectingItemsControlImpl, SelectingItemsControlImpl, TemplatedControlImpl,
    },
};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
};
#[repr(C)]
pub struct RadioMenuFlyoutItem {
    base: ToggleMenuFlyoutItem,
    cached_checked: Cell<bool>,
    cached_group: RefCell<Option<String>>,
    safe_uncheck: Cell<bool>,
    last_registered: RefCell<Option<String>>,
}
ferro_class!(RadioMenuFlyoutItem:ToggleMenuFlyoutItem);
ferro_class_info!(RadioMenuFlyoutItem{new:RadioMenuFlyoutItem::new,markup:{namespace:"FerroRibbon.Compatibility",methods:[
static fn GetAreCheckStatesEnabled(Ref<FerroObject>)->bool => |o:Ref<FerroObject>|RadioMenuFlyoutItem::get_are_check_states_enabled(&o),
static fn SetAreCheckStatesEnabled(Ref<FerroObject>,bool) => |o:Ref<FerroObject>,v|RadioMenuFlyoutItem::set_are_check_states_enabled(&o,v),
]}});
ferro_impl_classes!(RadioMenuFlyoutItem:StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ItemsControlImpl,SelectingItemsControlImpl,HeaderedSelectingItemsControlImpl,MenuItemImpl,MenuFlyoutItemImpl);
ferroui_base::ferro_thread_local! {static SELECTION:RefCell<HashMap<String,WeakRef<RadioMenuFlyoutItem>>>=RefCell::new(HashMap::new());}
ferro_properties! {impl RadioMenuFlyoutItem{
 pub fn is_checked_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("IsChecked",false)}
 pub fn group_name_property()->StyledProperty<Option<String>>{FerroProperty::register::<Self,_>("GroupName",Some(String::new()))}
 pub fn are_check_states_enabled_property()->AttachedProperty<bool>{FerroProperty::register_attached::<Self,FerroObject,_>("AreCheckStatesEnabled",false)}
}}
impl RadioMenuFlyoutItem {
    fn static_constructor() {
        Self::is_checked_property()
            .changed()
            .add_class_handler::<Self>(|this, _| this.on_checked_changed())
            .detach();
        Self::group_name_property()
            .changed()
            .add_class_handler::<Self>(|this, _| {
                this.cached_group.replace(this.group_name());
            })
            .detach();
        Self::are_check_states_enabled_property()
            .changed()
            .add_class_handler::<FerroObject>(|_this, e| {
                if e.get_new_value::<bool>()
                    && let Some(sub) = e.sender().downcast_ref::<MenuFlyoutSubItem>()
                {
                    sub.loaded(|sender, _| {
                        if let Some(sub) = sender.downcast_ref::<MenuFlyoutSubItem>() {
                            let mut checked = false;
                            // Source exhausts the collection even after finding a checked radio.
                            for r in sub
                                .items()
                                .iter()
                                .flatten()
                                .filter_map(|c| c.cast::<RadioMenuFlyoutItem>())
                            {
                                if !checked {
                                    checked = r.is_checked();
                                }
                            }
                            sub.pseudo_classes().set(":checked", checked);
                        }
                    });
                }
            })
            .detach();
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: ToggleMenuFlyoutItem::construct(),
            cached_checked: Cell::new(false),
            cached_group: RefCell::new(Some(String::new())),
            safe_uncheck: Cell::new(false),
            last_registered: RefCell::default(),
        })
    }
    pub fn is_checked(&self) -> bool {
        self.get_value(Self::is_checked_property())
    }
    pub fn set_is_checked(&self, v: bool) {
        self.set_value(Self::is_checked_property(), v);
    }
    pub fn group_name(&self) -> Option<String> {
        self.get_value(Self::group_name_property())
    }
    pub fn set_group_name(&self, v: Option<String>) {
        self.set_value(Self::group_name_property(), v);
    }
    pub fn get_are_check_states_enabled(o: &FerroObject) -> bool {
        o.get_value(Self::are_check_states_enabled_property())
    }
    pub fn set_are_check_states_enabled(o: &FerroObject, v: bool) {
        o.set_value(Self::are_check_states_enabled_property(), v);
    }
    fn on_checked_changed(&self) {
        if MenuItem::is_checked(self) != self.is_checked() {
            self.safe_uncheck.set(true);
            MenuItem::set_is_checked(self, self.is_checked());
            self.safe_uncheck.set(false);
            self.update_group();
        }
        self.cached_checked.set(self.is_checked());
    }
    fn on_internal_checked_changed(&self) {
        if !MenuItem::is_checked(self) {
            if self.safe_uncheck.get() {
                self.set_is_checked(false);
            } else {
                MenuItem::set_is_checked(self, true);
            }
        } else if !self.is_checked() {
            self.set_is_checked(true);
            self.update_group();
        }
    }
    fn update_group(&self) {
        if !self.is_checked() {
            return;
        }
        let group = self.group_name().expect("ArgumentNull: radio GroupName");
        let previous = SELECTION.with_borrow(|map| map.get(&group).and_then(WeakRef::upgrade));
        if let Some(previous) = previous
            && !previous.ptr_eq(&self.to_ref())
        {
            previous.set_is_checked(false);
        }
        let last = self.last_registered.borrow().clone();
        if let Some(last) = last
            && last != group
        {
            SELECTION.with_borrow_mut(|map| {
                if map.get(&last).is_some_and(|r| r.points_to(&self.to_ref())) {
                    map.remove(&last);
                }
            });
        }
        self.last_registered.replace(Some(group.clone()));
        SELECTION.with_borrow_mut(|map| {
            map.insert(group, self.to_ref().downgrade());
        });
    }
    fn on_unloaded(&self) {
        if self.cached_checked.get() {
            let key = self
                .last_registered
                .borrow()
                .clone()
                .or_else(|| self.cached_group.borrow().clone());
            if let Some(key) = key {
                SELECTION.with_borrow_mut(|map| {
                    if map.get(&key).is_some_and(|r| r.points_to(&self.to_ref())) {
                        map.remove(&key);
                    }
                });
            }
        }
    }
}
impl FerroObjectImpl for RadioMenuFlyoutItem {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.pseudo_classes().set(":toggle", false);
        this.pseudo_classes().set(":radio", true);
        this.add_property_changed_handler(std::rc::Rc::new(|e| {
            if e.property() == MenuItem::is_checked_property().as_property()
                && let Some(this) = e.sender().downcast_ref::<Self>()
            {
                this.on_internal_checked_changed();
            }
        }));
        this.loaded(|s, _| {
            if let Some(this) = s.downcast_ref::<Self>() {
                this.update_group();
            }
        });
        this.unloaded(|s, _| {
            if let Some(this) = s.downcast_ref::<Self>() {
                this.on_unloaded();
            }
        });
    }
}
