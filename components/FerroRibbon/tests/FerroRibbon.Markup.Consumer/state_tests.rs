//! Independent compiled/runtime source-style state markup and actual public root methods.
use ferroribbon::{
    controls::{Ribbon, RibbonButton},
    metadata::state_types::*,
};
use ferroribbon_core::{RibbonDensity, RibbonDisplayMode, state::*};
use ferroui_base::{
    BoxedValue, StaticType,
    data::core::{ValueType, ValueTypes},
    metadata::MarkupTyped,
};
use ferroui_controls::{Control, StackPanel, TextBlock, Window};
use std::rc::Rc;
fn invoke<T: StaticType>(method: &str, args: &[Option<BoxedValue>]) -> Option<BoxedValue> {
    let info = T::TYPE.markup().unwrap();
    (info.find_methods(method).next().unwrap().invoke)(args).unwrap()
}
fn convert<T: 'static>(value: Option<BoxedValue>) -> BoxedValue {
    ValueTypes::try_convert(value.as_ref(), ValueType::of::<T>())
        .flatten()
        .unwrap()
}
#[test]
fn independent_state_markup_collections_bindings_and_public_root_restore() {
    const NAME: &str =
        "state_tests::independent_state_markup_collections_bindings_and_public_root_restore";
    if std::env::var_os("FERRORIBBON_STATE_CONSUMER_ISOLATED").is_none() {
        let o = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", NAME, "--nocapture"])
            .env("FERRORIBBON_STATE_CONSUMER_ISOLATED", "1")
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
                    include_str!("State.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/State.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let panel = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<StackPanel>()
                .unwrap();
            let r = panel.children().get(0).cast::<Ribbon>().unwrap();
            assert_eq!(
                panel
                    .children()
                    .get(1)
                    .cast::<TextBlock>()
                    .unwrap()
                    .text()
                    .as_deref(),
                Some("Personal")
            );
            let state = convert::<Rc<RibbonState>>(r.tag());
            let state = state.downcast_ref::<Rc<RibbonState>>().unwrap().clone();
            let c = state.customization().unwrap();
            assert_eq!(
                c.custom_tabs().unwrap().get(0).unwrap().id(),
                Some("custom".into())
            );
            let w = Window::new();
            w.set_content(Some(Control::boxed(panel)));
            w.show();
            let root = Some(Control::boxed(r.clone()));
            invoke::<Ribbon>(
                "ApplyState",
                &[root.clone(), Some(Rc::new(Some(state.clone())))],
            );
            assert_eq!(r.display_mode(), RibbonDisplayMode::Simplified);
            assert_eq!(r.density(), RibbonDensity::Compact);
            assert_eq!(r.selected_tab().unwrap().id().as_deref(), Some("custom"));
            let source = r.find_item(Some("cut".into())).unwrap();
            let copy = r
                .selected_tab()
                .unwrap()
                .groups()
                .get(0)
                .unwrap()
                .items()
                .get(0)
                .unwrap()
                .cast::<RibbonButton>()
                .unwrap();
            assert!(copy.is::<crate::button_consumer::ConsumerRibbonButton>());
            source
                .cast::<RibbonButton>()
                .unwrap()
                .set_label(Some("Updated".into()));
            assert_eq!(copy.label().as_deref(), Some("Updated"));
            let saved = invoke::<Ribbon>("GetState", std::slice::from_ref(&root));
            let saved = convert::<Rc<RibbonState>>(saved);
            let saved = saved.downcast_ref::<Rc<RibbonState>>().unwrap();
            assert!(!Rc::ptr_eq(saved, &state));
            let json = invoke::<Ribbon>("SaveStateToJson", std::slice::from_ref(&root)).unwrap();
            let json = json.downcast_ref::<String>().unwrap().clone();
            assert!(json.contains("custom"));
            let deserialize = RibbonStateSerializerMetadata::MARKUP
                .find_methods("Deserialize")
                .next()
                .unwrap();
            let decoded = (deserialize.invoke)(&[Some(Rc::new(Some(json.clone())))]).unwrap();
            let decoded = convert::<Option<Rc<RibbonState>>>(decoded);
            assert!(
                decoded
                    .downcast_ref::<Option<Rc<RibbonState>>>()
                    .unwrap()
                    .is_some()
            );
            let list = c.tab_order().unwrap();
            let handle = Some(Rc::new(list.clone()) as BoxedValue);
            let add = RibbonStateStringsMetadata::MARKUP
                .find_methods("Add")
                .next()
                .unwrap();
            (add.invoke)(&[handle.clone(), Some(Rc::new(Some("insert".to_string())))]).unwrap();
            assert_eq!(list.get(2), Some("insert".into()));
            let get = RibbonStateStringsMetadata::MARKUP.indexers[0].get.unwrap();
            let last = get(&[handle, Some(Rc::new(2_i32))]).unwrap();
            let last = convert::<Option<String>>(last);
            assert_eq!(
                last.downcast_ref::<Option<String>>().unwrap(),
                &Some("insert".into())
            );
            invoke::<Ribbon>("ResetCustomization", std::slice::from_ref(&root));
            assert_eq!(r.tabs().count(), 1);
            source
                .cast::<RibbonButton>()
                .unwrap()
                .set_label(Some("After unlink".into()));
            assert_eq!(copy.label().as_deref(), Some("Updated"));
            invoke::<Ribbon>("LoadStateFromJson", &[root, Some(Rc::new(Some(json)))]);
            assert_eq!(r.selected_tab().unwrap().id().as_deref(), Some("custom"));
            w.close();
        }
    });
}

#[test]
fn native_string_metadata_rejects_unpaired_utf16_without_corrupting_core_state() {
    super::tests::run(|| {
        let state = RibbonState::new();
        let malformed = ferroribbon_core::RibbonString::from_utf16(vec![0xd800]);
        state.set_selected_tab_id(Some(malformed.clone()));
        let getter = RibbonStateMetadata::MARKUP
            .find_property("SelectedTabId")
            .unwrap()
            .get
            .unwrap();
        let result = getter(&[Some(Rc::new(state.clone()))]);
        assert!(
            result.is_err(),
            "native String must not silently replace UTF-16"
        );
        assert_eq!(state.selected_tab_id(), Some(malformed.clone()));
        let list =
            ferroribbon_core::collections::RibbonItems::from_values(vec![Some(malformed.clone())]);
        let getter = RibbonStateStringsMetadata::MARKUP.indexers[0].get.unwrap();
        assert!(getter(&[Some(Rc::new(list.clone())), Some(Rc::new(0_i32))]).is_err());
        assert_eq!(list.get(0), Some(malformed));
        // A valid supplementary pair crosses the same native API losslessly.
        let valid = ferroribbon_core::RibbonString::from_utf16(vec![0xd83d, 0xde00]);
        state.set_selected_tab_id(Some(valid));
        let getter = RibbonStateMetadata::MARKUP
            .find_property("SelectedTabId")
            .unwrap()
            .get
            .unwrap();
        let text = convert::<Option<String>>(getter(&[Some(Rc::new(state))]).unwrap());
        assert_eq!(
            text.downcast_ref::<Option<String>>().unwrap().as_deref(),
            Some("😀")
        );
    });
}
