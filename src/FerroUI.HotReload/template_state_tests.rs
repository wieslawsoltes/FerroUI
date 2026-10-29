use super::*;
use ferroui_base::{threading::Dispatcher, Ref, StyledElement};
use ferroui_controls::{
    testing::{TestServices, UnitTestApplication},
    ContentControl, Control, TextBox, Window,
};
use ferroui_markup_xaml::{
    FromXamlObject, RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument,
};
use std::rc::Rc;

fn source(width: usize, text: &str) -> String {
    format!(
        r#"<ContentControl xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" Theme="{{StaticResource RootTheme}}">
<ContentControl.Resources>
<ControlTheme x:Key="RootTheme" TargetType="ContentControl">
<Setter Property="Template"><ControlTemplate>
<Border Width="{width}"><TextBox Name="entry" {text}/></Border>
</ControlTemplate></Setter>
</ControlTheme>
</ContentControl.Resources>
</ContentControl>"#
    )
}
fn entry(root: &ContentControl) -> Ref<TextBox> {
    root.get_visual_descendants()
        .find_map(|value| {
            value
                .cast::<TextBox>()
                .filter(|text| text.name().as_deref() == Some("entry"))
        })
        .unwrap()
}
#[test]
fn root_template_without_content_preserves_text_cursor_focus_and_releases_parts() {
    let mut services = TestServices::styled_window();
    let focus = TestServices::real_focus();
    services.input_manager = focus.input_manager;
    services.keyboard_device = focus.keyboard_device;
    services.keyboard_navigation = focus.keyboard_navigation;
    let _application = UnitTestApplication::start(services);
    ferroui_controls::register_types();
    ferroui_markup_xaml::register_types();
    let uri = "ferres://template-state/View.xaml";
    let baseline = source(200, "");
    let document = RuntimeXamlLoaderDocument::with_base_uri(
        Some(
            ferroui_base::utilities::Uri::try_create(
                uri,
                ferroui_base::utilities::UriKind::Absolute,
            )
            .unwrap(),
        ),
        &baseline,
    );
    let root = Ref::<ContentControl>::from_xaml_object(
        &ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load_document(document, None).unwrap(),
    )
    .unwrap();
    let window = Window::new();
    window.set_content(Some(Control::boxed(&root.clone().upcast::<Control>())));
    window.show();
    Dispatcher::ui_thread().run_jobs(None);
    assert!(root.content().is_none());
    let target: Rc<dyn ReloadTarget> = Rc::new(
        ComponentReloadTarget::content_control(
            &root,
            uri.into(),
            baseline.clone(),
            RuntimeXamlLoaderConfiguration::default(),
        )
        .with_owned_resources()
        .with_owned_root_static_resource_property(StyledElement::theme_property()),
    );
    let mut session = HotReloadSession::new();
    session.register_document(uri.into(), baseline).unwrap();
    session.register_target(&target);
    for width in [220, 240] {
        let old = entry(&root);
        let weak = old.downgrade();
        old.set_text(Some("typed input"));
        old.set_caret_index(7);
        old.set_selection_start(7);
        old.set_selection_end(2);
        assert!(old.focus());
        let caret = old.caret_index();
        session.submit(uri.into(), source(width, ""));
        let result = session.apply_pending().unwrap();
        assert_eq!(result.status, ReloadStatus::Applied, "{result:?}");
        Dispatcher::ui_thread().run_jobs(None);
        let new = entry(&root);
        assert!(!new.ptr_eq(&old));
        assert_eq!(new.text().as_deref(), Some("typed input"));
        assert_eq!(
            (
                new.caret_index(),
                new.selection_start(),
                new.selection_end()
            ),
            (caret, 7, 2)
        );
        assert!(new.is_focused());
        drop(old);
        Dispatcher::ui_thread().run_jobs(None);
        assert!(weak.upgrade().is_none());
    }
    window.close();
}

struct Fixture {
    root: Ref<ContentControl>,
    window: Ref<Window>,
    session: HotReloadSession,
    _target: Rc<dyn ReloadTarget>,
    uri: String,
}
impl Fixture {
    fn new(source: &str, uri: &str) -> Self {
        ferroui_controls::register_types();
        ferroui_markup_xaml::register_types();
        let document = RuntimeXamlLoaderDocument::with_base_uri(
            Some(
                ferroui_base::utilities::Uri::try_create(
                    uri,
                    ferroui_base::utilities::UriKind::Absolute,
                )
                .unwrap(),
            ),
            source,
        );
        let root = Ref::<ContentControl>::from_xaml_object(
            &ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load_document(document, None)
                .unwrap(),
        )
        .unwrap();
        let window = Window::new();
        window.set_content(Some(Control::boxed(&root.clone().upcast::<Control>())));
        window.show();
        Dispatcher::ui_thread().run_jobs(None);
        let target: Rc<dyn ReloadTarget> = Rc::new(
            ComponentReloadTarget::content_control(
                &root,
                uri.into(),
                source.into(),
                RuntimeXamlLoaderConfiguration::default(),
            )
            .with_owned_resources()
            .with_owned_root_static_resource_property(StyledElement::theme_property()),
        );
        let mut session = HotReloadSession::new();
        session
            .register_document(uri.into(), source.into())
            .unwrap();
        session.register_target(&target);
        Self {
            root,
            window,
            session,
            _target: target,
            uri: uri.into(),
        }
    }
    fn apply(&mut self, source: String) {
        self.session.submit(self.uri.clone(), source);
        let outcome = self.session.apply_pending().unwrap();
        assert_eq!(outcome.status, ReloadStatus::Applied, "{outcome:?}");
        Dispatcher::ui_thread().run_jobs(None);
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.window.close();
    }
}

#[test]
fn template_authored_changes_and_two_way_bindings_win_over_saved_input() {
    let _application = UnitTestApplication::start(TestServices::styled_window());
    for (before, after, expected) in [
        ("", r#"Text="authored""#, Some("authored")),
        (r#"Text="original""#, "", None),
        (
            "",
            r#"Text="{Binding Text, Mode=TwoWay}""#,
            Some("model value"),
        ),
    ] {
        let mut fixture = Fixture::new(&source(200, before), "ferres://template-state/Guards.xaml");
        let model = TextBox::new();
        model.set_text(Some("model value"));
        fixture.root.set_data_context(Some(Rc::new(model.clone())));
        let old = entry(&fixture.root);
        old.set_text(Some("typed input"));
        old.set_caret_index(5);
        fixture.apply(source(220, after));
        assert_eq!(entry(&fixture.root).text().as_deref(), expected);
        assert_eq!(
            model.text().as_deref(),
            Some("model value"),
            "restoration wrote stale text through a binding"
        );
    }
}
fn repeated_source(width: usize) -> String {
    format!(
        r#"<ContentControl xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">
<ContentControl.Resources><ControlTheme x:Key="Editor" TargetType="ContentControl">
<Setter Property="Template"><ControlTemplate><StackPanel Width="{width}">
<TextBox Name="entry"/>
<ContentControl Name="nested"><ContentControl.Template><ControlTemplate><TextBox Name="entry"/></ControlTemplate></ContentControl.Template></ContentControl>
</StackPanel></ControlTemplate></Setter></ControlTheme></ContentControl.Resources>
<StackPanel><TextBox Name="entry"/><ContentControl Name="left" Theme="{{StaticResource Editor}}"/><ContentControl Name="right" Theme="{{StaticResource Editor}}"/></StackPanel>
</ContentControl>"#
    )
}
fn fields(root: &ContentControl) -> Vec<Ref<TextBox>> {
    let main = root.get_control::<TextBox>("entry");
    let mut result = vec![main];
    for name in ["left", "right"] {
        let owner = root.get_control::<ContentControl>(name);
        let mut found: Vec<_> = owner
            .get_visual_descendants()
            .filter_map(|value| {
                value
                    .cast::<TextBox>()
                    .filter(|text| text.name().as_deref() == Some("entry"))
            })
            .collect();
        assert_eq!(found.len(), 2);
        result.append(&mut found);
    }
    result
}
#[test]
fn nested_template_instances_keep_names_separate_through_rollback_and_rapid_edits() {
    let _application = UnitTestApplication::start(TestServices::styled_window());
    let mut fixture = Fixture::new(
        &repeated_source(200),
        "ferres://template-state/Instances.xaml",
    );
    let old = fields(&fixture.root);
    for (i, field) in old.iter().enumerate() {
        field.set_text(Some(&format!("draft {i}")));
    }
    fixture
        .session
        .submit(fixture.uri.clone(), repeated_source(220));
    let reached = std::cell::Cell::new(false);
    let outcome = fixture
        .session
        .apply_pending_with(|| {
            reached.set(true);
            Err(ReloadError::Failed("rollback".into()))
        })
        .unwrap();
    assert!(reached.get(), "{outcome:?}");
    assert_eq!(outcome.status, ReloadStatus::Failed);
    for (i, field) in fields(&fixture.root).iter().enumerate() {
        assert!(field.ptr_eq(&old[i]));
    }
    for width in [240, 260] {
        fixture
            .session
            .submit(fixture.uri.clone(), repeated_source(width));
        let outcome = fixture.session.apply_pending().unwrap();
        assert_eq!(outcome.status, ReloadStatus::Applied, "{outcome:?}");
    }
    Dispatcher::ui_thread().run_jobs(None);
    for (i, field) in fields(&fixture.root).iter().enumerate() {
        assert!(!field.ptr_eq(&old[i]));
        assert_eq!(
            field.text().as_deref(),
            Some(format!("draft {i}").as_str()),
            "part {i} crossed its source or instance scope"
        );
    }
    let weak: Vec<_> = old.iter().map(Ref::downgrade).collect();
    drop(old);
    assert!(weak.iter().all(|field| field.upgrade().is_none()));
}

#[test]
fn duplicate_anonymous_template_owners_are_not_matched_by_position() {
    let _application = UnitTestApplication::start(TestServices::styled_window());
    let source = |width| {
        repeated_source(width)
            .replace(" Name=\"left\"", "")
            .replace(" Name=\"right\"", "")
    };
    let mut fixture = Fixture::new(&source(200), "ferres://template-state/Ambiguous.xaml");
    let texts = || {
        fixture
            .root
            .get_visual_descendants()
            .filter_map(|value| {
                value
                    .cast::<TextBox>()
                    .filter(|text| text.name().as_deref() == Some("entry"))
            })
            .collect::<Vec<_>>()
    };
    for text in texts() {
        text.set_text(Some("private input"));
    }
    fixture.apply(source(220));
    let main = fixture.root.get_control::<TextBox>("entry");
    assert_eq!(main.text().as_deref(), Some("private input"));
    let parts: Vec<_> = fixture
        .root
        .get_visual_descendants()
        .filter_map(|value| {
            value
                .cast::<TextBox>()
                .filter(|text| text.name().as_deref() == Some("entry") && !text.ptr_eq(&main))
        })
        .collect();
    assert_eq!(parts.len(), 4);
    assert!(parts.iter().all(|text| text.text().is_none()));
}

#[test]
fn template_scroll_and_selection_restore_after_layout() {
    use ferroui_base::Vector;
    use ferroui_controls::{ListBox, ScrollViewer};
    let _application = UnitTestApplication::start(TestServices::styled_window());
    let source = |width| {
        source(width, "").replace(r#"<TextBox Name="entry" />"#, r#"<StackPanel><ScrollViewer Name="scroll" Height="60"><ScrollViewer.Template><ControlTemplate><ScrollContentPresenter Name="PART_ContentPresenter" Content="{TemplateBinding Content}" CanVerticallyScroll="True"/></ControlTemplate></ScrollViewer.Template><Border Height="900" Width="100"/></ScrollViewer><ListBox Name="choices"><x:String>a</x:String><x:String>b</x:String><x:String>c</x:String></ListBox></StackPanel>"#)
    };
    let mut fixture = Fixture::new(&source(200), "ferres://template-state/Scroll.xaml");
    let scroll = fixture
        .root
        .get_visual_descendants()
        .find_map(|node| {
            node.cast::<ScrollViewer>()
                .filter(|value| value.name().as_deref() == Some("scroll"))
        })
        .unwrap();
    let choices = fixture
        .root
        .get_visual_descendants()
        .find_map(|node| node.cast::<ListBox>())
        .unwrap();
    scroll.set_offset(Vector::new(0.0, 120.0));
    choices.set_selected_index(1);
    assert_eq!(scroll.offset().y, 120.0);
    fixture.apply(source(220));
    let after = fixture
        .root
        .get_visual_descendants()
        .find_map(|node| {
            node.cast::<ScrollViewer>()
                .filter(|value| value.name().as_deref() == Some("scroll"))
        })
        .unwrap();
    let selection = fixture
        .root
        .get_visual_descendants()
        .find_map(|node| node.cast::<ListBox>())
        .unwrap();
    assert!(!after.ptr_eq(&scroll));
    assert_eq!(after.offset().y, 120.0);
    assert_eq!(selection.selected_index(), 1);
}

#[test]
fn data_template_state_is_scoped_to_its_named_presenter() {
    let _application = UnitTestApplication::start(TestServices::styled_window());
    let source = |width| {
        format!(
            r#"<ContentControl xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><StackPanel>
<ContentControl Name="left" Content="Left"><ContentControl.ContentTemplate><DataTemplate><TextBox Name="entry" Width="{width}"/></DataTemplate></ContentControl.ContentTemplate></ContentControl>
<ContentControl Name="right" Content="Right"><ContentControl.ContentTemplate><DataTemplate><TextBox Name="entry" Width="{width}"/></DataTemplate></ContentControl.ContentTemplate></ContentControl>
</StackPanel></ContentControl>"#
        )
    };
    let mut fixture = Fixture::new(&source(200), "ferres://template-state/Data.xaml");
    for name in ["left", "right"] {
        let owner = fixture.root.get_control::<ContentControl>(name);
        entry(&owner).set_text(Some(name));
    }
    fixture.apply(source(220));
    for name in ["left", "right"] {
        let owner = fixture.root.get_control::<ContentControl>(name);
        assert_eq!(entry(&owner).text().as_deref(), Some(name));
    }
}

#[test]
fn one_anonymous_template_instance_uses_its_component_scope() {
    let _application = UnitTestApplication::start(TestServices::styled_window());
    let source = |width| {
        format!(
            r#"<ContentControl xmlns="https://github.com/ferroui"><StackPanel><ContentControl><ContentControl.Template><ControlTemplate><TextBox Name="entry" Width="{width}"/></ControlTemplate></ContentControl.Template></ContentControl></StackPanel></ContentControl>"#
        )
    };
    let mut fixture = Fixture::new(&source(200), "ferres://template-state/Anonymous.xaml");
    entry(&fixture.root).set_text(Some("unique draft"));
    fixture.apply(source(220));
    assert_eq!(entry(&fixture.root).text().as_deref(), Some("unique draft"));
}

#[test]
fn template_locations_use_utf16_columns_and_xml_line_endings() {
    let _application = UnitTestApplication::start(TestServices::styled_window());
    for (index, separator) in ["", "\r", "\r\n"].iter().enumerate() {
        let document = |width| {
            source(width, "")
                .replace(
                    "<ControlTemplate>",
                    &format!("<!--{}--><ControlTemplate>", "é🦀".repeat(80)),
                )
                .replace('\n', separator)
        };
        let mut fixture = Fixture::new(
            &document(200),
            &format!("ferres://template-location{index}/View.xaml"),
        );
        entry(&fixture.root).set_text(Some("typed"));
        fixture.apply(document(220));
        assert_eq!(
            entry(&fixture.root).text().as_deref(),
            Some("typed"),
            "separator {separator:?}"
        );
    }
}

#[test]
fn unnamed_development_keys_work_on_initial_template_population() {
    let _application = UnitTestApplication::start(TestServices::styled_window());
    let document = |width| {
        source(width, "")
            .replace(
                "xmlns:x=",
                "xmlns:hr=\"https://github.com/ferroui/hot-reload\" xmlns:x=",
            )
            .replace("Name=\"entry\"", "hr:Key=\"draft\"")
    };
    let mut fixture = Fixture::new(&document(200), "ferres://template-keys/View.xaml");
    let find = |root: &ContentControl| {
        root.get_visual_descendants()
            .find_map(|node| node.cast::<TextBox>())
            .unwrap()
    };
    let old = find(&fixture.root);
    assert!(old.name().is_none());
    assert_eq!(
        ferroui_markup_xaml::reload_identity::key(&old).as_deref(),
        Some("draft")
    );
    let weak = old.downgrade();
    old.set_text(Some("typed"));
    fixture.apply(document(220));
    let new = find(&fixture.root);
    assert!(new.name().is_none());
    assert_eq!(new.text().as_deref(), Some("typed"));
    drop(old);
    assert!(weak.upgrade().is_none());
}
