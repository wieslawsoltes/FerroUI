use super::*;
use crate::{TypeInfo, apply_text_edits};

const URI: &str = "untitled:/Commands.xaml";
const START: &str = "<Border xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' xmlns:vm='using:Models' x:DataType='vm:ViewModel'>";

fn model(name: &str, properties: &[(&str, &str)]) -> TypeInfo {
    TypeInfo {
        name: name.into(),
        namespace: "using:Models".into(),
        properties: properties
            .iter()
            .map(|(name, kind)| {
                (
                    (*name).into(),
                    PropertyInfo {
                        name: (*name).into(),
                        type_name: (*kind).into(),
                        read_only: true,
                        ..Default::default()
                    },
                )
            })
            .collect(),
        ..Default::default()
    }
}

fn fixture(button: &str) -> (String, LanguageService, CommandBindingRequest) {
    let source = format!("{START}\r\n  <!-- 🪴 -->{button}\r\n</Border>");
    let mut service = LanguageService::default();
    service.schema_mut().register(model(
        "ViewModel",
        &[("Actions", "Actions"), ("Order", "String")],
    ));
    service.schema_mut().register(model(
        "Actions",
        &[
            (
                "Save",
                "Option<std::rc::Rc<dyn ferroui_base::input::ICommand>>",
            ),
            ("Publish", "ICommand"),
            ("Title", "String"),
        ],
    ));
    service.open(URI, &source, 4).unwrap();
    let position = service
        .index(URI)
        .unwrap()
        .position(source.find("<Button").unwrap() + 1)
        .unwrap();
    (
        source,
        service,
        CommandBindingRequest {
            version: 4,
            position,
            prefix: String::new(),
        },
    )
}

fn bind(path: &str) -> CommandSelection {
    CommandSelection::Bind { path: path.into() }
}

#[test]
fn follows_registered_nested_members_and_plans_one_crlf_source_edit() {
    let (source, service, request) =
        fixture("<Button Command='{CompiledBinding Path=Actions.Publish}'/>");
    let root = service.command_bindings(URI, &request).unwrap();
    assert!(
        root.members
            .iter()
            .any(|member| member.path == "Actions" && member.children)
    );
    assert!(root.slots[0].parameter.as_ref().unwrap().editable);
    let nested = service
        .command_bindings(
            URI,
            &CommandBindingRequest {
                prefix: "Actions".into(),
                ..request.clone()
            },
        )
        .unwrap();
    assert!(
        nested
            .members
            .iter()
            .any(|member| member.path == "Actions.Save" && member.command)
    );
    assert!(
        nested
            .members
            .iter()
            .any(|member| member.path == "Actions.Title" && !member.command)
    );
    let plan = service
        .plan_command_binding(
            URI,
            &request,
            &CommandBindingEdit {
                property: "Command".into(),
                command: bind("Actions.Save"),
                parameter: bind("Order"),
            },
        )
        .unwrap();
    let [document] = plan.document_changes.as_slice() else {
        panic!("Expected one versioned buffer");
    };
    assert_eq!(document.version, 4);
    let changed = apply_text_edits(&source, &document.edits).unwrap();
    assert_eq!(
        changed,
        source
            .replace("Path=Actions.Publish", "Path=Actions.Save")
            .replace("/>", " CommandParameter='{Binding Order}'/>")
    );
    assert!(
        service
            .command_bindings(
                URI,
                &CommandBindingRequest {
                    version: 3,
                    ..request
                }
            )
            .is_err()
    );
}

#[test]
fn protects_non_datacontext_expressions_and_rejects_non_command_leaves() {
    let (source, service, request) = fixture("<Button Command='{Binding #Other.Command}' />");
    let model = service.command_bindings(URI, &request).unwrap();
    assert!(!model.slots[0].binding.editable);
    let range = model.slots[0].binding.range.unwrap();
    let span = service.index(URI).unwrap().span(range).unwrap();
    assert_eq!(&source[span.range()], "{Binding #Other.Command}");
    let mut change = CommandBindingEdit {
        property: "Command".into(),
        command: CommandSelection::Clear,
        parameter: CommandSelection::Keep,
    };
    assert!(
        service
            .plan_command_binding(URI, &request, &change)
            .unwrap_err()
            .0
            .contains("in source")
    );
    change.command = bind("Actions.Title");
    assert!(
        service
            .plan_command_binding(URI, &request, &change)
            .unwrap_err()
            .0
            .contains("ICommand")
    );
    change.command = CommandSelection::Keep;
    change.parameter = bind("Order");
    let plan = service
        .plan_command_binding(URI, &request, &change)
        .unwrap();
    let result = apply_text_edits(&source, &plan.document_changes[0].edits).unwrap();
    assert!(result.contains("Command='{Binding #Other.Command}'"));
    assert!(result.contains("CommandParameter='{Binding Order}'"));
}
