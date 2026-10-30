// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Native descriptors retaining the original options and immutable capability result.
use ferrodock_model::{
    core::{
        DockCapability, DockCapabilityEvaluation, DockCapabilityValueSource, DockWindowOptions,
        DockWindowOwnerMode, IDock, IDockWindow, IDockable,
    },
    DockCapabilityResolver,
};
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupType, MarkupTyped},
};
use std::rc::Rc;

/// Descriptor carrier for the original sealed DockWindowOptions class.
#[doc(hidden)]
pub trait DockWindowOptionsMetadata {}
ferroui_base::ferro_markup_type!(class dyn DockWindowOptionsMetadata as "DockWindowOptions" {
    namespace:"FerroDock.Model.Core",this:Rc<DockWindowOptions>,handles:[Rc<DockWindowOptions>,Option<Rc<DockWindowOptions>>],
    constructors:[()=>DockWindowOptions::new],
    properties:[
        OwnerMode:DockWindowOwnerMode{get:|this:&Rc<DockWindowOptions>|this.owner_mode(),set:|this:&Rc<DockWindowOptions>,value|this.set_owner_mode(value)},
        ParentWindow:Option<Rc<dyn IDockWindow>>{get:|this:&Rc<DockWindowOptions>|this.parent_window(),set:|this:&Rc<DockWindowOptions>,value|this.set_parent_window(value)},
        IsModal:bool{get:|this:&Rc<DockWindowOptions>|this.is_modal(),set:|this:&Rc<DockWindowOptions>,value|this.set_is_modal(value)},
        ShowInTaskbar:Option<bool>{get:|this:&Rc<DockWindowOptions>|this.show_in_taskbar(),set:|this:&Rc<DockWindowOptions>,value|this.set_show_in_taskbar(value)},
    ],
    methods:[try fn ApplyTo(Option<Rc<dyn IDockWindow>>) => (|this:&Rc<DockWindowOptions>,window:Option<Rc<dyn IDockWindow>>|{
        let window=window.ok_or("Value cannot be null. (Parameter 'window')")?;
        this.apply_to(Some(window.as_ref()));Ok::<(),&str>(())
    })],
});

/// Descriptor carrier for the original immutable DockCapabilityEvaluation class.
#[doc(hidden)]
pub trait DockCapabilityEvaluationMetadata {}
ferroui_base::ferro_markup_type!(class dyn DockCapabilityEvaluationMetadata as "DockCapabilityEvaluation" {
    namespace:"FerroDock.Model.Core",this:Rc<DockCapabilityEvaluation>,handles:[Rc<DockCapabilityEvaluation>,Option<Rc<DockCapabilityEvaluation>>],
    constructors:[(DockCapability,bool,Option<bool>,Option<bool>,Option<bool>,bool,DockCapabilityValueSource,String)=>DockCapabilityEvaluation::new],
    properties:[
        Capability:DockCapability{get:|this:&Rc<DockCapabilityEvaluation>|this.capability()},
        BaseValue:bool{get:|this:&Rc<DockCapabilityEvaluation>|this.base_value()},
        RootPolicyValue:Option<bool>{get:|this:&Rc<DockCapabilityEvaluation>|this.root_policy_value()},
        DockPolicyValue:Option<bool>{get:|this:&Rc<DockCapabilityEvaluation>|this.dock_policy_value()},
        DockableOverrideValue:Option<bool>{get:|this:&Rc<DockCapabilityEvaluation>|this.dockable_override_value()},
        EffectiveValue:bool{get:|this:&Rc<DockCapabilityEvaluation>|this.effective_value()},
        EffectiveSource:DockCapabilityValueSource{get:|this:&Rc<DockCapabilityEvaluation>|this.effective_source()},
        DiagnosticMessage:String{get:|this:&Rc<DockCapabilityEvaluation>|this.diagnostic_message().to_owned()},
    ],
});

/// Descriptor carrier for the original public capability resolver.
#[doc(hidden)]
pub struct DockCapabilityResolverMetadata;
ferroui_base::ferro_markup_type!(class DockCapabilityResolverMetadata as "DockCapabilityResolver" {
    namespace:"FerroDock.Model",
    methods:[
        static fn Evaluate(Option<Rc<dyn IDockable>>,DockCapability)->Rc<DockCapabilityEvaluation> => DockCapabilityResolver::evaluate,
        static fn Evaluate(Option<Rc<dyn IDockable>>,DockCapability,Option<Rc<dyn IDock>>)->Rc<DockCapabilityEvaluation> => DockCapabilityResolver::evaluate_with_context,
        static fn IsEnabled(Option<Rc<dyn IDockable>>,DockCapability)->bool => (|dockable,capability|DockCapabilityResolver::is_enabled(dockable,capability,None)),
        static fn IsEnabled(Option<Rc<dyn IDockable>>,DockCapability,Option<Rc<dyn IDock>>)->bool => DockCapabilityResolver::is_enabled,
        static fn ResolveOperationDock(Option<Rc<dyn IDockable>>)->Option<Rc<dyn IDock>> => DockCapabilityResolver::resolve_operation_dock,
        static fn ResolveDropTargetDock(Option<Rc<dyn IDockable>>)->Option<Rc<dyn IDock>> => DockCapabilityResolver::resolve_drop_target_dock,
    ],
});

pub(crate) fn register() {
    ferroui_base::__ferro_compiler_metadata! {
        ferroui_base::metadata::register_type_rust_paths(&[
            (||std::any::TypeId::of::<Rc<DockWindowOptions>>(),"std::rc::Rc<ferrodock_model::core::DockWindowOptions>"),
            (||std::any::TypeId::of::<Rc<DockCapabilityEvaluation>>(),"std::rc::Rc<ferrodock_model::core::DockCapabilityEvaluation>"),
        ]);
    }
    MarkupType::register_all(&[
        <dyn DockWindowOptionsMetadata>::MARKUP,
        <dyn DockCapabilityEvaluationMetadata>::MARKUP,
        DockCapabilityResolverMetadata::MARKUP,
    ]);
    ValueTypes::register_reference::<DockWindowOptions>();
    ValueTypes::register_reference::<DockCapabilityEvaluation>();
    ValueTypes::register_nullable::<Rc<DockCapabilityEvaluation>>();
    ValueTypes::register_reference_identity::<Rc<dyn IDockWindow>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
}
