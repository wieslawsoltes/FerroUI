//! The test types of the upstream folder `Xaml/`: the shared ones
//! (`TestControl`, `NonControl`, ...) and the ones each test file declares.

use ferroui_base::metadata::MarkupTyped;
use ferroui_base::StaticType;

use crate::support::TypeModule;

pub mod initialization_order_tracker;
pub mod non_control;
pub mod test_control;
pub mod test_selector_control;
pub mod test_templated_control;

pub mod basic_tests;
pub mod control_template_tests;
pub mod control_theme_tests;
pub mod data_template_tests;
pub mod design_mode_tests;
pub mod event_tests;
pub mod ferro_intrinsics_tests;
pub mod generic_template_tests;
pub mod items_panel_template_tests;
pub mod merge_resource_include_tests;
pub mod parent_stack_provider_tests;
pub mod provide_value_target_tests;
pub mod resource_dictionary_tests;
pub mod style_include_tests;
pub mod style_tests;
pub mod theme_dictionaries_tests;
pub mod tree_data_template_tests;
pub mod xaml_source_info_tests;

pub use initialization_order_tracker::InitializationOrderTracker;
pub use non_control::NonControl;
pub use test_control::{AttachedPropertyOwner, TestControl};
pub use test_selector_control::{TestSelectorControl, TestSelectorControlExtension};
pub use test_templated_control::TestTemplatedControl;

/// The shared types of the folder.
pub(crate) const MODULE: TypeModule = TypeModule {
    types: &[
        <AttachedPropertyOwner as StaticType>::TYPE,
        InitializationOrderTracker::TYPE,
        NonControl::TYPE,
        TestControl::TYPE,
        TestSelectorControl::TYPE,
        TestSelectorControlExtension::TYPE,
        TestTemplatedControl::TYPE,
    ],
    markup_types: &[<AttachedPropertyOwner as MarkupTyped>::MARKUP],
    value_types: || {},
};
