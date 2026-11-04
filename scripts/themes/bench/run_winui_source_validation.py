#!/usr/bin/env python3
"""Run focused development gates against one immutable repository revision.

This compiles actual framework test crates for the selected source change. The
metadata suite checks Base and downstream Controls expansion; the control-theme
suites check the runtime, emitter and differential corpus, then type-check fresh
WinUI output. The explicit control-theme-regions suite additionally requires the
region runtime/grammar cases and both generated program forms. Filters narrow
execution, not compilation. The coercion suite checks owned-value cleanup,
metadata registration/inheritance, property behavior and downstream typed APIs.
The inheritance suite adds inherited-change lifetime and registration checks.
The property-capabilities suite combines those cases with default coercion and
shared boxed delivery, then checks fresh WinUI generation as well as Controls.
The additive nullable-operations suite preserves that exact inventory, runs all
24 existing value-type tests and requires each frozen nullable cleanup oracle.
The additive coercion-construction suite retains the nullable inventory and
requires each frozen metadata/effective-value construction cleanup oracle.
The additive property-transition suite retains those exact cases and checks
the original-frame oracles for the shared write and notification engines.
These development suites make no
production size, throughput, PGO or shipping-acceptance claim. Toolchain, commands,
source identity, lockfile, logs and resource reports survive success or failure.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import signal
import subprocess
import sys
import time


HERE = Path(__file__).resolve().parent
HOST = "x86_64-unknown-linux-gnu"

# Counts belong only to the explicit frozen region/property suites. Existing
# metadata and whole-theme command plans and acceptance rules remain unchanged.
REGION_EXPECTED_SUMMARIES = {
    "construction-runtime": {"passed": 2, "failed": 0, "ignored": 0},
    "region-runtime": {"passed": 3, "failed": 0, "ignored": 0},
    "construction-grammar": {"passed": 6, "failed": 0, "ignored": 0},
    "region-grammar": {"passed": 7, "failed": 0, "ignored": 0},
    "emitter-differential": {"passed": 25, "failed": 0, "ignored": 2},
}

COERCION_EXPECTED_SUMMARIES = {
    "coercion-oracles": {"passed": 5, "failed": 0, "ignored": 0},
    "property-runtime": {"passed": 307, "failed": 0, "ignored": 0},
    "property-metadata": {"passed": 14, "failed": 0, "ignored": 0},
    "coercer-registration": {"passed": 1, "failed": 0, "ignored": 0},
    "metadata": {"passed": 66, "failed": 0, "ignored": 0},
}

INHERITANCE_EXPECTED_SUMMARIES = {
    **COERCION_EXPECTED_SUMMARIES,
    "inheritance-oracles": {"passed": 4, "failed": 0, "ignored": 0},
    "property-registry": {"passed": 12, "failed": 0, "ignored": 0},
}

PROPERTY_CAPABILITIES_EXPECTED_SUMMARIES = {
    **INHERITANCE_EXPECTED_SUMMARIES,
    "coercion-oracles": {"passed": 7, "failed": 0, "ignored": 0},
    "default-coercion-oracles": {"passed": 3, "failed": 0, "ignored": 0},
    "metadata": {"passed": 67, "failed": 0, "ignored": 0},
}

PROPERTY_CAPABILITIES_EXPECTED_PROGRAMS = {
    "construction_programs": 18,
    "region_programs": 34,
}

# Cargo filters are substrings: metadata:: also selects these four existing
# property-runtime tests. Preserve both executions and guard their exact names.
PROPERTY_CAPABILITIES_EXPECTED_OVERLAPS = {
    "tests::ferro_object_tests_metadata::default_value_can_be_overridden_in_add_ownered_property": ["metadata", "property-runtime"],
    "tests::ferro_object_tests_metadata::default_value_can_be_overridden_in_derived_class": ["metadata", "property-runtime"],
    "tests::ferro_object_tests_metadata::unset_value_can_be_overridden_in_add_ownered_property": ["metadata", "property-runtime"],
    "tests::ferro_object_tests_metadata::unset_value_can_be_overridden_in_derived_class": ["metadata", "property-runtime"],
}


# Additive nullable gate: these complete name sets are frozen with the reviewed source.
NULLABLE_PROPERTY_REQUIRED_TESTS = (
    'property_store::effective_value::coercion_tests::recoercion_matches_original_owned_value_frames_and_unwind',
    'property_store::effective_value::coercion_tests::recoercion_matches_original_reentrant_reads_writes_and_priority_snapshots',
    'property_store::effective_value::coercion_tests::metadata_coercion_keeps_clone_only_api_and_callback_pair_lifetimes',
    'property_store::effective_value::coercion_tests::metadata_replacement_matches_original_callback_drop_unwind',
    'property_store::effective_value::coercion_tests::recoercion_keeps_nan_equality',
    'property_store::effective_value::inheritance_tests::inherited_notification_matches_original_values_defaults_and_unwind',
    'property_store::effective_value::inheritance_tests::inherited_notification_matches_original_clone_equality_and_handler_reentry',
    'property_store::effective_value::inheritance_tests::inherited_notifications_keep_source_metadata_across_overrides_and_attached_hosts',
    'property_store::effective_value::inheritance_tests::explicit_inheritance_registration_and_opt_out_keep_public_behavior',
    'ferro_property::styled_property_options_tests::registration_transfers_the_original_coercer',
    'ferro_property_metadata::metadata_table_tests::colliding_cache_entries_follow_replaced_and_removed_registrations',
    'ferro_property_metadata::metadata_table_tests::unregister_invalidates_inherited_and_relocated_overrides',
    'tests::ferro_property_registry_tests::registered_properties_count_reflects_newly_added_attached_property',
    'ferro_property_metadata::coercion_request_tests::borrowed_coercion_requests_keep_the_single_pointer_layout_and_unbounded_types',
    'property_store::effective_value::coercion_tests::end_reevaluation_tests::end_reevaluation_matches_original_owned_values_reentry_and_unwind',
    'property_store::effective_value::coercion_tests::end_reevaluation_tests::end_reevaluation_keeps_nan_and_the_retained_uncommon_callback',
    'property_store::value_store::default_coercion_tests::default_coercion_matches_original_metadata_values_factory_and_unwind',
    'property_store::value_store::default_coercion_tests::default_coercion_keeps_metadata_reload_and_returned_entry_cleanup_during_reentry',
    'property_store::value_store::default_coercion_tests::default_coercion_retains_partial_eq_nan_and_absent_callback_behavior',
    'tests::ferro_object_tests_binding::boxed_frame_disposes_the_subscription_returned_after_synchronous_completion_or_error',
    'tests::ferro_object_tests_binding::boxed_frame_subscribes_when_unmasked_and_restores_default_after_disposal',
    'tests::ferro_object_tests_binding::boxed_bindings_snapshot_values_instead_of_retaining_the_incoming_box',
    'tests::ferro_object_tests_binding::retained_boxed_source_observer_does_not_keep_a_disposed_bindings_values_alive',
    'tests::ferro_object_tests_binding::boxed_frame_keeps_its_first_fallback_across_unsubscribe_and_resubscribe',
    'tests::ferro_object_tests_binding::boxed_notification_keeps_snapshots_and_drops_them_after_payload_clone',
    'tests::ferro_object_tests_binding::boxed_binding_conversion_releases_its_temporary_before_delivery_or_clone_unwind',
    'tests::ferro_object_tests_binding::boxed_input_drop_runs_after_the_delivery_releases_its_binding',
)

NULLABLE_VALUE_TYPE_TESTS = (
    'data::core::value_type::tests::nullable_batches_install_typed_casts_in_declaration_order',
    'data::core::value_type::tests::registering_a_reference_twice_preserves_a_custom_handle_cast',
    'data::core::value_type::tests::replacing_one_conversion_route_preserves_the_other_registered_views',
    'data::core::value_type::tests::registered_numeric_conversions_keep_exact_target_types_and_checked_edges',
    'data::core::value_type::tests::registered_casts_of_reference_types_apply_to_every_form',
    'data::core::value_type::tests::reference_casts_make_nothing_else_assignable',
    'data::core::value_type::tests::reference_object_is_the_object_form_of_a_handle',
    'data::core::value_type::tests::borrowed_and_boxed_casts_share_the_callback_and_allow_reentrant_lookup',
    'data::core::value_type::tests::enumerations_convert_to_numbers_and_numbers_to_enumerations',
    'data::core::value_type::tests::values_without_a_display_form_print_the_full_name_of_their_type',
    'data::core::value_type::tests::types_print_their_full_name',
    'data::core::value_type::tests::a_number_becomes_text_as_the_culture_formats_it',
    'data::core::value_type::tests::text_becomes_a_number_as_the_culture_writes_it',
    'data::core::value_type::tests::every_number_converts_to_every_numeric_type',
    'data::core::value_type::tests::floating_point_numbers_and_decimals_convert_to_each_other',
    'data::core::value_type::tests::a_decimal_converts_to_an_integer_rounded_and_in_range',
    'data::core::value_type::tests::numbers_convert_between_the_other_types_as_the_runtime_converts_them',
    'data::core::value_type::tests::booleans_and_numbers_convert_to_each_other',
    'data::core::value_type::tests::characters_and_integers_convert_to_each_other',
    'data::core::value_type::tests::numbers_and_text_convert_to_each_other',
    'data::core::value_type::tests::the_implicit_conversions_of_numbers_are_those_of_the_language',
    'data::core::value_type::tests::the_implicit_conversions_of_a_character_and_of_a_boolean_are_those_of_the_table',
    'data::core::value_type::tests::values_are_ordered_with_the_right_operand_converted_to_the_type_of_the_left_one',
    'data::core::value_type::tests::registered_orderings_compare_values_of_the_type',
)
NULLABLE_OPERATION_TESTS = (
    'data::core::value_type::nullable_operation_tests::ordinary_nullable_operations_match_original_cleanup_oracles',
    'data::core::value_type::nullable_operation_tests::nullable_operations_preserve_nested_exact_types_and_nulls',
    'data::core::value_type::nullable_operation_tests::nullable_clone_reentry_preserves_current_snapshot_and_later_overrides',
    'data::core::value_type::nullable_operation_tests::nullable_registration_keeps_borrowed_cast_and_conversion_views_independent',
    'data::core::value_type::nullable_operation_tests::reference_nullable_operations_keep_non_clone_identity_and_registration_order',
    'data::core::value_type::nullable_operation_tests::nullable_operation_layout_keeps_enclosing_registrations_compact',
)
NULLABLE_EXPECTED_SUMMARIES = {
    **PROPERTY_CAPABILITIES_EXPECTED_SUMMARIES,
    "value-types": {"passed": len(NULLABLE_VALUE_TYPE_TESTS), "failed": 0, "ignored": 0},
    "nullable-oracles": {"passed": len(NULLABLE_OPERATION_TESTS), "failed": 0, "ignored": 0},
}


# Populated only from the reviewed, frozen construction source before execution.
COERCION_CONSTRUCTION_TESTS = (
    'property_store::effective_value::construction_tests::frozen_metadata_construction_matches_original_owned_frames_and_unwind',
    'property_store::effective_value::construction_tests::public_type_safe_metadata_keeps_unfrozen_clone_only_behavior',
    'property_store::effective_value::construction_tests::frozen_metadata_generation_keeps_reentrant_input_ownership',
    'property_store::effective_value::construction_tests::effective_construction_matches_original_default_and_inherited_frames',
    'property_store::effective_value::construction_tests::effective_construction_keeps_metadata_snapshot_during_clone_reentry',
)
COERCION_CONSTRUCTION_EXPECTED_SUMMARIES = {
    **NULLABLE_EXPECTED_SUMMARIES,
    "construction-oracles": {"passed": len(COERCION_CONSTRUCTION_TESTS), "failed": 0, "ignored": 0},
}

PROPERTY_TRANSITION_TESTS = (
    'property_store::effective_value::transition_tests::shared_write_matches_original_operations_and_unwind',
    'property_store::effective_value::transition_tests::shared_write_keeps_live_priority_reads_under_reentry',
    'property_store::effective_value::transition_tests::shared_notification_keeps_snapshot_lifetimes_and_unwind',
    'property_store::effective_value::transition_tests::borrowed_transition_frames_keep_storage_layout_and_nan_semantics',
)
PROPERTY_TRANSITION_EXPECTED_SUMMARIES = {
    **COERCION_CONSTRUCTION_EXPECTED_SUMMARIES,
    "transition-oracles": {"passed": len(PROPERTY_TRANSITION_TESTS), "failed": 0, "ignored": 0},
}


def identity(path):
    path = Path(path).resolve(strict=True)
    with path.open("rb") as stream:
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
    return {"path": str(path), "bytes": path.stat().st_size, "sha256": digest}


def save(path, value):
    temporary = path.with_suffix(".tmp")
    temporary.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")
    temporary.replace(path)


def git(source, *arguments):
    return subprocess.check_output(["git", "-C", str(source), *arguments], text=True).strip()


def suite_commands(cargo, suite):
    """Fixed command plans: an input selects a suite, never arbitrary shell code."""
    common = [cargo, "test", "--locked", "--target", HOST, "--lib"]
    if suite == "metadata":
        base = [*common, "-p", "ferroui-base", "--features", "compiler-metadata,hot-reload"]
        tests = [
            (label, [*base, test_filter, "--", "--test-threads=1"])
            for label, test_filter in (("metadata", "metadata::"), ("object-smoke", "ferro_object_tests::"),
                                       ("properties", "tests::ferro_object_tests"))
        ]
        downstream = [cargo, "check", "--locked", "--target", HOST, "--lib", "-p", "ferroui-base",
                      "-p", "ferroui-controls", "--features", "ferroui-base/compiler-metadata,ferroui-base/hot-reload"]
    elif suite in {"coercion", "inheritance", "property-capabilities"}:
        base = [*common, "-p", "ferroui-base", "--features", "compiler-metadata,hot-reload"]
        tests = [
            (label, [*base, test_filter, "--", "--test-threads=1"])
            for label, test_filter in (
                ("coercion-oracles", "property_store::effective_value::coercion_tests::"),
                ("property-runtime", "tests::ferro_object_tests"),
                ("property-metadata", "tests::ferro_property_tests::"),
                ("coercer-registration", "ferro_property::styled_property_options_tests::"),
                ("metadata", "metadata::"),
            )
        ]
        if suite in {"inheritance", "property-capabilities"}:
            tests.insert(1, (
                "inheritance-oracles", [*base, "property_store::effective_value::inheritance_tests::",
                                        "--", "--test-threads=1"],
            ))
            tests.insert(4, (
                "property-registry", [*base, "tests::ferro_property_registry_tests::", "--", "--test-threads=1"],
            ))
        if suite == "property-capabilities":
            tests.insert(2, (
                "default-coercion-oracles", [*base, "property_store::value_store::default_coercion_tests::",
                                             "--", "--test-threads=1"],
            ))
        downstream = [cargo, "check", "--locked", "--target", HOST, "--lib", "-p", "ferroui-base",
                      "-p", "ferroui-controls", "--features", "ferroui-base/compiler-metadata,ferroui-base/hot-reload"]
    elif suite in {"control-theme", "control-theme-regions"}:
        tests = [
            ("construction-runtime", [*common, "-p", "ferroui-markup-xaml",
                                       "control_theme_program::tests::", "--", "--test-threads=1"]),
            ("construction-grammar", [*common, "-p", "ferroui-markup-xaml-loader", "--features", "emitter",
                                       "control_theme_programs::tests::", "--", "--test-threads=1"]),
            ("emitter-differential", [*common, "-p", "ferroui-markup-xaml-tests",
                                      "emitter::differential_tests::", "--", "--test-threads=1"]),
        ]
        if suite == "control-theme-regions":
            tests.insert(1, (
                "region-runtime", [*common, "-p", "ferroui-markup-xaml",
                                   "style_region::tests::", "--", "--test-threads=1"],
            ))
            tests.insert(3, (
                "region-grammar", [*common, "-p", "ferroui-markup-xaml-loader", "--features", "emitter",
                                   "control_theme_programs::regions::tests::", "--", "--test-threads=1"],
            ))
        downstream = [cargo, "check", "--locked", "--target", HOST, "--lib", "-p", "ferroui-themes-winui",
                      "--features", "solid-theme"]
    else:
        raise ValueError(f"unknown source validation suite: {suite}")
    commands = [*tests, ("downstream", downstream)]
    if suite == "property-capabilities":
        commands.append(("fresh-winui", [cargo, "check", "--locked", "--target", HOST, "--lib",
                                          "-p", "ferroui-themes-winui", "--features", "solid-theme"]))
    return commands, {label for label, _ in tests}


def successful_tests(contents, label):
    summaries = re.findall(r"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;", contents)
    if len(summaries) != 1 or int(summaries[0][0]) == 0 or int(summaries[0][1]) != 0:
        raise RuntimeError(f"{label} did not report one nonempty successful test suite")
    passed = set(re.findall(r"(?m)^test ([A-Za-z_][A-Za-z_0-9:]*)(?: - should panic)? \.\.\. ok$", contents))
    if len(passed) != int(summaries[0][0]):
        raise RuntimeError(f"{label} test names do not match its successful summary")
    return dict(zip(("passed", "failed", "ignored"), map(int, summaries[0]))), passed


def successful_property_capabilities_tests(contents, label):
    summary, passed = successful_tests(contents, label)
    executions = re.findall(r"(?m)^test ([A-Za-z_][A-Za-z_0-9:]*)(?: - should panic)? \.\.\. ok$", contents)
    if len(executions) != len(passed):
        raise RuntimeError(f"{label} contains duplicate successful test execution lines")
    return summary, passed


def property_capabilities_inventory(rows, required_tests):
    expected = PROPERTY_CAPABILITIES_EXPECTED_SUMMARIES
    labels = [row["label"] for row in rows]
    if len(labels) != len(expected) or set(labels) != set(expected):
        raise RuntimeError("property-capabilities requires exactly one result for each of its eight test phases")
    observed = {}
    executions = 0
    for row in rows:
        label = row["label"]
        if row.get("complete") is not True or row.get("exit") != 0 or row.get("test_summary") != expected[label]:
            raise RuntimeError(f"{label} does not match its completed frozen test summary")
        names = row["passed_tests"]
        if len(names) != expected[label]["passed"] or len(set(names)) != len(names):
            raise RuntimeError(f"{label} execution count or distinct test names do not match its summary")
        executions += len(names)
        for name in names:
            observed.setdefault(name, []).append(label)
    overlaps = {name: sorted(phases) for name, phases in observed.items() if len(phases) > 1}
    if overlaps != PROPERTY_CAPABILITIES_EXPECTED_OVERLAPS:
        raise RuntimeError("property-capabilities test overlaps differ from the four frozen property-runtime/metadata cases")
    if executions != 415 or len(observed) != 411:
        raise RuntimeError("property-capabilities requires exactly 415 successful executions and 411 distinct tests")
    if len(required_tests) != 27 or len(set(required_tests)) != 27 or not set(required_tests) <= observed.keys():
        raise RuntimeError("property-capabilities did not pass all 27 distinct required regressions")
    return {"total_passed_executions": executions, "total_distinct_passed": len(observed),
            "repeated_test_executions": executions - len(observed), "test_name_overlaps": overlaps}


def nullable_suite_commands(cargo):
    """Extend the unchanged property-capability plan with two fixed filters."""
    if not NULLABLE_OPERATION_TESTS:
        raise ValueError("Nullable oracle names must be frozen before this suite can execute")
    commands, labels = suite_commands(cargo, "property-capabilities")
    template = commands[0][1]
    assert template[-2:] == ["--", "--test-threads=1"]
    additions = [
        ("value-types", [*template[:-3], "data::core::value_type::tests::", *template[-2:]]),
        ("nullable-oracles", [*template[:-3], "data::core::value_type::nullable_operation_tests::", *template[-2:], "--show-output"]),
    ]
    assert [label for label, _ in commands[-2:]] == ["downstream", "fresh-winui"]
    return [*commands[:-2], *additions, *commands[-2:]], labels | {label for label, _ in additions}


def nullable_operation_inventory(rows, required_tests):
    """Keep the original 415/411 proof and independently verify both added sets."""
    if not NULLABLE_OPERATION_TESTS:
        raise RuntimeError("Nullable oracle names have not been frozen")
    labels = [row["label"] for row in rows]
    if len(labels) != len(NULLABLE_EXPECTED_SUMMARIES) or set(labels) != set(NULLABLE_EXPECTED_SUMMARIES):
        raise RuntimeError("nullable-operations requires all ten exact test phases")
    expected_required = {*NULLABLE_PROPERTY_REQUIRED_TESTS, *NULLABLE_OPERATION_TESTS}
    if len(required_tests) != len(expected_required) or set(required_tests) != expected_required:
        raise RuntimeError("nullable-operations requires exactly the original 27 and every frozen nullable oracle")
    base_rows = [row for row in rows if row["label"] in PROPERTY_CAPABILITIES_EXPECTED_SUMMARIES]
    preserved = property_capabilities_inventory(base_rows, list(NULLABLE_PROPERTY_REQUIRED_TESTS))
    base_names = {name for row in base_rows for name in row["passed_tests"]}
    observed = set(base_names)
    for label, expected_names in (("value-types", NULLABLE_VALUE_TYPE_TESTS), ("nullable-oracles", NULLABLE_OPERATION_TESTS)):
        row, = [item for item in rows if item["label"] == label]
        names = row["passed_tests"]
        if row.get("complete") is not True or row.get("exit") != 0 or row.get("test_summary") != NULLABLE_EXPECTED_SUMMARIES[label]:
            raise RuntimeError(f"{label} does not match its exact completed nullable summary")
        if len(names) != len(expected_names) or len(set(names)) != len(names) or set(names) != set(expected_names):
            raise RuntimeError(f"{label} differs from its complete frozen test-name set")
        if observed.intersection(names):
            raise RuntimeError(f"{label} unexpectedly overlaps another test phase")
        observed.update(names)
    if not set(required_tests) <= observed:
        raise RuntimeError("A mandatory nullable regression did not pass")
    executions = preserved["total_passed_executions"] + len(NULLABLE_VALUE_TYPE_TESTS) + len(NULLABLE_OPERATION_TESTS)
    distinct = preserved["total_distinct_passed"] + len(NULLABLE_VALUE_TYPE_TESTS) + len(NULLABLE_OPERATION_TESTS)
    if len(observed) != distinct:
        raise RuntimeError("Nullable distinct-name accounting differs")
    return {"total_passed_executions": executions, "total_distinct_passed": distinct,
            "repeated_test_executions": 4, "test_name_overlaps": preserved["test_name_overlaps"],
            "preserved_property_capabilities_inventory": preserved,
            "additional_value_type_tests": sorted(NULLABLE_VALUE_TYPE_TESTS),
            "additional_nullable_oracles": sorted(NULLABLE_OPERATION_TESTS)}


def coercion_construction_suite_commands(cargo):
    """Add one frozen construction filter without changing the nullable plan."""
    if len(COERCION_CONSTRUCTION_TESTS) != 5:
        raise ValueError("The five construction oracle names must be frozen before this suite can execute")
    commands, labels = nullable_suite_commands(cargo)
    template = commands[0][1]
    assert template[-2:] == ["--", "--test-threads=1"]
    addition = (
        "construction-oracles",
        [*template[:-3], "property_store::effective_value::construction_tests::", *template[-2:]],
    )
    assert [label for label, _ in commands[-2:]] == ["downstream", "fresh-winui"]
    return [*commands[:-2], addition, *commands[-2:]], labels | {addition[0]}


def coercion_construction_inventory(rows, required_tests):
    """Preserve the actual nullable inventory and require five disjoint oracles."""
    if len(COERCION_CONSTRUCTION_TESTS) != 5 or len(set(COERCION_CONSTRUCTION_TESTS)) != 5:
        raise RuntimeError("The five distinct construction oracle names have not been frozen")
    labels = [row["label"] for row in rows]
    if len(labels) != len(COERCION_CONSTRUCTION_EXPECTED_SUMMARIES) or set(labels) != set(COERCION_CONSTRUCTION_EXPECTED_SUMMARIES):
        raise RuntimeError("coercion-construction requires all eleven exact test phases")
    nullable_required = [*NULLABLE_PROPERTY_REQUIRED_TESTS, *NULLABLE_OPERATION_TESTS]
    expected_required = {*nullable_required, *COERCION_CONSTRUCTION_TESTS}
    if len(expected_required) != 38 or len(required_tests) != 38 or set(required_tests) != expected_required:
        raise RuntimeError("coercion-construction requires exactly the original 33 and five frozen construction oracles")
    base_rows = [row for row in rows if row["label"] in NULLABLE_EXPECTED_SUMMARIES]
    preserved = nullable_operation_inventory(base_rows, nullable_required)
    base_names = {name for row in base_rows for name in row["passed_tests"]}
    row, = [item for item in rows if item["label"] == "construction-oracles"]
    names = row["passed_tests"]
    if row.get("complete") is not True or row.get("exit") != 0 or row.get("test_summary") != COERCION_CONSTRUCTION_EXPECTED_SUMMARIES["construction-oracles"]:
        raise RuntimeError("construction-oracles does not match its exact completed summary")
    if len(names) != 5 or len(set(names)) != 5 or set(names) != set(COERCION_CONSTRUCTION_TESTS):
        raise RuntimeError("construction-oracles differs from its complete frozen test-name set")
    if base_names.intersection(names):
        raise RuntimeError("construction-oracles unexpectedly overlaps a preserved test phase")
    observed = base_names | set(names)
    executions = preserved["total_passed_executions"] + len(names)
    if executions != 450 or len(observed) != 446 or not expected_required <= observed:
        raise RuntimeError("coercion-construction requires 450 executions, 446 distinct names and all 38 mandatory cases")
    return {**preserved, "total_passed_executions": executions, "total_distinct_passed": len(observed),
            "preserved_nullable_inventory": preserved, "additional_construction_oracles": sorted(names)}


def property_transition_suite_commands(cargo):
    """Append one typed-transition filter to the unchanged construction plan."""
    if len(PROPERTY_TRANSITION_TESTS) != 4:
        raise ValueError("The four property transition oracle names must be frozen before execution")
    commands, labels = coercion_construction_suite_commands(cargo)
    template = commands[0][1]
    assert template[-2:] == ["--", "--test-threads=1"]
    addition = (
        "transition-oracles",
        [*template[:-3], "property_store::effective_value::transition_tests::", *template[-2:]],
    )
    assert [label for label, _ in commands[-3:]] == ["construction-oracles", "downstream", "fresh-winui"]
    return [*commands[:-2], addition, *commands[-2:]], labels | {addition[0]}


def property_transition_inventory(rows, required_tests):
    """Require the original construction inventory and four disjoint oracles."""
    if len(PROPERTY_TRANSITION_TESTS) != 4 or len(set(PROPERTY_TRANSITION_TESTS)) != 4:
        raise RuntimeError("The four distinct property transition oracle names have not been frozen")
    labels = [row["label"] for row in rows]
    if len(labels) != len(PROPERTY_TRANSITION_EXPECTED_SUMMARIES) or set(labels) != set(PROPERTY_TRANSITION_EXPECTED_SUMMARIES):
        raise RuntimeError("property-transition requires all twelve exact test phases")
    preserved_required = [*NULLABLE_PROPERTY_REQUIRED_TESTS, *NULLABLE_OPERATION_TESTS, *COERCION_CONSTRUCTION_TESTS]
    expected_required = {*preserved_required, *PROPERTY_TRANSITION_TESTS}
    if len(expected_required) != 42 or len(required_tests) != 42 or set(required_tests) != expected_required:
        raise RuntimeError("property-transition requires exactly the original 38 and four frozen transition oracles")
    base_rows = [row for row in rows if row["label"] in COERCION_CONSTRUCTION_EXPECTED_SUMMARIES]
    preserved = coercion_construction_inventory(base_rows, preserved_required)
    base_names = {name for row in base_rows for name in row["passed_tests"]}
    row, = [item for item in rows if item["label"] == "transition-oracles"]
    names = row["passed_tests"]
    if row.get("complete") is not True or row.get("exit") != 0 or row.get("test_summary") != PROPERTY_TRANSITION_EXPECTED_SUMMARIES["transition-oracles"]:
        raise RuntimeError("transition-oracles does not match its exact completed summary")
    if len(names) != 4 or len(set(names)) != 4 or set(names) != set(PROPERTY_TRANSITION_TESTS):
        raise RuntimeError("transition-oracles differs from its complete frozen test-name set")
    if base_names.intersection(names):
        raise RuntimeError("transition-oracles unexpectedly overlaps a preserved test phase")
    observed = base_names | set(names)
    executions = preserved["total_passed_executions"] + len(names)
    if executions != 454 or len(observed) != 450 or not expected_required <= observed:
        raise RuntimeError("property-transition requires 454 executions, 450 distinct names and all 42 mandatory cases")
    return {**preserved, "total_passed_executions": executions, "total_distinct_passed": len(observed),
            "preserved_construction_inventory": preserved, "additional_transition_oracles": sorted(names)}


def collect_generated_winui(target, evidence):
    generated = []
    for path in sorted(target.rglob("compiled_xaml.rs")):
        if any(part.startswith("ferroui-themes-winui-") for part in path.parts):
            row = identity(path)
            contents = path.read_text()
            row["construction_programs"] = contents.count("rt::control_theme_program::run(")
            row["region_programs"] = contents.count("rt::style_region::run(")
            retained = evidence / "generated" / f"{len(generated):03d}-{path.parent.parent.name}.rs"
            retained.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(path, retained)
            row["retained"] = identity(retained)
            generated.append(row)
    return generated


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", required=True, type=Path)
    parser.add_argument("--revision", required=True)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--suite", choices=("metadata", "coercion", "inheritance", "control-theme", "control-theme-regions", "property-capabilities", "nullable-operations", "coercion-construction", "property-transition"), default="metadata")
    parser.add_argument("--required-test", action="append", default=[])
    parser.add_argument("--plan-only", action="store_true")
    args = parser.parse_args()
    source = args.source.resolve(strict=True)
    if not re.fullmatch(r"[0-9a-f]{40}", args.revision):
        parser.error("--revision must be a full immutable commit SHA")
    if any(not re.fullmatch(r"[A-Za-z_][A-Za-z_0-9:]*", name) for name in args.required_test):
        parser.error("--required-test must be a Rust test path")
    if args.suite == "property-capabilities" and (len(args.required_test) != 27 or len(set(args.required_test)) != 27):
        parser.error("property-capabilities requires the 27 distinct frozen regression names")
    if args.suite == "nullable-operations":
        expected_required = {*NULLABLE_PROPERTY_REQUIRED_TESTS, *NULLABLE_OPERATION_TESTS}
        if not NULLABLE_OPERATION_TESTS:
            parser.error("Nullable oracle names must be frozen before this suite can execute")
        if len(args.required_test) != len(expected_required) or set(args.required_test) != expected_required:
            parser.error("nullable-operations requires exactly the original 27 and all frozen nullable oracle names")
    if args.suite == "coercion-construction":
        expected_required = {*NULLABLE_PROPERTY_REQUIRED_TESTS, *NULLABLE_OPERATION_TESTS, *COERCION_CONSTRUCTION_TESTS}
        if len(COERCION_CONSTRUCTION_TESTS) != 5 or len(expected_required) != 38:
            parser.error("The five construction oracle names must be frozen before this suite can execute")
        if len(args.required_test) != 38 or set(args.required_test) != expected_required:
            parser.error("coercion-construction requires exactly the original 33 and all five construction oracle names")
    if args.suite == "property-transition":
        expected_required = {*NULLABLE_PROPERTY_REQUIRED_TESTS, *NULLABLE_OPERATION_TESTS, *COERCION_CONSTRUCTION_TESTS, *PROPERTY_TRANSITION_TESTS}
        if len(PROPERTY_TRANSITION_TESTS) != 4 or len(expected_required) != 42:
            parser.error("The four property transition oracle names must be frozen before execution")
        if len(args.required_test) != 42 or set(args.required_test) != expected_required:
            parser.error("property-transition requires exactly the original 38 and all four transition oracle names")
    if git(source, "rev-parse", "HEAD") != args.revision:
        parser.error("source checkout does not match --revision")
    if git(source, "status", "--porcelain", "--untracked-files=all"):
        parser.error("source checkout has modified or untracked inputs")
    output = args.output.resolve()
    if output.exists() or output.is_relative_to(source):
        parser.error("use a fresh output directory outside the source checkout")
    evidence = output / "evidence"
    evidence.mkdir(parents=True)
    temporary = output / "temporary"
    temporary.mkdir()
    prefix = output / "toolchain/install"
    environment = os.environ.copy()
    removed = []
    for key in list(environment):
        if key.startswith(("CARGO_PROFILE_", "FERROUI_PGO_")) or key in {
            "RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "RUSTC", "RUSTC_WRAPPER",
            "RUSTC_WORKSPACE_WRAPPER", "RUSTC_BOOTSTRAP", "LLVM_PROFILE_FILE",
        }:
            removed.append(key)
            del environment[key]
    controlled = {
        "TMPDIR": str(temporary), "CARGO_HOME": str(output / "cargo-home"),
        "CARGO_TARGET_DIR": str(output / "target"), "CARGO_BUILD_JOBS": "1",
        "CARGO_INCREMENTAL": "0", "RAYON_NUM_THREADS": "1", "MALLOC_ARENA_MAX": "1",
        "MALLOC_CONF": "narenas:1,tcache:false,dirty_decay_ms:0,muzzy_decay_ms:0,background_thread:false,retain:false",
        "CARGO_TERM_COLOR": "never", "RUST_BACKTRACE": "1",
        "CARGO_PROFILE_DEV_OPT_LEVEL": "0", "CARGO_PROFILE_DEV_DEBUG": "0",
        "CARGO_PROFILE_DEV_DEBUG_ASSERTIONS": "true",
        "CARGO_PROFILE_DEV_CODEGEN_UNITS": "128", "CARGO_PROFILE_DEV_LTO": "false",
        "CARGO_PROFILE_TEST_OPT_LEVEL": "0", "CARGO_PROFILE_TEST_DEBUG": "0",
        "CARGO_PROFILE_TEST_DEBUG_ASSERTIONS": "true",
        "CARGO_PROFILE_TEST_CODEGEN_UNITS": "128", "CARGO_PROFILE_TEST_LTO": "false",
    }
    environment.update(controlled)
    commands = [
        ("bootstrap", [sys.executable, str(HERE / "bootstrap_toolchain.py"),
                       "--work-dir", str(output / "toolchain")]),
    ]
    cargo = str(prefix / "bin/cargo")
    if args.suite == "property-transition":
        planned, test_labels = property_transition_suite_commands(cargo)
    elif args.suite == "coercion-construction":
        planned, test_labels = coercion_construction_suite_commands(cargo)
    else:
        planned, test_labels = nullable_suite_commands(cargo) if args.suite == "nullable-operations" else suite_commands(cargo, args.suite)
    commands.extend(planned)
    report = {
        "schema": 2, "scope": __doc__, "suite": args.suite, "revision": args.revision,
        "tree": git(source, "rev-parse", "HEAD^{tree}"), "source": str(source),
        "producer": identity(__file__), "bootstrap_producer": identity(HERE / "bootstrap_toolchain.py"),
        "lockfile_before": identity(source / "Cargo.lock"), "commands": [],
        "environment": {"removed_names": sorted(removed), "controlled": controlled},
        "started_unix": time.time(), "complete": False, "plan_only": args.plan_only,
        "production_acceptance": False,
        "required_tests": args.required_test,
        "required_metadata_tests": args.required_test if args.suite == "metadata" else [],
    }
    if args.suite == "control-theme-regions":
        report["expected_test_summaries"] = REGION_EXPECTED_SUMMARIES
    elif args.suite == "coercion":
        report["expected_test_summaries"] = COERCION_EXPECTED_SUMMARIES
    elif args.suite == "inheritance":
        report["expected_test_summaries"] = INHERITANCE_EXPECTED_SUMMARIES
    elif args.suite == "property-capabilities":
        report["expected_test_summaries"] = PROPERTY_CAPABILITIES_EXPECTED_SUMMARIES
        report["expected_total_passed"] = 415
        report["expected_total_distinct_passed"] = 411
        report["expected_name_overlaps"] = PROPERTY_CAPABILITIES_EXPECTED_OVERLAPS
        report["expected_generated_program_counts"] = PROPERTY_CAPABILITIES_EXPECTED_PROGRAMS
    elif args.suite == "nullable-operations":
        report["expected_test_summaries"] = NULLABLE_EXPECTED_SUMMARIES
        report["expected_total_passed"] = 415 + len(NULLABLE_VALUE_TYPE_TESTS) + len(NULLABLE_OPERATION_TESTS)
        report["expected_total_distinct_passed"] = 411 + len(NULLABLE_VALUE_TYPE_TESTS) + len(NULLABLE_OPERATION_TESTS)
        report["expected_name_overlaps"] = PROPERTY_CAPABILITIES_EXPECTED_OVERLAPS
        report["expected_generated_program_counts"] = PROPERTY_CAPABILITIES_EXPECTED_PROGRAMS
        report["expected_additional_test_names"] = {"value-types": list(NULLABLE_VALUE_TYPE_TESTS), "nullable-oracles": list(NULLABLE_OPERATION_TESTS)}
    elif args.suite == "coercion-construction":
        report["expected_test_summaries"] = COERCION_CONSTRUCTION_EXPECTED_SUMMARIES
        report["expected_total_passed"] = 450
        report["expected_total_distinct_passed"] = 446
        report["expected_name_overlaps"] = PROPERTY_CAPABILITIES_EXPECTED_OVERLAPS
        report["expected_generated_program_counts"] = PROPERTY_CAPABILITIES_EXPECTED_PROGRAMS
        report["expected_additional_test_names"] = {
            "value-types": list(NULLABLE_VALUE_TYPE_TESTS), "nullable-oracles": list(NULLABLE_OPERATION_TESTS),
            "construction-oracles": list(COERCION_CONSTRUCTION_TESTS),
        }
    elif args.suite == "property-transition":
        report["expected_test_summaries"] = PROPERTY_TRANSITION_EXPECTED_SUMMARIES
        report["expected_total_passed"] = 454
        report["expected_total_distinct_passed"] = 450
        report["expected_name_overlaps"] = PROPERTY_CAPABILITIES_EXPECTED_OVERLAPS
        report["expected_generated_program_counts"] = PROPERTY_CAPABILITIES_EXPECTED_PROGRAMS
        report["expected_additional_test_names"] = {
            "value-types": list(NULLABLE_VALUE_TYPE_TESTS), "nullable-oracles": list(NULLABLE_OPERATION_TESTS),
            "construction-oracles": list(COERCION_CONSTRUCTION_TESTS), "transition-oracles": list(PROPERTY_TRANSITION_TESTS),
        }
    tracked = evidence / "tracked-inputs.txt"
    tracked.write_text(git(source, "ls-files", "--stage") + "\n")
    report["tracked_inputs"] = identity(tracked)
    manifest = evidence / "validation.json"
    save(manifest, report)
    if args.plan_only:
        report["planned_commands"] = [{"label": label, "argv": command} for label, command in commands]
        save(manifest, report)
        print(manifest)
        return
    def interrupted(signum, _frame):
        raise InterruptedError(f"Source validation interrupted by signal {signum}")

    previous_sigterm = signal.signal(signal.SIGTERM, interrupted)
    passed_tests = set()
    try:
        for label, command in commands:
            # Every requested regression must have run before the downstream
            # compile. An empty or stale filter can never become acceptance.
            if label == "downstream":
                missing = set(args.required_test) - passed_tests
                if missing:
                    raise RuntimeError(f"required regressions did not pass: {sorted(missing)}")
                report["required_tests_passed"] = sorted(set(args.required_test) & passed_tests)
                if args.suite == "property-capabilities":
                    report.update(property_capabilities_inventory(
                        [row for row in report["commands"] if "test_summary" in row], args.required_test))
                elif args.suite == "nullable-operations":
                    report.update(nullable_operation_inventory(
                        [row for row in report["commands"] if "test_summary" in row], args.required_test))
                elif args.suite == "coercion-construction":
                    report.update(coercion_construction_inventory(
                        [row for row in report["commands"] if "test_summary" in row], args.required_test))
                elif args.suite == "property-transition":
                    report.update(property_transition_inventory(
                        [row for row in report["commands"] if "test_summary" in row], args.required_test))
            log = evidence / (label + ".log")
            resources = evidence / (label + "-resources.txt")
            argv = ["/usr/bin/time", "-v", "-o", str(resources), *command]
            row = {"label": label, "argv": argv, "cwd": str(source), "started_unix": time.time(), "complete": False}
            report["commands"].append(row)
            save(manifest, report)
            print(f"START {label}", flush=True)
            process = None
            try:
                with log.open("w") as stream:
                    process = subprocess.Popen(argv, cwd=source, env=environment, stdout=stream,
                                               stderr=subprocess.STDOUT, start_new_session=True)
                    row["exit"] = process.wait()
                if row["exit"]:
                    raise RuntimeError(f"{label} exited {row['exit']}; see {log}")
                if label in test_labels:
                    contents = log.read_text()
                    parse_tests = successful_property_capabilities_tests if args.suite in {"property-capabilities", "nullable-operations", "coercion-construction", "property-transition"} else successful_tests
                    row["test_summary"], passed = parse_tests(contents, label)
                    row["passed_tests"] = sorted(passed)
                    expected = report.get("expected_test_summaries", {}).get(label)
                    if expected is not None and row["test_summary"] != expected:
                        raise RuntimeError(f"{label} summary {row['test_summary']} does not match frozen {args.suite} suite {expected}")
                    passed_tests.update(passed)
                    if label == "metadata" and args.suite == "metadata":
                        for name in args.required_test:
                            if name not in passed:
                                raise RuntimeError(f"required metadata regression did not pass: {name}")
                row["complete"] = True
            finally:
                if process is not None and process.poll() is None:
                    os.killpg(process.pid, signal.SIGTERM)
                    try:
                        process.wait(timeout=15)
                    except subprocess.TimeoutExpired:
                        os.killpg(process.pid, signal.SIGKILL)
                        process.wait()
                    row["exit"] = process.returncode
                row["finished_unix"] = time.time()
                if log.exists():
                    row["log"] = identity(log)
                if resources.exists():
                    row["resources"] = identity(resources)
                save(manifest, report)
                print(f"END {label}: exit={row.get('exit')} complete={row['complete']}", flush=True)
            if label == "bootstrap":
                installation = output / "toolchain/installation.json"
                shutil.copy2(installation, evidence / "installation.json")
                report["toolchain"] = identity(evidence / "installation.json")
                environment["RUSTC"] = str(prefix / "bin/rustc")
                environment["PATH"] = str(prefix / "bin") + os.pathsep + environment["PATH"]
                environment["LD_LIBRARY_PATH"] = str(prefix / "lib") + (
                    os.pathsep + environment["LD_LIBRARY_PATH"] if environment.get("LD_LIBRARY_PATH") else "")
                report["environment"]["toolchain_path_prefix"] = str(prefix / "bin")
                report["environment"]["toolchain_library_prefix"] = str(prefix / "lib")
                report["environment"]["rustc"] = environment["RUSTC"]
        if args.suite in {"control-theme", "control-theme-regions", "property-capabilities", "nullable-operations", "coercion-construction", "property-transition"}:
            report["generated_winui"] = collect_generated_winui(output / "target", evidence)
            if not any(row["construction_programs"] for row in report["generated_winui"]):
                raise RuntimeError("fresh WinUI output contains no accepted ControlTheme construction programs")
            if args.suite in {"control-theme-regions", "property-capabilities", "nullable-operations", "coercion-construction", "property-transition"}:
                report["generated_program_counts"] = {
                    key: sum(row[key] for row in report["generated_winui"])
                    for key in ("construction_programs", "region_programs")
                }
                if not any(row["region_programs"] for row in report["generated_winui"]):
                    raise RuntimeError("fresh WinUI output contains no accepted Style region programs")
                if args.suite in {"property-capabilities", "nullable-operations", "coercion-construction", "property-transition"} and report["generated_program_counts"] != PROPERTY_CAPABILITIES_EXPECTED_PROGRAMS:
                    raise RuntimeError("fresh WinUI output does not match the frozen 18 whole-theme and 34 Style region programs")
        report["complete"] = True
    except BaseException as error:
        report["failure"] = str(error)
        raise
    finally:
        previously_complete = report["complete"]
        if args.suite in {"control-theme", "control-theme-regions", "property-capabilities", "nullable-operations", "coercion-construction", "property-transition"} and "generated_winui" not in report:
            try:
                report["generated_winui"] = collect_generated_winui(output / "target", evidence)
            except Exception as error:
                report["generated_evidence_error"] = str(error)
                report["complete"] = False
                report.setdefault("failure", "Could not preserve generated WinUI evidence")
        try:
            report["source_after"] = {
                "head": git(source, "rev-parse", "HEAD"), "tree": git(source, "rev-parse", "HEAD^{tree}"),
                "status": git(source, "status", "--porcelain", "--untracked-files=all"),
                "lockfile": identity(source / "Cargo.lock"),
            }
            after = report["source_after"]
            report["source_unchanged"] = (after["head"] == report["revision"] and after["tree"] == report["tree"]
                                          and not after["status"] and after["lockfile"] == report["lockfile_before"])
        except Exception as error:
            report["source_verification_error"] = str(error)
            report["source_unchanged"] = False
        if not report["source_unchanged"]:
            report["complete"] = False
            report.setdefault("failure", "Source identity changed or could not be verified after execution")
        report["finished_unix"] = time.time()
        save(manifest, report)
        signal.signal(signal.SIGTERM, previous_sigterm)
        if previously_complete and not report["source_unchanged"]:
            raise RuntimeError(report["failure"])
    print(manifest)


if __name__ == "__main__":
    main()
