//! The tests of part A of the crate: the test files of `tests/Xaml.Behaviors.Interactions.UnitTests/Custom`
//! of the reference that test the types of the folders of part A (`Core/`, `ExecuteCommand/`,
//! `Actions/`, `Logic/`), one module per test file, and their support files.
//!
//! The other test files of the folder test the types of the other parts, which port them:
//! `AnimationAdapterTests`, `InvokeButtonClickActionTests` and `ToolTipTriggerTests` (part C),
//! `ClickEventTriggerTests`, `FocusTrapBehaviorTests`, `InlineEditBehaviorTests` and
//! `ValidationRuleTests` (part B), `XmlnsDefinitionRuntimeXamlTests` (part D).

pub(crate) mod command;
#[allow(dead_code)]
pub(crate) mod observable_command;
pub(crate) mod support;

mod attached_to_visual_tree_lifecycle_tests;
mod detached_from_visual_tree_trigger_tests;
mod execute_command_behavior_base_tests;
mod focus_next_element_action_tests;
mod observable_trigger_behavior_tests;
mod routed_event_trigger_behavior_tests;
