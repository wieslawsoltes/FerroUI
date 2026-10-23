//! The options of the generator and of the attribute (`GetGeneratorOptions` and the parsing of the
//! MSBuild properties of the reference; the attribute is C# source there).

use crate::options::{parse_bool, parse_list, parse_rules, GeneratorOptions, ReplacementRule};
use crate::source::parse_attribute_text;

#[test]
fn reads_lists_and_rules_separated_by_commas() {
    assert_eq!(parse_list(" a , b,,c "), ["a", "b", "c"]);
    assert_eq!(
        parse_rules("ViewModel=, =x, broken ,Vm = Page"),
        [ReplacementRule::new("ViewModel", ""), ReplacementRule::new("Vm", "Page")]
    );
    assert_eq!(parse_bool(" TRUE "), Some(true));
    assert_eq!(parse_bool("yes"), None);
}

#[test]
fn appends_the_default_rules_after_the_configured_ones() {
    let mut options = GeneratorOptions::default();
    options.set("type_name_replacement_rules", "ViewModel=Screen").expect("an option");
    let rules = options.effective_type_name_rules();
    assert_eq!(rules, [ReplacementRule::new("ViewModel", "Screen"), ReplacementRule::new("ViewModel", "View")]);
    let namespaces = GeneratorOptions::default().effective_namespace_rules();
    assert_eq!(namespaces, [ReplacementRule::new("view_models", "views"), ReplacementRule::new("_view_model", "_view")]);
}

#[test]
fn keeps_the_defaults_of_unparsable_values() {
    let mut options = GeneratorOptions::default();
    options.set("strip_generic_arity_from_view_name", "maybe").expect("an option");
    options.set("interface_prefixes_to_strip", " , ").expect("an option");
    assert!(options.strip_generic_arity_from_view_name);
    assert_eq!(options.interface_prefixes_to_strip, ["I"]);
    assert!(options.set("unknown", "x").is_err());
}

#[test]
fn reads_the_options_of_the_attribute() {
    let attribute = parse_attribute_text(
        "id = \"main\", generate_view_locator = true, generated_adapter_resolution_mode = ViewResolutionMode::Exact, view_model_mapping_contracts = [IViewFor, crate::Contract], data_template_match_types = [Base<i32>]",
    )
    .expect("the options are read");
    assert_eq!(attribute.id.as_deref(), Some("main"));
    assert_eq!(attribute.generate_view_locator, Some(true));
    assert_eq!(attribute.generate_data_template, None);
    assert_eq!(attribute.generated_adapter_resolution_mode.as_deref(), Some("Exact"));
    assert_eq!(attribute.view_model_mapping_contracts, ["IViewFor", "crate :: Contract"]);
    assert_eq!(attribute.data_template_match_types, ["Base < i32 >"]);
    assert!(parse_attribute_text("unknown = true").is_err());
}
