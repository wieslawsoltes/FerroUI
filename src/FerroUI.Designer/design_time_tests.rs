use super::*;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{ContentControl, Control, ItemsControl, TextBlock};
use std::rc::Rc;

const NS: &str = "xmlns='https://github.com/ferroui' xmlns:d='http://schemas.microsoft.com/expression/blend/2008' xmlns:mc='http://schemas.openxmlformats.org/markup-compatibility/2006' mc:Ignorable='d'";
fn named(document: &Document, name: &str) -> NodeId {
    document.ast().find_name(name).unwrap()
}

#[test]
fn design_edits_preserve_application_values_aliases_and_one_transaction() {
    let source = "<TextBlock xmlns='https://github.com/ferroui' xmlns:d='using:Application' xmlns:preview='http://schemas.microsoft.com/expression/blend/2008' xmlns:c='http://schemas.openxmlformats.org/markup-compatibility/2006' c:Ignorable='preview other' Text='{Binding Title}' d:Other='retain'><TextBlock.Tag><!-- keep --></TextBlock.Tag></TextBlock>";
    let mut document = Document::new(source).unwrap();
    let root = document.ast().root();
    let edits = plan_design_time_edits(
        &document,
        &[
            DesignTimeEdit::SetProperty {
                node: root,
                property: "Text".into(),
                value: Some("Preview & text".into()),
            },
            DesignTimeEdit::SetProperty {
                node: root,
                property: "DesignWidth".into(),
                value: Some("420".into()),
            },
        ],
    )
    .unwrap();
    document.apply("Design data", edits).unwrap();
    assert!(document.source().contains("Text='{Binding Title}'"));
    assert!(document.source().contains("d:Other='retain'"));
    assert!(document.source().contains("preview:Text="));
    assert!(document.source().contains("c:Ignorable='preview other'"));
    assert!(document.source().contains("<!-- keep -->"));
    assert!(
        plan_design_time_edits(
            &document,
            &[DesignTimeEdit::SetProperty {
                node: root,
                property: "Text".into(),
                value: Some("Preview & text".into())
            }]
        )
        .unwrap()
        .is_empty()
    );
    document.undo().unwrap();
    assert_eq!(document.source(), source);
}

#[test]
fn design_edit_batch_rejects_invalid_values_before_any_change() {
    let document = Document::new("<TextBlock Text='Runtime'/>").unwrap();
    let before = document.source().to_owned();
    let commands = [
        DesignTimeEdit::SetProperty {
            node: document.ast().root(),
            property: "Text".into(),
            value: Some("Preview".into()),
        },
        DesignTimeEdit::SampleItems {
            node: document.ast().root(),
            count: Some(201),
        },
    ];
    assert!(plan_design_time_edits(&document, &commands).is_err());
    assert_eq!(document.source(), before);
    assert!(
        plan_design_time_edits(
            &document,
            &[DesignTimeEdit::SetProperty {
                node: document.ast().root(),
                property: "DesignWidth".into(),
                value: Some("NaN".into())
            }]
        )
        .is_err()
    );
}

#[test]
fn design_projection_maps_unicode_siblings_and_honors_namespace_identity() {
    let source = format!(
        "<StackPanel {NS} xmlns:app='using:Application'><TextBlock Name='One' Text='Żółć' d:Text='Long sample text'/><TextBlock Name='Two' Text='two' app:Text='custom'/><d:Button Name='Placeholder' Content='Design only'/></StackPanel>"
    );
    let document = Document::new(&source).unwrap();
    let projection = project(&document, &source, true).unwrap().unwrap();
    let one = projection
        .document
        .ast()
        .node(named(&projection.document, "One"))
        .unwrap();
    assert_eq!(one.attribute("Text"), Some("Long sample text"));
    let two = projection
        .document
        .ast()
        .node(named(&projection.document, "Two"))
        .unwrap();
    assert_eq!(two.attribute("app:Text"), Some("custom"));
    for name in ["One", "Two", "Placeholder"] {
        assert_eq!(
            projection
                .identities
                .get(&named(&projection.document, name)),
            Some(&named(&document, name))
        );
    }
    assert_eq!(
        projection
            .document
            .ast()
            .node(named(&projection.document, "Placeholder"))
            .unwrap()
            .namespace_uri
            .as_deref(),
        Some(FRAMEWORK)
    );
    assert_eq!(document.source(), source);
}

#[test]
fn design_projection_overrides_property_elements_and_can_disable_all_design_values() {
    let source = format!(
        "<ContentControl {NS} Content='Runtime'><d:ContentControl.Content><TextBlock Name='Sample' Text='Preview'/></d:ContentControl.Content></ContentControl>"
    );
    let document = Document::new(&source).unwrap();
    let projection = project(&document, &source, true).unwrap().unwrap();
    let root = projection
        .document
        .ast()
        .node(projection.document.ast().root())
        .unwrap();
    assert!(root.attribute("Content").is_none());
    assert_eq!(
        projection
            .identities
            .get(&named(&projection.document, "Sample")),
        Some(&named(&document, "Sample"))
    );
    let disabled = project(&document, &source, false).unwrap().unwrap();
    assert_eq!(
        disabled
            .document
            .ast()
            .node(disabled.document.ast().root())
            .unwrap()
            .attribute("Content"),
        Some("Runtime")
    );
    assert!(disabled.document.ast().find_name("Sample").is_none());
}

#[test]
fn design_projection_rejects_duplicate_effective_properties_and_fake_sample_extensions() {
    let source = format!("<Grid {NS}><Border d:DesignWidth='200' d:Width='300'/></Grid>");
    let document = Document::new(&source).unwrap();
    assert!(
        project(&document, &source, true)
            .err()
            .unwrap()
            .contains("more than one")
    );
    let source = format!(
        "<ItemsControl {NS} xmlns:app='using:App' d:ItemsSource='{{app:SampleData ItemCount=3}}'/>"
    );
    let document = Document::new(&source).unwrap();
    let projection = project(&document, &source, true).unwrap().unwrap();
    assert!(projection.samples.is_empty());
    assert_eq!(
        projection
            .document
            .ast()
            .node(projection.document.ast().root())
            .unwrap()
            .attribute("ItemsSource"),
        Some("{app:SampleData ItemCount=3}")
    );
}

#[test]
fn design_object_authoring_and_reset_leave_runtime_property_elements_intact() {
    let source = "<ContentControl xmlns='https://github.com/ferroui'><ContentControl.Content><TextBlock Text='Runtime'/></ContentControl.Content></ContentControl>";
    let mut document = Document::new(source).unwrap();
    let root = document.ast().root();
    let command = DesignTimeEdit::SetObject {
        node: root,
        property: "Content".into(),
        source: Some("<TextBlock Text='Preview'/><!-- draft comment -->".into()),
    };
    let edits = plan_design_time_edits(&document, &[command]).unwrap();
    document.apply("Design object", edits).unwrap();
    assert!(design_time_properties(&document, root).unwrap()[0].object);
    assert!(
        document.source().contains(
            "<ContentControl.Content><TextBlock Text='Runtime'/></ContentControl.Content>"
        )
    );
    let reset = plan_design_time_edits(
        &document,
        &[DesignTimeEdit::SetProperty {
            node: root,
            property: "Content".into(),
            value: None,
        }],
    )
    .unwrap();
    document.apply("Reset", reset).unwrap();
    assert!(design_time_properties(&document, root).unwrap().is_empty());
    assert!(document.source().contains("<TextBlock Text='Runtime'/>"));
}

#[test]
fn native_design_values_toggle_without_source_history_changes_and_keep_mapping() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = format!(
        "<TextBlock {NS} Name='Title' Text='Application' d:Text='Preview' d:Width='420' d:DesignWidth='760'/>"
    );
    let mut session = DesignerSession::new(source.clone(), ContentControl::new()).unwrap();
    let root = session.document().ast().root();
    assert!(
        matches!(session.preview_status(), crate::PreviewStatus::Ready { .. }),
        "{:?}",
        session.preview_status()
    );
    let preview = session
        .preview
        .visual(root)
        .unwrap()
        .cast::<TextBlock>()
        .unwrap();
    assert_eq!(preview.text().as_deref(), Some("Preview"));
    assert_eq!(preview.width(), 420.0);
    assert_eq!(session.design_preview_size(), (Some(760.0), None));
    let revision = session.document().revision();
    session.set_design_time_enabled(false).unwrap();
    assert_eq!(
        session
            .preview
            .visual(root)
            .unwrap()
            .cast::<TextBlock>()
            .unwrap()
            .text()
            .as_deref(),
        Some("Application")
    );
    assert_eq!(session.document().source(), source);
    assert_eq!(session.document().revision(), revision);
}

#[test]
fn native_sample_records_feed_nested_bindings_and_leave_application_source_unchanged() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = format!(
        "<StackPanel {NS} d:DataContext='{{d:SampleData}}'><TextBlock Name='Name' Text='{{Binding Person.FirstName}}' Grid.Row='{{Binding Row}}'/><CheckBox Name='Active' IsChecked='{{Binding IsActive}}'/><Slider Name='Amount' Value='{{Binding Amount}}'/></StackPanel>"
    );
    let session = DesignerSession::new(source.clone(), ContentControl::new()).unwrap();
    assert!(
        matches!(session.preview_status(), crate::PreviewStatus::Ready { .. }),
        "{:?}",
        session.preview_status()
    );
    let text = session
        .preview
        .visual(named(session.document(), "Name"))
        .unwrap()
        .cast::<TextBlock>()
        .unwrap();
    assert_eq!(text.text().as_deref(), Some("First Name 1"));
    let check = session
        .preview
        .visual(named(session.document(), "Active"))
        .unwrap()
        .cast::<ferroui_controls::CheckBox>()
        .unwrap();
    assert_eq!(check.is_checked(), Some(true));
    assert_eq!(ferroui_controls::Grid::get_row(&text), 10);
    let amount = session
        .preview
        .visual(named(session.document(), "Amount"))
        .unwrap()
        .cast::<ferroui_controls::Slider>()
        .unwrap();
    assert_eq!(amount.value(), 10.0);
    assert_eq!(session.document().source(), source);
}

#[test]
fn native_sample_items_use_requested_count_and_preserve_items_binding() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = format!(
        "<ItemsControl {NS} ItemsSource='{{Binding ActualItems}}' d:ItemsSource='{{d:SampleData ItemCount=3}}'><ItemsControl.ItemTemplate><DataTemplate><TextBlock Text='{{Binding Title}}'/></DataTemplate></ItemsControl.ItemTemplate></ItemsControl>"
    );
    let session = DesignerSession::new(source.clone(), ContentControl::new()).unwrap();
    assert!(
        matches!(session.preview_status(), crate::PreviewStatus::Ready { .. }),
        "{:?}",
        session.preview_status()
    );
    let control = session
        .preview
        .visual(session.document().ast().root())
        .unwrap()
        .cast::<ItemsControl>()
        .unwrap();
    assert_eq!(
        control.items_source().unwrap().count(),
        3,
        "{:?}",
        session.preview_notices()
    );
    assert_eq!(session.document().source(), source);
}

#[test]
fn design_data_drafts_reject_source_selection_scope_and_permission_changes() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut session = DesignerSession::new(
        "<TextBlock xmlns='https://github.com/ferroui' Text='Runtime'/>",
        ContentControl::new(),
    )
    .unwrap();
    let node = session.document().ast().root();
    session.select([node]);
    let snapshot = DesignTimeSnapshot::capture(&session);
    session.select([]);
    session.select([node]);
    assert!(
        !snapshot.is_current(&session),
        "A selection round trip retires the draft."
    );
    let snapshot = DesignTimeSnapshot::capture(&session);
    session.set_design_time_enabled(false).unwrap();
    assert!(!snapshot.is_current(&session));
    let snapshot = DesignTimeSnapshot::capture(&session);
    session.set_read_only(true);
    let edits = [DesignTimeEdit::SetProperty {
        node,
        property: "Text".into(),
        value: Some("Preview".into()),
    }];
    assert!(snapshot.apply(&mut session, &edits).is_err());
    session.set_read_only(false);
    let snapshot = DesignTimeSnapshot::capture(&session);
    snapshot.apply(&mut session, &edits).unwrap();
    assert!(!snapshot.is_current(&session));
    assert_eq!(
        session.design_time_properties(node).unwrap()[0].value,
        "Preview"
    );
}

#[test]
fn registered_factory_receives_authored_source_and_populates_projected_values() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = format!(
        "<TextBlock {NS} xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' x:Class='Example.Title' Text='Runtime' d:Text='Preview'/>"
    );
    let mut session = DesignerSession::new(source.clone(), ContentControl::new()).unwrap();
    let expected = source.clone();
    let mut options = crate::PreviewHostOptions::default();
    options.register_root(
        "Example.Title",
        Rc::new(move |request| {
            assert_eq!(request.source, expected);
            assert!(
                request
                    .preview_source
                    .as_deref()
                    .unwrap()
                    .contains("Text='Preview'")
            );
            request.populate(Some(Control::boxed(TextBlock::new())))
        }),
    );
    session.configure_preview(options).unwrap();
    assert_eq!(
        session
            .preview
            .visual(session.document().ast().root())
            .unwrap()
            .cast::<TextBlock>()
            .unwrap()
            .text()
            .as_deref(),
        Some("Preview")
    );
    assert_eq!(session.document().source(), source);
}

#[test]
fn native_sample_data_uses_reflection_only_in_the_generated_preview_scope() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = format!(
        "<StackPanel {NS} xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' x:CompileBindings='True' x:DataType='TextBlock' d:DataContext='{{d:SampleData}}'><TextBlock Name='Title' Text='{{Binding Text}}'/></StackPanel>"
    );
    let session = DesignerSession::new(source.clone(), ContentControl::new()).unwrap();
    assert!(
        matches!(session.preview_status(), crate::PreviewStatus::Ready { .. }),
        "{:?}",
        session.preview_status()
    );
    let text = session
        .preview
        .visual(named(session.document(), "Title"))
        .unwrap()
        .cast::<TextBlock>()
        .unwrap();
    assert_eq!(text.text().as_deref(), Some("Text 1"));
    assert_eq!(session.document().source(), source);
    assert!(!source.contains("ReflectionBinding"));
}

#[test]
fn native_sample_assignment_does_not_touch_a_same_line_sibling() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = format!(
        "<StackPanel {NS}><StackPanel Name='Sample' d:DataContext='{{d:SampleData}}'><TextBlock Name='First' Text='{{Binding Name}}'/></StackPanel><StackPanel Name='Untouched'><TextBlock Name='Second' Text='{{Binding Name}}'/></StackPanel></StackPanel>"
    );
    let session = DesignerSession::new(source, ContentControl::new()).unwrap();
    assert!(
        matches!(session.preview_status(), crate::PreviewStatus::Ready { .. }),
        "{:?}",
        session.preview_status()
    );
    let first = session
        .preview
        .visual(named(session.document(), "First"))
        .unwrap()
        .cast::<TextBlock>()
        .unwrap();
    assert_eq!(first.text().as_deref(), Some("Name 1"));
    let untouched = session
        .preview
        .visual(named(session.document(), "Untouched"))
        .unwrap()
        .cast::<ferroui_base::StyledElement>()
        .unwrap();
    assert!(untouched.data_context().is_none());
}

#[test]
fn native_top_level_preview_uses_the_same_sample_data_as_the_artboard() {
    use ferroui_markup_xaml::FromXamlObject;
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = format!("<ItemsControl {NS} d:ItemsSource='{{d:SampleData ItemCount=4}}'/>");
    let mut session = DesignerSession::new(source.clone(), ContentControl::new()).unwrap();
    let window = session.open_native_preview().unwrap();
    let content = window.content().unwrap();
    let items = ferroui_base::Ref::<ItemsControl>::from_xaml_object(&content).unwrap();
    assert_eq!(items.items_source().unwrap().count(), 4);
    assert_eq!(session.document().source(), source);
    session.close_native_preview();
}

#[test]
fn root_design_size_hints_do_not_fix_native_root_layout_dimensions() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = format!("<Border {NS} d:DesignWidth='760' d:DesignHeight='480'/>");
    let session = DesignerSession::new(source, ContentControl::new()).unwrap();
    assert_eq!(session.design_preview_size(), (Some(760.0), Some(480.0)));
    let root = session
        .preview
        .visual(session.document().ast().root())
        .unwrap()
        .cast::<Control>()
        .unwrap();
    assert!(root.width().is_nan());
    assert!(root.height().is_nan());
}

#[test]
fn preview_factories_observe_scoped_design_mode_and_restore_on_failure() {
    use ferroui_controls::Design;
    let _app = UnitTestApplication::start(TestServices::styled_window());
    Design::set_is_design_mode(false);
    let source = "<TextBlock xmlns='https://github.com/ferroui' Text='Preview'/>";
    let mut options = crate::PreviewHostOptions::default();
    options.register_root(
        "TextBlock",
        Rc::new(|request| {
            assert!(Design::is_design_mode());
            request.populate(Some(Control::boxed(TextBlock::new())))
        }),
    );
    options
        .load(source, "ferres://Preview/DesignMode.xaml")
        .unwrap();
    assert!(!Design::is_design_mode());
    options.register_root(
        "TextBlock",
        Rc::new(|_| {
            assert!(Design::is_design_mode());
            Err("Factory intentionally rejected".into())
        }),
    );
    assert!(
        options
            .load(source, "ferres://Preview/DesignMode.xaml")
            .is_err()
    );
    assert!(!Design::is_design_mode());
    Design::set_is_design_mode(true);
    options.configuration.design_mode = false;
    options.register_root(
        "TextBlock",
        Rc::new(|request| {
            assert!(!Design::is_design_mode());
            request.populate(Some(Control::boxed(TextBlock::new())))
        }),
    );
    options
        .load(source, "ferres://Preview/DesignMode.xaml")
        .unwrap();
    assert!(Design::is_design_mode());
    Design::set_is_design_mode(false);
}
