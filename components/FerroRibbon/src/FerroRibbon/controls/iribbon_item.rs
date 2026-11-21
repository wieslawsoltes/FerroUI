//! Native contracts from RibbonSpace.Uno/Primitives/IRibbonItem.cs.
use crate::primitives::RibbonMetrics;
use ferroribbon_core::layout::RibbonSizeDefinition;
use ferroribbon_core::model::RibbonItemsSource;
use ferroribbon_core::{RibbonGroupState, RibbonItemSize, RibbonSimplifiedVisibility};
use ferroui_base::{BoxedValue, FerroObject, Ref};
use ferroui_controls::Control;
use std::{rc::Rc, sync::Arc};

/// MenuFlyoutItemBase maps to the common native Control base of menu items/separators.
pub type RibbonOverflowMenuItems = RibbonItemsSource<Option<Ref<Control>>>;
/// A fresh repeatable sequence from the original callback, retaining actual elements.
pub type RibbonKeyTipTargets = Rc<dyn Fn() -> RibbonItemsSource<Option<Ref<Control>>>>;

pub trait IRibbonItem {
    fn as_object(&self) -> Ref<FerroObject>;
    fn id(&self) -> Option<ferroribbon_core::RibbonString>;
    fn label(&self) -> Option<ferroribbon_core::RibbonString>;
    fn icon(&self) -> Option<BoxedValue>;
    fn key_tip(&self) -> Option<ferroribbon_core::RibbonString>;
    fn command_id(&self) -> Option<ferroribbon_core::RibbonString>;
    fn size(&self) -> RibbonItemSize;
    fn effective_size_definition(&self) -> RibbonSizeDefinition;
    fn current_size(&self) -> RibbonItemSize;
    fn simplified_visibility(&self) -> RibbonSimplifiedVisibility;
    fn show_label_in_simplified_mode(&self) -> bool;
    fn can_add_to_quick_access(&self) -> bool;
    fn apply_layout(&self, layout: RibbonItemLayout);
    fn create_linked_copy(&self) -> Option<Ref<Control>>;
    fn create_overflow_menu_items(&self) -> RibbonOverflowMenuItems;
    fn on_key_tip(&self) -> Rc<RibbonKeyTipResult>;
    fn invoke(&self) -> bool;
}
impl PartialEq for dyn IRibbonItem {
    fn eq(&self, other: &Self) -> bool {
        self.as_object() == other.as_object()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RibbonItemLayout {
    pub size: RibbonItemSize,
    pub metrics: Option<Arc<RibbonMetrics>>,
    pub is_simplified: bool,
    pub show_label: Option<bool>,
    pub group_state: RibbonGroupState,
}
impl RibbonItemLayout {
    pub fn new(size: RibbonItemSize, metrics: Option<Arc<RibbonMetrics>>) -> Self {
        Self {
            size,
            metrics,
            is_simplified: false,
            show_label: None,
            group_state: RibbonGroupState::Large,
        }
    }
}
pub struct RibbonKeyTipResult {
    next_scope: Option<RibbonKeyTipTargets>,
}
impl PartialEq for RibbonKeyTipResult {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
ferroui_base::ferro_thread_local! {static CLOSE: Rc<RibbonKeyTipResult> = RibbonKeyTipResult::new();}
impl RibbonKeyTipResult {
    /// Represents the public default constructor and object-initializer NextScope setter.
    pub fn new_with_next_scope(next_scope: Option<RibbonKeyTipTargets>) -> Rc<Self> {
        Rc::new(Self { next_scope })
    }
    pub fn new() -> Rc<Self> {
        Self::new_with_next_scope(None)
    }
    pub fn close() -> Rc<Self> {
        CLOSE.with(Clone::clone)
    }
    pub fn scope(targets: Option<RibbonKeyTipTargets>) -> Rc<Self> {
        Self::new_with_next_scope(targets)
    }
    pub fn next_scope(&self) -> Option<RibbonKeyTipTargets> {
        self.next_scope.clone()
    }
}

pub trait IRibbonItemEditable: IRibbonItem {
    // C# `new` properties have distinct interface slots. In particular the
    // editable Id is raw, while IRibbonItem.Id may use Name/CommandId fallback.
    // UFCS selects the intended getter when both traits are in scope.
    fn id(&self) -> Option<ferroribbon_core::RibbonString>;
    fn label(&self) -> Option<ferroribbon_core::RibbonString>;
    fn icon(&self) -> Option<BoxedValue>;
    fn size(&self) -> RibbonItemSize;
    fn key_tip(&self) -> Option<ferroribbon_core::RibbonString>;
    fn command_id(&self) -> Option<ferroribbon_core::RibbonString>;
    fn simplified_visibility(&self) -> RibbonSimplifiedVisibility;
    fn can_add_to_quick_access(&self) -> bool;
    fn set_id(&self, value: Option<ferroribbon_core::RibbonString>);
    fn set_label(&self, value: Option<ferroribbon_core::RibbonString>);
    fn set_icon(&self, value: Option<BoxedValue>);
    fn large_icon(&self) -> Option<BoxedValue>;
    fn set_large_icon(&self, value: Option<BoxedValue>);
    fn set_size(&self, value: RibbonItemSize);
    fn size_definition(&self) -> Option<ferroribbon_core::RibbonString>;
    fn set_size_definition(&self, value: Option<ferroribbon_core::RibbonString>);
    fn set_key_tip(&self, value: Option<ferroribbon_core::RibbonString>);
    fn screen_tip(&self) -> Option<BoxedValue>;
    fn set_screen_tip(&self, value: Option<BoxedValue>);
    fn set_command_id(&self, value: Option<ferroribbon_core::RibbonString>);
    fn shortcut(&self) -> Option<ferroribbon_core::RibbonString>;
    fn set_shortcut(&self, value: Option<ferroribbon_core::RibbonString>);
    fn set_simplified_visibility(&self, value: RibbonSimplifiedVisibility);
    fn simplified_label(&self) -> super::RibbonSimplifiedLabel;
    fn set_simplified_label(&self, value: super::RibbonSimplifiedLabel);
    fn show_label(&self) -> bool;
    fn set_show_label(&self, value: bool);
    fn set_can_add_to_quick_access(&self, value: bool);
}
impl PartialEq for dyn IRibbonItemEditable {
    fn eq(&self, other: &Self) -> bool {
        self.as_object() == other.as_object()
    }
}
