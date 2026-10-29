//! Compiled interface bindings retain original models and their optional events.
use ferrodock_model::core::IDockable;
use ferrodock_model_reactiveui::controls::Document;
use ferroui_base::{threading::CancellationToken, Ref};
use ferroui_controls::{AppBuilder, Application, StackPanel, TextBlock, TextBox, Window};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
use std::rc::Rc;
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "ferrodock-core-contract-bindings",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
#[test]
fn original_dockable_interface_bindings_follow_title_owner_replacement_and_two_way_edits() {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            || {
                ferrodock_ferro::register_types();
                ferroui_markup_xaml::register_types();
                crate::compiled_markup::register();
                let value = crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferrodock_markup_tests/DockableContractConsumer.xaml",
                )
                .unwrap()
                .unwrap();
                let panel = value.downcast_ref::<Ref<StackPanel>>().unwrap();
                let title = panel.children().get(0).cast::<TextBlock>().unwrap();
                let editor = panel.children().get(1).cast::<TextBox>().unwrap();
                let owner_text = panel.children().get(2).cast::<TextBlock>().unwrap();
                let model = Document::new();
                let owner = Document::new();
                model.set_title("First".into());
                owner.set_id("Owner A".into());
                model.set_owner(Some(owner.clone()));
                panel.set_data_context(Some(ferrodock_ferro::model_projection::to_dockable_value(
                    model.clone(),
                )));
                let window = Window::new();
                window.set_content(Some(Rc::new(panel.clone())));
                window.show();
                window.update_layout();
                assert_eq!(title.text().as_deref(), Some("First"));
                assert_eq!(editor.text().as_deref(), Some("First"));
                assert_eq!(owner_text.text().as_deref(), Some("Owner A"));
                model.set_title("Live".into());
                assert_eq!(title.text().as_deref(), Some("Live"));
                assert_eq!(editor.text().as_deref(), Some("Live"));
                editor.set_text(Some("Edited"));
                assert_eq!(model.title(), "Edited");
                assert_eq!(title.text().as_deref(), Some("Edited"));
                owner.set_id("Live owner".into());
                assert_eq!(owner_text.text().as_deref(), Some("Live owner"));
                let second_owner = Document::new();
                second_owner.set_id("Owner B".into());
                model.set_owner(Some(second_owner.clone()));
                owner.set_id("Detached".into());
                assert_eq!(owner_text.text().as_deref(), Some("Owner B"));
                second_owner.set_id("New owner".into());
                assert_eq!(owner_text.text().as_deref(), Some("New owner"));
                let replacement = Document::new();
                replacement.set_title("Replacement".into());
                panel.set_data_context(Some(ferrodock_ferro::model_projection::to_dockable_value(
                    replacement.clone(),
                )));
                model.set_title("Detached model".into());
                assert_eq!(title.text().as_deref(), Some("Replacement"));
                replacement.set_title("Still live".into());
                assert_eq!(title.text().as_deref(), Some("Still live"));
                panel.set_data_context(None);
                window.close();
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}
