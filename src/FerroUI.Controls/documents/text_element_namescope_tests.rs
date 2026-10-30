use super::TextElement;
use crate::documents::{Hyperlink, Inline, InlineUIContainer, Paragraph, Run, Span};
use crate::test_support::test_scope;
use crate::{Border, Button};
use ferroui_base::controls::{ChildNameScope, NameScope, NameScopeRef};
use ferroui_base::metadata::{from_markup_value, into_markup_value};
use ferroui_base::{FerroObject, Ref};
#[allow(unused_imports)]
use crate::{RichTextBlock, RichTextBlockOverflow};

#[test]
fn text_element_find_name_stops_at_nearest_scope_even_when_name_is_missing() {
    let _scope = test_scope();
    let paragraph = Paragraph::new();
    let span = Span::new();
    let run = Run::new();
    span.inlines().add(run.clone());
    paragraph.inlines().add(span.clone());
    let outer = NameScopeRef::new(NameScope::new());
    let inner = NameScopeRef::new(NameScope::new());
    let outer_target = Border::new();
    let inner_target = Border::new();
    outer.register("target", outer_target.clone().upcast());
    outer.register("outerOnly", outer_target.clone().upcast());
    inner.register("target", inner_target.clone().upcast());
    NameScope::set_name_scope(&paragraph, Some(outer));
    NameScope::set_name_scope(&span, Some(inner));

    assert_eq!(
        paragraph.find_name("target"),
        Some(outer_target.clone().upcast())
    );
    assert_eq!(run.find_name("target"), Some(inner_target.upcast()));
    assert!(run.find_name("outerOnly").is_none());
    let own = NameScopeRef::new(NameScope::new());
    own.register("target", run.clone().upcast());
    NameScope::set_name_scope(&run, Some(own));
    assert_eq!(run.find_name("target"), Some(run.clone().upcast()));
    NameScope::set_name_scope(&run, None);
    NameScope::set_name_scope(&span, None);
    assert_eq!(run.find_name("target"), Some(outer_target.upcast()));
}

#[test]
fn text_element_find_name_preserves_child_scope_completion_policy() {
    let _scope = test_scope();
    let parent = NameScopeRef::new(NameScope::new());
    let outer_target = Border::new();
    parent.register("target", outer_target.clone().upcast());
    parent.register("outerOnly", outer_target.clone().upcast());
    parent.complete();
    let child = NameScopeRef::new(ChildNameScope::new(parent));
    let span = Span::new();
    let run = Run::new();
    span.inlines().add(run.clone());
    child.register("target", run.clone().upcast());
    NameScope::set_name_scope(&span, Some(child.clone()));

    assert_eq!(run.find_name("target"), Some(run.clone().upcast()));
    assert!(run.find_name("outerOnly").is_none());
    child.complete();
    assert_eq!(run.find_name("target"), Some(run.clone().upcast()));
    assert_eq!(run.find_name("outerOnly"), Some(outer_target.upcast()));
}

#[test]
fn text_element_find_name_rechecks_reparenting_and_replaced_scopes() {
    let _scope = test_scope();
    let first = Span::new();
    let second = Span::new();
    let run = Run::new();
    first.inlines().add(run.clone());
    let first_scope = NameScopeRef::new(NameScope::new());
    let second_scope = NameScopeRef::new(NameScope::new());
    first_scope.register("target", first.clone().upcast());
    second_scope.register("target", second.clone().upcast());
    NameScope::set_name_scope(&first, Some(first_scope));
    NameScope::set_name_scope(&second, Some(second_scope));
    assert_eq!(run.find_name("target"), Some(first.clone().upcast()));

    first.inlines().clear();
    assert!(run.find_name("target").is_none());
    second.inlines().add(run.clone());
    assert_eq!(run.find_name("target"), Some(second.clone().upcast()));
    let replacement = NameScopeRef::new(NameScope::new());
    let target = Border::new();
    replacement.register("target", target.clone().upcast());
    NameScope::set_name_scope(&second, Some(replacement));
    assert_eq!(run.find_name("target"), Some(target.upcast()));
    NameScope::set_name_scope(&second, None);
    assert!(run.find_name("target").is_none());
}

#[test]
fn text_element_find_name_uses_registration_and_does_not_keep_scope_owner_alive() {
    let _scope = test_scope();
    let run = Run::new();
    run.set_name(Some("source".to_owned()));
    assert!(run.find_name("source").is_none());
    let span = Span::new();
    let weak_span = span.downgrade();
    span.inlines().add(run.clone());
    let scope = NameScopeRef::new(NameScope::new());
    NameScope::set_name_scope(&span, Some(scope.clone()));
    assert!(
        run.find_name("source").is_none(),
        "assigning Name does not register code-created content"
    );
    scope.register("source", run.clone().upcast());
    scope.register("owner", span.clone().upcast());
    assert_eq!(run.find_name("source"), Some(run.clone().upcast()));
    assert_eq!(run.find_name("owner"), Some(span.clone().upcast()));
    drop(span);
    assert!(
        weak_span.upgrade().is_none(),
        "lookup does not cache or retain the owner"
    );
    assert!(run.find_name("owner").is_none());
}

#[test]
fn text_element_find_name_metadata_returns_the_same_live_object_or_none() {
    let _scope = test_scope();
    let span = Span::new();
    let run = Run::new();
    span.inlines().add(run.clone());
    let scope = NameScopeRef::new(NameScope::new());
    scope.register("target", run.clone().upcast());
    NameScope::set_name_scope(&span, Some(scope));
    let method = TextElement::TYPE
        .markup()
        .unwrap()
        .find_methods("FindName")
        .next()
        .expect("FindName is available through inherited TextElement metadata");
    for (name, expected) in [
        ("target", Some(run.clone().upcast::<FerroObject>())),
        ("missing", None),
    ] {
        let value = (method.invoke)(&[
            into_markup_value(run.clone()),
            into_markup_value(name.to_owned()),
        ])
        .unwrap();
        assert_eq!(
            from_markup_value::<Option<Ref<FerroObject>>>(&value),
            Some(expected)
        );
    }
    // Name is a registered direct property. Supplementary MarkupType members
    // deliberately omit it; the XAML type system resolves the shared registry.
    let name = Inline::TYPE
        .find_property("Name")
        .expect("Name is inherited through the registered property metadata");
    let declared_name = ferroui_base::StyledElement::name_property().as_property();
    assert!(
        std::ptr::eq(name, declared_name),
        "the inherited property is not duplicated"
    );
    let info: &dyn ferroui_base::data::core::IPropertyInfo = name;
    assert!(info.can_get());
    assert!(info.can_set());
    let target = into_markup_value(run.clone()).unwrap();
    let renamed = into_markup_value("renamed".to_owned()).unwrap();
    info.set(&*target, Some(&renamed)).unwrap();
    assert_eq!(run.name().as_deref(), Some("renamed"));
    let value = info.get(&*target).unwrap();
    assert_eq!(
        value.downcast_ref::<String>().map(String::as_str),
        Some("renamed")
    );
    assert_eq!(run.find_name("target"), Some(run.upcast()));
}

/// A text element in a control template finds the names of that template (the scope
/// `TemplateApplied` carries) without the template root getting a name scope of its
/// own: name lookup of the controls of the template is unchanged
/// (`docs/porting/winui-rich-text.md`, decision D7).
#[test]
fn text_element_find_name_reaches_the_name_scope_of_its_template() {
    use crate::primitives::TemplatedControl;
    use crate::templates::FuncControlTemplate;
    use crate::{Decorator, TextBlock};

    let _scope = test_scope();
    let target = TemplatedControl::new();
    let run = Run::with_text(Some("text"));
    let named = Border::new();
    let template_run = run.clone();
    let template_named = named.clone();
    target.set_template(Some(FuncControlTemplate::new(move |_, scope| {
        let root = Decorator::new();
        let text = TextBlock::new();
        text.inlines().unwrap().add(template_run.clone());
        root.set_child(&text);
        scope.register("named", template_named.clone().upcast());
        root.upcast()
    })));
    target.apply_template();

    assert_eq!(run.find_name("named"), Some(named.upcast()));
    assert!(run.find_name("missing").is_none());
    let root = target.visual_children().get(0).cast::<Decorator>().unwrap();
    assert!(NameScope::get_name_scope(&root).is_none());
}

#[test]
fn text_element_find_name_shares_control_and_text_registrations() {
    let _scope = test_scope();
    let display = RichTextBlock::new();
    let paragraph = Paragraph::new();
    let span = Span::new();
    let link = Hyperlink::new();
    let run = Run::with_text(Some("lookup"));
    let button = Button::new();
    link.inlines().add(run.clone());
    span.inlines().add(link.clone());
    span.inlines()
        .add(InlineUIContainer::with_child(button.clone()));
    paragraph.inlines().add(span.clone());
    display.blocks().add(paragraph.clone());
    let scope = NameScopeRef::new(NameScope::new());
    NameScope::set_name_scope(&display, Some(scope.clone()));
    for (name, object) in [
        ("display", display.clone().upcast::<FerroObject>()),
        ("paragraph", paragraph.clone().upcast()),
        ("span", span.clone().upcast()),
        ("link", link.clone().upcast()),
        ("run", run.clone().upcast()),
        ("button", button.clone().upcast()),
    ] {
        scope.register(name, object.clone());
        assert_eq!(run.find_name(name), Some(object.clone()), "{name}");
        assert_eq!(paragraph.find_name(name), Some(object), "{name}");
    }
    assert_eq!(display.find_control::<Run>("run"), Some(run));
    assert!(
        link.find_name("Run").is_none(),
        "names remain case-sensitive"
    );
    assert!(link.find_name("").is_none());
    assert!(link.find_name("missing").is_none());
}
