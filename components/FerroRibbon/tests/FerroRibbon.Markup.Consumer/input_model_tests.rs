//! Public Core models drive the same native input controls through both XAML loaders.
use ferroribbon::{controls::*, metadata::models::*};
use ferroribbon_core::{commands::RibbonRelayCommand, model::*};
use ferroui_base::{
    BoxedValue, Rect, Size,
    data::core::{ValueType, ValueTypes},
    input::{InputElement, Key, KeyEventArgs},
    metadata::MarkupTyped,
    threading::Dispatcher,
};
use ferroui_controls::{Control, StackPanel, TextBlock, TextBox, Window};
use serde_json::{Value, json};
use std::{cell::RefCell, rc::Rc};
fn cast<T: Clone + 'static>(v: &BoxedValue) -> T {
    ValueTypes::try_cast(v, ValueType::of::<T>())
        .unwrap()
        .downcast_ref::<T>()
        .unwrap()
        .clone()
}
fn drain() {
    Dispatcher::ui_thread().run_jobs(None);
}
#[test]
fn original_input_constructor_arities_and_uncoerced_defaults_match_metadata() {
    crate::tests::run(|| {
        let source: Value =
            serde_json::from_str(include_str!("../fixtures/model-metadata.json")).unwrap();
        let args: Vec<Option<BoxedValue>> = vec![
            Some(Rc::new("id".to_owned())),
            Some(Rc::new("Label".to_owned())),
            Some(Rc::new(-123.25f64)),
        ];
        let mut actual = vec![];
        for meta in [
            RibbonSpinnerModelMetadata::MARKUP,
            RibbonTextBoxModelMetadata::MARKUP,
            RibbonSliderModelMetadata::MARKUP,
        ] {
            for ctor in meta.constructors {
                let arity = ctor.parameters.len();
                let v = (ctor.invoke)(&args[..arity]).unwrap().unwrap();
                let item = cast::<Rc<dyn RibbonItemModelImpl>>(&v);
                let spin =
                    try_cast_model::<dyn RibbonSpinnerModelImpl>(Some(item.clone())).unwrap();
                let text =
                    try_cast_model::<dyn RibbonTextBoxModelImpl>(Some(item.clone())).unwrap();
                let slider =
                    try_cast_model::<dyn RibbonSliderModelImpl>(Some(item.clone())).unwrap();
                let id = item.id().map(|s| s.to_utf8().unwrap());
                if arity == 0 {
                    assert!(
                        id.as_ref().unwrap().len() == 32
                            && id.as_ref().unwrap().bytes().all(|b| b.is_ascii_hexdigit())
                    );
                }
                actual.push(json!({"type":meta.name,"arity":arity,"id":if arity==0{Some("<generated-guid>".to_owned())}else{id},"label":item.label().map(|s|s.to_utf8().unwrap()),"size":item.size().raw(),
                    "value":spin.as_ref().map(|m|m.value()).or_else(||slider.as_ref().map(|m|m.value())),
                    "minimum":spin.as_ref().map(|m|m.minimum()).or_else(||slider.as_ref().map(|m|m.minimum())),
                    "maximum":spin.as_ref().map(|m|m.maximum()).or_else(||slider.as_ref().map(|m|m.maximum())),
                    "step":spin.as_ref().map(|m|m.increment()).or_else(||slider.as_ref().map(|m|m.step_frequency())),
                    "width":spin.as_ref().map(|m|m.input_width()).or_else(||slider.as_ref().map(|m|m.slider_width())).or_else(||text.as_ref().map(|m|m.input_width())),
                    "format":spin.as_ref().and_then(|m|m.format()).map(|s|s.to_utf8().unwrap()),"unit":spin.as_ref().and_then(|m|m.unit()).map(|s|s.to_utf8().unwrap()),
                    "text":text.as_ref().and_then(|m|m.text()).map(|s|s.to_utf8().unwrap()),"placeholder":text.as_ref().and_then(|m|m.placeholder()).map(|s|s.to_utf8().unwrap())}));
            }
        }
        let mut expected = source["corpus"]["inputConstructors"].clone();
        for entry in expected.as_array_mut().unwrap() {
            for key in ["value", "minimum", "maximum", "step", "width"] {
                if !entry[key].is_null() {
                    entry[key] = json!(entry[key].as_f64().unwrap());
                }
            }
        }
        assert_eq!(Value::Array(actual), expected);
    });
}
#[test]
fn compiled_runtime_input_models_two_way_commands_replacement_and_detachment() {
    crate::tests::run(|| {
        let _theme = crate::tests::NativeThemeScope::new();
        for runtime in [false, true] {
            let v = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("InputModels.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/InputModels.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let host = ValueTypes::as_object(&*v)
                .unwrap()
                .cast::<StackPanel>()
                .unwrap();
            let resource = |name: &str| {
                host.resources()
                    .try_get_resource(&name.into(), None)
                    .flatten()
                    .unwrap()
            };
            let spin = cast::<Rc<dyn RibbonSpinnerModelImpl>>(&resource("Spinner"));
            let text = cast::<Rc<dyn RibbonTextBoxModelImpl>>(&resource("Text"));
            let slider = cast::<Rc<dyn RibbonSliderModelImpl>>(&resource("Slider"));
            let r = host.children().get(0).cast::<Ribbon>().unwrap();
            drain();
            let group = r.tabs().get(0).unwrap().groups().get(0).unwrap();
            let s = group
                .items()
                .get(0)
                .unwrap()
                .cast::<RibbonSpinner>()
                .unwrap();
            let t = group
                .items()
                .get(1)
                .unwrap()
                .cast::<RibbonTextBox>()
                .unwrap();
            let l = group
                .items()
                .get(2)
                .unwrap()
                .cast::<RibbonSlider>()
                .unwrap();
            assert_eq!(
                (
                    s.minimum(),
                    s.maximum(),
                    s.increment(),
                    s.value(),
                    s.input_width()
                ),
                (-10., 30., 0.25, 7.25, 85.)
            );
            assert_eq!(s.format().as_deref(), Some("0.00"));
            assert_eq!(s.unit().as_deref(), Some("pt"));
            assert_eq!(t.text().as_deref(), Some("café 😀"));
            assert_eq!(t.placeholder_text().as_deref(), Some("Search"));
            assert_eq!(t.input_width(), 160.);
            assert_eq!(
                (
                    l.minimum(),
                    l.maximum(),
                    l.step_frequency(),
                    l.value(),
                    l.input_width()
                ),
                (-5., 50., 2.5, 12.5, 155.)
            );
            assert!(Rc::ptr_eq(
                &Ribbon::get_model(&s).unwrap(),
                &(spin.clone() as Rc<dyn RibbonNodeModelImpl>)
            ));
            s.set_numeric_value(8.5);
            l.set_numeric_value(17.5);
            t.set_text(Some("e\u{301} 😀".to_owned()));
            drain();
            assert_eq!(spin.value(), 8.5);
            assert_eq!(slider.value(), 17.5);
            assert_eq!(text.text().unwrap().to_utf8().unwrap(), "e\u{301} 😀");
            assert_eq!(
                host.children()
                    .get(2)
                    .cast::<TextBlock>()
                    .unwrap()
                    .text()
                    .as_deref(),
                Some("8.5")
            );
            assert_eq!(
                host.children()
                    .get(3)
                    .cast::<TextBlock>()
                    .unwrap()
                    .text()
                    .as_deref(),
                Some("17.5")
            );
            spin.set_value(9.75);
            slider.set_value(22.5);
            text.set_text(None);
            drain();
            assert_eq!(s.value(), 9.75);
            assert_eq!(l.value(), 22.5);
            assert_eq!(t.text().as_deref(), Some(""));
            assert!(text.text().is_none());
            host.children()
                .get(1)
                .cast::<TextBox>()
                .unwrap()
                .set_text(Some("external"));
            drain();
            assert_eq!(t.text().as_deref(), Some("external"));
            spin.set_unit(None);
            spin.set_input_width(92.);
            slider.set_slider_width(170.);
            text.set_placeholder(None);
            assert_eq!(s.unit(), None);
            assert_eq!(s.input_width(), 92.);
            assert_eq!(l.input_width(), 170.);
            assert_eq!(t.placeholder_text(), None);

            // Use the actual template part and routed Enter, with command replacement.
            let calls = Rc::new(RefCell::new(Vec::<String>::new()));
            let out = calls.clone();
            text.set_command(Some(RibbonRelayCommand::new_with_parameter(
                move |p| {
                    out.borrow_mut()
                        .push(p.unwrap().downcast_ref::<String>().unwrap().clone())
                },
                None,
            )));
            let w = Window::new();
            w.set_content(Some(Control::boxed(host.clone())));
            w.show();
            w.measure(Size::new(1100., 400.));
            w.arrange(Rect::new(0., 0., 1100., 400.));
            drain();
            let numeric_calls = Rc::new(RefCell::new(Vec::<f64>::new()));
            let out = numeric_calls.clone();
            spin.set_command(Some(RibbonRelayCommand::new_with_parameter(
                move |p| {
                    out.borrow_mut()
                        .push(*p.unwrap().downcast_ref::<f64>().unwrap())
                },
                None,
            )));
            let out = numeric_calls.clone();
            slider.set_command(Some(RibbonRelayCommand::new_with_parameter(
                move |p| {
                    out.borrow_mut()
                        .push(*p.unwrap().downcast_ref::<f64>().unwrap())
                },
                None,
            )));
            let spin_part = s
                .get_visual_descendants()
                .find_map(|c| c.cast::<TextBox>())
                .unwrap();
            spin_part.set_text(Some("10.50"));
            let mut key = KeyEventArgs::new();
            key.key = Key::Enter;
            key.set_routed_event(Some(InputElement::key_down_event()));
            spin_part.raise_event(&key);
            assert!(key.handled());
            assert_eq!(spin.value(), 10.5);
            assert_eq!(s.value(), 10.5);
            assert_eq!(&*numeric_calls.borrow(), &[10.5]);
            let slider_part = l
                .get_visual_descendants()
                .find_map(|c| c.cast::<ferroribbon::compat::RibbonInputSlider>())
                .unwrap();
            let mut key = KeyEventArgs::new();
            key.key = Key::Right;
            key.set_routed_event(Some(InputElement::key_down_event()));
            slider_part.raise_event(&key);
            assert!(key.handled());
            // Original Uno keyboard Step uses SmallChange=1 and previous-step ties,
            // independently of the model's StepFrequency=2.5.
            let source: Value =
                serde_json::from_str(include_str!("../fixtures/slider.json")).unwrap();
            let case = source["corpus"]["cases"]
                .as_array()
                .unwrap()
                .iter()
                .find(|c| c["name"] == "input_model_fractional_keyboard")
                .unwrap();
            let expected_key_value =
                f64::from_bits(u64::from_str_radix(case["value"].as_str().unwrap(), 16).unwrap());
            assert_eq!(key.handled(), case["Handled"].as_bool().unwrap());
            assert_eq!(slider.value(), expected_key_value);
            assert_eq!(l.value(), expected_key_value);
            assert_eq!(&*numeric_calls.borrow(), &[10.5, expected_key_value]);
            let inner = t
                .get_visual_descendants()
                .find_map(|c| c.cast::<TextBox>())
                .unwrap();
            let enter = || {
                let mut e = KeyEventArgs::new();
                e.key = Key::Enter;
                e.set_routed_event(Some(InputElement::key_down_event()));
                inner.raise_event(&e);
                assert!(e.handled());
            };
            inner.set_text(Some("command"));
            drain();
            enter();
            assert_eq!(&*calls.borrow(), &["command"]);
            let out = calls.clone();
            text.set_command(Some(RibbonRelayCommand::new_with_parameter(
                move |p| {
                    out.borrow_mut().push(format!(
                        "new:{}",
                        p.unwrap().downcast_ref::<String>().unwrap()
                    ))
                },
                Some(Rc::new(|p| {
                    p.and_then(|p| p.downcast_ref::<String>())
                        .is_some_and(|s| s != "blocked")
                })),
            )));
            enter();
            assert_eq!(&*calls.borrow(), &["command", "new:command"]);
            inner.set_text(Some("blocked"));
            drain();
            enter();
            assert_eq!(calls.borrow().len(), 2);

            let replacement = RibbonModel::new();
            let tab = RibbonTabModel::new(Some("new".into()), Some("New".into()));
            replacement.tabs().add(Some(tab));
            r.set_model(Some(replacement));
            drain();
            spin.set_value(2.);
            slider.set_value(3.);
            text.set_text(Some("detached".into()));
            assert_eq!(s.value(), 10.5);
            assert_eq!(l.value(), expected_key_value);
            assert_eq!(t.text().as_deref(), Some("blocked"));
            s.set_numeric_value(4.);
            l.set_numeric_value(5.);
            t.set_text(Some("old element".to_owned()));
            assert_eq!(spin.value(), 2.);
            assert_eq!(slider.value(), 3.);
            assert_eq!(text.text().unwrap().to_utf8().unwrap(), "detached");
            assert_eq!(r.tabs().count(), 1);
            assert_eq!(r.tabs().get(0).unwrap().id().as_deref(), Some("new"));
            r.set_model(None);
            w.close();
        }
    });
}
