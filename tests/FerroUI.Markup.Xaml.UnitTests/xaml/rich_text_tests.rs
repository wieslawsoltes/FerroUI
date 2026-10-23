//! Rich document/control declarations through the runtime XAML host. The same
//! fixture belongs to the compiled Rust emitter corpus, and the production theme
//! crates execute their actual compiled templates in their own integration tests.

use crate::support::app::{styled_window_application, xaml_test_base};
use crate::support::loader::{parse, try_load};
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::media::{FontStyle, FontWeight};
use ferroui_base::interactivity::IRoutedEventArgs;
use ferroui_base::metadata::{from_markup_value, into_markup_value, MarkupType, MarkupTyped};
use ferroui_base::Ref;
use ferroui_controls::documents::{InlineSnapshotKind, TextHighlightRange};
use ferroui_controls::{
    Button, ContextMenuEventArgs, Control, RichEditBox, RichEditBoxSelectionChangingEventArgs,
    RichEditBoxTextChangingEventArgs, RichEditCompositionEventArgs, RichTextBlock, RichTextBlockOverflow,
    TextBox, UserControl, Window,
};

const DOCUMENT: &str = include_str!("../emitter/fixtures/rich_text_document.xaml");

#[test]
fn rich_context_menu_arguments_metadata_keeps_coordinates_read_only_and_handled_live() {
    let _scope = xaml_test_base();
    let markup = MarkupType::find("FerroUI.Controls", "ContextMenuEventArgs")
        .expect("context-menu event arguments are registered for markup");
    assert!(std::ptr::eq(markup, <ContextMenuEventArgs as MarkupTyped>::MARKUP));
    assert!(markup.constructors.is_empty(), "event arguments are supplied by the control");
    let args = ContextMenuEventArgs::new(12.5, -4.25);
    let value = into_markup_value(args.share());
    for (name, expected) in [("CursorLeft", 12.5), ("CursorTop", -4.25)] {
        let property = markup.find_property(name).expect("the coordinate is exposed through metadata");
        assert!(property.set.is_none(), "{name} is read-only");
        let actual = (property.get.expect("the coordinate has a getter"))(&[value.clone()]).unwrap();
        assert_eq!(from_markup_value::<f64>(&actual), Some(expected));
    }
    let base = markup.base_type().expect("the metadata retains the routed-argument base");
    let handled = base.find_property("Handled").expect("Handled is inherited from routed-event arguments");
    (handled.set.expect("Handled has a setter"))(&[value, into_markup_value(true)]).unwrap();
    assert!(args.handled(), "metadata mutates the live arguments the control raised");
}

#[test]
fn rich_composition_event_arguments_metadata_reads_the_shared_handler_payload() {
    let _scope = xaml_test_base();
    let mut composition = RichEditCompositionEventArgs::default();
    composition.text = "accepted 😀".to_owned();
    let composition_value = into_markup_value(composition.share());
    let composition_type = <RichEditCompositionEventArgs as MarkupTyped>::MARKUP;
    let text = composition_type.find_property("Text").unwrap();
    assert!(text.set.is_none());
    let actual = (text.get.unwrap())(&[composition_value]).expect("composition metadata reads the shared handler arguments");
    assert_eq!(from_markup_value::<String>(&actual).as_deref(), Some("accepted 😀"));
    let base = composition_type.base_type().expect("composition arguments retain their routed-argument base");
    assert!(base.find_property("Handled").is_some());
}

#[test]
fn rich_selection_changing_event_arguments_metadata_mutates_the_shared_cancel_flag() {
    let _scope = xaml_test_base();
    let selection = RichEditBoxSelectionChangingEventArgs::default();
    let selection_value = into_markup_value(selection.share());
    let selection_type = <RichEditBoxSelectionChangingEventArgs as MarkupTyped>::MARKUP;
    let cancel = selection_type.find_property("Cancel").unwrap();
    (cancel.set.unwrap())(&[selection_value.clone(), into_markup_value(true)])
        .expect("selection metadata mutates the live cancel flag");
    assert!(selection.cancel());
    assert_eq!(from_markup_value::<bool>(&(cancel.get.unwrap())(&[selection_value]).unwrap()), Some(true));
    let base = selection_type.base_type().expect("selection-changing arguments retain their routed-argument base");
    assert!(base.find_property("Handled").is_some());
}

#[test]
fn rich_text_changing_event_arguments_metadata_reads_the_shared_handler_payload() {
    let _scope = xaml_test_base();
    let changing = RichEditBoxTextChangingEventArgs::default();
    let changing_type = <RichEditBoxTextChangingEventArgs as MarkupTyped>::MARKUP;
    let content = changing_type.find_property("IsContentChanging").unwrap();
    assert!(content.set.is_none());
    let actual = (content.get.unwrap())(&[into_markup_value(changing.share())])
        .expect("text-changing metadata reads the shared handler arguments");
    assert_eq!(from_markup_value::<bool>(&actual), Some(false));
    let base = changing_type.base_type().expect("text-changing arguments retain their routed-argument base");
    assert!(base.find_property("Handled").is_some());
}

#[test]
fn rich_text_markup_preserves_nested_document_content_and_copied_highlight_values() {
    let _scope = xaml_test_base();
    let root = parse::<Ref<UserControl>>(DOCUMENT);
    let display = root.get_control::<RichTextBlock>("display");
    assert_eq!(display.text(), "Bold italic 😀\nSecond");
    let snapshot = display.document().snapshot();
    assert_eq!(snapshot.paragraphs.len(), 2);
    assert!(matches!(
        snapshot.paragraphs[0].inlines[0].kind,
        InlineSnapshotKind::Bold(_)
    ));
    assert!(matches!(
        snapshot.paragraphs[0].inlines[1].kind,
        InlineSnapshotKind::Italic(_)
    ));
    assert!(matches!(
        snapshot.paragraphs[0].inlines[2].kind,
        InlineSnapshotKind::Underline(_)
    ));
    assert_eq!(
        display
            .document()
            .character_format_at(0)
            .unwrap()
            .font_weight,
        Some(FontWeight::Bold)
    );
    assert_eq!(
        display
            .document()
            .character_format_at(5)
            .unwrap()
            .font_style,
        Some(FontStyle::Italic)
    );
    let highlighter = display.text_highlighters().get(0);
    assert_eq!(highlighter.ranges().to_vec(), vec![
        TextHighlightRange::new(0, 4),
        TextHighlightRange::new(5, 6),
        TextHighlightRange::new(12, 2),
    ]);
    let copied = highlighter
        .ranges()
        .get(0)
        .expect("the declared highlight range is loaded");
    assert_eq!(copied, TextHighlightRange::new(0, 4));
    copied.set_length(9);
    assert_eq!(copied.length(), 9);
    assert_eq!(
        highlighter
            .ranges()
            .get(0)
            .expect("the stored highlight range remains present")
            .length(),
        4
    );
}

#[test]
fn rich_text_range_markup_initializes_a_value_and_preserves_signed_fields() {
    let _scope = xaml_test_base();
    let range = parse::<TextHighlightRange>(r#"<TextRange xmlns="https://github.com/ferroui" StartIndex="-2" Length="2147483647" />"#);
    assert_eq!(range.start_index(), -2);
    assert_eq!(range.length(), i32::MAX);
    assert!(range.range().is_none());
    let empty = parse::<TextHighlightRange>(r#"<TextRange xmlns="https://github.com/ferroui" />"#);
    assert_eq!(empty, TextHighlightRange::new(0, 0));
}

#[test]
fn rich_text_range_markup_rejects_invalid_fields_and_constructor_arguments() {
    let _scope = xaml_test_base();
    for document in [
        r#"<TextRange xmlns="https://github.com/ferroui" StartIndex="2147483648" />"#,
        r#"<TextRange xmlns="https://github.com/ferroui" Length="invalid" />"#,
        r#"<TextRange xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><x:Arguments><x:Int32>1</x:Int32></x:Arguments></TextRange>"#,
        r#"<TextRange xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><x:Arguments><x:String>invalid</x:String><x:Int32>1</x:Int32></x:Arguments></TextRange>"#,
    ] {
        assert!(try_load(document).is_err(), "the range must reject invalid input: {document}");
    }
}

#[test]
fn rich_text_range_initialization_keeps_the_normal_value_type_guard() {
    let _scope = xaml_test_base();
    let error = try_load(r#"<Thickness xmlns="https://github.com/ferroui" />"#)
        .expect_err("unrelated value types still require their supported conversion syntax");
    assert!(error.message().contains("Value types can only be loaded via converters"), "{}", error.message());
}

#[test]
fn rich_edit_markup_commands_bind_the_real_owner_and_follow_read_only_state() {
    let _app = styled_window_application();
    let root = parse::<Ref<UserControl>>(DOCUMENT);
    let editor = root.get_control::<RichEditBox>("editor");
    let button = root.get_control::<Button>("bold");
    let window = Window::new();
    window.set_content(Some(Control::boxed(root)));
    window.show();
    editor.select_all();
    assert_eq!(editor.text(), "Edit 😀");
    assert_eq!(
        from_markup_value::<String>(&editor.header()),
        Some("Document".to_owned())
    );
    assert!(button.is_enabled());
    button
        .command()
        .expect("the markup command resolves ToggleBold")
        .execute(None);
    assert!(editor.is_bold());
    assert!(editor.can_undo());
    editor.set_is_read_only(true);
    assert!(!button.is_enabled());
    window.close();
}

#[test]
fn rich_edit_text_binding_updates_the_document_in_both_directions() {
    let _app = styled_window_application();
    let root = parse::<Ref<UserControl>>(
        r#"
      <UserControl xmlns="https://github.com/ferroui">
        <StackPanel>
          <TextBox Name="source" Text="first" />
          <RichEditBox Name="editor" Text="{Binding #source.Text, Mode=TwoWay}" />
        </StackPanel>
      </UserControl>"#,
    );
    let source = root.get_control::<TextBox>("source");
    let editor = root.get_control::<RichEditBox>("editor");
    let window = Window::new();
    window.set_content(Some(Control::boxed(root)));
    window.show();
    assert_eq!(editor.text(), "first");
    source.set_text(Some("second 😀"));
    assert_eq!(editor.text(), "second 😀");
    editor.select_all();
    editor.insert_text("replacement").unwrap();
    assert_eq!(source.text().as_deref(), Some("replacement"));
    assert!(editor.undo().unwrap());
    assert_eq!(source.text().as_deref(), Some("second 😀"));
    assert!(editor.can_redo());
    assert!(editor.redo().unwrap());
    assert_eq!(source.text().as_deref(), Some("replacement"));
    assert!(editor.can_undo());
    window.close();
}

#[test]
fn rich_text_markup_links_overflow_to_the_same_document() {
    let _app = styled_window_application();
    let _text = TextTestScope::new();
    let root = parse::<Ref<UserControl>>(
        r#"
      <UserControl xmlns="https://github.com/ferroui">
        <StackPanel>
          <RichTextBlock Name="source" Height="20" Width="80" TextWrapping="Wrap" OverflowContentTarget="{Binding #overflow}">
            <Paragraph><Run Text="First second third fourth fifth sixth" /></Paragraph>
          </RichTextBlock>
          <RichTextBlockOverflow Name="overflow" Width="80" Height="120" />
        </StackPanel>
      </UserControl>"#,
    );
    let source = root.get_control::<RichTextBlock>("source");
    let overflow = root.get_control::<RichTextBlockOverflow>("overflow");
    let window = Window::new();
    window.set_content(Some(Control::boxed(root)));
    window.show();
    window.update_layout();
    assert_eq!(source.overflow_content_target(), Some(overflow.clone()));
    assert_eq!(source.document(), overflow.document());
    assert!(source.has_overflow_content());
    assert!(!overflow.line_metrics().is_empty());
    window.close();
}

#[test]
fn rich_input_scope_markup_preserves_objects_order_and_live_name_changes() {
    use ferroui_base::input::InputScopeNameValue as Scope;
    use ferroui_base::input::text_input::TextInputOptions;
    let _scope = xaml_test_base();
    let root = parse::<Ref<UserControl>>(include_str!("../emitter/fixtures/rich_input_scope.xaml"));
    let short = root.get_control::<RichEditBox>("shortScope");
    assert_eq!(&*short.input_scope().unwrap().snapshot(), &[Scope::EmailSmtpAddress]);
    let full = root.get_control::<RichEditBox>("fullScope");
    let scope = full.input_scope().unwrap();
    assert_eq!(&*scope.snapshot(), &[Scope::Hiragana, Scope::KatakanaFullWidth]);
    let options = TextInputOptions::from_styled_element(&full);
    assert_eq!(options.effective_input_scope(), Some(Scope::Hiragana));
    scope.names().get(0).set_name_value(Scope::ChatWithoutEmoji);
    assert_eq!(TextInputOptions::from_styled_element(&full).effective_input_scope(), Some(Scope::ChatWithoutEmoji));
    assert_eq!(options.effective_input_scope(), Some(Scope::Hiragana));
    assert!(root.get_control::<RichEditBox>("nullScope").input_scope().is_none());
}

#[test]
fn rich_input_scope_markup_accepts_every_defined_name_and_rejects_invalid_text() {
    use ferroui_base::input::InputScopeNameValue as Scope;
    let _scope = xaml_test_base();
    for scope in Scope::ALL {
        let source = format!(r#"<RichEditBox xmlns="https://github.com/ferroui" InputScope="{scope:?}" />"#);
        let editor = parse::<Ref<RichEditBox>>(&source);
        assert_eq!(&*editor.input_scope().unwrap().snapshot(), &[scope]);
    }
    for value in ["unknown", "2", "-1", "Url,Number"] {
        let source = format!(r#"<RichEditBox xmlns="https://github.com/ferroui" InputScope="{value}" />"#);
        assert!(try_load(&source).is_err(), "{source}");
    }

    // The framework drops whitespace-only values of non-string properties
    // before conversion. An empty attribute is therefore not a request to
    // parse an empty name or to clear an existing scope; x:Null clears it.
    use crate::support::helpers::boxed;
    use crate::support::loader::load_with_root;
    let editor = RichEditBox::new();
    editor.set_input_scope(Scope::Url);
    load_with_root(
        r#"<RichEditBox xmlns="https://github.com/ferroui" InputScope="" />"#,
        None,
        boxed(editor.clone()),
    );
    assert_eq!(&*editor.input_scope().unwrap().snapshot(), &[Scope::Url]);
    load_with_root(
        r#"<RichEditBox xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" InputScope="{x:Null}" />"#,
        None,
        boxed(editor.clone()),
    );
    assert!(editor.input_scope().is_none());
}

#[test]
fn rich_input_scope_name_markup_supports_native_enum_constructor_and_content() {
    use ferroui_base::input::{InputScopeName, InputScopeNameValue as Scope};
    let _scope = xaml_test_base();
    for source in [
        r#"<InputScopeName xmlns="https://github.com/ferroui" NameValue="TelephoneNumber" />"#,
        r#"<InputScopeName xmlns="https://github.com/ferroui">TelephoneNumber</InputScopeName>"#,
        r#"<InputScopeName xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><x:Arguments><InputScopeNameValue>TelephoneNumber</InputScopeNameValue></x:Arguments></InputScopeName>"#,
    ] {
        assert_eq!(parse::<Ref<InputScopeName>>(source).name_value(), Scope::TelephoneNumber);
    }
}

#[test]
fn rich_text_find_name_uses_xaml_names_across_controls_and_text_elements() {
    use ferroui_controls::documents::{Hyperlink, Paragraph, Run};

    let _scope = xaml_test_base();
    let root = parse::<Ref<UserControl>>(
        r#"
<UserControl xmlns="https://github.com/ferroui"
             xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">
    <StackPanel>
        <Button x:Name="action" />
        <RichTextBlock x:Name="display">
            <Paragraph x:Name="paragraph">
                <Hyperlink x:Name="link" NavigateUri="https://example.test">
                    <Run x:Name="source" Text="named content" />
                </Hyperlink>
            </Paragraph>
        </RichTextBlock>
    </StackPanel>
</UserControl>"#,
    );
    let source = root.get_control::<Run>("source");
    let paragraph = root.get_control::<Paragraph>("paragraph");
    let link = root.get_control::<Hyperlink>("link");
    let action = root.get_control::<Button>("action");
    let display = root.get_control::<RichTextBlock>("display");
    assert_eq!(source.name().as_deref(), Some("source"));
    assert_eq!(source.find_name("source"), Some(source.clone().upcast()));
    assert_eq!(
        source.find_name("paragraph"),
        Some(paragraph.clone().upcast())
    );
    assert_eq!(source.find_name("link"), Some(link.upcast()));
    assert_eq!(source.find_name("action"), Some(action.upcast()));
    assert_eq!(paragraph.find_name("display"), Some(display.upcast()));
    assert_eq!(paragraph.find_name("source"), Some(source.upcast()));
    assert!(paragraph.find_name("missing").is_none());
}

fn apply_rich_text_template_with_scope(
    control: &ferroui_controls::primitives::TemplatedControl,
) -> ferroui_base::controls::NameScopeRef {
    use ferroui_controls::primitives::TemplatedControl;
    use std::cell::RefCell;
    use std::rc::Rc;

    let applied = Rc::new(RefCell::new(None));
    let observed = applied.clone();
    let token = control.template_applied(move |_, event| {
        assert!(observed.borrow().is_none());
        *observed.borrow_mut() = Some(event.name_scope().clone());
    });
    control.apply_template();
    assert!(control.remove_handler(TemplatedControl::template_applied_event(), token));
    let scope = applied.borrow_mut().take().expect("the actual TemplateApplied scope");
    scope
}

#[test]
fn rich_text_find_name_uses_each_xaml_template_instance_scope() {
    use ferroui_controls::documents::Run;
    use ferroui_controls::{ContentControl, StackPanel};

    let _scope = xaml_test_base();
    let root = parse::<Ref<UserControl>>(
        r#"
<UserControl xmlns="https://github.com/ferroui"
             xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">
    <UserControl.Resources>
        <ControlTemplate x:Key="textTemplate">
            <StackPanel x:Name="templateRoot">
                <Button x:Name="target" />
                <RichTextBlock>
                    <Paragraph><Run x:Name="source" Text="template content" /></Paragraph>
                </RichTextBlock>
            </StackPanel>
        </ControlTemplate>
    </UserControl.Resources>
    <StackPanel>
        <Button x:Name="target" />
        <Button x:Name="outerOnly" />
        <ContentControl x:Name="first" Template="{StaticResource textTemplate}" />
        <ContentControl x:Name="second" Template="{StaticResource textTemplate}" />
    </StackPanel>
</UserControl>"#,
    );
    let first = root.get_control::<ContentControl>("first");
    let second = root.get_control::<ContentControl>("second");
    let outer_target = root.get_control::<Button>("target");
    let outer_only = root.get_control::<Button>("outerOnly");
    let first_scope = apply_rich_text_template_with_scope(&first);
    let second_scope = apply_rich_text_template_with_scope(&second);
    assert!(first_scope != second_scope);
    // Get the source from the actual template contract. FindName below must
    // discover that scope itself; the fixture attaches no name scopes.
    let first_source = first_scope.get_as::<Run>("source");
    let second_source = second_scope.get_as::<Run>("source");
    let first_target = first_scope.get_as::<Button>("target");
    let second_target = second_scope.get_as::<Button>("target");
    let first_root = first_scope.get_as::<StackPanel>("templateRoot");
    let second_root = second_scope.get_as::<StackPanel>("templateRoot");
    assert_eq!(first_root, first.get_visual_children()[0]);
    assert_eq!(second_root, second.get_visual_children()[0]);
    assert_eq!(first_source.find_name("templateRoot"), Some(first_root.upcast()));
    assert_eq!(second_source.find_name("templateRoot"), Some(second_root.upcast()));
    assert_ne!(first_source, second_source);
    assert_ne!(first_target, second_target);
    assert_ne!(first_target, outer_target);
    assert_ne!(second_target, outer_target);
    assert_eq!(
        first_source.find_name("source"),
        Some(first_source.clone().upcast())
    );
    assert_eq!(
        second_source.find_name("source"),
        Some(second_source.clone().upcast())
    );
    assert_eq!(
        first_source.find_name("target"),
        Some(first_target.upcast())
    );
    assert_eq!(
        second_source.find_name("target"),
        Some(second_target.upcast())
    );
    // FerroUI's completed ChildNameScope keeps its existing fallback to
    // the containing XAML scope; local template names still win above.
    assert_eq!(
        first_source.find_name("outerOnly"),
        Some(outer_only.clone().upcast())
    );
    assert_eq!(
        second_source.find_name("outerOnly"),
        Some(outer_only.upcast())
    );
    assert!(root.find_control::<Run>("source").is_none());
}


#[test]
fn rich_text_find_name_template_replacement_releases_old_scope_and_declaring_tree() {
    use ferroui_base::controls::NameScope;
    use ferroui_controls::documents::Run;
    use ferroui_controls::{ContentControl, StackPanel};
    use std::rc::Rc;

    let _scope = xaml_test_base();
    let root = parse::<Ref<UserControl>>(
        r#"
<UserControl xmlns="https://github.com/ferroui"
             xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">
    <UserControl.Resources>
        <ControlTemplate x:Key="firstTemplate">
            <StackPanel x:Name="templateRoot">
                <Button x:Name="target" />
                <RichTextBlock>
                    <Paragraph><Run x:Name="source" Text="first" /></Paragraph>
                </RichTextBlock>
            </StackPanel>
        </ControlTemplate>
        <ControlTemplate x:Key="replacementTemplate">
            <StackPanel x:Name="templateRoot">
                <Button x:Name="target" />
                <RichTextBlock>
                    <Paragraph><Run x:Name="source" Text="replacement" /></Paragraph>
                </RichTextBlock>
            </StackPanel>
        </ControlTemplate>
    </UserControl.Resources>
    <StackPanel>
        <Button x:Name="outerOnly" />
        <ContentControl x:Name="owner" Template="{StaticResource firstTemplate}" />
        <ContentControl x:Name="replacement" Template="{StaticResource replacementTemplate}" />
    </StackPanel>
</UserControl>"#,
    );
    let owner = root.get_control::<ContentControl>("owner");
    let replacement = root.get_control::<ContentControl>("replacement");
    let outer = NameScope::get_name_scope(&root).unwrap();
    let weak_outer = Rc::downgrade(&outer.0);
    let weak_root = root.downgrade();
    let weak_owner = owner.downgrade();
    let first_scope = apply_rich_text_template_with_scope(&owner);
    let weak_first_scope = Rc::downgrade(&first_scope.0);
    let first_source = first_scope.get_as::<Run>("source");
    let weak_first_source = first_source.downgrade();
    let weak_first_root = first_scope.get_as::<StackPanel>("templateRoot").downgrade();
    assert_eq!(first_source.text().as_deref(), Some("first"));
    assert_eq!(first_source.find_name("source"), Some(first_source.clone().upcast()));
    drop(first_source);
    drop(first_scope);

    owner.set_template(replacement.template());
    let current_scope = apply_rich_text_template_with_scope(&owner);
    assert!(weak_first_scope.upgrade().is_none());
    assert!(weak_first_source.upgrade().is_none());
    assert!(weak_first_root.upgrade().is_none());
    let source = current_scope.get_as::<Run>("source");
    let target = current_scope.get_as::<Button>("target");
    assert_eq!(source.text().as_deref(), Some("replacement"));
    assert_eq!(source.find_name("source"), Some(source.clone().upcast()));
    assert_eq!(source.find_name("target"), Some(target.clone().upcast()));
    assert_eq!(source.find_name("outerOnly"), outer.find("outerOnly"));
    assert!(root.find_control::<Run>("source").is_none());

    let weak_current_scope = Rc::downgrade(&current_scope.0);
    let weak_current_source = source.downgrade();
    let weak_current_target = target.downgrade();
    drop(current_scope);
    drop(source);
    drop(target);
    drop(outer);
    drop(replacement);
    drop(owner);
    drop(root);
    assert!(weak_root.upgrade().is_none());
    assert!(weak_outer.upgrade().is_none());
    assert!(weak_owner.upgrade().is_none());
    assert!(weak_current_scope.upgrade().is_none());
    assert!(weak_current_source.upgrade().is_none());
    assert!(weak_current_target.upgrade().is_none());
}
