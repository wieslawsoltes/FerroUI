use super::*;

const MARKUP: &str = r#"<ui:Button xmlns:ui="https://github.com/ferroui" xmlns:addin="using:Custom">
    <ui:Interaction.Behaviors>
        <ui:BehaviorCollection>
            <ui:EventTriggerBehavior EventName='Click'>
                <ui:EventTriggerBehavior.Actions>
                    <ui:ActionCollection>
                        <!-- Keep the promotion control's actions 🧭 -->
                        <ui:ChangePropertyAction Value='0.6' PropertyName='Opacity' />
                        <ui:AddClassAction ClassName="promoted" />
                        <addin:Publish>
                            <addin:Publish.Options><!-- exact --></addin:Publish.Options>
                        </addin:Publish>
                    </ui:ActionCollection>
                </ui:EventTriggerBehavior.Actions>
            </ui:EventTriggerBehavior>
        </ui:BehaviorCollection>
    </ui:Interaction.Behaviors>
</ui:Button>"#;

fn read(document: &Document) -> BehaviorSource {
    BehaviorSource::read(document, document.ast().root())
        .unwrap()
        .remove(0)
}

fn change(document: &mut Document, edit: BehaviorEdit) {
    let behavior = read(document).node;
    let edits = BehaviorSource::plan(document, behavior, edit).unwrap();
    document.apply("Edit behavior", edits).unwrap();
}

#[test]
fn behavior_fields_change_only_owned_attributes() {
    let text = MARKUP.replace('\n', "\r\n");
    let mut document = Document::new(&text).unwrap();
    change(
        &mut document,
        BehaviorEdit::SetTrigger(BehaviorTrigger::Event {
            event: "PointerPressed".into(),
            source: None,
        }),
    );
    let expected = text.replace("EventName='Click'", "EventName='PointerPressed'");
    assert_eq!(document.source(), expected);
    let node = read(&document).actions[0].node;
    change(
        &mut document,
        BehaviorEdit::SetAction {
            node,
            value: BehaviorAction::ChangeProperty {
                target: None,
                property: "Opacity".into(),
                value: "0.85".into(),
            },
        },
    );
    assert_eq!(
        document.source(),
        expected.replace("Value='0.6'", "Value='0.85'")
    );
    document.undo().unwrap();
    assert_eq!(document.source(), expected);
}

#[test]
fn nested_behavior_action_order_and_add_keep_the_collection_scope() {
    let mut document = Document::new(MARKUP).unwrap();
    let source = read(&document);
    assert_eq!(source.actions.len(), 3);
    change(
        &mut document,
        BehaviorEdit::MoveAction {
            node: source.actions[0].node,
            index: 2,
        },
    );
    let moved = read(&document);
    assert!(matches!(
        moved.actions[2].value,
        BehaviorAction::ChangeProperty { .. }
    ));
    assert!(document
        .source()
        .contains("<!-- Keep the promotion control's actions 🧭 -->"));
    assert!(document
        .source()
        .contains("<addin:Publish.Options><!-- exact --></addin:Publish.Options>"));
    document.undo().unwrap();
    assert_eq!(document.source(), MARKUP);

    change(
        &mut document,
        BehaviorEdit::AppendAction(BehaviorAction::RemoveClass {
            target: None,
            class: "pending".into(),
        }),
    );
    let appended = read(&document);
    let id = appended.actions[3].node;
    let node = document.ast().node(id).unwrap();
    assert_eq!(node.name, "ui:RemoveClassAction");
    assert_eq!(
        node.parent
            .and_then(|id| document.ast().node(id))
            .unwrap()
            .local_name(),
        "ActionCollection"
    );
    change(&mut document, BehaviorEdit::RemoveAction(id));
    assert_eq!(read(&document).actions.len(), 3);
}

#[test]
fn bound_and_foreign_behavior_owners_are_protected() {
    let document = Document::new(MARKUP).unwrap();
    let source = read(&document);
    let id = source.actions[2].node;
    assert!(BehaviorSource::owner(&document, id).is_err());
    assert!(BehaviorSource::plan(
        &document,
        source.node,
        BehaviorEdit::RemoveAction(document.ast().root())
    )
    .is_err());
    assert!(BehaviorSource::plan(
        &document,
        source.node,
        BehaviorEdit::SetAction {
            node: id,
            value: BehaviorAction::CallMethod {
                target: None,
                method: "Changed".into()
            },
        }
    )
    .is_err());
    let bound = Document::new(r#"<Button Interaction.Behaviors="{Binding Actions}" />"#).unwrap();
    assert!(!BehaviorSource::can_attach(&bound, bound.ast().root()));
}

#[test]
fn behavior_does_not_replace_object_sources_or_style_owned_actions() {
    let document = Document::new(r#"<Button xmlns="https://github.com/ferroui">
        <Interaction.Behaviors>
            <EventTriggerBehavior EventName="Click">
                <EventTriggerBehavior.SourceObject><Button /></EventTriggerBehavior.SourceObject>
                <EventTriggerBehavior.Styles><Style Selector="EventTriggerBehavior" /></EventTriggerBehavior.Styles>
            </EventTriggerBehavior>
        </Interaction.Behaviors>
    </Button>"#).unwrap();
    let source = read(&document);
    assert!(!source.can_edit_trigger(&document));
    assert!(!source.can_edit_actions());
    assert!(BehaviorSource::plan(
        &document,
        source.node,
        BehaviorEdit::AppendAction(BehaviorAction::CallMethod {
            target: None,
            method: "Refresh".into(),
        })
    )
    .is_err());
}

#[test]
fn raw_behavior_replacement_preserves_source_and_requires_one_scoped_object() {
    let mut document = Document::new(MARKUP).unwrap();
    let node = read(&document).node;
    let replacement =
        "<ui:EventTriggerBehavior EventName='Click'><!-- keep exact --></ui:EventTriggerBehavior>";
    assert!(BehaviorSource::replace_source(&document, node, "<Break></Invalid>").is_err());
    assert!(BehaviorSource::replace_source(&document, node, "<One /> <Two />").is_err());
    let edits = BehaviorSource::replace_source(&document, node, replacement).unwrap();
    document.apply("Replace behavior source", edits).unwrap();
    assert!(document.source().contains(replacement));
    document.undo().unwrap();
    assert_eq!(document.source(), MARKUP);
}
