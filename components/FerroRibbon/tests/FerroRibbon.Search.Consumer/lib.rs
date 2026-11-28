//! Independent consumer deliberately has no direct ferroribbon-core dependency.
extern crate self as ferroribbon_search_consumer;
use ferroui_base::metadata::MarkupDelegate;
ferroui_base::ferro_thread_local! { static ACTION_INVOCATIONS: std::cell::Cell<i32> = const { std::cell::Cell::new(0) }; }
pub struct SearchActions;
ferroui_base::ferro_static_type!(SearchActions);
impl SearchActions {
    pub fn help() -> MarkupDelegate {
        MarkupDelegate::action(|| ACTION_INVOCATIONS.with(|v| v.set(v.get() + 1)))
    }
    pub fn reset() {
        ACTION_INVOCATIONS.with(|v| v.set(0));
    }
    pub fn calls() -> i32 {
        ACTION_INVOCATIONS.with(std::cell::Cell::get)
    }
}
ferroui_base::ferro_markup_type!(static SearchActions {type_info:SearchActions,namespace:"FerroRibbon.Search.Consumer",static_properties:[Help:MarkupDelegate{get:SearchActions::help}]});
#[allow(clippy::clone_on_copy, clippy::if_same_then_else)]
mod generated {
    ferroui_markup_xaml::include_compiled_xaml!();
}
pub use generated::{compiled_markup, compiled_xaml};
#[cfg(test)]
mod tests {
    use super::compiled_xaml;
    #[test]
    fn search_record_binding_uses_only_public_library_exports() {
        use ferroui_headless::FerroHeadlessPlatformExtensions;
        ferroui_headless::HeadlessUnitTestSession::start_new(|| {
            ferroui_controls::AppBuilder::configure::<ferroui_controls::Application>()
                .use_headless(Default::default())
        })
        .dispatch(
            || {
                ferroribbon::register_types();
                crate::SearchActions::reset();
                let value = compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_search_consumer/Records.xaml",
                )
                .unwrap()
                .unwrap();
                let root = ferroui_base::data::core::ValueTypes::as_object(&*value)
                    .unwrap()
                    .cast::<ferroui_controls::Control>()
                    .unwrap();
                let scope = ferroui_base::controls::NameScope::get_name_scope(&root).unwrap();
                let record = root
                    .resources()
                    .try_get_resource(&"Entry".into(), None)
                    .flatten()
                    .unwrap();
                let entry = ferroui_base::metadata::from_markup_value::<
                    std::rc::Rc<ferroribbon::search::RibbonSearchEntry>,
                >(&Some(record))
                .unwrap();
                assert!(ferroribbon::controls::Ribbon::new().execute_search_entry(Some(entry)));
                assert_eq!(
                    crate::SearchActions::calls(),
                    1,
                    "compiled static action target retains its nominal type"
                );
                let w = ferroui_controls::Window::new();
                w.set_content(Some(ferroui_controls::Control::boxed(root)));
                w.show();
                w.update_layout();
                assert_eq!(
                    scope
                        .find_as::<ferroui_controls::TextBlock>("Label")
                        .unwrap()
                        .text()
                        .as_deref(),
                    Some("Help")
                );
                assert_eq!(
                    scope
                        .find_as::<ferroui_controls::TextBlock>("Recent")
                        .unwrap()
                        .text()
                        .as_deref(),
                    Some("5")
                );
                w.close();
            },
            ferroui_base::threading::CancellationToken::none(),
        )
        .wait()
        .unwrap();
    }
}

#[cfg(test)]
mod typed_action_tests {
    use ferroui_base::{
        BoxedValue,
        metadata::{MarkupDelegate, MarkupTyped, from_markup_value},
    };
    use ferroui_headless::{FerroHeadlessPlatformExtensions, HeadlessUnitTestSession};
    use std::{cell::Cell, rc::Rc};

    #[test]
    fn public_native_action_survives_erased_record_constructor_without_core_dependency() {
        HeadlessUnitTestSession::start_new(|| {
            ferroui_controls::AppBuilder::configure::<ferroui_controls::Application>()
                .use_headless(Default::default())
        })
        .dispatch(
            || {
                ferroribbon::register_types();
                let calls = Rc::new(Cell::new(0));
                let seen = calls.clone();
                let action: BoxedValue =
                    Rc::new(MarkupDelegate::action(move || seen.set(seen.get() + 1)));
                let info = ferroribbon::metadata::search::RibbonSearchEntryMetadata::MARKUP;
                let ctor = info
                    .constructors
                    .iter()
                    .find(|c| c.parameters.len() == 8)
                    .unwrap();
                let value = (ctor.invoke)(&[
                    Some(Rc::new("action".to_string())),
                    Some(Rc::new("Help".to_string())),
                    None,
                    None,
                    None,
                    None,
                    Some(action.clone()),
                    Some(Rc::new(false)),
                ])
                .unwrap();
                let entry = from_markup_value::<Rc<ferroribbon::search::RibbonSearchEntry>>(&value)
                    .unwrap();
                let target = (info.find_property("Target").unwrap().get.unwrap())(&[value])
                    .unwrap()
                    .unwrap();
                assert!(Rc::ptr_eq(&target, &action));
                let root = ferroribbon::controls::Ribbon::new();
                assert!(root.execute_search_entry(Some(entry.clone())));
                assert!(root.execute_search_entry(Some(entry)));
                assert_eq!(
                    calls.get(),
                    2,
                    "snapshot disabled flag does not gate the source Action"
                );
            },
            ferroui_base::threading::CancellationToken::none(),
        )
        .wait()
        .unwrap();
    }
}

#[cfg(test)]
mod native_search_ui_tests {
    #[test]
    fn independent_compiled_search_box_resolves_library_template_and_element_binding() {
        use ferroui_base::{
            controls::NameScope,
            data::core::ValueTypes,
            threading::{CancellationToken, Dispatcher},
        };
        use ferroui_controls::{Control, TextBox};
        use ferroui_headless::FerroHeadlessPlatformExtensions;
        ferroui_headless::HeadlessUnitTestSession::start_new(|| {
            ferroui_controls::AppBuilder::configure::<ferroui_controls::Application>()
                .use_headless(Default::default())
        })
        .dispatch(
            || {
                ferroribbon::register_types();
                let value = crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_search_consumer/SearchBox.xaml",
                )
                .unwrap()
                .unwrap();
                let root = ValueTypes::as_object(&*value)
                    .unwrap()
                    .cast::<Control>()
                    .unwrap();
                let scope = NameScope::get_name_scope(&root).unwrap();
                let ribbon = scope
                    .find("ConsumerRibbon")
                    .unwrap()
                    .cast::<ferroribbon::controls::Ribbon>()
                    .unwrap();
                let search = scope
                    .find("ConsumerSearch")
                    .unwrap()
                    .cast::<ferroribbon::controls::RibbonSearchBox>()
                    .unwrap();
                Dispatcher::ui_thread().run_jobs(None);
                assert!(search.ribbon().unwrap().ptr_eq(&ribbon));
                assert_eq!(search.max_results(), 7);
                assert_eq!(search.placeholder_text().as_deref(), Some("Find a command"));
                // Resolve the actual reusable ControlTheme using only public resources.
                let theme = root
                    .resources()
                    .try_get_resource(
                        &<ferroribbon::controls::RibbonSearchBox as ferroui_base::StaticType>::TYPE
                            .into(),
                        None,
                    )
                    .flatten()
                    .unwrap();
                search.set_theme(Some(
                    ValueTypes::as_object(&*theme)
                        .unwrap()
                        .cast::<ferroui_base::styling::ControlTheme>()
                        .unwrap(),
                ));
                search.apply_styling();
                search.apply_template();
                let mut pending = vec![search.clone().upcast::<Control>()];
                let mut part = None;
                while let Some(control) = pending.pop() {
                    if control.name().as_deref() == Some("PART_TextBox") {
                        part = control.cast::<TextBox>();
                        break;
                    }
                    pending.extend(
                        control
                            .visual_children()
                            .snapshot()
                            .iter()
                            .filter_map(|child| child.cast::<Control>()),
                    );
                }
                assert_eq!(
                    part.unwrap().placeholder_text().as_deref(),
                    Some("Find a command")
                );
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
    }
}
