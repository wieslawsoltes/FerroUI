//! Template parts the compiler resolved (#1039): a compiled template of a control of the
//! prototype hands over its parts, and every lookup gives the object the name lookup gives. The
//! same documents through the run-time loader keep the name lookup. A file of its own: the
//! documents of the corpus, and `generated.rs`, stay as they are.

use std::cell::RefCell;
use std::rc::Rc;

use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::IServiceProvider;
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::primitives::{TemplateAppliedEventArgs, TemplatedControl};
use ferroui_controls::Control;
use ferroui_markup_xaml::xaml_il::runtime::XamlIlRuntimeHelpers;
use ferroui_markup_xaml::RuntimeXamlLoaderConfiguration;
use ferroui_markup_xaml_loader::rust_emitter::{generate_file, GeneratedFile};

use super::generated_template_parts as generated;
use crate::support::app::xaml_test_base;
use crate::support::loader::try_load;

const GENERATED_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/emitter/generated_template_parts.rs");

/// The documents, with the part names each control reads, at their declared positions.
const DOCUMENTS: &[(&str, &str, &[(usize, &str)])] = &[
    (
        "progress_bar.xaml",
        r#"<ProgressBar xmlns="https://github.com/ferroui"><ProgressBar.Template>
            <ControlTemplate TargetType="ProgressBar">
                <Border><Border Name="PART_Indicator"/></Border>
            </ControlTemplate>
        </ProgressBar.Template></ProgressBar>"#,
        &[(0, "PART_Indicator")],
    ),
    (
        "scroll_viewer.xaml",
        r#"<ScrollViewer xmlns="https://github.com/ferroui"><ScrollViewer.Template>
            <ControlTemplate TargetType="ScrollViewer">
                <Grid><ScrollBar Name="PART_VerticalScrollBar"/></Grid>
            </ControlTemplate>
        </ScrollViewer.Template></ScrollViewer>"#,
        // The horizontal bar is an optional part this template leaves out.
        &[(0, "PART_HorizontalScrollBar"), (1, "PART_VerticalScrollBar")],
    ),
    (
        "toggle_switch.xaml",
        r#"<ToggleSwitch xmlns="https://github.com/ferroui"><ToggleSwitch.Template>
            <ControlTemplate TargetType="ToggleSwitch">
                <Grid Name="PART_MovingKnobs"><Panel Name="PART_SwitchKnob"/></Grid>
            </ControlTemplate>
        </ToggleSwitch.Template></ToggleSwitch>"#,
        &[(0, "PART_MovingKnobs"), (3, "PART_SwitchKnob")],
    ),
];

fn sources() -> Vec<(&'static str, &'static str)> {
    DOCUMENTS.iter().map(|(name, source, _)| (*name, *source)).collect()
}

fn generate() -> GeneratedFile {
    generate_file(generated::ASSEMBLY_NAME, generated::ROOT_URI, &sources(), &RuntimeXamlLoaderConfiguration::new(), &[])
}

fn build_generated(name: &str) -> BoxedValue {
    let (_, build) = generated::DOCUMENTS.iter().find(|(document, _)| *document == name).expect("emitted");
    let provider: Rc<dyn IServiceProvider> = XamlIlRuntimeHelpers::create_root_service_provider_v3(None);
    build(Some(provider)).expect("the generated function builds")
}

fn control(root: &BoxedValue) -> Ref<TemplatedControl> {
    ValueTypes::as_object(&**root).unwrap().cast::<TemplatedControl>().unwrap()
}

/// Applies the template of `control`: whether its parts were compiled, and whether each part read
/// by its position is the object its name finds.
fn apply(control: &Ref<TemplatedControl>, parts: &'static [(usize, &'static str)]) -> (bool, Vec<bool>) {
    let seen = Rc::new(RefCell::new(None));
    let sink = seen.clone();
    control.add_handler(TemplatedControl::template_applied_event(), move |_, e: &TemplateAppliedEventArgs| {
        let same = parts
            .iter()
            .map(|(index, name)| e.part::<Control>(*index, name) == e.name_scope().find_as::<Control>(name))
            .collect::<Vec<_>>();
        *sink.borrow_mut() = Some((e.has_compiled_parts(), same));
    });
    control.apply_template();
    let result = seen.borrow_mut().take();
    result.expect("the template was applied")
}

#[test]
fn generated_template_parts_output_is_up_to_date() {
    let _base = xaml_test_base();
    assert!(generate().source == include_str!("generated_template_parts.rs"), "regenerate generated_template_parts.rs");
}

#[test]
#[ignore = "writes emitter/generated_template_parts.rs; run it to regenerate the checked-in output"]
fn regenerate_template_parts_output() {
    let _base = xaml_test_base();
    let file = generate();
    for (name, reason) in &file.documents {
        assert!(reason.is_none(), "{name} is not eligible: {reason:?}");
    }
    std::fs::write(GENERATED_PATH, file.source).unwrap();
}

#[test]
fn a_compiled_template_hands_over_the_parts_the_name_lookup_finds() {
    let _base = xaml_test_base();
    for (name, _, parts) in DOCUMENTS {
        let (compiled, same) = apply(&control(&build_generated(name)), parts);
        assert!(compiled, "{name}: the parts were not handed over");
        assert!(same.iter().all(|same| *same), "{name}: {same:?}");
    }
}

#[test]
fn a_template_loaded_at_run_time_keeps_the_name_lookup() {
    let _base = xaml_test_base();
    for (name, source, parts) in DOCUMENTS {
        let (compiled, same) = apply(&control(&try_load(source).unwrap()), parts);
        assert!(!compiled, "{name}");
        assert!(same.iter().all(|same| *same), "{name}: {same:?}");
    }
}

#[test]
fn parts_built_outside_a_control_do_not_reach_the_next_template() {
    let _base = xaml_test_base();
    let (name, source, parts) = &DOCUMENTS[0];
    // A compiled template built on its own publishes its parts, and nothing takes them.
    let compiled = control(&build_generated(name));
    compiled.template().unwrap().build(&compiled);
    let (handed_over, _) = apply(&control(&try_load(source).unwrap()), parts);
    assert!(!handed_over);
}

#[test]
fn a_replaced_template_keeps_working() {
    let _base = xaml_test_base();
    let (name, source, parts) = &DOCUMENTS[0];
    let target = control(&build_generated(name));
    assert!(apply(&target, parts).0);
    // The template of the same document through the loader replaces the compiled one.
    let replacement = control(&try_load(source).unwrap());
    target.set_template(replacement.template());
    let (compiled, same) = apply(&target, parts);
    assert!(!compiled);
    assert!(same.iter().all(|same| *same), "{same:?}");
}
