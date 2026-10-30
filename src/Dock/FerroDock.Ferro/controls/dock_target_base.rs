// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::IDockTarget;
use crate::automation::peers::DockTargetAutomationPeer;
use crate::contract::DockOperationHandler;
use ferrodock_model::core::{DockOperation, DragAction};
use ferrodock_settings::DockProperties;
use ferroui_base::controls::INameScope;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::reactive::{AnonymousObserver, IDisposable};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Point, Rect, Ref,
    StyledElementImpl, StyledProperty, Visual, VisualImpl, WeakRef,
};
use ferroui_base::{FerroObject, FerroObjectExtensions};
use ferroui_controls::automation::peers::AutomationPeer;
use ferroui_controls::primitives::{
    TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt,
};
use ferroui_controls::templates::IControlTemplate;
use ferroui_controls::{Control, ControlImpl, TopLevel};
use ferroui_system::collections::generic::Dictionary;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
/// Shared source operation dictionary, preserving native System enumeration order.
pub type DockTargetOperations = Rc<RefCell<Dictionary<DockOperation, Ref<Control>>>>;
type RememberedParts = Vec<(DockOperation, WeakRef<Control>)>;
const INDICATORS: [&str; 9] = [
    "PART_TopIndicator",
    "PART_BottomIndicator",
    "PART_LeftIndicator",
    "PART_RightIndicator",
    "PART_CenterIndicator",
    "PART_OuterLeftIndicator",
    "PART_OuterTopIndicator",
    "PART_OuterRightIndicator",
    "PART_OuterBottomIndicator",
];
const SELECTORS: [&str; 9] = [
    "PART_TopSelector",
    "PART_BottomSelector",
    "PART_LeftSelector",
    "PART_RightSelector",
    "PART_CenterSelector",
    "PART_OuterLeftSelector",
    "PART_OuterTopSelector",
    "PART_OuterRightSelector",
    "PART_OuterBottomSelector",
];
/// Original abstract target base, with template registration and operation hit testing.
#[repr(C)]
pub struct DockTargetBase {
    base: TemplatedControl,
    applied_template: RefCell<Option<Weak<dyn IControlTemplate>>>,
    applied_indicators: RefCell<RememberedParts>,
    applied_selectors: RefCell<RememberedParts>,
    registrations_invalid: Cell<bool>,
    indicators: RefCell<DockTargetOperations>,
    selectors: RefCell<DockTargetOperations>,
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    handle: RefCell<Weak<TargetHandle>>,
}
ferro_class!(DockTargetBase:TemplatedControl, virtuals DockTargetBaseImpl:TemplatedControlImpl {
    /// Gets the operation returned when no selector or indicator resolves a drop.
    fn default_dock_operation(this)->DockOperation;
});
ferro_class_info!(DockTargetBase {
    interfaces:[Rc<dyn IDockTarget> => DockTargetBase::as_dock_target],
    markup:{attributes:[
        TemplatePart("PART_TopIndicator",type(Ref<ferroui_controls::Panel>)),
        TemplatePart("PART_TopSelector",type(Ref<Control>)),
        TemplatePart("PART_BottomIndicator",type(Ref<ferroui_controls::Panel>)),
        TemplatePart("PART_BottomSelector",type(Ref<Control>)),
        TemplatePart("PART_LeftIndicator",type(Ref<ferroui_controls::Panel>)),
        TemplatePart("PART_LeftSelector",type(Ref<Control>)),
        TemplatePart("PART_RightIndicator",type(Ref<ferroui_controls::Panel>)),
        TemplatePart("PART_RightSelector",type(Ref<Control>)),
        TemplatePart("PART_CenterIndicator",type(Ref<ferroui_controls::Panel>)),
        TemplatePart("PART_CenterSelector",type(Ref<Control>)),
        TemplatePart("PART_OuterLeftIndicator",type(Ref<Control>)),
        TemplatePart("PART_OuterLeftSelector",type(Ref<Control>)),
        TemplatePart("PART_OuterTopIndicator",type(Ref<Control>)),
        TemplatePart("PART_OuterTopSelector",type(Ref<Control>)),
        TemplatePart("PART_OuterRightIndicator",type(Ref<Control>)),
        TemplatePart("PART_OuterRightSelector",type(Ref<Control>)),
        TemplatePart("PART_OuterBottomIndicator",type(Ref<Control>)),
        TemplatePart("PART_OuterBottomSelector",type(Ref<Control>)),
    ]}
});
ferro_impl_classes!(DockTargetBase:StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl);
impl DockTargetBaseImpl for DockTargetBase {
    fn default_dock_operation(_this: &Self) -> DockOperation {
        DockOperation::Window
    }
}
impl FerroObjectImpl for DockTargetBase {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        for (property, class) in [
            (Self::show_horizontal_targets_property(), ":horizontal"),
            (Self::show_vertical_targets_property(), ":vertical"),
            (
                Self::is_global_dock_available_property(),
                ":global-available",
            ),
            (Self::is_global_dock_active_property(), ":global-active"),
        ] {
            let weak = this.to_ref().downgrade();
            let subscription = FerroObject::get_observable(this, property).subscribe(Rc::new(
                AnonymousObserver::new(move |value| {
                    if let Some(owner) = weak.upgrade() {
                        owner.pseudo_classes().set(class, value);
                    }
                }),
            ));
            this.subscriptions.borrow_mut().push(subscription);
        }
    }
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        if change.property() == TemplatedControl::template_property().as_property() {
            this.registrations_invalid.set(true);
            this.indicator_operations().borrow_mut().clear();
            this.selectors_operations().borrow_mut().clear();
        }
        Self::parent_on_property_changed(this, change);
    }
}
impl ControlImpl for DockTargetBase {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        DockTargetAutomationPeer::new(this).upcast()
    }
}
impl TemplatedControlImpl for DockTargetBase {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);
        this.applied_template
            .replace(this.template().map(|template| Rc::downgrade(&template)));
        this.register_template_parts(&**e.name_scope());
        this.applied_indicators
            .replace(remember_parts(&this.indicator_operations()));
        this.applied_selectors
            .replace(remember_parts(&this.selectors_operations()));
    }
}
ferro_properties! {impl DockTargetBase {
    /// Identifies the source `ShowIndicatorsOnly` styled property.
    pub fn show_indicators_only_property()->StyledProperty<bool> { FerroProperty::register::<DockTargetBase,_>("ShowIndicatorsOnly",false) }
    /// Identifies the source `ShowHorizontalTargets` styled property.
    pub fn show_horizontal_targets_property()->StyledProperty<bool> { FerroProperty::register::<DockTargetBase,_>("ShowHorizontalTargets",true) }
    /// Identifies the source `ShowVerticalTargets` styled property.
    pub fn show_vertical_targets_property()->StyledProperty<bool> { FerroProperty::register::<DockTargetBase,_>("ShowVerticalTargets",true) }
    /// Identifies the source `IsGlobalDockAvailable` styled property.
    pub fn is_global_dock_available_property()->StyledProperty<bool> { FerroProperty::register::<DockTargetBase,_>("IsGlobalDockAvailable",false) }
    /// Identifies the source `IsGlobalDockActive` styled property.
    pub fn is_global_dock_active_property()->StyledProperty<bool> { FerroProperty::register::<DockTargetBase,_>("IsGlobalDockActive",false) }
}}
impl DockTargetBase {
    /// Constructs the original base fields for a derived native target.
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            applied_template: RefCell::new(None),
            applied_indicators: RefCell::new(Vec::new()),
            applied_selectors: RefCell::new(Vec::new()),
            registrations_invalid: Cell::new(false),
            indicators: RefCell::new(Rc::new(RefCell::new(Dictionary::new()))),
            selectors: RefCell::new(Rc::new(RefCell::new(Dictionary::new()))),
            subscriptions: RefCell::new(Vec::new()),
            handle: RefCell::new(Weak::new()),
        }
    }
    /// Gets `ShowIndicatorsOnly`.
    pub fn show_indicators_only(&self) -> bool {
        self.get_value(Self::show_indicators_only_property())
    }
    /// Sets `ShowIndicatorsOnly`.
    pub fn set_show_indicators_only(&self, value: bool) {
        self.set_value(Self::show_indicators_only_property(), value);
    }
    /// Gets `ShowHorizontalTargets`.
    pub fn show_horizontal_targets(&self) -> bool {
        self.get_value(Self::show_horizontal_targets_property())
    }
    /// Sets `ShowHorizontalTargets`.
    pub fn set_show_horizontal_targets(&self, value: bool) {
        self.set_value(Self::show_horizontal_targets_property(), value);
    }
    /// Gets `ShowVerticalTargets`.
    pub fn show_vertical_targets(&self) -> bool {
        self.get_value(Self::show_vertical_targets_property())
    }
    /// Sets `ShowVerticalTargets`.
    pub fn set_show_vertical_targets(&self, value: bool) {
        self.set_value(Self::show_vertical_targets_property(), value);
    }
    /// Gets `IsGlobalDockAvailable`.
    pub fn is_global_dock_available(&self) -> bool {
        self.get_value(Self::is_global_dock_available_property())
    }
    /// Sets `IsGlobalDockAvailable`.
    pub fn set_is_global_dock_available(&self, value: bool) {
        self.set_value(Self::is_global_dock_available_property(), value);
    }
    /// Gets `IsGlobalDockActive`.
    pub fn is_global_dock_active(&self) -> bool {
        self.get_value(Self::is_global_dock_active_property())
    }
    /// Sets `IsGlobalDockActive`.
    pub fn set_is_global_dock_active(&self, value: bool) {
        self.set_value(Self::is_global_dock_active_property(), value);
    }
    /// Gets the original protected indicator dictionary without copying entries.
    pub fn indicator_operations(&self) -> DockTargetOperations {
        self.indicators.borrow().clone()
    }
    /// Replaces the original protected indicator dictionary.
    pub fn set_indicator_operations(&self, value: DockTargetOperations) {
        self.indicators.replace(value);
    }
    /// Gets the original protected selector dictionary without copying entries.
    pub fn selectors_operations(&self) -> DockTargetOperations {
        self.selectors.borrow().clone()
    }
    /// Replaces the original protected selector dictionary.
    pub fn set_selectors_operations(&self, value: DockTargetOperations) {
        self.selectors.replace(value);
    }
    /// Gets a registered selector without applying the template or evaluating policy.
    pub fn get_selector(&self, operation: DockOperation) -> Option<Ref<Control>> {
        self.verify_access();
        self.ensure_template_registrations();
        self.selectors_operations()
            .borrow()
            .get(&operation)
            .cloned()
    }
    fn is_operation_enabled(&self, operation: DockOperation) -> bool {
        match operation {
            DockOperation::Left
            | DockOperation::Right
            | DockOperation::OuterLeft
            | DockOperation::OuterRight => self.show_horizontal_targets(),
            DockOperation::Top
            | DockOperation::Bottom
            | DockOperation::OuterTop
            | DockOperation::OuterBottom => self.show_vertical_targets(),
            _ => true,
        }
    }
    fn register_template_parts(&self, scope: &dyn INameScope) {
        self.registrations_invalid.set(false);
        self.indicator_operations().borrow_mut().clear();
        self.selectors_operations().borrow_mut().clear();
        for name in INDICATORS {
            self.add_indicator(name, scope);
        }
        for name in SELECTORS {
            self.add_selector(name, scope);
        }
    }
    fn ensure_template_registrations(&self) {
        if !self.registrations_invalid.get() {
            return;
        }
        let applied = self
            .applied_template
            .borrow()
            .as_ref()
            .and_then(Weak::upgrade);
        if let Some(applied) = applied {
            if self
                .template()
                .as_ref()
                .is_some_and(|current| Rc::ptr_eq(&applied, current))
            {
                self.restore_parts(
                    &self.applied_indicators.borrow(),
                    &self.indicator_operations(),
                );
                self.restore_parts(
                    &self.applied_selectors.borrow(),
                    &self.selectors_operations(),
                );
                self.registrations_invalid.set(false);
            }
        }
    }
    fn restore_parts(&self, parts: &RememberedParts, operations: &DockTargetOperations) {
        for (operation, weak) in parts {
            if let Some(control) = weak
                .upgrade()
                .filter(|control| self.is_visual_ancestor_of(control))
            {
                operations.borrow_mut().insert(*operation, control);
            }
        }
    }
    /// Adds the named indicator using its attached operation, rather than its part name.
    pub fn add_indicator(&self, name: &str, scope: &dyn INameScope) {
        if let Some(control) = scope.find(name).and_then(|o| o.cast::<Control>()) {
            self.indicator_operations().borrow_mut().insert(
                DockProperties::get_indicator_dock_operation(&control),
                control,
            );
        }
    }
    /// Adds the named selector using its attached operation, rather than its part name.
    pub fn add_selector(&self, name: &str, scope: &dyn INameScope) {
        if let Some(control) = scope.find(name).and_then(|o| o.cast::<Control>()) {
            self.selectors_operations().borrow_mut().insert(
                DockProperties::get_indicator_dock_operation(&control),
                control,
            );
        }
    }
    /// Resolves the current operation and updates selector/indicator opacity in source order.
    pub fn get_dock_operation(
        &self,
        point: Point,
        drop_control: &Control,
        relative_to: &Visual,
        action: DragAction,
        validate: &DockOperationHandler<'_>,
        visible: Option<&DockOperationHandler<'_>>,
    ) -> DockOperation {
        self.ensure_template_registrations();
        if self.show_indicators_only() {
            self.operation_indicators_only(point, drop_control, relative_to, action, visible)
        } else {
            self.operation_from_selectors(point, relative_to, action, validate, visible)
        }
    }
    fn operation_indicators_only(
        &self,
        point: Point,
        drop_control: &Control,
        relative_to: &Visual,
        action: DragAction,
        visible: Option<&DockOperationHandler<'_>>,
    ) -> DockOperation {
        let operation = DockProperties::get_indicator_dock_operation(drop_control);
        let indicator = self
            .indicator_operations()
            .borrow()
            .get(&operation)
            .cloned();
        let selector = self
            .selectors_operations()
            .borrow()
            .get(&operation)
            .cloned();
        for (_, other) in entries(self.indicator_operations()) {
            if indicator.as_ref() != Some(&other) {
                other.set_opacity(0.0);
            }
        }
        if !self.is_operation_enabled(operation) {
            if let Some(indicator) = indicator {
                indicator.set_opacity(0.0);
            }
            if let Some(selector) = selector {
                selector.set_opacity(0.0);
            }
            return self.default_dock_operation();
        }
        if self.invalidate_indicator_only(
            Some(drop_control),
            indicator.as_deref(),
            point,
            relative_to,
            operation,
            action,
            visible,
        ) {
            operation
        } else {
            self.default_dock_operation()
        }
    }
    fn operation_from_selectors(
        &self,
        point: Point,
        relative_to: &Visual,
        action: DragAction,
        validate: &DockOperationHandler<'_>,
        visible: Option<&DockOperationHandler<'_>>,
    ) -> DockOperation {
        let mut result = self.default_dock_operation();
        for (operation, indicator) in entries(self.indicator_operations()) {
            let selector = self
                .selectors_operations()
                .borrow()
                .get(&operation)
                .cloned();
            if !self.is_operation_enabled(operation) {
                if let Some(selector) = selector {
                    selector.set_opacity(0.0);
                }
                indicator.set_opacity(0.0);
                continue;
            }
            if self.invalidate_indicator(
                selector.as_deref(),
                Some(&indicator),
                point,
                relative_to,
                operation,
                action,
                validate,
                visible,
            ) {
                result = operation;
            }
        }
        for (operation, indicator) in entries(self.indicator_operations()) {
            if operation != result {
                indicator.set_opacity(0.0);
            }
        }
        result
    }
    /// Tests reference membership in the original protected selector dictionary.
    pub fn is_dock_target_selector(&self, selector: &Control) -> bool {
        self.selectors_operations()
            .borrow()
            .values()
            .any(|control| control.ptr_eq(&selector.to_ref()))
    }
    #[allow(clippy::too_many_arguments)]
    fn invalidate_indicator(
        &self,
        selector: Option<&Control>,
        indicator: Option<&Control>,
        point: Point,
        relative_to: &Visual,
        operation: DockOperation,
        action: DragAction,
        validate: &DockOperationHandler<'_>,
        visible: Option<&DockOperationHandler<'_>>,
    ) -> bool {
        let (Some(selector), Some(indicator)) = (selector, indicator) else {
            return false;
        };
        let registered = self.is_dock_target_selector(selector);
        if visible.is_some_and(|visible| !visible(point, operation, action, relative_to)) {
            indicator.set_opacity(0.0);
            if registered {
                selector.set_opacity(0.0);
            }
            return false;
        }
        if registered {
            selector.set_opacity(1.0);
        }
        if let Some(position) = translate_to_selector(relative_to, point, selector) {
            if is_selector_hit(selector, position)
                && validate(point, operation, action, relative_to)
            {
                indicator.set_opacity(0.5);
                return true;
            }
        }
        indicator.set_opacity(0.0);
        false
    }
    #[allow(clippy::too_many_arguments)]
    fn invalidate_indicator_only(
        &self,
        selector: Option<&Control>,
        indicator: Option<&Control>,
        point: Point,
        relative_to: &Visual,
        operation: DockOperation,
        action: DragAction,
        visible: Option<&DockOperationHandler<'_>>,
    ) -> bool {
        let (Some(selector), Some(indicator)) = (selector, indicator) else {
            return false;
        };
        if visible.is_some_and(|visible| !visible(point, operation, action, relative_to)) {
            indicator.set_opacity(0.0);
            return false;
        }
        if translate_to_selector(relative_to, point, selector).is_some() {
            indicator.set_opacity(0.5);
            return true;
        }
        indicator.set_opacity(0.0);
        false
    }
    /// Implements the original explicit IDockTarget.Reset operation.
    pub fn reset(&self) {
        for (_, control) in
            entries(self.indicator_operations()).chain(entries(self.selectors_operations()))
        {
            control.set_opacity(0.0);
        }
        self.set_show_horizontal_targets(true);
        self.set_show_vertical_targets(true);
        self.set_is_global_dock_available(false);
        self.set_is_global_dock_active(false);
    }
    fn as_dock_target(this: Ref<Self>) -> Rc<dyn IDockTarget> {
        if let Some(handle) = this.handle.borrow().upgrade() {
            return handle;
        }
        let handle = Rc::new(TargetHandle(this.to_ref()));
        this.handle.replace(Rc::downgrade(&handle));
        handle
    }
}
struct TargetHandle(Ref<DockTargetBase>);
impl IDockTarget for TargetHandle {
    fn reset(&self) {
        self.0.reset();
    }
}
fn remember_parts(parts: &DockTargetOperations) -> RememberedParts {
    parts
        .borrow()
        .iter()
        .map(|(operation, control)| (*operation, control.downgrade()))
        .collect()
}
// Shared System preserves source enumeration order. Read each entry live, releasing
// storage before policy/property callbacks. Its unexposed version/removal-slot
// protocol remains an explicit runtime parity gap, not a snapshot workaround.
fn entries(parts: DockTargetOperations) -> impl Iterator<Item = (DockOperation, Ref<Control>)> {
    let mut index = 0;
    std::iter::from_fn(move || {
        let entry = parts
            .borrow()
            .iter()
            .nth(index)
            .map(|(operation, control)| (*operation, control.clone()));
        index += 1;
        entry
    })
}
fn is_selector_hit(selector: &Control, point: Point) -> bool {
    for visual in selector.get_visuals_at(point) {
        if visual.ptr_eq(&selector.to_ref()) || selector.is_visual_ancestor_of(&visual) {
            return true;
        }
    }
    Rect::from_size(selector.bounds().size()).contains(point)
}
fn translate_to_selector(relative_to: &Visual, point: Point, selector: &Control) -> Option<Point> {
    let source = TopLevel::get_top_level(Some(relative_to));
    let target = TopLevel::get_top_level(Some(selector));
    if let (Some(source), Some(target)) = (source, target) {
        if source == target {
            return relative_to
                .translate_point(point, &source)
                .and_then(|position| source.translate_point(position, selector));
        }
        return Some(selector.point_to_client(relative_to.point_to_screen(point)));
    }
    relative_to.translate_point(point, selector)
}
