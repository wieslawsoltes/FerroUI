//! Source RibbonItemHelper.
use super::IRibbonItem;
use ferroui_base::data::core::{
    BindingExpression, ValueType, ValueTypes, expression_nodes::TemplatedParentNode,
};
use ferroui_base::data::{BindingOperations, BindingPriority};
use ferroui_base::input::InputElement;
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{
    AttachedProperty, BoxedValue, FerroObject, FerroProperty, Nullable, Ref, WeakRef,
    ferro_properties, ferro_static_type,
};
use ferroui_controls::{Control, IconElement, primitives::TemplatedControl};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct RibbonSimplifiedLabel(i32);
ferroui_base::styled_value!(RibbonSimplifiedLabel);
#[allow(non_upper_case_globals)]
impl RibbonSimplifiedLabel {
    pub const Auto: Self = Self(0);
    pub const Show: Self = Self(1);
    pub const Hide: Self = Self(2);
    pub const fn raw(self) -> i32 {
        self.0
    }
    pub const fn from_raw(value: i32) -> Self {
        Self(value)
    }
}

/// Internal source contract. Implementations must point to the actual native item.
pub(crate) trait IRibbonItemInternal: IRibbonItem {
    fn shortcut(&self) -> Option<ferroribbon_core::RibbonString>;
    fn screen_tip(&self) -> Option<BoxedValue>;
    fn linked_properties(&self) -> Vec<&'static FerroProperty>;
    fn on_unlinked(&self);
    fn owned_values(&self) -> &OwnedItemValues;
}

/// Private per-item storage replaces the source weak table without a global
/// strong-value path back to its weak key or extra property notifications.
#[derive(Default)]
pub(crate) struct OwnedItemValues {
    entries: RefCell<HashMap<u32, BoxedValue>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RibbonItemHelperError {
    pub property: &'static str,
    pub reason: &'static str,
}
impl std::fmt::Display for RibbonItemHelperError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RibbonItemHelper {}: {}", self.property, self.reason)
    }
}
impl std::error::Error for RibbonItemHelperError {}

fn item_text(
    value: Option<ferroribbon_core::RibbonString>,
    property: &'static str,
) -> Result<Option<String>, RibbonItemHelperError> {
    value
        .map(|v| {
            v.to_utf8().map_err(|_| RibbonItemHelperError {
                property,
                reason: "native text cannot represent an unpaired UTF-16 surrogate",
            })
        })
        .transpose()
}

fn owned_equal<T: ferroui_base::PropertyValue>(previous: &T, local: &T) -> bool {
    // Source object.Equals sees one native object even when application code
    // reboxes it through a different derived/base handle type.
    let previous_any = previous as &dyn std::any::Any;
    let local_any = local as &dyn std::any::Any;
    if let (Some(previous), Some(local)) = (
        previous_any.downcast_ref::<BoxedValue>(),
        local_any.downcast_ref::<BoxedValue>(),
    ) && let (Some(previous), Some(local)) = (
        ValueTypes::as_object(&**previous),
        ValueTypes::as_object(&**local),
    ) {
        return previous == local;
    }
    previous == local
}
impl PartialEq for dyn IRibbonItemInternal {
    fn eq(&self, other: &Self) -> bool {
        self.as_object() == other.as_object()
    }
}

static ITEMS_VERSION: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);
pub struct RibbonItemHelper;
ferro_static_type!(RibbonItemHelper);
ferro_properties! {impl RibbonItemHelper {
    pub fn owner_property()->AttachedProperty<Option<ferroui_base::ElementRef<FerroObject>>> {
        ValueTypes::register_element_ref::<FerroObject>();
        FerroProperty::register_attached::<RibbonItemHelper,FerroObject,_>("Owner",None)
    }
    pub fn source_item_property()->AttachedProperty<Option<Ref<Control>>> {
        FerroProperty::register_attached::<RibbonItemHelper,FerroObject,_>("SourceItem",None)
    }
}}
struct LinkState {
    source: WeakRef<FerroObject>,
    target: WeakRef<FerroObject>,
    syncing: Cell<bool>,
    source_registered: Cell<bool>,
    target_registered: Cell<bool>,
    source_token: Cell<u64>,
    target_token: Cell<u64>,
    suppressed_source_change: Cell<Option<Rc<dyn IDisposable>>>,
}
struct ResetSync<'a>(&'a Cell<bool>);
impl Drop for ResetSync<'_> {
    fn drop(&mut self) {
        self.0.set(false)
    }
}
impl LinkState {
    fn copy(
        &self,
        owner: &FerroObject,
        to_target: bool,
        read: &'static FerroProperty,
        write: &'static FerroProperty,
    ) {
        if self.syncing.get() {
            return;
        }
        let other = if to_target {
            self.target.upgrade()
        } else {
            self.source.upgrade()
        };
        let registered = if to_target {
            &self.source_registered
        } else {
            &self.target_registered
        };
        let token = if to_target {
            self.source_token.get()
        } else {
            self.target_token.get()
        };
        let Some(other) = other else {
            if registered.replace(false) {
                if to_target {
                    self.disconnect_suppressed_source_change();
                }
                defer_unregister(owner.to_ref(), token)
            }
            return;
        };
        self.syncing.set(true);
        let _reset = ResetSync(&self.syncing);
        other.set_value_untyped(
            write,
            &*owner.get_value_untyped(read),
            BindingPriority::LocalValue,
        );
    }
    fn unregister(&self) {
        self.disconnect_suppressed_source_change();
        if self.source_registered.get()
            && let Some(source) = self.source.upgrade()
        {
            self.source_registered.set(false);
            source.remove_property_changed_handler(self.source_token.get());
        }
        if self.target_registered.get()
            && let Some(target) = self.target.upgrade()
        {
            self.target_registered.set(false);
            target.remove_property_changed_handler(self.target_token.get());
        }
    }

    fn disconnect_suppressed_source_change(&self) {
        if let Some(subscription) = self.suppressed_source_change.take() {
            subscription.dispose();
        }
    }
}
struct Registrations {
    target: WeakRef<FerroObject>,
    links: Vec<Rc<LinkState>>,
}
#[derive(Default)]
struct Registry {
    targets: HashMap<usize, Registrations>,
    operations: usize,
}
ferroui_base::ferro_thread_local! {static LINKS:RefCell<Registry>=RefCell::new(Registry::default());}
fn key(object: &FerroObject) -> usize {
    object as *const FerroObject as usize
}
fn defer_unregister(owner: Ref<FerroObject>, token: u64) {
    let queued_owner = owner.clone();
    let operation = Dispatcher::ui_thread().invoke_async_local_with_priority(
        move || {
            queued_owner.remove_property_changed_handler(token);
        },
        DispatcherPriority::NORMAL,
    );
    if operation.status() == ferroui_base::threading::DispatcherOperationStatus::Aborted {
        owner.remove_property_changed_handler(token);
    }
}

fn internal(item: &FerroObject) -> Option<Rc<dyn IRibbonItemInternal>> {
    let value: BoxedValue = Rc::new(item.to_ref());
    ValueTypes::try_convert(Some(&value), ValueType::of::<Rc<dyn IRibbonItemInternal>>())
        .flatten()
        .and_then(|v| v.downcast_ref::<Rc<dyn IRibbonItemInternal>>().cloned())
}
impl RibbonItemHelper {
    pub(crate) fn items_version() -> i32 {
        ITEMS_VERSION.load(std::sync::atomic::Ordering::Relaxed)
    }
    pub(crate) fn increment_items_version() {
        // Source static Int32 ++ is a shared read/write, not an interlocked
        // increment. Keep wrapping and possible concurrent lost updates.
        let value = Self::items_version();
        ITEMS_VERSION.store(value.wrapping_add(1), std::sync::atomic::Ordering::Relaxed);
    }
    pub fn get_source_item(element: &FerroObject) -> Option<Ref<Control>> {
        element.get_value(Self::source_item_property())
    }
    pub fn set_source_item(element: &FerroObject, value: impl Into<Nullable<Control>>) {
        element.set_value(Self::source_item_property(), value.into().0);
    }
    pub fn link_one_way(
        source: &FerroObject,
        target: &FerroObject,
        property: &'static FerroProperty,
    ) {
        Self::link(source, target, property, false);
    }
    pub fn link(
        source: &FerroObject,
        target: &FerroObject,
        property: &'static FerroProperty,
        two_way: bool,
    ) {
        Self::link_property(source, target, property, two_way, property);
    }
    fn link_property(
        source: &FerroObject,
        target: &FerroObject,
        property: &'static FerroProperty,
        two_way: bool,
        source_value: &'static FerroProperty,
    ) {
        // Initial write precedes BOTH subscription installation and registry creation.
        target.set_value_untyped(
            property,
            &*source.get_value_untyped(source_value),
            BindingPriority::LocalValue,
        );
        let state = Rc::new(LinkState {
            source: source.to_ref().downgrade(),
            target: target.to_ref().downgrade(),
            syncing: Cell::new(false),
            source_registered: Cell::new(true),
            target_registered: Cell::new(two_way),
            source_token: Cell::new(0),
            target_token: Cell::new(0),
            suppressed_source_change: Cell::new(None),
        });
        let callback = state.clone();
        state
            .source_token
            .set(source.add_property_changed_handler(Rc::new(move |e| {
                if (e.property() == property || e.property() == source_value)
                    && e.is_effective_value_change()
                {
                    callback.copy(e.sender(), true, source_value, property)
                }
            })));
        if property != source_value {
            let callback = state.clone();
            let subscription = source.suppressed_local_value_changed(property, move |owner| {
                callback.copy(owner, true, source_value, property)
            });
            state.suppressed_source_change.set(Some(subscription));
        }
        if two_way {
            let callback = state.clone();
            state
                .target_token
                .set(target.add_property_changed_handler(Rc::new(move |e| {
                    if e.property() == property && e.is_effective_value_change() {
                        callback.copy(e.sender(), false, property, property)
                    }
                })));
        }
        LINKS.with_borrow_mut(|registry| {
            registry.operations = registry.operations.wrapping_add(1);
            if registry.operations % 128 == 0 {
                registry
                    .targets
                    .retain(|_, row| row.target.upgrade().is_some())
            }
            let row = registry
                .targets
                .entry(key(target))
                .or_insert_with(|| Registrations {
                    target: target.to_ref().downgrade(),
                    links: Vec::new(),
                });
            // A dead weak key may have had its address reused. Dropping its registration data is not Unlink.
            if row.target.upgrade().is_none() {
                *row = Registrations {
                    target: target.to_ref().downgrade(),
                    links: Vec::new(),
                };
            }
            row.links.push(state);
        });
    }
    pub fn unlink(target: &FerroObject) {
        let links = LINKS.with_borrow_mut(|registry| registry.targets.remove(&key(target)));
        if let Some(links) = links {
            for link in links.links {
                link.unregister()
            }
        }
        Self::remove_subscription_before_dispose(target);
        if let Some(copy) = internal(target)
            && Self::get_source_item(target).is_some()
        {
            copy.on_unlinked()
        }
    }
    pub fn link_common(source: &Control, target: &Control) {
        Self::set_source_item(target, source.to_ref().cast::<Control>().unwrap());
        // WinUI reads effective IsEnabled and notifies the link for both
        // inherited changes and local writes while the parent is disabled.
        Self::link_property(
            source,
            target,
            InputElement::is_enabled_property().as_property(),
            false,
            InputElement::is_effectively_enabled_property().as_property(),
        );
        Self::link(
            source,
            target,
            ferroui_base::Visual::is_visible_property().as_property(),
            false,
        );
        if let (Some(source_item), Some(_target_item)) = (internal(source), internal(target))
            && source.get_type() == target.get_type()
        {
            for property in source_item.linked_properties() {
                Self::link(source, target, property, false)
            }
        }
        target.set_data_context(source.data_context());
    }
}
ferroui_base::ferro_markup_type!(interface dyn IRibbonItemInternal as "IRibbonItemInternal" {
    namespace:"FerroRibbon.Controls",
    handles:[Rc<dyn IRibbonItemInternal>,Option<Rc<dyn IRibbonItemInternal>>],
    this:Rc<dyn IRibbonItemInternal>,
    properties:[Shortcut:Option<ferroribbon_core::RibbonString> {get:|this:&Rc<dyn IRibbonItemInternal>|this.shortcut()},ScreenTip:Option<BoxedValue> {get:|this:&Rc<dyn IRibbonItemInternal>|this.screen_tip()}],
});
ferroui_base::ferro_markup_type!(static RibbonItemHelper {
    type_info:RibbonItemHelper,
    namespace:"FerroRibbon.Controls",
    static_properties:[MenuIconConverter:Option<ferroui_base::metadata::MarkupDelegate>{get:RibbonItemHelper::menu_icon_converter,set:RibbonItemHelper::set_menu_icon_converter}],
    methods:[
        static try fn CreateMenuIcon(Option<BoxedValue>)->Option<Ref<ferroui_controls::IconElement>> => RibbonItemHelper::create_menu_icon,
        static try fn CreateMenuItem(Ref<Control>,Option<String>,Option<BoxedValue>,Option<ferroui_base::metadata::MarkupDelegate>)->Ref<Control> => |item:Ref<Control>,label,icon,action|RibbonItemHelper::create_menu_item(&item,label,icon,action,None),
        static try fn CreateMenuItem(Ref<Control>,Option<String>,Option<BoxedValue>,Option<ferroui_base::metadata::MarkupDelegate>,Option<bool>)->Ref<Control> => |item:Ref<Control>,label,icon,action,checked|RibbonItemHelper::create_menu_item(&item,label,icon,action,checked),
        static fn GetOwner(Ref<FerroObject>)->Option<Rc<dyn super::IRibbonItemOwner>> => |element:Ref<FerroObject>|RibbonItemHelper::get_owner(&element),
        static try fn SetOwner(Ref<FerroObject>,Option<Rc<dyn super::IRibbonItemOwner>>) => |element:Ref<FerroObject>,owner:Option<Rc<dyn super::IRibbonItemOwner>>|RibbonItemHelper::set_owner(&element,owner),
        static fn FindOwner(Option<Ref<FerroObject>>)->Option<Rc<dyn super::IRibbonItemOwner>> => |element:Option<Ref<FerroObject>>|RibbonItemHelper::find_owner(element.as_deref()),
        static try fn ResolveCommand(Ref<FerroObject>) => |element:Ref<FerroObject>|RibbonItemHelper::resolve_command(&element),
        static fn NotifyInvoked(Ref<Control>) => |element:Ref<Control>|RibbonItemHelper::notify_invoked(&element,None),
        static fn NotifyInvoked(Ref<Control>,Option<BoxedValue>) => |element:Ref<Control>,parameter:Option<BoxedValue>|RibbonItemHelper::notify_invoked(&element,parameter),
        static try fn UpdateToolTip(Option<Ref<Control>>) => |item:Option<Ref<Control>>|RibbonItemHelper::update_tool_tip(item),
        static try fn UpdateAutomation(Option<Ref<Control>>) => |item:Option<Ref<Control>>|RibbonItemHelper::update_automation(item),
        static fn InvalidateHostLayout(Ref<Control>) => |item:Ref<Control>|RibbonItemHelper::invalidate_host_layout(&item),
        static fn Execute(Rc<dyn IRibbonCommandSource>,Option<BoxedValue>)->bool => |source:Rc<dyn IRibbonCommandSource>,parameter:Option<BoxedValue>|RibbonItemHelper::execute(source.as_ref(),parameter.as_ref()),
        static fn GetSourceItem(Ref<FerroObject>)->Option<Ref<Control>> => |element:Ref<FerroObject>| RibbonItemHelper::get_source_item(&element),
        static fn SetSourceItem(Ref<FerroObject>,Option<Ref<Control>>) => |element:Ref<FerroObject>,source:Option<Ref<Control>>| RibbonItemHelper::set_source_item(&element,source),
        static fn Link(Ref<FerroObject>,Ref<FerroObject>,&'static FerroProperty) => |source:Ref<FerroObject>,target:Ref<FerroObject>,property:&'static FerroProperty| RibbonItemHelper::link_one_way(&source,&target,property),
        static fn Link(Ref<FerroObject>,Ref<FerroObject>,&'static FerroProperty,bool) => |source:Ref<FerroObject>,target:Ref<FerroObject>,property:&'static FerroProperty,two_way:bool| RibbonItemHelper::link(&source,&target,property,two_way),
        static fn Unlink(Ref<FerroObject>) => |target:Ref<FerroObject>|RibbonItemHelper::unlink(&target),
        static fn UnlinkTree(Option<Ref<FerroObject>>) => |root:Option<Ref<FerroObject>>|RibbonItemHelper::unlink_tree(root),
        static fn LinkCommon(Ref<Control>,Ref<Control>) => |source:Ref<Control>,target:Ref<Control>|RibbonItemHelper::link_common(&source,&target),
    ],
});
impl RibbonItemHelper {
    pub fn unlink_tree(root: Option<Ref<FerroObject>>) {
        let Some(root) = root else { return };
        if let Some(container) = root.cast::<super::RibbonItemsContainer>() {
            for child in container.items().iter() {
                Self::unlink_tree(child.map(|child| child.upcast()));
            }
        } else if let Some(panel) = root.cast::<ferroui_controls::Panel>() {
            let children = panel.children();
            let snapshot = children.snapshot();
            for child in snapshot.iter() {
                assert!(
                    Rc::ptr_eq(&snapshot, &children.snapshot()),
                    "UnlinkTree children changed during enumeration"
                );
                Self::unlink_tree(Some(child.clone().upcast()));
            }
            assert!(
                Rc::ptr_eq(&snapshot, &children.snapshot()),
                "UnlinkTree children changed during enumeration"
            );
        } else if let Some(viewer) = root.cast::<ferroui_controls::ScrollViewer>() {
            Self::unlink_tree(viewer.content().and_then(|v| ValueTypes::as_object(&*v)));
        } else if let Some(border) = root.cast::<ferroui_controls::Border>() {
            Self::unlink_tree(border.child().map(|child| child.upcast()));
        }
        Self::unlink(&root);
    }
}
pub(crate) fn register_contract_values() {
    ValueTypes::register_nullable::<Rc<dyn super::IRibbonCheckable>>();
    ValueTypes::register_interface_object::<Rc<dyn super::IRibbonCheckable>>(|value| {
        Some(value.as_object())
    });
    ValueTypes::register_nullable::<Rc<dyn super::IRibbonItemOwner>>();
    ValueTypes::register_interface_object::<Rc<dyn super::IRibbonItemOwner>>(|value| {
        Some(value.as_object())
    });
    ValueTypes::register_nullable::<Rc<dyn IRibbonItem>>();
    ValueTypes::register_interface_object::<Rc<dyn IRibbonItem>>(|value| Some(value.as_object()));
    ValueTypes::register_nullable::<Rc<dyn IRibbonItemInternal>>();
    ValueTypes::register_interface_object::<Rc<dyn IRibbonCommandSource>>(|value| {
        Some(value.as_object())
    });
    ValueTypes::register_nullable::<Rc<dyn IRibbonLayoutHost>>();
    ValueTypes::register_interface_object::<Rc<dyn IRibbonLayoutHost>>(|value| {
        Some(value.as_object())
    });
    ValueTypes::register_interface_object::<Rc<dyn IRibbonItemInternal>>(|value| {
        Some(value.as_object())
    });
}

impl RibbonItemHelper {
    pub(super) fn is_enabled(item: &Control) -> bool {
        // FerroUI also derives elements such as TextBlock and Panel from Control.
        if (item.is::<TemplatedControl>() && !item.is::<IconElement>()) || internal(item).is_some()
        {
            item.is_effectively_enabled()
        } else {
            true
        }
    }

    fn set_owned_value<T: ferroui_base::StyledValue>(
        item: &Control,
        state: &OwnedItemValues,
        property: &'static ferroui_base::StyledProperty<Option<T>>,
        value: Option<T>,
    ) -> Result<(), RibbonItemHelperError> {
        let previous = state.entries.borrow().get(&property.id()).cloned();
        let local = item.get_local_value(property);
        let owned = match local.as_ref() {
            None | Some(None) => true,
            Some(Some(local)) => previous
                .as_ref()
                .and_then(|v| v.downcast_ref::<T>())
                .is_some_and(|previous| owned_equal(previous, local)),
        };
        if !owned {
            return Ok(());
        }
        if let Some(value) = value {
            let old = state
                .entries
                .borrow_mut()
                .insert(property.id(), Rc::new(value.clone()));
            drop(old);
            item.set_value(property, Some(value));
        } else if previous.is_some() {
            let old = state.entries.borrow_mut().remove(&property.id());
            drop(old);
            // Inspect only the local binding; an animation can hide it. Native
            // compiled and parsed bindings both expose their actual source node.
            let template_parent =
                BindingOperations::get_local_binding_expression_base(item, property.as_property())
                    .is_some_and(|expression| {
                        expression
                            .as_any()
                            .downcast_ref::<BindingExpression>()
                            .and_then(|binding| binding.nodes().first())
                            .is_some_and(|node| {
                                (node.as_ref() as &dyn std::any::Any).is::<TemplatedParentNode>()
                            })
                    });
            if template_parent
                || item.is_local_binding_expression_initializing(property.as_property())
            {
                item.clear_local_value(property);
            } else {
                item.clear_local_value_and_binding(property);
            }
        }
        Ok(())
    }

    pub fn update_tool_tip(
        item: impl Into<Nullable<Control>>,
    ) -> Result<(), RibbonItemHelperError> {
        let Some(object) = item.into().0 else {
            return Ok(());
        };
        let item: &Control = &object;
        use ferroui_controls::{ToolTip, automation::AutomationProperties};
        let Some(ribbon) = internal(item) else {
            if item.downcast_ref::<super::RibbonControlBase>().is_some() {
                return Err(RibbonItemHelperError {
                    property: "IRibbonItemInternal",
                    reason: "native common item interface registration is unavailable",
                });
            }
            return Ok(());
        };
        let is_enabled = item.is_effectively_enabled();
        let content = super::RibbonScreenTipService::create_with_enabled(
            item_text(ribbon.label(), "Label")?,
            ribbon.screen_tip(),
            item_text(ribbon.shortcut(), "Shortcut")?,
            is_enabled,
        )
        .map_err(|e| RibbonItemHelperError {
            property: e.property,
            reason: "ScreenTip text is not representable by native text",
        })?;
        let tip = super::RibbonScreenTipService::create_tool_tip(content);
        let state = ribbon.owned_values();
        Self::set_owned_value(
            item,
            state,
            ToolTip::tip_property(),
            tip.map(Control::boxed),
        )?;
        let name = if ribbon.label().is_some_and(|v| !v.is_empty()) {
            let label = ribbon.label().ok_or(RibbonItemHelperError {
                property: "Label",
                reason: "label became null between the source emptiness check and Replace",
            })?;
            Some(item_text(Some(label), "Label")?.unwrap().replace('\n', " "))
        } else {
            None
        };
        Self::set_owned_value(item, state, AutomationProperties::name_property(), name)?;
        let shortcut = if ribbon.shortcut().is_some_and(|v| !v.is_empty()) {
            item_text(ribbon.shortcut(), "Shortcut")?
        } else {
            None
        };
        Self::set_owned_value(
            item,
            state,
            AutomationProperties::accelerator_key_property(),
            shortcut,
        )?;
        let key_tip = if ribbon.key_tip().is_some_and(|v| !v.is_empty()) {
            item_text(ribbon.key_tip(), "KeyTip")?
        } else {
            None
        };
        Self::set_owned_value(
            item,
            state,
            AutomationProperties::access_key_property(),
            key_tip,
        )
    }

    pub fn update_automation(
        item: impl Into<Nullable<Control>>,
    ) -> Result<(), RibbonItemHelperError> {
        let Some(object) = item.into().0 else {
            return Ok(());
        };
        let item: &Control = &object;
        use ferroui_controls::automation::AutomationProperties;
        let boxed: BoxedValue = Rc::new(item.to_ref());
        let public = ValueTypes::try_convert(Some(&boxed), ValueType::of::<Rc<dyn IRibbonItem>>())
            .flatten()
            .and_then(|v| v.downcast_ref::<Rc<dyn IRibbonItem>>().cloned());
        let ribbon = public.or_else(|| internal(item).map(|v| -> Rc<dyn IRibbonItem> { v }));
        if ribbon.is_none() && item.downcast_ref::<super::RibbonControlBase>().is_some() {
            return Err(RibbonItemHelperError {
                property: "IRibbonItem",
                reason: "native common item interface registration is unavailable",
            });
        }
        if let Some(ribbon) = ribbon
            && let Some(id) = ribbon.id()
            && !id.is_empty()
            && item
                .get_local_value(AutomationProperties::automation_id_property())
                .is_none()
        {
            item.set_value(
                AutomationProperties::automation_id_property(),
                item_text(Some(id), "Id")?,
            );
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "../item_owned_tests.rs"]
mod owned_tests;

#[cfg(test)]
#[path = "../owned_binding_runtime_tests.rs"]
mod owned_binding_runtime_tests;

/// Host interface used by the original layout-cache invalidation walk.
pub trait IRibbonLayoutHost {
    fn as_object(&self) -> Ref<FerroObject>;
    fn invalidate_items_layout(&self);
}
impl PartialEq for dyn IRibbonLayoutHost {
    fn eq(&self, other: &Self) -> bool {
        self.as_object() == other.as_object()
    }
}
ferroui_base::ferro_markup_type!(interface dyn IRibbonLayoutHost as "IRibbonLayoutHost" {
    namespace:"FerroRibbon.Controls",
    handles:[Rc<dyn IRibbonLayoutHost>,Option<Rc<dyn IRibbonLayoutHost>>],
    this:Rc<dyn IRibbonLayoutHost>,
    methods:[fn InvalidateItemsLayout()=>|this:&Rc<dyn IRibbonLayoutHost>|this.invalidate_items_layout()],
});
fn parent(element: &FerroObject) -> Option<Ref<FerroObject>> {
    element
        .downcast_ref::<ferroui_base::Visual>()
        .and_then(|v| v.visual_parent())
        .map(|p| p.upcast())
        .or_else(|| {
            element
                .downcast_ref::<Control>()
                .and_then(|c| c.parent())
                .map(|p| p.upcast())
        })
}
impl RibbonItemHelper {
    pub fn find_ancestor<T: ferroui_base::PropertyValue>(
        element: Option<&FerroObject>,
    ) -> Option<T> {
        let mut current = element.and_then(parent);
        let mut guard = 0;
        while let Some(item) = current {
            guard += 1;
            if guard > 256 {
                break;
            }
            let value: BoxedValue = Rc::new(item.clone());
            if let Some(value) =
                ValueTypes::try_convert(Some(&value), ValueType::of::<T>()).flatten()
            {
                return value.downcast_ref::<T>().cloned();
            }
            current = parent(&item);
        }
        None
    }
    pub fn invalidate_host_layout(item: &Control) {
        item.invalidate_measure();
        if let Some(host) = Self::find_ancestor::<Rc<dyn IRibbonLayoutHost>>(Some(item)) {
            host.invalidate_items_layout()
        }
    }
    pub fn get_id(
        item: &Control,
        id: Option<ferroribbon_core::RibbonString>,
        command_id: Option<ferroribbon_core::RibbonString>,
    ) -> Option<ferroribbon_core::RibbonString> {
        id.filter(|id| !id.is_empty())
            .or_else(|| item.name().filter(|name| !name.is_empty()).map(Into::into))
            .or_else(|| command_id.filter(|id| !id.is_empty()))
    }
}

pub trait IRibbonCommandSource {
    fn as_object(&self) -> Ref<FerroObject>;
    fn command(&self) -> Option<Rc<dyn ferroui_base::input::ICommand>>;
    fn set_command(&self, value: Option<Rc<dyn ferroui_base::input::ICommand>>);
    fn command_parameter(&self) -> Option<BoxedValue>;
    fn set_command_parameter(&self, value: Option<BoxedValue>);
}
impl PartialEq for dyn IRibbonCommandSource {
    fn eq(&self, other: &Self) -> bool {
        self.as_object() == other.as_object()
    }
}
ferroui_base::ferro_markup_type!(interface dyn IRibbonCommandSource as "IRibbonCommandSource" {
 namespace:"FerroRibbon.Controls",handles:[Rc<dyn IRibbonCommandSource>,Option<Rc<dyn IRibbonCommandSource>>],this:Rc<dyn IRibbonCommandSource>,
 properties:[Command:Option<Rc<dyn ferroui_base::input::ICommand>>{get:|this:&Rc<dyn IRibbonCommandSource>|this.command(),set:|this:&Rc<dyn IRibbonCommandSource>,value|this.set_command(value)},CommandParameter:Option<BoxedValue>{get:|this:&Rc<dyn IRibbonCommandSource>|this.command_parameter(),set:|this:&Rc<dyn IRibbonCommandSource>,value|this.set_command_parameter(value)}],
});
impl RibbonItemHelper {
    pub fn execute(source: &dyn IRibbonCommandSource, parameter: Option<&BoxedValue>) -> bool {
        let Some(command) = source.command() else {
            return false;
        };
        if !command.can_execute(parameter) {
            return false;
        }
        command.execute(parameter);
        true
    }
}

#[path = "ribbon_item_helper_owner.rs"]
mod owner;
