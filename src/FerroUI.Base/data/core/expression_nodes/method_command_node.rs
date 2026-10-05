use super::{append_member, ExpressionNode, NodeState};
use crate::data::core::WeakValue;
use crate::data::model::{Event, ICommand, INotifyPropertyChanged};
use crate::reactive::{Disposable, IDisposable};
use crate::data::BindingError;
use crate::threading::{Dispatcher, DispatcherPriority};
use crate::{AnyValue, BoxedValue};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

type Execute = Rc<dyn Fn(&dyn AnyValue, Option<&BoxedValue>)>;
type CanExecute = Rc<dyn Fn(&dyn AnyValue, Option<&BoxedValue>) -> bool>;
type InpcLookup = Rc<dyn Fn(&dyn AnyValue) -> Option<&dyn INotifyPropertyChanged>>;

/// A node that turns a method of its source into a command.
pub struct MethodCommandNode {
    this: Weak<MethodCommandNode>,
    state: NodeState,
    method_name: Box<str>,
    execute: Execute,
    can_execute: Option<CanExecute>,
    depends_on_properties: Vec<Box<str>>,
    inpc: Option<InpcLookup>,
    command: RefCell<Option<Rc<MethodCommand>>>,
    token: Cell<Option<u64>>,
}

impl MethodCommandNode {
    /// Creates the node. `inpc` gives access to the source's property change
    /// notifications, which re-query the command's state when one of
    /// `depends_on_properties` changes.
    pub fn new(
        method_name: &str,
        execute: Execute,
        can_execute: Option<CanExecute>,
        depends_on_properties: &[&str],
        inpc: Option<InpcLookup>,
    ) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            state: NodeState::new(),
            method_name: method_name.into(),
            execute,
            can_execute,
            depends_on_properties: depends_on_properties.iter().map(|s| (*s).into()).collect(),
            inpc,
            command: RefCell::new(None),
            token: Cell::new(None),
        })
    }

    fn remove_handler(&self, source: &BoxedValue) {
        if let (Some(token), Some(inpc)) = (self.token.take(), &self.inpc) {
            if let Some(inpc) = inpc(&**source) {
                inpc.property_changed().remove(token);
            }
        }
    }
}

impl ExpressionNode for MethodCommandNode {
    fn state(&self) -> &NodeState {
        &self.state
    }

    fn build_string(&self, builder: &mut String) {
        append_member(builder, &self.method_name);
        builder.push_str("()");
    }

    fn on_source_changed(&self, source: Option<&BoxedValue>, _data_validation_error: Option<&BindingError>) {
        if !self.state.validate_non_null_source(source) {
            return;
        }
        let source = source.expect("validated");
        if let Some(inpc) = self.inpc.as_ref().and_then(|f| f(&**source)) {
            let weak = self.this.clone();
            let token = inpc.property_changed().add(Rc::new(move |name: &str| {
                if let Some(this) = weak.upgrade() {
                    if name.is_empty() || this.depends_on_properties.iter().any(|d| &**d == name) {
                        let command = this.command.borrow().clone();
                        if let Some(command) = command {
                            command.raise_can_execute_changed();
                        }
                    }
                }
            }));
            self.token.set(Some(token));
        }
        let command = MethodCommand::new(WeakValue::new(source), self.execute.clone(), self.can_execute.clone());
        self.command.replace(Some(command.clone()));
        let command: Rc<dyn ICommand> = command;
        self.state.set_value(Some(Rc::new(command)), None);
    }

    fn unsubscribe(&self, old_source: &BoxedValue) {
        self.remove_handler(old_source);
    }
}

impl Drop for MethodCommandNode {
    fn drop(&mut self) {
        if let Some(source) = self.state.source() {
            self.remove_handler(&source);
        }
    }
}

/// The command created for a method of a binding source. It holds the source
/// weakly.
pub struct MethodCommand {
    this: Weak<MethodCommand>,
    target: WeakValue,
    execute: Execute,
    can_execute: Option<CanExecute>,
    can_execute_changed: Event<()>,
}

impl MethodCommand {
    pub(crate) fn new(target: WeakValue, execute: Execute, can_execute: Option<CanExecute>) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            target,
            execute,
            can_execute,
            can_execute_changed: Event::new(),
        })
    }

    /// Raises the "can execute changed" event, at input priority on the
    /// dispatcher.
    pub fn raise_can_execute_changed(&self) {
        // The command belongs to the thread that created it, so the
        // notification is posted to that thread's dispatcher.
        let weak = self.this.clone();
        Dispatcher::current_dispatcher().post_local(
            move || {
                if let Some(this) = weak.upgrade() {
                    this.can_execute_changed.raise(&());
                }
            },
            DispatcherPriority::INPUT,
        );
    }
}

impl ICommand for MethodCommand {
    fn can_execute(&self, parameter: Option<&BoxedValue>) -> bool {
        match self.target.upgrade() {
            Some(target) => match &self.can_execute {
                Some(can_execute) => can_execute(&*target, parameter),
                None => true,
            },
            None => false,
        }
    }

    fn execute(&self, parameter: Option<&BoxedValue>) {
        if let Some(target) = self.target.upgrade() {
            (self.execute)(&*target, parameter);
        }
    }

    fn can_execute_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        let token = self.can_execute_changed.add(Rc::new(move |_: &()| handler()));
        let weak = self.this.clone();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.can_execute_changed.remove(token);
            }
        })
    }
}
