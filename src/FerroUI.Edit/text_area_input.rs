use super::*;
use ferroui_base::input::KeyBinding;
use ferroui_base::interactivity::{RoutedEvent, RoutedEventArgs, RoutingStrategies};
use ferroui_base::reactive::Subscription;
use std::collections::HashSet;
use std::fmt;

#[cfg(test)]
#[path = "text_area_input_tests.rs"]
mod tests;

/// A native editor input extension. The same trait can replace the default
/// editor controls or run as a temporary keyboard layer ahead of them.
///
/// Use weak references when retaining the text area. An attachment owns its
/// native subscriptions, and drops them before calling `detach`.
pub trait EditorInputHandler {
    /// Children attach in this order, carry their own scoped registrations,
    /// and detach before their parent. The tree is read at attachment time.
    fn child_handlers(&self) -> Vec<Rc<dyn EditorInputHandler>> {
        Vec::new()
    }
    fn attach(&self, _area: &TextArea, _bindings: &mut EditorInputBindings) {}
    /// Also called during destruction. A dying area cannot be revived with
    /// `to_ref()`; compare weak handles with `to_weak()`.
    fn detach(&self, _area: &TextArea) {}
    fn on_key_down(&self, _area: &TextArea, _event: &KeyEventArgs) {}
    fn on_key_up(&self, _area: &TextArea, _event: &KeyEventArgs) {}
}

/// Scoped FerroUI subscriptions and command bindings for an input handler.
/// Key bindings run only if the handler and higher layers leave the key
/// unhandled. The bindings keep using their live command and gesture values.
#[derive(Default)]
pub struct EditorInputBindings {
    subscriptions: Vec<Subscription>,
    keys: Vec<Ref<KeyBinding>>,
}

impl EditorInputBindings {
    pub fn track(&mut self, subscription: Subscription) {
        self.subscriptions.push(subscription);
    }

    pub fn add_key_binding(&mut self, binding: Ref<KeyBinding>) {
        self.keys.push(binding);
    }
}

/// The primary handler for keyboard commands, pointer selection and text
/// drag-and-drop. Committed text is a separate TextArea input service.
#[derive(Clone, Default)]
pub enum EditorInputMode {
    #[default]
    Default,
    Custom(Rc<dyn EditorInputHandler>),
    Disabled,
}

impl PartialEq for EditorInputMode {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Default, Self::Default) | (Self::Disabled, Self::Disabled) => true,
            (Self::Custom(a), Self::Custom(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }
}

impl Eq for EditorInputMode {}

/// Identifies one connection to a text area's temporary input stack.
#[derive(Clone)]
pub struct EditorInputLayer {
    area: WeakRef<TextArea>,
    id: u64,
}

impl EditorInputLayer {
    /// Detaches this handler and all handlers above it, newest first. Returns
    /// false if the connection has already been removed or the area is gone.
    pub fn pop(&self) -> Result<bool, EditorInputHandlerError> {
        match self.area.upgrade() {
            Some(area) => area.pop_input_layer(self.id),
            None => Ok(false),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorInputHandlerError {
    AlreadyAttached,
    ChangingHandlers,
}

impl fmt::Display for EditorInputHandlerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::AlreadyAttached => {
                "An input handler may only appear once in the editor's active input tree"
            }
            Self::ChangingHandlers => "The editor is attaching or detaching input handlers",
        })
    }
}

impl std::error::Error for EditorInputHandlerError {}

struct InputConnection {
    handler: Rc<dyn EditorInputHandler>,
    bindings: RefCell<Option<EditorInputBindings>>,
    children: Vec<InputConnection>,
}

/// Validates ownership and captures the declared children before modifying any
/// attachment. Cycles and repeated handlers are rejected as duplicate nodes.
struct InputHandlerTree {
    handler: Rc<dyn EditorInputHandler>,
    children: Vec<InputHandlerTree>,
}

fn identity(handler: &Rc<dyn EditorInputHandler>) -> *const () {
    Rc::as_ptr(handler).cast()
}

impl InputHandlerTree {
    fn new(
        handler: Rc<dyn EditorInputHandler>,
        seen: &mut HashSet<*const ()>,
    ) -> Result<Self, EditorInputHandlerError> {
        if !seen.insert(identity(&handler)) {
            return Err(EditorInputHandlerError::AlreadyAttached);
        }
        let children = handler
            .child_handlers()
            .into_iter()
            .map(|child| Self::new(child, seen))
            .collect::<Result<_, _>>()?;
        Ok(Self { handler, children })
    }

    fn connect(self, area: &TextArea) -> InputConnection {
        let mut bindings = EditorInputBindings::default();
        self.handler.attach(area, &mut bindings);
        InputConnection {
            handler: self.handler,
            bindings: RefCell::new(Some(bindings)),
            children: self
                .children
                .into_iter()
                .map(|child| child.connect(area))
                .collect(),
        }
    }
}

impl InputConnection {
    fn identities(&self, attached: &mut HashSet<*const ()>) {
        attached.insert(identity(&self.handler));
        for child in &self.children {
            child.identities(attached);
        }
    }

    fn disconnect(&self, area: &TextArea) {
        let bindings = self.bindings.take();
        drop(bindings);
        for child in self.children.iter().rev() {
            child.disconnect(area);
        }
        self.handler.detach(area);
    }

    fn key(&self, area: &TextArea, event: &KeyEventArgs, down: bool) {
        if self.bindings.borrow().is_none() || event.handled() {
            return;
        }
        if down {
            self.handler.on_key_down(area, event);
            if event.handled() {
                return;
            }
            let keys = self.bindings.borrow().as_ref().map(|b| b.keys.clone());
            for binding in keys.into_iter().flatten() {
                if event.handled() || self.bindings.borrow().is_none() {
                    break;
                }
                binding.try_handle(event);
            }
        } else {
            self.handler.on_key_up(area, event);
        }
        for child in &self.children {
            if event.handled() {
                break;
            }
            child.key(area, event, down);
        }
    }
}

#[derive(Default)]
pub(super) struct InputHandlers {
    mode: RefCell<EditorInputMode>,
    primary: RefCell<Option<Rc<InputConnection>>>,
    stack: RefCell<Vec<(u64, Rc<InputConnection>)>>,
    next_id: Cell<u64>,
    changing: Cell<bool>,
}

struct HandlerChange<'a>(&'a Cell<bool>);

impl Drop for HandlerChange<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

impl InputHandlers {
    fn changing(&self) -> Result<HandlerChange<'_>, EditorInputHandlerError> {
        if self.changing.replace(true) {
            return Err(EditorInputHandlerError::ChangingHandlers);
        }
        Ok(HandlerChange(&self.changing))
    }

    fn detach_stack(&self, area: &TextArea, from: usize) {
        let removed = self.stack.borrow_mut().split_off(from);
        for (_, connection) in removed.into_iter().rev() {
            connection.disconnect(area);
        }
    }

    fn detach_primary(&self, area: &TextArea) {
        let connection = self.primary.take();
        if let Some(connection) = connection {
            connection.disconnect(area);
        }
    }
}

impl TextArea {
    ferroui_base::ferro_routed_event!(
        pub fn active_input_handler_changed_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<TextArea, _>(
                "ActiveInputHandlerChanged",
                RoutingStrategies::BUBBLE,
            )
        }
    );

    pub fn active_input_mode(&self) -> EditorInputMode {
        self.input_handlers.mode.borrow().clone()
    }

    /// Replaces the primary attachment and first detaches the temporary stack
    /// in reverse attachment order. Reassigning the same mode has no effect.
    pub fn set_active_input_mode(
        &self,
        mode: EditorInputMode,
    ) -> Result<(), EditorInputHandlerError> {
        let handlers = &self.input_handlers;
        let change = handlers.changing()?;
        if *handlers.mode.borrow() == mode {
            return Ok(());
        }
        let tree = match &mode {
            EditorInputMode::Custom(handler) => {
                Some(InputHandlerTree::new(handler.clone(), &mut HashSet::new())?)
            }
            _ => None,
        };
        handlers.detach_stack(self, 0);
        if self.uses_default_editor_input() {
            self.cancel_pointer_selection();
        }
        handlers.mode.replace(EditorInputMode::Disabled);
        handlers.detach_primary(self);
        handlers.mode.replace(mode);
        if let Some(tree) = tree {
            handlers.primary.replace(Some(Rc::new(tree.connect(self))));
        }
        self.update_text_drag_option();
        drop(change);
        self.raise_event(&RoutedEventArgs::with_event(
            Self::active_input_handler_changed_event(),
        ));
        Ok(())
    }

    /// Returns the currently attached temporary handlers, newest first.
    pub fn stacked_input_handlers(&self) -> Vec<Rc<dyn EditorInputHandler>> {
        self.input_handlers
            .stack
            .borrow()
            .iter()
            .rev()
            .map(|(_, connection)| connection.handler.clone())
            .collect()
    }

    pub fn push_stacked_input_handler(
        &self,
        handler: Rc<dyn EditorInputHandler>,
    ) -> Result<EditorInputLayer, EditorInputHandlerError> {
        let handlers = &self.input_handlers;
        let _change = handlers.changing()?;
        let mut seen = HashSet::new();
        if let Some(primary) = handlers.primary.borrow().as_ref() {
            primary.identities(&mut seen);
        }
        for (_, entry) in handlers.stack.borrow().iter() {
            entry.identities(&mut seen);
        }
        let tree = InputHandlerTree::new(handler, &mut seen)?;
        let connection = Rc::new(tree.connect(self));
        let id = handlers.next_id.get();
        handlers.next_id.set(id + 1);
        handlers.stack.borrow_mut().push((id, connection));
        Ok(EditorInputLayer {
            area: self.to_ref().downgrade(),
            id,
        })
    }

    fn pop_input_layer(&self, id: u64) -> Result<bool, EditorInputHandlerError> {
        let handlers = &self.input_handlers;
        let Some(index) = handlers
            .stack
            .borrow()
            .iter()
            .position(|(candidate, _)| *candidate == id)
        else {
            return Ok(false);
        };
        let _change = handlers.changing()?;
        handlers.detach_stack(self, index);
        Ok(true)
    }

    pub(super) fn register_input_handler_events() {
        for (event, down) in [
            (InputElement::key_down_event(), true),
            (InputElement::key_up_event(), false),
        ] {
            event
                .add_class_handler_with::<Self>(
                    move |area, event| {
                        if area.is_hosted_child_event(event) {
                            return;
                        }
                        let stack = area.input_handlers.stack.borrow().clone();
                        for (_, connection) in stack.into_iter().rev() {
                            if event.handled() {
                                break;
                            }
                            connection.key(area, event, down);
                        }
                    },
                    RoutingStrategies::TUNNEL,
                    false,
                )
                .detach();
        }
    }

    /// Runs a custom primary handler and reports whether default editing
    /// commands should run for this event.
    pub(super) fn dispatch_primary_key(&self, event: &KeyEventArgs, down: bool) -> bool {
        let connection = self.input_handlers.primary.borrow().clone();
        if let Some(connection) = connection {
            connection.key(self, event, down);
            return false;
        }
        self.uses_default_editor_input()
    }

    pub(super) fn uses_default_editor_input(&self) -> bool {
        matches!(*self.input_handlers.mode.borrow(), EditorInputMode::Default)
    }

    pub(crate) fn is_changing_input_handlers(&self) -> bool {
        self.input_handlers.changing.get()
    }

    pub(super) fn disconnect_input_handlers(&self) {
        let handlers = &self.input_handlers;
        let _change = handlers
            .changing()
            .expect("text area dropped during input attachment");
        handlers.detach_stack(self, 0);
        handlers.detach_primary(self);
    }
}
