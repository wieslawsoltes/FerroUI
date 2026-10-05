use crate::i_native_menu_exporter_events_impl_bridge::INativeMenuExporterEventsImplBridge;
use crate::{NativeMenuItem, NativeMenuItemBase};
use ferroui_base::collections::{FerroList, NotifyCollectionChangedEventArgs, ResetBehavior};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, DirectProperty, FerroObject,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, IntoRef, Ref, WeakRef,
};
use std::cell::RefCell;
use std::rc::Rc;

/// A menu that is shown by the platform: the menu of a window or of the
/// application, the menu of a tray icon or of the dock.
#[repr(C)]
pub struct NativeMenu {
    base: FerroObject,
    items: FerroList<Ref<NativeMenuItemBase>>,
    /// The item the menu is the submenu of. A back pointer: the item owns
    /// its menu.
    parent: RefCell<Option<WeakRef<NativeMenuItem>>>,
    needs_update: HandlerList<dyn Fn(&NativeMenu)>,
    opening: HandlerList<dyn Fn(&NativeMenu)>,
    closed: HandlerList<dyn Fn(&NativeMenu)>,
}

ferro_class!(NativeMenu: FerroObject);
ferro_class_info!(NativeMenu { new: NativeMenu::new });

impl FerroObjectImpl for NativeMenu {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        let weak = this.to_ref().downgrade();
        this.items.set_validator(Some(Rc::new({
            let weak = weak.clone();
            move |item: &Ref<NativeMenuItemBase>| Self::validate(&weak, item)
        })));
        this.items.add_collection_changed(Rc::new(
            move |e: &NotifyCollectionChangedEventArgs<'_, Ref<NativeMenuItemBase>>| Self::items_changed(&weak, e),
        ));
    }
}

/// The native menu viewed through the interface it implements.
struct NativeMenuHandle(Ref<NativeMenu>);

impl INativeMenuExporterEventsImplBridge for NativeMenuHandle {
    fn raise_needs_update(&self) {
        for (_, handler) in self.0.needs_update.snapshot().iter() {
            handler(&self.0);
        }
    }

    fn raise_opening(&self) {
        for (_, handler) in self.0.opening.snapshot().iter() {
            handler(&self.0);
        }
    }

    fn raise_closed(&self) {
        for (_, handler) in self.0.closed.snapshot().iter() {
            handler(&self.0);
        }
    }
}

ferro_properties! {
    impl NativeMenu, also [NativeMenu::register_export_properties] {
        /// Defines the `Parent` property.
        pub fn parent_property() -> DirectProperty<NativeMenu, Option<Ref<NativeMenuItem>>> {
            FerroProperty::register_direct::<NativeMenu, _>("Parent", |o| o.parent(), None, None)
        }
    }
}

impl NativeMenu {
    fn static_constructor() {
        Self::static_constructor_export();
    }

    /// Creates the class data; see [`FerroObject::construct`].
    pub fn construct() -> Self {
        let items = FerroList::new();
        items.set_reset_behavior(ResetBehavior::Remove);
        Self {
            base: FerroObject::construct(),
            items,
            parent: RefCell::new(None),
            needs_update: HandlerList::new(),
            opening: HandlerList::new(),
            closed: HandlerList::new(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The items of the menu.
    pub fn items(&self) -> FerroList<Ref<NativeMenuItemBase>> {
        self.items.clone()
    }

    /// Raised when the menu requests an update.
    ///
    /// Use this event to add, remove or modify menu items before a menu is
    /// shown or a hotkey is pressed.
    pub fn needs_update(&self, handler: impl Fn(&NativeMenu) + 'static) -> Rc<dyn IDisposable> {
        self.subscribe(|menu| &menu.needs_update, handler)
    }

    /// Raised before the menu is opened.
    ///
    /// Do not update the menu in this event; use
    /// [`needs_update`](Self::needs_update).
    pub fn opening(&self, handler: impl Fn(&NativeMenu) + 'static) -> Rc<dyn IDisposable> {
        self.subscribe(|menu| &menu.opening, handler)
    }

    /// Raised after the menu is closed.
    ///
    /// Do not update the menu in this event; use
    /// [`needs_update`](Self::needs_update).
    pub fn closed(&self, handler: impl Fn(&NativeMenu) + 'static) -> Rc<dyn IDisposable> {
        self.subscribe(|menu| &menu.closed, handler)
    }

    fn subscribe(
        &self,
        event: fn(&NativeMenu) -> &HandlerList<dyn Fn(&NativeMenu)>,
        handler: impl Fn(&NativeMenu) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = event(self).add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                event(&this).remove(token);
            }
        })
    }

    fn validate(this: &WeakRef<NativeMenu>, item: &Ref<NativeMenuItemBase>) {
        if let Some(parent) = item.parent() {
            panic!(
                "The menu item {} already has a parent {} while trying to add it as a child of {}.",
                debug_display(item),
                debug_display(&parent),
                this.upgrade().map_or_else(|| NativeMenu::TYPE.name().to_string(), |this| debug_display(&this)),
            );
        }
    }

    fn items_changed(this: &WeakRef<NativeMenu>, e: &NotifyCollectionChangedEventArgs<'_, Ref<NativeMenuItemBase>>) {
        for i in e.old_items {
            i.set_parent(None);
        }
        for i in e.new_items {
            i.set_parent(this.upgrade());
        }
    }

    /// The item the menu is the submenu of.
    pub fn parent(&self) -> Option<Ref<NativeMenuItem>> {
        let parent = self.parent.borrow().clone();
        parent.and_then(|parent| parent.upgrade())
    }

    pub(crate) fn set_parent(&self, value: Option<Ref<NativeMenuItem>>) {
        let old = self.parent();
        if old == value {
            return;
        }
        *self.parent.borrow_mut() = value.as_ref().map(|value| value.downgrade());
        self.raise_direct_property_changed(Self::parent_property(), &old, &value);
    }

    /// The menu viewed as the contract through which the platform exporter
    /// of the menu raises its events.
    pub fn to_exporter_events_bridge(&self) -> Rc<dyn INativeMenuExporterEventsImplBridge> {
        Rc::new(NativeMenuHandle(self.to_ref()))
    }

    /// Adds an item to the menu.
    pub fn add(&self, item: impl IntoRef<NativeMenuItemBase>) {
        self.items.add(item.into_ref());
    }

    /// Enumerates the items of the menu.
    pub fn iter(&self) -> std::vec::IntoIter<Ref<NativeMenuItemBase>> {
        self.items.to_vec().into_iter()
    }
}

impl IntoIterator for &NativeMenu {
    type Item = Ref<NativeMenuItemBase>;
    type IntoIter = std::vec::IntoIter<Ref<NativeMenuItemBase>>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// The text that describes an object of the native menu classes in
/// diagnostics: the name of its class and, for a menu item, its header.
pub(crate) fn debug_display(object: &FerroObject) -> String {
    let mut builder = String::new();
    match object.downcast_ref::<NativeMenuItem>() {
        Some(item) => item.build_debug_display(&mut builder, true),
        None => builder.push_str(&debug_type_name(object)),
    }
    builder
}

/// The name of the type of an object in a description: the simple name
/// for the types of the framework, the namespace-qualified name for any
/// other type.
pub(crate) fn debug_type_name(object: &FerroObject) -> String {
    let type_ = object.get_type();
    let namespace = type_.namespace();

    if namespace == "FerroUI" || namespace.starts_with("FerroUI.") {
        type_.name().to_string()
    } else {
        type_.full_name()
    }
}

/// Appends ` (name = value)` to a description, or extends the parenthesis
/// the description ends with. Nothing is appended for no value or an empty
/// one, and a long value is cut.
pub(crate) fn append_optional_value(builder: &mut String, name: &str, value: Option<&str>) {
    const MAX_VALUE_LENGTH: usize = 50;

    let Some(value) = value.filter(|value| !value.is_empty()) else { return };

    if builder.ends_with(')') {
        builder.pop();
        builder.push_str(", ");
    } else {
        builder.push_str(" (");
    }

    builder.push_str(name);
    builder.push_str(" = ");

    // The reference counts UTF-16 code units.
    let units: Vec<u16> = value.encode_utf16().collect();
    if units.len() > MAX_VALUE_LENGTH {
        builder.push_str(&String::from_utf16_lossy(&units[..MAX_VALUE_LENGTH - 1]));
        builder.push('…');
    } else {
        builder.push_str(value);
    }

    builder.push(')');
}
