//! Actual compiled/runtime binding consumer of the selected SDK's markup stream.
use ferroui_base::data::core::plugins::TaskValue;
use std::rc::Rc;

pub struct DialogFutureViewModel {
    outcome: TaskValue,
}
impl PartialEq for DialogFutureViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
ferroui_base::ferro_markup_type!(class DialogFutureViewModel {
 namespace:"FerroRibbon.Consumer", this:Rc<DialogFutureViewModel>, handles:[Rc<DialogFutureViewModel>],
 properties:[Outcome:TaskValue{get:|v:&Rc<DialogFutureViewModel>|v.outcome.clone()}]
});

#[cfg(test)]
mod tests {
    use super::*;
    use ferroribbon::controls::{RibbonColorDialog, RibbonColorDialogHost};
    use ferroui_base::{
        data::core::ValueTypes,
        media::Color,
        metadata::{MarkupType, MarkupTyped, from_markup_value, into_markup_value},
        threading::Dispatcher,
    };
    use ferroui_controls::{ContentControl, Control, Window};

    #[test]
    fn both_loaders_stream_the_native_dialog_result_from_public_show_async_metadata() {
        crate::tests::run(|| {
            let _theme = crate::tests::NativeThemeScope::new();
            ValueTypes::register_reference::<DialogFutureViewModel>();
            MarkupType::register_all(&[DialogFutureViewModel::MARKUP]);
            for runtime in [false, true] {
                for primary in [false, true] {
                    let value = if runtime {
                        ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                            include_str!("DialogFuture.xaml"),
                            Some(&crate::CONSUMER_ASSEMBLY),
                        )
                        .unwrap()
                    } else {
                        crate::compiled_xaml::try_load(
                            None,
                            "ferres://ferroribbon_markup_consumer/DialogFuture.xaml",
                        )
                        .unwrap()
                        .unwrap()
                    };
                    let content = ValueTypes::as_object(&*value)
                        .unwrap()
                        .cast::<ContentControl>()
                        .unwrap();
                    let window = Window::new();
                    window.set_content(Some(Control::boxed(content.clone())));
                    window.show();
                    window.update_layout();
                    let method = RibbonColorDialog::MARKUP
                        .methods
                        .iter()
                        .find(|m| m.parameters.len() == 2)
                        .unwrap();
                    let value = (method.invoke)(&[
                        into_markup_value(Some(
                            window.clone().upcast::<ferroui_controls::TopLevel>(),
                        )),
                        into_markup_value(Color::from_rgb(1, 2, 3)),
                    ])
                    .unwrap();
                    let outcome = from_markup_value::<TaskValue>(&value).unwrap();
                    assert!(!outcome.is_completed());
                    content.set_data_context(Some(Rc::new(Rc::new(DialogFutureViewModel {
                        outcome: outcome.clone(),
                    }))));
                    Dispatcher::ui_thread().run_jobs(None);
                    assert!(content.content().is_none());
                    let host = window
                        .get_visual_descendants()
                        .find_map(|v| v.cast::<RibbonColorDialogHost>())
                        .unwrap();
                    if primary {
                        host.primary_button().unwrap().on_click();
                    } else {
                        host.close_button().unwrap().on_click();
                    }
                    assert!(!outcome.is_completed(), "result follows Closed callbacks");
                    Dispatcher::ui_thread().run_jobs(None);
                    assert!(outcome.is_completed_successfully());
                    let color = content
                        .content()
                        .map(|v| *v.downcast_ref::<Color>().unwrap());
                    assert_eq!(color, primary.then(|| Color::from_rgb(1, 2, 3)));
                    content.set_data_context(None);
                    window.close();
                    window.set_content(None);
                }
            }
        });
    }
}
