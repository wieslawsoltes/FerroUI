//! Unit tests of bindings created from markup, ported from the upstream test
//! suite (one file per upstream test class).
//!
//! Property definitions and the property registry are per-thread, and the
//! test harness runs every test on its own thread, so each test starts from
//! a clean set of property registrations.

#![allow(dead_code)]

mod test_support;

mod binding_tests;
mod binding_tests_converters;
mod binding_tests_element_name;
mod binding_tests_method;
mod binding_tests_relative_source;
mod binding_tests_self;
mod binding_tests_source;
mod binding_tests_templated_parent;
mod binding_tests_data_validation;
mod binding_tests_delay;
mod binding_tests_logging;
mod multi_binding_tests;
mod multi_binding_tests_converters;
mod template_binding_tests;
mod expression_node_factory_tests;
mod expression_observer_builder_tests_attached_property;
mod expression_observer_builder_tests_ferro_property;
mod expression_observer_builder_tests_method;
mod expression_observer_builder_tests_negation;
mod expression_observer_builder_tests_property;
mod delayed_binding_tests;
mod selector_parser_tests;
mod container_query_parser_tests;
