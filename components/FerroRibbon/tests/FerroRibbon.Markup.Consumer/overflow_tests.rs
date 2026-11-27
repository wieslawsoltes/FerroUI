#[test]
fn independent_compiled_runtime_display_groups_index_and_overflow_api() {
    const NAME: &str =
        "overflow_tests::independent_compiled_runtime_display_groups_index_and_overflow_api";
    if std::env::var_os("FERRORIBBON_CONSUMER_OVERFLOW_ISOLATED").is_none() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", NAME, "--nocapture"])
            .env("FERRORIBBON_CONSUMER_OVERFLOW_ISOLATED", "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }

    super::tests::run(|| {
        use ferroribbon::controls::Ribbon;
        use ferroui_base::{StaticType, data::core::ValueTypes};
        use ferroui_controls::{Control, StackPanel, TextBlock, Window};
        let _theme = super::tests::NativeThemeScope::new();
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("Overflow.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/Overflow.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let panel = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<StackPanel>()
                .unwrap();
            let root = panel.children().get(0).cast::<Ribbon>().unwrap();
            let label = panel.children().get(1).cast::<TextBlock>().unwrap();
            assert_eq!(label.text().as_deref(), Some("Commands"));
            let tab_for_changes = root.tabs().get(0).unwrap();
            let inserted = ferroribbon::controls::RibbonGroup::new();
            inserted.set_header(Some("Inserted".into()));
            tab_for_changes.groups().insert(0, Some(inserted));
            assert_eq!(label.text().as_deref(), Some("Inserted"));
            tab_for_changes.groups().remove_at(0);
            assert_eq!(label.text().as_deref(), Some("Commands"));
            let tab = root.tabs().get(0).unwrap();
            let w = Window::new();
            w.set_content(Some(Control::boxed(panel)));
            w.show();
            w.measure(ferroui_base::Size::new(500., 250.));
            w.arrange(ferroui_base::Rect::new(0., 0., 500., 250.));
            let method = <ferroribbon::controls::RibbonTab as StaticType>::TYPE
                .markup()
                .unwrap()
                .find_methods("ShowOverflow")
                .next()
                .unwrap();
            (method.invoke)(&[Some(Control::boxed(tab))]).unwrap();
            w.close();
            w.set_content(None);
        }
    });
}
