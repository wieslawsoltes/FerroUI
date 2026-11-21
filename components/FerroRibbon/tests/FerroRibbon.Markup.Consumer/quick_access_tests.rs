//! Actual reusable QAT library markup, metadata and retained native controls.
use ferroribbon::controls::{Ribbon, RibbonButton, RibbonQuickAccessToolBar};
use ferroui_base::{
    Ref, StaticType,
    data::core::{ValueType, ValueTypes},
    input::{InputElement, Key, KeyEventArgs},
    threading::Dispatcher,
};
use ferroui_controls::{Control, StackPanel, TextBlock, Window};
use std::rc::Rc;
#[test]
fn independent_qat_compiled_and_runtime_markup_identity_bindings_and_source_api() {
    const NAME: &str = "quick_access_tests::independent_qat_compiled_and_runtime_markup_identity_bindings_and_source_api";
    if std::env::var_os("FERRORIBBON_QAT_CONSUMER_ISOLATED").is_none() {
        let o = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", NAME, "--nocapture"])
            .env("FERRORIBBON_QAT_CONSUMER_ISOLATED", "1")
            .output()
            .unwrap();
        assert!(
            o.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        );
        return;
    }
    super::tests::run(|| {
        let _theme = super::tests::NativeThemeScope::new();
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("QuickAccess.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/QuickAccess.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let panel = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<StackPanel>()
                .unwrap();
            let r = panel.children().get(0).cast::<Ribbon>().unwrap();
            let count = panel.children().get(1).cast::<TextBlock>().unwrap();
            assert_eq!(count.text().as_deref(), Some("1"));
            assert_eq!(r.quick_access_candidate_ids().get(0), Some("cut".into()));
            let vm = ValueTypes::try_convert(
                r.data_context().as_ref(),
                ValueType::of::<Rc<crate::command_consumer::CommandViewModel>>(),
            )
            .flatten()
            .unwrap()
            .downcast_ref::<Rc<crate::command_consumer::CommandViewModel>>()
            .unwrap()
            .clone();
            let w = Window::new();
            w.set_content(Some(Control::boxed(panel)));
            w.show();
            w.measure(ferroui_base::Size::new(900., 400.));
            w.arrange(ferroui_base::Rect::new(0., 0., 900., 400.));
            Dispatcher::ui_thread().run_jobs(None);
            let q = r.quick_access_tool_bar().unwrap();
            assert!(
                q.cast::<crate::quick_access_consumer::ConsumerQuickAccessToolBar>()
                    .unwrap()
                    .templates()
                    > 0
            );
            assert!(!q.show_customize_button());
            assert!(q.show_labels());
            assert!(q.get_visual_root().is_some());
            assert!(!q.customize_button().unwrap().is_visible());
            let save = q.items().get(0).unwrap().cast::<RibbonButton>().unwrap();
            save.perform_click();
            assert_eq!(vm.executed(), 1);
            let info = <Ribbon as StaticType>::TYPE.markup().unwrap();
            let root = Some(Control::boxed(r.clone()));
            use ferroui_base::metadata::MarkupTyped;
            let ids = (info
                .find_property("DefaultQuickAccessItemIds")
                .unwrap()
                .get
                .unwrap())(std::slice::from_ref(&root))
            .unwrap();
            let first=(ferroribbon::metadata::quick_access_collections::RibbonQuickAccessItemIdsMetadata::MARKUP.indexers[0].get.unwrap())(&[ids,Some(Rc::new(0_i32))]).unwrap().unwrap();
            assert_eq!(
                first.downcast_ref::<String>().map(String::as_str),
                Some("save")
            );
            let add = info
                .find_methods("AddToQuickAccess")
                .find(|m| ValueTypes::type_full_name((m.parameters[0])()) == "System.String")
                .unwrap();
            (add.invoke)(&[root.clone(), Some(Rc::new("cut".to_string()))]).unwrap();
            assert_eq!(q.items().count(), 2);
            assert_eq!(count.text().as_deref(), Some("2"));
            let source = r.find_item(Some("cut".into())).unwrap();
            let copy = q.items().get(1).unwrap().cast::<RibbonButton>().unwrap();
            assert_eq!(
                ferroribbon::controls::RibbonItemHelper::get_source_item(&copy),
                Some(source.clone())
            );
            copy.perform_click();
            assert_eq!(vm.executed(), 2);
            let items = q.items();
            (info
                .find_methods("ToggleQuickAccessPosition")
                .next()
                .unwrap()
                .invoke)(std::slice::from_ref(&root))
            .unwrap();
            assert!(r.quick_access_tool_bar().unwrap().ptr_eq(&q));
            assert!(q.items() == items);
            assert!(q.items().get(0).unwrap().ptr_eq(&save));
            r.show_key_tips();
            Dispatcher::ui_thread().run_jobs(None);
            for position in [
                ferroribbon_core::RibbonQuickAccessPosition::AboveRibbon,
                ferroribbon_core::RibbonQuickAccessPosition::BelowRibbon,
            ] {
                r.cancel_key_tips();
                r.set_quick_access_position(position);
                w.update_layout();
                r.show_key_tips();
                Dispatcher::ui_thread().run_jobs(None);
                let badge = r
                    .active_key_tips()
                    .iter()
                    .flatten()
                    .find(|b| b.target().is_some_and(|target| target.ptr_eq(&save)))
                    .expect("actual downstream QAT badge");
                let bounds = ferroui_base::Rect::from_size(save.bounds().size())
                    .transform_to_aabb(save.transform_to_visual(&w).unwrap());
                let size = badge.desired_size();
                let x = (bounds.x + (bounds.width - size.width) / 2.)
                    .clamp(0., (w.bounds().width - size.width).max(0.));
                let y = (bounds.bottom() - size.height / 2. - 2.)
                    .clamp(0., (w.bounds().height - size.height).max(0.));
                assert_eq!(
                    (
                        ferroui_controls::Canvas::get_left(&badge),
                        ferroui_controls::Canvas::get_top(&badge)
                    ),
                    (x, y),
                    "runtime={runtime}, {position:?}"
                );
            }
            let mut e = KeyEventArgs::new();
            e.key = Key::D1;
            e.set_routed_event(Some(InputElement::key_down_event()));
            r.raise_event(&e);
            assert!(e.handled());
            assert_eq!(vm.executed(), 3);
            let remove = info.find_methods("RemoveFromQuickAccess").next().unwrap();
            (remove.invoke)(&[root, Some(Control::boxed(copy.clone()))]).unwrap();
            assert_eq!(count.text().as_deref(), Some("1"));
            source
                .cast::<RibbonButton>()
                .unwrap()
                .set_label(Some("Changed after removal".into()));
            assert_eq!(copy.label().as_deref(), Some("Cut"));
            let peer =
                ferroui_controls::automation::peers::ControlAutomationPeer::create_peer_for_element(
                    &q,
                );
            assert_eq!(
                peer.get_automation_control_type(),
                ferroui_controls::automation::peers::AutomationControlType::ToolBar
            );
            let ctor = <RibbonQuickAccessToolBar as StaticType>::TYPE
                .markup()
                .unwrap();
            let built = (ctor.constructors[0].invoke)(&[]).unwrap().unwrap();
            assert!(
                ValueTypes::as_object(&*built)
                    .unwrap()
                    .is::<RibbonQuickAccessToolBar>()
            );
            assert!(r.can_float_groups());
            let group = r.tabs().get(0).unwrap().groups().get(0).unwrap();
            assert_eq!(
                group.floating_position(),
                ferroui_base::Point::new(12., 24.)
            );
            let panel = group.items_panel();
            let group_info = <ferroribbon::controls::RibbonGroup as StaticType>::TYPE
                .markup()
                .unwrap();
            let float = group_info
                .find_methods("Float")
                .find(|m| m.parameters.len() == 1)
                .unwrap();
            (float.invoke)(&[
                Some(Control::boxed(group.clone())),
                Some(Rc::new(ferroui_base::Point::new(50., 60.))),
            ])
            .unwrap();
            assert!(group.is_floating_panel_open());
            assert!(group.items_panel().ptr_eq(&panel));
            let groups = (info.find_property("FloatingGroups").unwrap().get.unwrap())(&[Some(
                Control::boxed(r.clone()),
            )])
            .unwrap();
            let view = groups
                .unwrap()
                .downcast_ref::<ferroribbon::metadata::ribbon_collections::RibbonFloatingGroups>()
                .unwrap()
                .clone();
            let found =
                (ferroribbon::metadata::ribbon_collections::RibbonFloatingGroupsMetadata::MARKUP
                    .indexers[0]
                    .get
                    .unwrap())(&[Some(Rc::new(view)), Some(Rc::new(0_i32))])
                .unwrap()
                .unwrap();
            assert!(ValueTypes::as_object(&*found).unwrap().ptr_eq(&group));
            (info
                .find_methods("ReturnAllPanelsToRibbon")
                .next()
                .unwrap()
                .invoke)(&[Some(Control::boxed(r.clone()))])
            .unwrap();
            assert!(!group.is_floating());
            assert!(group.items_panel().ptr_eq(&panel));
            let _: Ref<RibbonQuickAccessToolBar> = q;
            w.close();
            w.set_content(None);
            Dispatcher::ui_thread().run_jobs(None);
        }
    });
}
