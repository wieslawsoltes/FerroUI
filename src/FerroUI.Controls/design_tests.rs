//! The tests of the reference `DesignTests` are in the module `design_tests`
//! at the end; the others are tests of this port.

use crate::templates::{FuncDataTemplate, FuncTemplate, IDataTemplate};
use crate::test_support::test_scope;
use crate::{Border, Control, Design};
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::styling::{IStyle, Style};
use ferroui_base::{BoxedValue, FerroObject, Ref};
use std::rc::Rc;

fn as_object<T: ferroui_base::ObjectType + ferroui_base::Upcast<FerroObject>>(value: &Ref<T>) -> Ref<FerroObject> {
    value.clone().upcast()
}

#[test]
fn design_mode_is_off_by_default() {
    assert!(!Design::is_design_mode());
    Design::set_is_design_mode(true);
    assert!(Design::is_design_mode());
    Design::set_is_design_mode(false);
}

#[test]
fn attached_properties_round_trip() {
    let _scope = test_scope();
    let control = Border::new();

    assert_eq!(Design::get_width(&control), 0.0);
    assert_eq!(Design::get_height(&control), 0.0);
    assert!(Design::get_data_context(&control).is_none());

    Design::set_width(&control, 100.0);
    Design::set_height(&control, 50.0);
    Design::set_data_context(&control, Some(Rc::new(1_i32)));

    assert_eq!(Design::get_width(&control), 100.0);
    assert_eq!(Design::get_height(&control), 50.0);
    assert!(Design::get_data_context(&control).is_some());
}

#[test]
fn apply_design_mode_properties_binds_the_properties_that_are_set() {
    let _scope = test_scope();
    let source = Border::new();
    let target = Border::new();

    Design::set_width(&source, 100.0);
    Design::set_data_context(&source, Some(Rc::new("design".to_string())));
    Design::apply_design_mode_properties(&target, &source);

    assert_eq!(target.width(), 100.0);
    assert!(target.height().is_nan());
    assert!(target.data_context().is_some());

    // The values are bound, not copied.
    Design::set_width(&source, 200.0);
    assert_eq!(target.width(), 200.0);
}

#[test]
fn preview_with_of_an_object() {
    let _scope = test_scope();
    let resources = ResourceDictionary::new();
    let preview: Ref<Control> = Border::new().upcast();

    assert!(Design::get_preview_with(&resources).is_none());

    Design::set_preview_with(&as_object(&resources), Some(preview.clone()));
    assert!(Design::get_preview_with(&resources).unwrap().ptr_eq(&preview));

    Design::set_preview_with(&as_object(&resources), None);
    assert!(Design::get_preview_with(&resources).is_none());

    let p = preview.clone();
    Design::set_preview_with_template(&as_object(&resources), Some(FuncTemplate::new(move || Some(p.clone()))));
    assert!(Design::get_preview_with(&resources).unwrap().ptr_eq(&preview));
}

#[test]
fn a_visual_cannot_be_previewed_with_a_control() {
    let _scope = test_scope();
    let visual = Border::new();
    let preview: Ref<Control> = Border::new().upcast();

    Design::set_preview_with(&as_object(&visual), Some(preview));

    assert!(Design::get_preview_with(&visual).is_none());
}

#[test]
fn preview_with_of_a_style_is_shared_with_its_object() {
    let _scope = test_scope();
    let style = Style::new();
    let handle: Rc<dyn IStyle> = style.clone().into();
    let preview: Ref<Control> = Border::new().upcast();

    Design::set_preview_with_for_style(&handle, Some(preview.clone()));

    assert!(Design::get_preview_with_for_style(&handle).unwrap().ptr_eq(&preview));
    assert!(Design::get_preview_with(&style).unwrap().ptr_eq(&preview));

    let p = preview.clone();
    Design::set_preview_with_template_for_style(&handle, Some(FuncTemplate::new(move || Some(p.clone()))));
    assert!(Design::get_preview_with_for_style(&handle).unwrap().ptr_eq(&preview));

    Design::set_preview_with_for_style(&handle, None);
    assert!(Design::get_preview_with(&style).is_none());
}

#[test]
fn preview_with_and_data_context_of_a_data_template() {
    let _scope = test_scope();
    let template: Rc<dyn IDataTemplate> =
        FuncDataTemplate::new(|_: Option<&BoxedValue>| true, |_, _| None::<Ref<Control>>, false);
    let other: Rc<dyn IDataTemplate> =
        FuncDataTemplate::new(|_: Option<&BoxedValue>| true, |_, _| None::<Ref<Control>>, false);
    let preview: Ref<Control> = Border::new().upcast();

    assert!(Design::get_data_context_for_data_template(&template).is_none());
    Design::set_data_context_for_data_template(&template, Some(Rc::new(5_i32)));
    assert!(Design::get_data_context_for_data_template(&template).is_some());
    assert!(Design::get_data_context_for_data_template(&other).is_none());

    Design::set_preview_with_for_data_template(&template, Some(preview.clone()));
    assert!(Design::get_preview_with_for_data_template(&template).unwrap().ptr_eq(&preview));
    assert!(Design::get_preview_with_for_data_template(&other).is_none());

    let p = preview.clone();
    Design::set_preview_with_template_for_data_template(&template, Some(FuncTemplate::new(move || Some(p.clone()))));
    assert!(Design::get_preview_with_for_data_template(&template).unwrap().ptr_eq(&preview));
}

mod create_preview_with_control {
    use super::*;
    use crate::{Application, ContentControl, Panel, PreviewTarget};

    fn lines(message: &Ref<Control>) -> Vec<String> {
        let panel = message.cast::<Panel>().expect("a message is a panel");
        panel
            .children()
            .snapshot()
            .iter()
            .map(|line| line.clone().cast::<crate::TextBlock>().and_then(|line| line.text()).unwrap())
            .collect()
    }

    #[test]
    fn a_control_is_previewed_as_itself() {
        let _scope = test_scope();
        let control: Ref<Control> = Border::new().upcast();

        let preview = Design::create_preview_with_control(&PreviewTarget::Object(control.clone().upcast())).unwrap();

        assert!(preview.ptr_eq(&control));
    }

    #[test]
    fn a_style_is_added_to_the_control_it_is_previewed_with() {
        let _scope = test_scope();
        let style = Style::new();
        let handle: Rc<dyn IStyle> = style.clone().into();
        let substitute: Ref<Control> = Border::new().upcast();
        Design::set_preview_with_for_style(&handle, Some(substitute.clone()));

        // As a style and as an object.
        let preview = Design::create_preview_with_control(&PreviewTarget::Style(handle)).unwrap();
        assert!(preview.ptr_eq(&substitute));
        assert_eq!(substitute.styles().count(), 1);

        let style = Style::new();
        Design::set_preview_with_for_style(&style.clone().into(), Some(substitute.clone()));
        let preview = Design::create_preview_with_control(&PreviewTarget::Object(as_object(&style))).unwrap();
        assert!(preview.ptr_eq(&substitute));
        assert_eq!(substitute.styles().count(), 2);
    }

    #[test]
    fn a_style_without_preview_gets_a_message() {
        let _scope = test_scope();
        let style = Style::new();

        let preview = Design::create_preview_with_control(&PreviewTarget::Object(as_object(&style))).unwrap();
        let lines = lines(&preview);
        assert_eq!(lines.len(), 5);
        assert_eq!(lines[0], "Styles can't be previewed without Design.PreviewWith. Add");
        assert_eq!(lines[4], "before setters in your first Style");
    }

    #[test]
    fn a_resource_dictionary_is_merged_into_the_control_it_is_previewed_with() {
        let _scope = test_scope();
        let resources = ResourceDictionary::new();

        let preview = Design::create_preview_with_control(&PreviewTarget::Object(as_object(&resources))).unwrap();
        assert_eq!(lines(&preview)[4], "in your resource dictionary");

        let substitute: Ref<Control> = Border::new().upcast();
        Design::set_preview_with(&as_object(&resources), Some(substitute.clone()));
        let preview = Design::create_preview_with_control(&PreviewTarget::Object(as_object(&resources))).unwrap();

        assert!(preview.ptr_eq(&substitute));
        assert_eq!(substitute.resources().merged_dictionaries().count(), 1);
    }

    #[test]
    fn a_data_template_is_previewed_in_a_content_control() {
        let _scope = test_scope();
        let template: Rc<dyn IDataTemplate> =
            FuncDataTemplate::new(|_: Option<&BoxedValue>| true, |_, _| None::<Ref<Control>>, false);

        let preview = Design::create_preview_with_control(&PreviewTarget::DataTemplate(template.clone())).unwrap();
        assert_eq!(lines(&preview)[0], "IDataTemplate can't be previewed without Design.PreviewWith.");

        // With design data: a content control that shows the data.
        Design::set_data_context_for_data_template(&template, Some(Rc::new(5_i32)));
        let preview = Design::create_preview_with_control(&PreviewTarget::DataTemplate(template.clone())).unwrap();
        let content_control = preview.cast::<ContentControl>().unwrap();
        assert!(content_control.content_template().is_some());
        assert_eq!(content_control.content().and_then(|c| c.downcast_ref::<i32>().copied()), Some(5));
        assert!(content_control.data_context().is_some());

        // With a content control to preview with: it gets the template.
        let substitute = ContentControl::new();
        Design::set_preview_with_for_data_template(&template, Some(substitute.clone().upcast()));
        let preview = Design::create_preview_with_control(&PreviewTarget::DataTemplate(template)).unwrap();
        assert!(preview.ptr_eq(&substitute));
        assert!(substitute.content_template().is_some());
    }

    #[test]
    fn what_is_not_a_control_cannot_be_previewed() {
        let _scope = test_scope();
        let application = Application::new();
        let object = ferroui_base::FerroObject::new();

        for target in [PreviewTarget::Object(as_object(&application)), PreviewTarget::Object(object), PreviewTarget::Other]
        {
            let preview = Design::create_preview_with_control(&target).unwrap();
            let text = preview.cast::<crate::TextBlock>().and_then(|text| text.text());
            assert_eq!(text.as_deref(), Some("This file cannot be previewed in design view"));
        }

        // An object that is previewed with a control.
        let resources_like = ferroui_base::FerroObject::new();
        let substitute: Ref<Control> = Border::new().upcast();
        Design::set_preview_with(&resources_like, Some(substitute.clone()));
        let preview = Design::create_preview_with_control(&PreviewTarget::Object(resources_like)).unwrap();
        assert!(preview.ptr_eq(&substitute));
    }
}

#[test]
fn apply_design_mode_properties_adds_the_design_style_to_the_target() {
    use ferroui_base::styling::{style_ptr_eq, IStyle, Style};
    use std::rc::Rc;

    let source = crate::Border::new();
    let target = crate::Border::new();
    let style: Rc<dyn IStyle> = Style::new().into();
    Design::set_design_style(&source, style.clone());

    assert!(style_ptr_eq(&Design::get_design_style(&source).unwrap(), &style));

    Design::apply_design_mode_properties(&target, &source);

    assert_eq!(1, target.styles().count());
    assert!(style_ptr_eq(&target.styles().get(0), &style));
}

/// The reference `DesignTests`.
mod design_tests {
    use super::as_object;
    use crate::primitives::TemplatedControl;
    use crate::templates::{FuncDataTemplate, FuncTemplate, IDataTemplate};
    use crate::test_support::{boxed_str, string_of};
    use crate::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
    use crate::{Application, Border, Button, ContentControl, Control, Design, PreviewTarget, TextBlock, Window};
    use ferroui_base::controls::{ResourceDictionary, ResourceHostRef};
    use ferroui_base::data::BindingPriority;
    use ferroui_base::media::immutable::ImmutableSolidColorBrush;
    use ferroui_base::media::{Brushes, Color, Colors, IBrush};
    use ferroui_base::reactive::ObservableExt;
    use ferroui_base::styling::{IStyle, Selectors, Setter, Style};
    use ferroui_base::BoxedValue;
    use std::rc::Rc;

    fn start() -> UnitTestApplicationScope {
        UnitTestApplication::start(TestServices::styled_window())
    }

    /// A data template for strings that shows the string in a text block.
    fn string_data_template() -> Rc<dyn IDataTemplate> {
        FuncDataTemplate::for_type::<String>(
            |data, _| {
                let text_block = TextBlock::new();
                text_block.set_text(Some(data));
                Some(text_block.upcast())
            },
            false,
        )
    }

    fn same_template(expected: &Rc<dyn IDataTemplate>, actual: Option<Rc<dyn IDataTemplate>>) -> bool {
        actual.is_some_and(|actual| Rc::as_ptr(&actual) as *const () == Rc::as_ptr(expected) as *const ())
    }

    #[test]
    fn should_preview_resource_dictionary_with_template() {
        let _app = start();

        let dictionary = ResourceDictionary::new();
        dictionary.add_value("TestColor", Colors::GREEN);
        Design::set_preview_with_template(
            &as_object(&dictionary),
            Some(FuncTemplate::new(|| {
                let border = Border::new();
                // The reference binds the background with the dynamic resource
                // extension of the markup library, which this crate is not built
                // on: the binding is the observable of the resource, with the
                // conversion of a colour to a brush that the extension applies.
                let background = ResourceHostRef::from(&border).resource_observable("TestColor", None).select(
                    |value: Option<BoxedValue>| {
                        let color = value.and_then(|value| value.downcast_ref::<Color>().copied())?;
                        Some(Rc::new(ImmutableSolidColorBrush::new(color)) as Rc<dyn IBrush>)
                    },
                );
                border.bind(Border::background_property(), background, BindingPriority::LocalValue);
                Some(border.upcast())
            })),
        );

        let preview = Design::create_preview_with_control(&PreviewTarget::Object(as_object(&dictionary)));

        let border = preview.and_then(|preview| preview.cast::<Border>()).expect("a border");
        let background = border.background().expect("a background");
        assert_eq!(Colors::GREEN, background.as_solid_color_brush().expect("a solid colour brush").color());
    }

    #[test]
    fn should_preview_data_template_with_content_control() {
        let _app = start();

        const TEST_DATA: &str = "Test Data";
        let data_template = string_data_template();
        Design::set_preview_with_template_for_data_template(
            &data_template,
            Some(FuncTemplate::new(|| {
                let content_control = ContentControl::new();
                content_control.set_content(boxed_str(TEST_DATA));
                Some(content_control.upcast())
            })),
        );

        let preview = Design::create_preview_with_control(&PreviewTarget::DataTemplate(data_template.clone()));

        let preview_content_control = preview.and_then(|preview| preview.cast::<ContentControl>());
        let preview_content_control = preview_content_control.expect("a content control");
        assert_eq!(Some(TEST_DATA.to_string()), preview_content_control.content().as_ref().and_then(string_of));
        assert!(same_template(&data_template, preview_content_control.content_template()));
    }

    #[test]
    fn should_preview_data_template_with_data_context() {
        let _app = start();

        const TEST_DATA: &str = "Test Data";
        let data_template = string_data_template();
        Design::set_data_context_for_data_template(&data_template, boxed_str(TEST_DATA));

        let preview = Design::create_preview_with_control(&PreviewTarget::DataTemplate(data_template.clone()));

        let preview_content_control = preview.and_then(|preview| preview.cast::<ContentControl>());
        let preview_content_control = preview_content_control.expect("a content control");
        assert_eq!(Some(TEST_DATA.to_string()), preview_content_control.content().as_ref().and_then(string_of));
        assert!(same_template(&data_template, preview_content_control.content_template()));
    }

    #[test]
    fn should_preview_control_with_another_control() {
        let _app = start();

        let control = TextBlock::new();
        Design::set_preview_with_template(
            &as_object(&control),
            Some(FuncTemplate::new(|| Some(Border::new().upcast::<Control>()))),
        );

        let preview = Design::create_preview_with_control(&PreviewTarget::Object(as_object(&control)));

        assert!(preview.is_some_and(|preview| preview.is::<Border>()));
    }

    #[test]
    fn should_apply_design_mode_properties_from_control_to_window() {
        let _app = start();

        // Use-case: User previews a control, which is wrapped by the window.
        let window = Window::new();
        let control = ContentControl::new();
        window.set_content(Some(Control::boxed(control.clone())));

        Design::set_width(&control, 200.0);
        Design::set_height(&control, 150.0);
        Design::set_data_context(&control, boxed_str("TestDataContext"));
        let design_style: Rc<dyn IStyle> = Style::with_setters(
            Selectors::of_type::<ContentControl>(),
            [Setter::new(TemplatedControl::background_property(), Some(Brushes::yellow()))],
        )
        .into();
        Design::set_design_style(&control, design_style);

        Design::apply_design_mode_properties(&window, &control);

        assert_eq!(200.0, window.width());
        assert_eq!(150.0, window.height());
        assert_eq!(Some("TestDataContext".to_string()), window.data_context().as_ref().and_then(string_of));
        let background_property = TemplatedControl::background_property().as_property();
        assert!(window.styles().snapshot().iter().any(|s| {
            let style = s.as_object().and_then(|style| style.downcast_ref::<Style>()).expect("a style");
            let setter = style.setters().get(0);
            let setter = setter.as_any().and_then(|setter| setter.downcast_ref::<Setter>()).expect("a setter");
            setter.property().is_some_and(|property| std::ptr::eq(property, background_property))
        }));
    }

    #[test]
    fn should_not_throw_exception_on_generic_style() {
        let _app = start();

        let style = Style::with_selector(Selectors::of_type::<Button>());
        let preview = Design::create_preview_with_control(&PreviewTarget::Object(as_object(&style)));

        // We are not going to test specific content of the placeholder preview control.
        // But it should not throw and should not return null at least.
        assert!(preview.is_some());
    }

    #[test]
    fn should_not_throw_exception_on_generic_resource_dictionary() {
        let _app = start();

        let preview =
            Design::create_preview_with_control(&PreviewTarget::Object(as_object(&ResourceDictionary::new())));

        assert!(preview.is_some());
    }

    #[test]
    fn should_not_throw_exception_on_generic_data_template() {
        let _app = start();

        let preview = Design::create_preview_with_control(&PreviewTarget::DataTemplate(string_data_template()));

        assert!(preview.is_some());
    }

    #[test]
    fn should_not_throw_exception_on_application() {
        let _app = start();

        let app = Application::new();
        let preview = Design::create_preview_with_control(&PreviewTarget::Object(as_object(&app)));

        assert!(preview.is_some());
    }
}
