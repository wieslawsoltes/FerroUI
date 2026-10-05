use crate::i_clickable_control::as_clickable_control;
use crate::i_command_source::as_command_source;
use crate::utils::AncestorFinder;
use crate::{Control, TopLevel};
use ferroui_base::input::{ICommand, KeyBinding, KeyGesture};
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::reactive::{Disposable, IDisposable, ObservableExt};
use ferroui_base::{
    ferro_property, AttachedProperty, BoxedValue, FerroObject, FerroProperty, Ref, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// Defines the `HotKey` attached property: a key gesture that clicks a
/// control, or executes its command, wherever the keyboard focus is within
/// the top-level the control is in.
pub struct HotKeyManager;

ferroui_base::ferro_static_type!(HotKeyManager);

/// The command of the key binding registered for a control with a hot key.
struct HotkeyCommandWrapper {
    this: Weak<HotkeyCommandWrapper>,
    reference: WeakRef<Control>,
}

impl HotkeyCommandWrapper {
    fn new(control: &Ref<Control>) -> Rc<Self> {
        Rc::new_cyclic(|this| Self { this: this.clone(), reference: control.downgrade() })
    }

    /// The command a hot key press executes: the command of the control,
    /// or this wrapper when the control is only clickable.
    #[allow(dead_code)]
    fn get_command(&self) -> Option<Rc<dyn ICommand>> {
        if let Some(target) = self.reference.upgrade() {
            if let Some(command) = as_command_source(&target).and_then(|source| source.command()) {
                return Some(command);
            } else if as_clickable_control(&target).is_some() {
                return self.this.upgrade().map(|this| this as Rc<dyn ICommand>);
            }
        }

        None
    }
}

impl ICommand for HotkeyCommandWrapper {
    fn can_execute(&self, _parameter: Option<&BoxedValue>) -> bool {
        if let Some(target) = self.reference.upgrade() {
            if let Some(command_source) = as_command_source(&target) {
                if let Some(command) = command_source.command() {
                    return command_source.is_effectively_enabled()
                        && command.can_execute(command_source.command_parameter().as_ref());
                }
            }

            if let Some(clickable) = as_clickable_control(&target) {
                return clickable.is_effectively_enabled();
            }
        }

        false
    }

    fn execute(&self, _parameter: Option<&BoxedValue>) {
        if let Some(target) = self.reference.upgrade() {
            if let Some(command_source) = as_command_source(&target) {
                if let Some(command) = command_source.command() {
                    command.execute(command_source.command_parameter().as_ref());
                    return;
                }
            }

            if let Some(clickable) = as_clickable_control(&target) {
                if clickable.is_effectively_enabled() {
                    clickable.raise_click();
                }
            }
        }
    }

    fn can_execute_changed(&self, _handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        // The event is never raised.
        Disposable::empty()
    }
}

/// Keeps the key binding of one control with a hot key registered on the
/// top-level the control is in.
struct Manager {
    control: WeakRef<Control>,
    root: RefCell<Option<WeakRef<TopLevel>>>,
    parent_sub: RefCell<Option<Rc<dyn IDisposable>>>,
    hotkey_sub: RefCell<Option<Rc<dyn IDisposable>>>,
    hotkey: Cell<Option<KeyGesture>>,
    wrapper: Rc<HotkeyCommandWrapper>,
    binding: RefCell<Option<Ref<KeyBinding>>>,
}

impl Manager {
    fn new(control: &Ref<Control>) -> Rc<Self> {
        Rc::new(Self {
            control: control.downgrade(),
            root: RefCell::new(None),
            parent_sub: RefCell::new(None),
            hotkey_sub: RefCell::new(None),
            hotkey: Cell::new(None),
            wrapper: HotkeyCommandWrapper::new(control),
            binding: RefCell::new(None),
        })
    }

    fn init(self: &Rc<Self>) {
        let Some(control) = self.control.upgrade() else { return };

        // The control owns the manager through this change handler; every
        // other reference between the two is weak.
        let property = HotKeyManager::hot_key_property().as_property();
        let this = self.clone();
        let hotkey_sub = control.property_changed(move |e| {
            if e.property() == property {
                this.on_hotkey_changed(e.get_new_value::<Option<KeyGesture>>());
            }
        });
        *self.hotkey_sub.borrow_mut() = Some(hotkey_sub);
        self.on_hotkey_changed(control.get_value(HotKeyManager::hot_key_property()));

        let weak = Rc::downgrade(self);
        let parent_sub = AncestorFinder::create::<TopLevel>(&control.clone().upcast()).subscribe_fn(
            move |root: Option<Ref<TopLevel>>| {
                if let Some(this) = weak.upgrade() {
                    this.on_parent_changed(root);
                }
            },
        );
        *self.parent_sub.borrow_mut() = Some(parent_sub);
    }

    fn on_parent_changed(&self, control: Option<Ref<TopLevel>>) {
        self.unregister();
        *self.root.borrow_mut() = control.map(|root| root.downgrade());
        self.register();
    }

    fn on_hotkey_changed(&self, hotkey: Option<KeyGesture>) {
        match hotkey {
            // The subscription will be recreated by the static property
            // watcher.
            None => self.stop(),
            Some(hotkey) => {
                self.unregister();
                self.hotkey.set(Some(hotkey));
                self.register();
            }
        }
    }

    fn root(&self) -> Option<Ref<TopLevel>> {
        self.root.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    fn unregister(&self) {
        let binding = self.binding.borrow_mut().take();
        if let (Some(root), Some(binding)) = (self.root(), binding) {
            root.key_bindings().remove(&binding);
        }
    }

    fn register(&self) {
        if let (Some(root), Some(hotkey)) = (self.root(), self.hotkey.get()) {
            let binding = KeyBinding::new();
            binding.set_gesture(Some(hotkey));
            binding.set_command(Some(self.wrapper.clone()));
            *self.binding.borrow_mut() = Some(binding.clone());
            root.key_bindings().add(binding);
        }
    }

    fn stop(&self) {
        self.unregister();
        let parent_sub = self.parent_sub.borrow_mut().take();
        if let Some(parent_sub) = parent_sub {
            parent_sub.dispose();
        }
        let hotkey_sub = self.hotkey_sub.borrow_mut().take();
        if let Some(hotkey_sub) = hotkey_sub {
            hotkey_sub.dispose();
        }
    }
}

/// A manager that goes away with its control takes its key binding with it.
impl Drop for Manager {
    fn drop(&mut self) {
        self.stop();
    }
}

ferroui_base::ferro_properties! { impl HotKeyManager {
    ferro_property!(
        /// Defines the `HotKey` attached property.
        pub fn hot_key_property() -> AttachedProperty<Option<KeyGesture>> {
            FerroProperty::register_attached::<HotKeyManager, Control, _>("HotKey", None)
        }
    );
} }

impl HotKeyManager {
    fn static_constructor() {
        Self::hot_key_property().changed().subscribe(|args| {
            let Some(hotkey) = args.get_new_value::<Option<KeyGesture>>() else { return };

            let control = args.sender().downcast_ref::<Control>();
            let supported = control
                .is_some_and(|control| as_clickable_control(control).is_some() || as_command_source(control).is_some());

            let Some(control) = control.filter(|_| supported) else {
                if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::CONTROL) {
                    logger.log(
                        None,
                        &format!(
                            "The element {} does not implement IClickableControl nor ICommandSource and does not \
                             support binding a HotKey ({}).",
                            args.sender().get_type().name(),
                            hotkey
                        ),
                    );
                }
                return;
            };

            Manager::new(&control.to_ref()).init();
        });
    }

    /// Sets the value of the `HotKey` attached property on an object.
    pub fn set_hot_key(target: &FerroObject, value: Option<KeyGesture>) {
        target.set_value(Self::hot_key_property(), value)
    }

    /// Gets the value of the `HotKey` attached property on an object.
    pub fn get_hot_key(target: &FerroObject) -> Option<KeyGesture> {
        target.get_value(Self::hot_key_property())
    }
}
