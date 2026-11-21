//! Independent compiled/runtime source API, inherited native controls, tuple/list
//! access, attached tips, commands and event metadata. No OS-input claim.
use ferroribbon::controls::{Ribbon, RibbonButton};
use ferroribbon::metadata::key_tip_collections::{
    RibbonCurrentKeyTipsMetadata, RibbonKeyTipTupleMetadata,
};
use ferroui_base::{
    BoxedValue, StaticType,
    data::core::{ValueType, ValueTypes},
    metadata::{MarkupDelegate, MarkupTyped},
};
use ferroui_controls::{Control, StackPanel, TextBlock, Window};
use std::{cell::RefCell, rc::Rc};
struct Mode(Rc<RefCell<Vec<bool>>>);
impl PartialEq for Mode {
    fn eq(&self, o: &Self) -> bool {
        std::ptr::eq(self, o)
    }
}
ferroui_base::ferro_markup_type!(class Mode{this:Rc<Mode>,handles:[Rc<Mode>],methods:[fn Invoke(Option<BoxedValue>,bool)=>|v:&Rc<Mode>,_,b|v.0.borrow_mut().push(b)]});
#[test]
fn external_compiled_and_runtime_keytip_metadata_and_command_once() {
    const NAME: &str =
        "keyboard_tests::external_compiled_and_runtime_keytip_metadata_and_command_once";
    if std::env::var_os("FERRORIBBON_KEYBOARD_CONSUMER_ISOLATED").is_none() {
        let o = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", NAME, "--nocapture"])
            .env("FERRORIBBON_KEYBOARD_CONSUMER_ISOLATED", "1")
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
                    include_str!("Keyboard.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/Keyboard.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let panel = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<StackPanel>()
                .unwrap();
            let subclass = panel
                .children()
                .get(0)
                .cast::<crate::ribbon_consumer::ConsumerRibbon>()
                .unwrap();
            let r = subclass.clone().upcast::<Ribbon>();
            assert_eq!(r.application_button_key_tip().as_deref(), Some("FF"));
            assert_eq!(
                panel
                    .children()
                    .get(1)
                    .cast::<TextBlock>()
                    .unwrap()
                    .text()
                    .as_deref(),
                Some("0")
            );
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
            w.measure(ferroui_base::Size::new(800., 300.));
            w.arrange(ferroui_base::Rect::new(0., 0., 800., 300.));
            ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
            assert!(subclass.templates() > 0);
            let info = <Ribbon as StaticType>::TYPE.markup().unwrap();
            let root = Some(Control::boxed(r.clone()));
            ValueTypes::register_reference::<Mode>();
            let trace = Rc::new(RefCell::new(Vec::new()));
            let handler = MarkupDelegate::for_method(
                Some(Rc::new(Mode(trace.clone()))),
                Mode::MARKUP,
                &Mode::MARKUP.methods[0],
            );
            let event = info.find_event("KeyTipModeChanged").unwrap();
            (event.add)(&[root.clone(), Some(Rc::new(handler.clone()))]).unwrap();
            let invoke = |name: &str, args: Vec<Option<BoxedValue>>| {
                let m = info.find_methods(name).next().unwrap();
                let mut all = vec![root.clone()];
                all.extend(args);
                (m.invoke)(&all)
            };
            invoke("ShowKeyTips", vec![]).unwrap();
            assert!(r.is_key_tip_mode());
            let live = r.active_key_tips();
            ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(live.count(), 1);
            let method = info.find_methods("ProcessKeyTipInput").next().unwrap();
            assert_eq!(
                ValueTypes::type_full_name((method.parameters[0])()),
                "System.Char"
            );
            invoke("ProcessKeyTipInput", vec![Some(Rc::new('H'))]).unwrap();
            ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
            let entries = (info.find_property("CurrentKeyTips").unwrap().get.unwrap())(
                std::slice::from_ref(&root),
            )
            .unwrap();
            let tuple = (RibbonCurrentKeyTipsMetadata::MARKUP.indexers[0]
                .get
                .unwrap())(&[entries, Some(Rc::new(0_i32))])
            .unwrap();
            let tip = (RibbonKeyTipTupleMetadata::MARKUP
                .find_property("Item1")
                .unwrap()
                .get
                .unwrap())(std::slice::from_ref(&tuple))
            .unwrap()
            .unwrap();
            assert_eq!(tip.downcast_ref::<String>().map(String::as_str), Some("AB"));
            let target = (RibbonKeyTipTupleMetadata::MARKUP
                .find_property("Item2")
                .unwrap()
                .get
                .unwrap())(&[tuple])
            .unwrap()
            .unwrap();
            let button = ValueTypes::as_object(&*target)
                .unwrap()
                .cast::<RibbonButton>()
                .unwrap();
            assert_eq!(button.id().as_deref(), Some("run"));
            invoke("ProcessKeyTipInput", vec![Some(Rc::new('A'))]).unwrap();
            assert_eq!(vm.executed(), 0);
            invoke("ProcessKeyTipInput", vec![Some(Rc::new('B'))]).unwrap();
            assert_eq!(vm.executed(), 1);
            assert!(!r.is_key_tip_mode());
            assert_eq!(live.count(), 0);
            assert_eq!(&*trace.borrow(), &[true, false]);
            (event.remove.unwrap())(&[root.clone(), Some(Rc::new(handler))]).unwrap();
            invoke("ShowKeyTips", vec![]).unwrap();
            invoke("CancelKeyTips", vec![]).unwrap();
            assert_eq!(&*trace.borrow(), &[true, false]);
            assert!(invoke("ProcessKeyTipInput", vec![Some(Rc::new('😀'))]).is_err());
            w.close();
            w.set_content(None);
            ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
        }
    });
}

#[test]
fn downstream_compiled_and_runtime_ribbon_restores_native_input_element_focus() {
    if crate::test_process::isolated_case(concat!(
        module_path!(),
        "::downstream_compiled_and_runtime_ribbon_restores_native_input_element_focus"
    )) {
        return;
    }
    super::tests::run(|| {
        let _theme = super::tests::NativeThemeScope::new();
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("Keyboard.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/Keyboard.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let panel = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<StackPanel>()
                .unwrap();
            let subclass = panel
                .children()
                .get(0)
                .cast::<crate::ribbon_consumer::ConsumerRibbon>()
                .unwrap();
            let ribbon = subclass.clone().upcast::<Ribbon>();
            let inline_host = panel.children().get(2).cast::<TextBlock>().unwrap();
            let container = inline_host
                .inlines()
                .unwrap()
                .get(0)
                .cast::<ferroui_controls::documents::InlineUIContainer>()
                .unwrap();
            let link = container
                .child()
                .unwrap()
                .cast::<ferroui_controls::HyperlinkButton>()
                .unwrap();
            let document = ferroui_controls::TextBox::new();
            panel.children().add(document.clone());
            let input = ferroui_base::input::InputElement::new();
            input.set_focusable(true);
            panel.visual_children().add(input.clone().upcast());
            let window = Window::new();
            window.set_content(Some(Control::boxed(panel.clone())));
            window.show();
            window.measure(ferroui_base::Size::new(800., 300.));
            window.arrange(ferroui_base::Rect::new(0., 0., 800., 300.));
            let settle = || ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
            settle();
            assert!(subclass.templates() > 0);
            assert!(input.visual_root().is_some());
            assert!(input.to_ref().cast::<Control>().is_none());
            let header = ribbon.tabs().get(0).unwrap().header_element().unwrap();
            for before in [input.clone(), link.clone().upcast()] {
                assert!(before.visual_root().is_some());
                assert!(before.focus());
                ribbon.show_key_tips();
                settle();
                assert!(
                    window
                        .focus_manager()
                        .get_focused_element()
                        .is_some_and(|f| f.ptr_eq(&header))
                );
                ribbon.hide_key_tips();
                settle();
                assert!(
                    window
                        .focus_manager()
                        .get_focused_element()
                        .is_some_and(|f| f.ptr_eq(&before))
                );
            }
            assert!(document.focus());
            let weak = input.downgrade();
            panel.visual_children().remove(&input.clone().upcast());
            drop(input);
            assert!(
                weak.upgrade().is_none(),
                "completed KeyTips retain the old focus element"
            );
            let weak_link = link.downgrade();
            container.set_child(None);
            drop(link);
            // Rebuild the native text layout and let the compositor retire
            // the previous render data for the detached inline control.
            window.update_layout();
            ferroui_headless::FerroHeadlessPlatform::force_render_timer_tick(1);
            settle();
            assert!(
                weak_link.upgrade().is_none(),
                "completed KeyTips retain the inline link after layout"
            );
            window.close();
            window.set_content(None);
            settle();
        }
    });
}
