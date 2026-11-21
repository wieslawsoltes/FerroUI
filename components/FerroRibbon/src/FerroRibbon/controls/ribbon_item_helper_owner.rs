//! Original owner walk, catalog synchronization and invocation, separate from links.
//! OwnerRibbon-dependent fallback/context menus await the actual native Ribbon.
use super::*;
use crate::controls::{IRibbonCommandSource, IRibbonItemOwner};
use ferroribbon_core::commands::{ICommand as CoreCommand, RibbonCommandDescriptor};
use ferroui_system::component_model::INotifyPropertyChanged;
use std::rc::Weak;
fn cast<T: ferroui_base::PropertyValue>(item: &FerroObject) -> Option<T> {
    let value: BoxedValue = Rc::new(item.to_ref());
    ValueTypes::try_convert(Some(&value), ValueType::of::<T>())
        .flatten()
        .and_then(|value| value.downcast_ref::<T>().cloned())
}
/// Private native object-owned catalog state. Public item implementations do not
/// need the library's private IRibbonItemInternal contract. The subscription
/// points back weakly; dropping this slot releases its event registration.
#[derive(Default)]
struct CatalogCommandState {
    applied_command: RefCell<Option<Weak<dyn CoreCommand>>>,
    subscription: RefCell<Option<Rc<CommandSubscription>>>,
}
fn catalog_state(item: &FerroObject) -> Rc<CatalogCommandState> {
    if let Some(state) = item.instance_data::<CatalogCommandState>() {
        return state;
    }
    let state = Rc::new(CatalogCommandState::default());
    item.set_instance_data(Some(state.clone()));
    state
}
struct CommandSubscription {
    item: WeakRef<FerroObject>,
    descriptor: Rc<RibbonCommandDescriptor>,
    token: Cell<Option<u64>>,
}
impl CommandSubscription {
    fn new(item: &FerroObject, descriptor: Rc<RibbonCommandDescriptor>) -> Rc<Self> {
        let subscription = Rc::new(Self {
            item: item.to_ref().downgrade(),
            descriptor,
            token: Cell::new(None),
        });
        let weak = Rc::downgrade(&subscription);
        let token = subscription
            .descriptor
            .property_changed()
            .add(Rc::new(move |_| {
                if let Some(subscription) = weak.upgrade() {
                    subscription
                        .apply()
                        .unwrap_or_else(|error| panic!("{error}"));
                }
            }));
        subscription.token.set(Some(token));
        subscription
    }
    fn apply(&self) -> Result<(), RibbonItemHelperError> {
        let Some(item) = self.item.upgrade() else {
            self.dispose();
            return Ok(());
        };
        if let Some(control) = item.downcast_ref::<Control>() {
            control.set_is_enabled(self.descriptor.is_enabled());
        }
        if let Some(is_checked) = self.descriptor.is_checked()
            && let Some(checkable) = cast::<Rc<dyn crate::controls::IRibbonCheckable>>(&item)
        {
            checkable.set_is_checked(Some(is_checked));
        }
        if let Some(source) = cast::<Rc<dyn IRibbonCommandSource>>(&item)
            && let Some(command) = self.descriptor.command()
        {
            RibbonItemHelper::apply_catalog_command(&item, source.as_ref(), command)?;
        }
        Ok(())
    }
    fn dispose(&self) {
        if let Some(token) = self.token.take() {
            self.descriptor.property_changed().remove(token);
        }
    }
}
impl Drop for CommandSubscription {
    fn drop(&mut self) {
        self.dispose();
    }
}
impl RibbonItemHelper {
    pub fn get_owner(element: &FerroObject) -> Option<Rc<dyn IRibbonItemOwner>> {
        let reference = element.get_value(Self::owner_property());
        let object = ferroui_base::ElementRef::resolve(&reference)?;
        Some(
            cast::<Rc<dyn IRibbonItemOwner>>(&object)
                .expect("Owner element no longer declares IRibbonItemOwner in native metadata"),
        )
    }
    pub fn set_owner(
        element: &FerroObject,
        value: Option<Rc<dyn IRibbonItemOwner>>,
    ) -> Result<(), RibbonItemHelperError> {
        let object = value.as_ref().map(|owner| owner.as_object());
        if let Some(object) = &object {
            let interface=cast::<Rc<dyn IRibbonItemOwner>>(object).ok_or(RibbonItemHelperError{property:"Owner",reason:"owner object must declare its native IRibbonItemOwner interface for weak element dispatch"})?;
            if interface.as_object() != *object {
                return Err(RibbonItemHelperError {
                    property: "Owner",
                    reason: "native owner interface cast changed object identity",
                });
            }
        }
        element.set_value(
            Self::owner_property(),
            ferroui_base::ElementRef::from_nullable(object),
        );
        Ok(())
    }
    pub fn find_owner(element: Option<&FerroObject>) -> Option<Rc<dyn IRibbonItemOwner>> {
        let original = element.map(FerroObject::to_ref);
        let mut current = original.clone();
        let mut guard = 0;
        while let Some(item) = current {
            guard += 1;
            if guard > 256 {
                break;
            }
            if let Some(owner) = Self::get_owner(&item) {
                return Some(owner);
            }
            if original.as_ref() != Some(&item)
                && let Some(owner) = cast::<Rc<dyn IRibbonItemOwner>>(&item)
            {
                return Some(owner);
            }
            current = parent(&item);
        }
        None
    }
    pub(super) fn remove_subscription_before_dispose(item: &FerroObject) {
        if let Some(state) = item.instance_data::<CatalogCommandState>() {
            // Source Unlink removes the entry before disposing it.
            let subscription = state.subscription.replace(None);
            if let Some(subscription) = subscription {
                subscription.dispose();
            }
        }
    }
    pub(super) fn release_subscription(item: &FerroObject) {
        if let Some(state) = item.instance_data::<CatalogCommandState>() {
            // Resolve/Unload dispose first, then remove, without borrowing state
            // across event access or destruction.
            let subscription = state.subscription.borrow().clone();
            if let Some(subscription) = subscription {
                subscription.dispose();
            }
            let old = state.subscription.replace(None);
            drop(old);
        }
    }
    fn release_applied_command(item: &FerroObject, source: &dyn IRibbonCommandSource) {
        if let Some(state) = item.instance_data::<CatalogCommandState>() {
            let applied = state.applied_command.replace(None);
            if let Some(applied) = applied {
                // Source removes the marker, then reads Command even when an
                // obsolete native marker can no longer upgrade its weak target.
                let current = source.command();
                if let Some(applied) = applied.upgrade()
                    && current
                        .as_ref()
                        .and_then(crate::commands::source_core_command)
                        .is_some_and(|current| Rc::ptr_eq(&current, &applied))
                {
                    source.set_command(None)
                }
            }
        }
    }
    fn apply_catalog_command(
        item: &FerroObject,
        source: &dyn IRibbonCommandSource,
        command: Rc<dyn CoreCommand>,
    ) -> Result<(), RibbonItemHelperError> {
        let current = source.command();
        let current_core = current
            .as_ref()
            .and_then(crate::commands::source_core_command);
        let state = catalog_state(item);
        if current_core
            .as_ref()
            .is_some_and(|current| Rc::ptr_eq(current, &command))
        {
            state.applied_command.replace(Some(Rc::downgrade(&command)));
            return Ok(());
        }
        let replace = current.is_none() || {
            let applied = state
                .applied_command
                .borrow()
                .as_ref()
                .and_then(Weak::upgrade);
            current_core
                .as_ref()
                .zip(applied.as_ref())
                .is_some_and(|(current, applied)| Rc::ptr_eq(current, applied))
        };
        if replace {
            source.set_command(Some(crate::commands::adapt_core_command(command.clone())));
            // Track original Core identity without owning an obsolete command:
            // a command closure may strongly capture this control. Actual Command
            // properties and descriptors own commands; this marker never does.
            let old = state.applied_command.replace(Some(Rc::downgrade(&command)));
            drop(old);
        }
        Ok(())
    }
    pub fn resolve_command(item: &FerroObject) -> Result<(), RibbonItemHelperError> {
        let Some(source) = cast::<Rc<dyn IRibbonCommandSource>>(item) else {
            return Ok(());
        };
        Self::release_subscription(item);
        let command_id = cast::<Rc<dyn IRibbonItem>>(item).and_then(|item| item.command_id());
        let has_id = command_id.as_ref().is_some_and(|id| !id.is_empty());
        let owner = if has_id {
            Self::find_owner(Some(item))
        } else {
            None
        };
        let descriptor = owner
            .as_ref()
            .and_then(|owner| owner.command_catalog())
            .and_then(|catalog| catalog.find(command_id.as_ref()));
        if descriptor
            .as_ref()
            .is_none_or(|descriptor| descriptor.command().is_none())
        {
            Self::release_applied_command(item, source.as_ref());
        }
        if !has_id {
            return Ok(());
        }
        let Some(descriptor) = descriptor else {
            if let Some(ribbon) = owner.and_then(|owner| owner.owner_ribbon()) {
                ribbon.apply_command_state(
                    item.downcast_ref::<Control>()
                        .expect("IRibbonItem is not a native Control"),
                );
            }
            return Ok(());
        };
        let state = catalog_state(item);
        let subscription = CommandSubscription::new(item, descriptor);
        let old = state.subscription.replace(Some(subscription.clone()));
        drop(old);
        subscription.apply()
    }
    pub fn notify_invoked(item: &Control, parameter: Option<BoxedValue>) {
        let source = Self::get_source_item(item).unwrap_or_else(|| item.to_ref());
        let command_id = cast::<Rc<dyn IRibbonItem>>(item).and_then(|item| item.command_id());
        let owner = Self::find_owner(Some(item)).or_else(|| Self::find_owner(Some(&source)));
        let Some(owner) = owner else { return };
        let effective_parameter = parameter.or_else(|| {
            cast::<Rc<dyn IRibbonCommandSource>>(item).and_then(|source| source.command_parameter())
        });
        owner.on_item_invoked(source, command_id.clone(), effective_parameter.clone());
        // Re-read the item's command and owner's catalog AFTER user event code.
        if command_id.is_some()
            && let Some(source) = cast::<Rc<dyn IRibbonCommandSource>>(item)
            && source.command().is_none()
            && let Some(catalog) = owner.command_catalog()
        {
            catalog.execute(
                command_id.as_ref(),
                crate::commands::command_parameter(effective_parameter.as_ref()),
            );
        }
    }
    pub(crate) fn on_item_loaded(item: &Control) -> Result<(), RibbonItemHelperError> {
        Self::increment_items_version();
        Self::resolve_command(item)?;
        Self::update_tool_tip(item.to_ref())?;
        Self::update_automation(item.to_ref())?;
        if let Some(ribbon) = cast::<Rc<dyn IRibbonItem>>(item)
            && let Some(owner) = Self::find_owner(Some(item))
            && ribbon.as_object() != owner.as_object()
        {
            let metrics = owner.metrics();
            let current = if let Some(button) = item.downcast_ref::<crate::controls::RibbonButton>()
            {
                button.metrics()
            } else {
                item.downcast_ref::<crate::controls::RibbonControlBase>()
                    .and_then(|item| item.metrics())
            };
            if internal(item).is_some()
                && current != metrics
                && Self::find_ancestor::<Rc<dyn super::super::IRibbonLayoutHost>>(Some(item))
                    .is_none()
            {
                ribbon.apply_layout(crate::controls::RibbonItemLayout::new(
                    ribbon.current_size(),
                    metrics,
                ));
            }
        }
        Ok(())
    }
    pub(crate) fn on_item_unloaded(item: &Control) {
        Self::increment_items_version();
        Self::release_subscription(item);
    }
}

struct FlyoutRegistration {
    _opening: ferroui_base::reactive::Subscription,
}

impl RibbonItemHelper {
    pub(crate) fn track_flyout_owner(
        old: Option<Ref<ferroui_controls::primitives::FlyoutBase>>,
        new: Option<Ref<ferroui_controls::primitives::FlyoutBase>>,
    ) -> Result<(), RibbonItemHelperError> {
        use ferroui_controls::{Flyout, primitives::PopupFlyoutBase};
        if let Some(old) = old {
            drop(old.set_instance_data::<FlyoutRegistration>(None));
        }
        let Some(new) = new else { return Ok(()) };
        super::super::RibbonMenu::apply_theme(Some(&new));
        drop(new.set_instance_data::<FlyoutRegistration>(None));
        let popup = new.cast::<PopupFlyoutBase>().ok_or(RibbonItemHelperError {
            property: "Flyout",
            reason: "native non-popup FlyoutBase has no source-equivalent Opening event",
        })?;
        let weak = new.downgrade();
        let owner_property = Self::owner_property();
        let subscription = popup.opening(move |_args| {
            let Some(flyout) = weak.upgrade().and_then(|flyout| flyout.cast::<Flyout>()) else {
                return;
            };
            if let Some(content) = flyout
                .content()
                .and_then(|value| ValueTypes::as_object(&*value))
                .and_then(|value| value.cast::<Control>())
                && let Some(target) = flyout.target()
                && content.get_local_value(owner_property).is_none()
                && !content.has_local_value_binding(owner_property.as_property())
                && let Some(owner) = RibbonItemHelper::find_owner(Some(&target))
            {
                RibbonItemHelper::set_owner(&content, Some(owner))
                    .expect("hosted flyout owner metadata");
            }
        });
        // One registration per flyout. The callback and subscription hold weak references.
        new.set_instance_data(Some(Rc::new(FlyoutRegistration {
            _opening: subscription,
        })));
        Ok(())
    }
}

impl RibbonItemHelper {
    /// Source Attach installs RightTapped once during common item construction.
    /// The native gesture carries the same element-relative pointer position.
    pub(crate) fn attach_context_menu(item: &Control) {
        item.add_handler(
            ferroui_base::input::InputElement::right_tapped_event(),
            move |sender, e| {
                if e.handled() {
                    return;
                }
                let item = sender
                    .to_ref()
                    .cast::<Control>()
                    .expect("item context target");
                if let Some(root) =
                    Self::find_owner(Some(&item)).and_then(|owner| owner.owner_ribbon())
                    && root
                        .show_item_context_menu(item.clone(), e.get_position(Some(&item)))
                        .expect("item context menu")
                {
                    e.set_handled(true);
                }
            },
        );
    }
}
