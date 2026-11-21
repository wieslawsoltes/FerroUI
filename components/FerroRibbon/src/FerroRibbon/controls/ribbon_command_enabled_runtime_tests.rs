//! Product-level operations compared with unchanged reference-runtime traces.
use super::*;
use crate::{
    command_enabled_tests::ProbeCommand,
    controls::{RibbonButton, RibbonCheckBox, RibbonDropDownButton, RibbonToggleButton},
};
use ferroui_base::{
    Ref, StaticType,
    styling::{ControlTheme, Setter},
};
use ferroui_controls::{ContentControl, Control, StackPanel};
use serde_json::{Value, json};

struct Command {
    pattern: String,
    allowed: Rc<Cell<bool>>,
    throws: Rc<Cell<bool>>,
    probe: Rc<ProbeCommand>,
}
impl Command {
    fn new(pattern: &str) -> Self {
        let allowed = Rc::new(Cell::new(pattern == "allow"));
        let throws = Rc::new(Cell::new(false));
        let (a, t) = (allowed.clone(), throws.clone());
        let alternating = pattern == "alternating";
        Self {
            pattern: pattern.into(),
            allowed,
            throws,
            probe: ProbeCommand::new(move |count, _| {
                assert!(!t.get(), "query-failure");
                if alternating {
                    !count.is_multiple_of(2)
                } else {
                    a.get()
                }
            }),
        }
    }
    fn snapshot(&self) -> Value {
        json!({"Pattern":self.pattern,"Queries":self.probe.queries.get(),
            "Subscriptions":self.probe.subscriptions(),
            "parameters":*self.probe.parameters.borrow()})
    }
}

fn compare<T: CommandEnabledOwner + StaticType>(
    owner: Ref<T>,
    case: &Value,
    input: &Value,
) -> Vec<String> {
    let button = owner.native_button();
    let panel = StackPanel::new();
    let host = ContentControl::new();
    host.set_content(Some(Control::boxed(panel.clone())));
    let changes = Rc::new(RefCell::new(Vec::new()));
    let output = changes.clone();
    let token = button.property_changed(move |event| {
        if event.property() == InputElement::is_enabled_property().as_property()
            && event.is_effective_value_change()
        {
            let (_, value) = event.get_old_and_new_value::<bool>();
            output.borrow_mut().push(value);
        }
    });
    let mut commands = Vec::<Command>::new();
    let mut active = None;
    let mut peer: Option<Ref<RibbonButton>> = None;
    let mut failures = Vec::new();
    for (index, (operation, expected)) in input["operations"]
        .as_array()
        .unwrap()
        .iter()
        .zip(case["steps"].as_array().unwrap())
        .enumerate()
    {
        let op = operation["op"].as_str().unwrap();
        let value = &operation["value"];
        let result = catch_unwind(AssertUnwindSafe(|| match op {
            "command" => {
                active = value.as_str().map(|pattern| {
                    commands.push(Command::new(pattern));
                    commands.len() - 1
                });
                button.set_command(active.map(|i| commands[i].probe.clone() as Rc<dyn ICommand>));
            }
            "notify" => {
                let command = &commands[active.unwrap()];
                command.allowed.set(value.as_bool().unwrap());
                command.probe.notify();
            }
            "throw" => commands[active.unwrap()]
                .throws
                .set(value.as_bool().unwrap()),
            "peer_add" => {
                let other = RibbonButton::new();
                other.set_command(active.map(|i| commands[i].probe.clone() as Rc<dyn ICommand>));
                peer = Some(other);
            }
            "peer_remove" => {
                peer.take().unwrap().command_enablement().visual_leave();
            }
            "local" => button.set_is_enabled(value.as_bool().unwrap()),
            "clear" => button.clear_value(InputElement::is_enabled_property()),
            "parameter" => button.set_command_parameter(
                value
                    .as_str()
                    .map(|s| Rc::new(s.to_owned()) as ferroui_base::BoxedValue),
            ),
            "style" => {
                if value.is_null() {
                    button.set_theme(None);
                } else {
                    let theme = ControlTheme::new();
                    theme.set_target_type(Some(T::TYPE));
                    if let Some(value) = value.as_bool() {
                        theme
                            .setters()
                            .add(Setter::new(InputElement::is_enabled_property(), value));
                    }
                    button.set_theme(theme);
                }
                button.apply_styling();
            }
            "parent_add" => panel.children().add(button.to_ref()),
            "parent_remove" => {
                assert!(panel.children().remove(button.to_ref()));
            }
            "parent" => host.set_is_enabled(value.as_bool().unwrap()),
            // The source corpus directly calls its enhanced hooks too. Window
            // attachment, OS input and tree traversal have separate tests.
            "enter" => owner.command_enablement().visual_enter(&*owner),
            "leave" => owner.command_enablement().visual_leave(),
            _ => panic!("unsupported reference operation {op}"),
        }));
        let error = result.err().map(|failure| {
            failure
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| failure.downcast_ref::<&str>().map(|s| (*s).to_owned()))
                .unwrap_or_else(|| "non-string panic".into())
        });
        let actual = json!({"op":op,"enabled":button.is_enabled(),
            "local":button.get_local_value(InputElement::is_enabled_property()),
            "errorMessage":error,"changes":*changes.borrow(),
            "commands":commands.iter().map(Command::snapshot).collect::<Vec<_>>()});
        let mut expected = expected.clone();
        // The native Rust API propagates predicate panics; it does not emulate
        // the source exception type. The failure boundary/message are compared.
        expected.as_object_mut().unwrap().remove("error");
        if actual != expected {
            failures.push(format!(
                "{}:{}:{op}\nactual={actual}\nexpected={expected}",
                T::TYPE.name(),
                index
            ));
        }
    }
    token.dispose();
    failures
}

fn run_case(name: &'static str) {
    crate::tests::run(move || {
        let fixture: Value =
            serde_json::from_str(include_str!("../../../tests/fixtures/button-runtime.json"))
                .unwrap();
        let inputs: Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/button-runtime-cases.json"
        ))
        .unwrap();
        let case = fixture["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["name"] == name)
            .unwrap();
        let input = inputs
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["name"] == name)
            .unwrap();
        let mut failures = compare(RibbonButton::new(), case, input);
        failures.extend(compare(
            RibbonDropDownButton::new().upcast::<RibbonButton>(),
            case,
            input,
        ));
        failures.extend(compare(RibbonToggleButton::new(), case, input));
        failures.extend(compare(RibbonCheckBox::new(), case, input));
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    });
}

macro_rules! runtime_case {
    ($test:ident, $name:literal) => {
        #[test]
        fn $test() {
            run_case($name);
        }
    };
}
runtime_case!(
    actual_runtime_denial_and_requests,
    "denial-and-request-recovery"
);
runtime_case!(actual_runtime_clear_without_local, "clear-without-local");
runtime_case!(actual_runtime_clear_local, "clear-local");
runtime_case!(actual_runtime_styles, "styles");
runtime_case!(actual_runtime_style_with_local, "style-with-local");
runtime_case!(
    actual_runtime_ancestor_denied_command,
    "ancestor-with-denied-command"
);
runtime_case!(actual_runtime_ancestor_local, "ancestor-with-local");
runtime_case!(
    actual_runtime_ancestor_allowed_command,
    "ancestor-with-allowed-command"
);
runtime_case!(actual_runtime_enhanced_hooks, "enhanced-entry-leave");
runtime_case!(
    actual_runtime_alternating_parameters,
    "alternating-parameters"
);
runtime_case!(
    actual_runtime_query_error_boundaries,
    "query-error-boundaries"
);
runtime_case!(
    actual_runtime_shared_command_subscriptions,
    "shared-command-subscriptions"
);
