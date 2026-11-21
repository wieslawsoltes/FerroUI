//! Passive compiled/runtime markup with a separately composed async command.
use ferroribbon_core::commands::ICommand;
use std::rc::Rc;

pub struct AsyncCommandViewModel {
    command: Rc<dyn ICommand>,
}
impl AsyncCommandViewModel {
    pub fn new(command: Rc<dyn ICommand>) -> Rc<Self> {
        Rc::new(Self { command })
    }
}
impl PartialEq for AsyncCommandViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
ferroui_base::ferro_markup_type!(class AsyncCommandViewModel {
    namespace:"FerroRibbon.Consumer", this:Rc<AsyncCommandViewModel>,
    handles:[Rc<AsyncCommandViewModel>,Option<Rc<AsyncCommandViewModel>>],
    properties:[
        Command:Rc<dyn ICommand>{get:|this:&Rc<AsyncCommandViewModel>|this.command.clone()},
        Value:i32{get:|_this:&Rc<AsyncCommandViewModel>|7}
    ],
});

#[cfg(test)]
mod tests {
    use super::*;
    use ferroribbon::{commands::dispatcher_command_executor, controls::RibbonButton};
    use ferroribbon_core::commands::RibbonAsyncCommand;
    use ferroui_base::{
        data::core::ValueTypes,
        metadata::{MarkupType, MarkupTyped},
        threading::Dispatcher,
    };
    use ferroui_controls::{Control, Window};
    use futures_channel::oneshot;
    use std::cell::{Cell, RefCell};

    #[test]
    fn compiled_and_runtime_bindings_drive_the_same_async_command_and_native_enabled_state() {
        crate::tests::run(|| {
            let _theme = crate::tests::NativeThemeScope::new();
            ValueTypes::register_reference::<AsyncCommandViewModel>();
            MarkupType::register_all(&[AsyncCommandViewModel::MARKUP]);
            for runtime in [false, true] {
                let value = if runtime {
                    ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                        include_str!("AsyncCommand.xaml"),
                        Some(&crate::CONSUMER_ASSEMBLY),
                    )
                    .unwrap()
                } else {
                    crate::compiled_xaml::try_load(
                        None,
                        "ferres://ferroribbon_markup_consumer/AsyncCommand.xaml",
                    )
                    .unwrap()
                    .unwrap()
                };
                let button = ValueTypes::as_object(&*value)
                    .unwrap()
                    .cast::<RibbonButton>()
                    .unwrap();
                let (sender, receiver) = oneshot::channel::<()>();
                let receiver = Rc::new(RefCell::new(Some(receiver)));
                let runs = Rc::new(Cell::new(0));
                let calls = runs.clone();
                let command = RibbonAsyncCommand::new_with_parameter(
                    dispatcher_command_executor(),
                    move |parameter| {
                        assert_eq!(parameter.unwrap().downcast_ref::<i32>(), Some(&7));
                        calls.set(calls.get() + 1);
                        let receiver = receiver.borrow_mut().take().unwrap();
                        async move {
                            receiver.await.unwrap();
                            Ok(())
                        }
                    },
                    None,
                );
                button.set_data_context(Some(Rc::new(AsyncCommandViewModel::new(command.clone()))));
                let window = Window::new();
                window.set_content(Some(Control::boxed(button.clone())));
                window.show();
                Dispatcher::ui_thread().run_jobs(None);
                assert!(button.visual_children().count() > 0);
                assert!(button.is_enabled());
                assert!(Rc::ptr_eq(
                    &ferroribbon::commands::source_core_command(&button.command().unwrap())
                        .unwrap(),
                    &(command.clone() as Rc<dyn ICommand>)
                ));
                button.perform_click();
                assert_eq!(runs.get(), 1);
                assert!(command.is_running());
                assert!(!button.is_enabled());
                std::thread::spawn(move || sender.send(()).unwrap())
                    .join()
                    .unwrap();
                Dispatcher::ui_thread().run_jobs(None);
                assert!(!command.is_running());
                assert!(button.is_enabled());
                assert_eq!(runs.get(), 1);
                window.close();
            }
        });
    }
}
