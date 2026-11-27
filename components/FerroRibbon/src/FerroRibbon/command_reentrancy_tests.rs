//! Public command reentrancy compared with unchanged reference-runtime traces.
#![cfg(test)]
use ferroui_base::{
    BoxedValue,
    input::{ICommand, InputElement},
    reactive::{Disposable, IDisposable},
};
use ferroui_controls::Button;
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::Rc,
};
type Action = Rc<dyn Fn()>;
struct Command {
    name: &'static str,
    allowed: Cell<bool>,
    trace: Rc<RefCell<Vec<String>>>,
    queries: Cell<usize>,
    parameters: RefCell<Vec<Option<String>>>,
    handlers: Rc<RefCell<BTreeMap<u64, Action>>>,
    next: Cell<u64>,
    on_add: RefCell<Option<Action>>,
    on_remove: Rc<RefCell<Option<Action>>>,
    on_query: RefCell<Option<Action>>,
}
impl Command {
    fn new(name: &'static str, trace: Rc<RefCell<Vec<String>>>, allowed: bool) -> Rc<Self> {
        Rc::new(Self {
            name,
            allowed: Cell::new(allowed),
            trace,
            queries: Cell::new(0),
            parameters: RefCell::default(),
            handlers: Rc::default(),
            next: Cell::new(0),
            on_add: RefCell::default(),
            on_remove: Rc::default(),
            on_query: RefCell::default(),
        })
    }
    fn notify(&self) {
        self.trace
            .borrow_mut()
            .push(format!("{}:notify", self.name));
        let callbacks: Vec<_> = self.handlers.borrow().values().cloned().collect();
        for f in callbacks {
            f();
        }
    }
    fn snapshot(&self) -> Value {
        json!({"Name":self.name,"Queries":self.queries.get(),"Subscriptions":self.handlers.borrow().len(),"parameters":*self.parameters.borrow()})
    }
}
impl ICommand for Command {
    fn can_execute(&self, p: Option<&BoxedValue>) -> bool {
        let p = p.map(|x| x.downcast_ref::<String>().unwrap().clone());
        self.queries.set(self.queries.get() + 1);
        self.parameters.borrow_mut().push(p.clone());
        self.trace.borrow_mut().push(format!(
            "{}:query:{}",
            self.name,
            p.as_deref().unwrap_or("null")
        ));
        let action = self.on_query.borrow().clone();
        if let Some(f) = action {
            f();
        }
        self.allowed.get()
    }
    fn execute(&self, _: Option<&BoxedValue>) {
        panic!("state-only probe")
    }
    fn can_execute_changed(&self, handler: Action) -> Rc<dyn IDisposable> {
        let id = self.next.get();
        self.next.set(id + 1);
        self.handlers.borrow_mut().insert(id, handler);
        self.trace.borrow_mut().push(format!("{}:add", self.name));
        let action = self.on_add.borrow().clone();
        if let Some(f) = action {
            f();
        }
        let handlers = self.handlers.clone();
        let trace = self.trace.clone();
        let name = self.name;
        let on_remove = self.on_remove.clone();
        Disposable::create(move || {
            handlers.borrow_mut().remove(&id);
            trace.borrow_mut().push(format!("{name}:remove"));
            let f = on_remove.borrow().clone();
            if let Some(f) = f {
                f();
            }
        })
    }
}
fn once(action: impl Fn() + 'static) -> Action {
    let used = Cell::new(false);
    Rc::new(move || {
        if !used.replace(true) {
            action();
        }
    })
}
fn snapshot(
    step: &str,
    button: &Button,
    commands: &[Rc<Command>; 3],
    trace: &Rc<RefCell<Vec<String>>>,
) -> Value {
    let current = button.command();
    let name = commands
        .iter()
        .find(|c| {
            current
                .as_ref()
                .is_some_and(|actual| Rc::ptr_eq(actual, &(Rc::clone(*c) as Rc<dyn ICommand>)))
        })
        .map(|c| c.name);
    json!({"step":step,"command":name,"enabled":button.is_enabled(),"parameter":button.command_parameter().and_then(|v|v.downcast_ref::<String>().cloned()),"trace":*trace.borrow(),"commands":commands.iter().map(|c|c.snapshot()).collect::<Vec<_>>()})
}
fn compare(button: ferroui_base::Ref<Button>, expected: &Value) {
    let trace = Rc::new(RefCell::new(Vec::new()));
    let out = trace.clone();
    let _token = button.property_changed(move |e| {
        if e.property() == InputElement::is_enabled_property().as_property()
            && e.is_effective_value_change()
        {
            out.borrow_mut()
                .push(format!("enabled:{}", e.get_old_and_new_value::<bool>().1));
        }
    });
    let a = Command::new("a", trace.clone(), true);
    let b = Command::new("b", trace.clone(), true);
    let c = Command::new("c", trace.clone(), false);
    let commands = [a.clone(), b.clone(), c.clone()];
    let mut actual = Vec::new();
    match expected["name"].as_str().unwrap() {
        "add-replaces-command" => {
            let w = button.downgrade();
            let v = b.clone();
            a.on_add.replace(Some(once(move || {
                w.upgrade().unwrap().set_command(Some(v.clone()))
            })));
            button.set_command(Some(a.clone()));
            actual.push(snapshot("assigned-a", &button, &commands, &trace));
            a.notify();
            actual.push(snapshot("notify-a", &button, &commands, &trace));
            b.notify();
            actual.push(snapshot("notify-b", &button, &commands, &trace));
        }
        "remove-replaces-command" => {
            button.set_command(Some(a.clone()));
            let w = button.downgrade();
            let v = c.clone();
            a.on_remove.replace(Some(once(move || {
                w.upgrade().unwrap().set_command(Some(v.clone()))
            })));
            button.set_command(Some(b.clone()));
            actual.push(snapshot("assigned-b", &button, &commands, &trace));
            b.notify();
            actual.push(snapshot("notify-b", &button, &commands, &trace));
            c.notify();
            actual.push(snapshot("notify-c", &button, &commands, &trace));
        }
        "query-replaces-command" => {
            a.allowed.set(false);
            let w = button.downgrade();
            let v = b.clone();
            a.on_query.replace(Some(once(move || {
                w.upgrade().unwrap().set_command(Some(v.clone()))
            })));
            button.set_command(Some(a.clone()));
            actual.push(snapshot("assigned-a", &button, &commands, &trace));
            b.notify();
            actual.push(snapshot("notify-b", &button, &commands, &trace));
        }
        "coerce-replaces-parameter" => {
            button.set_command(Some(a.clone()));
            let w = button.downgrade();
            a.on_query.replace(Some(once(move || {
                w.upgrade()
                    .unwrap()
                    .set_command_parameter(Some(Rc::new("nested".to_owned())))
            })));
            button.set_is_enabled(false);
            actual.push(snapshot("local-false", &button, &commands, &trace));
            button.set_is_enabled(true);
            actual.push(snapshot("local-true", &button, &commands, &trace));
        }
        other => panic!("unknown {other}"),
    }
    button.set_command(None);
    actual.push(snapshot("clear-command", &button, &commands, &trace));
    let name = expected["name"].as_str().unwrap();
    if name == "coerce-replaces-parameter" {
        assert_eq!(
            Value::Array(actual),
            expected["snapshots"],
            "{}:{name}",
            button.get_type().name(),
        );
        return;
    }

    let (current, active, wrong, enabled) = if name == "remove-replaces-command" {
        ("c", 2, 1, false)
    } else {
        ("b", 1, 0, true)
    };
    let initial = &actual[0];
    assert_eq!(initial["command"], current, "{name}");
    assert_eq!(initial["enabled"], enabled, "{name}");
    for snapshot in &actual[..actual.len() - 1] {
        let subscriptions: Vec<_> = snapshot["commands"]
            .as_array()
            .unwrap()
            .iter()
            .map(|command| command["Subscriptions"].as_u64().unwrap())
            .collect();
        assert_eq!(subscriptions[active], 1, "{name}");
        assert_eq!(subscriptions[wrong], 0, "{name}");
        assert_eq!(subscriptions.iter().sum::<u64>(), 1, "{name}");
    }
    if actual.len() == 4 {
        let queries = |step: usize| {
            actual[step]["commands"][active]["Queries"]
                .as_u64()
                .unwrap()
        };
        assert_eq!(
            queries(1),
            queries(0),
            "{name} ignores abandoned notifications"
        );
        assert!(
            queries(2) > queries(1),
            "{name} receives the active notification"
        );
    }
    let cleared = actual.last().unwrap();
    assert!(cleared["command"].is_null());
    assert_eq!(cleared["enabled"], true);
    assert!(
        cleared["commands"]
            .as_array()
            .unwrap()
            .iter()
            .all(|command| command["Subscriptions"] == 0)
    );
}
fn run_case(name: &'static str) {
    crate::tests::run(move || {
        let fixture: Value =
            serde_json::from_str(include_str!("../../tests/fixtures/button-reentrancy.json"))
                .unwrap();
        let case = fixture["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|case| case["name"] == name)
            .unwrap();
        for button in [
            crate::controls::RibbonButton::new().upcast::<Button>(),
            crate::controls::RibbonDropDownButton::new().upcast::<Button>(),
            crate::controls::RibbonToggleButton::new().upcast::<Button>(),
            crate::controls::RibbonCheckBox::new().upcast::<Button>(),
        ] {
            compare(button, case);
        }
    });
}
#[test]
fn reentrant_add_keeps_the_replacement_subscribed() {
    run_case("add-replaces-command");
}
#[test]
fn reentrant_remove_keeps_the_replacement_subscribed() {
    run_case("remove-replaces-command");
}
#[test]
fn reentrant_query_uses_the_replacement_enablement() {
    run_case("query-replaces-command");
}
#[test]
fn actual_runtime_reentrant_coercion_replaces_parameter() {
    run_case("coerce-replaces-parameter");
}
