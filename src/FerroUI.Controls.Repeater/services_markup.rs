// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See LICENSE in the project root for license information.

//! Markup and binding metadata for the repeater's native service contracts.

// The imports of the markup of the test hooks, which is compiled with the feature `testing`.
#![cfg_attr(not(any(test, feature = "testing")), allow(unused_imports))]
use super::{
    BuildTreeScheduler, ElementFactoryGetArgs, ElementFactoryRecycleArgs, ElementFactoryTemplates,
    ElementOwner, IDataTemplateComponent, IDataTemplateSelector, IElementFactory,
    ItemCollectionTransition, ItemCollectionTransitionCompletedEventArgs,
    ItemCollectionTransitionOperation, ItemCollectionTransitionProgress,
    ItemCollectionTransitionProvider, ItemCollectionTransitionTriggers, ItemTemplateWrapper,
    Phaser, RepeaterBindingHelper, SelectTemplateEventArgs,
};
use ferroui_controls::templates::IDataTemplate;
use ferroui_controls::{Control, ItemsSource, ItemsSourceView};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{MarkupDelegate, MarkupType, MarkupTyped};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{ferro_markup_enum, ferro_markup_type, BoxedValue, FerroObject, Rect, Ref};
use std::rc::Rc;

ferro_markup_type!(class ElementFactoryGetArgs {
    namespace: "FerroUI.Controls.Repeater",
    handles: [ElementFactoryGetArgs, Option<ElementFactoryGetArgs>],
    constructors: [
        () => ElementFactoryGetArgs::default,
        (Option<BoxedValue>, Option<Ref<Control>>) => ElementFactoryGetArgs::new,
    ],
    properties: [
        Data: Option<BoxedValue> { get: ElementFactoryGetArgs::data, set: ElementFactoryGetArgs::set_data },
        Parent: Option<Ref<Control>> { get: ElementFactoryGetArgs::parent, set: ElementFactoryGetArgs::set_parent },
    ],
});

ferro_markup_type!(class ElementFactoryRecycleArgs {
    namespace: "FerroUI.Controls.Repeater",
    handles: [ElementFactoryRecycleArgs, Option<ElementFactoryRecycleArgs>],
    constructors: [(Ref<Control>, Option<Ref<Control>>) => ElementFactoryRecycleArgs::new],
    properties: [
        Element: Ref<Control> { get: ElementFactoryRecycleArgs::element, set: ElementFactoryRecycleArgs::set_element },
        Parent: Option<Ref<Control>> { get: ElementFactoryRecycleArgs::parent, set: ElementFactoryRecycleArgs::set_parent },
    ],
});

ferro_markup_type!(class SelectTemplateEventArgs {
    namespace: "FerroUI.Controls.Repeater",
    handles: [SelectTemplateEventArgs, Option<SelectTemplateEventArgs>],
    properties: [
        TemplateKey: String {
            get: SelectTemplateEventArgs::template_key,
            set: |args: &SelectTemplateEventArgs, value: String| args.set_template_key(value)
        },
        DataContext: Option<BoxedValue> { get: SelectTemplateEventArgs::data_context },
        Owner: Option<Ref<Control>> { get: SelectTemplateEventArgs::owner },
    ],
});

ferro_markup_type!(interface dyn IElementFactory as "IElementFactory" {
    namespace: "FerroUI.Controls.Repeater",
    handles: [Rc<dyn IElementFactory>, Option<Rc<dyn IElementFactory>>],
    this: Rc<dyn IElementFactory>,
    methods: [
        try fn GetElement(ElementFactoryGetArgs) -> Ref<Control> =>
            |factory: &Rc<dyn IElementFactory>, args: ElementFactoryGetArgs| factory.get_element(&args),
        try fn RecycleElement(ElementFactoryRecycleArgs) =>
            |factory: &Rc<dyn IElementFactory>, args: ElementFactoryRecycleArgs| factory.recycle_element(&args),
    ],
});

ferro_markup_type!(interface dyn IDataTemplateSelector as "IDataTemplateSelector" {
    namespace: "FerroUI.Controls.Repeater",
    handles: [Rc<dyn IDataTemplateSelector>, Option<Rc<dyn IDataTemplateSelector>>],
    this: Rc<dyn IDataTemplateSelector>,
    methods: [
        try fn SelectTemplate(Option<BoxedValue>) -> Option<Rc<dyn IDataTemplate>> =>
            |selector: &Rc<dyn IDataTemplateSelector>, item: Option<BoxedValue>| selector.try_select_template(item.as_ref()),
        try fn SelectTemplate(Option<BoxedValue>, Option<Ref<Control>>) -> Option<Rc<dyn IDataTemplate>> =>
            |selector: &Rc<dyn IDataTemplateSelector>, item: Option<BoxedValue>, owner: Option<Ref<Control>>| selector.try_select_template_with_container(item.as_ref(), owner.as_ref()),
    ],
});

ferro_markup_type!(interface dyn IDataTemplateComponent as "IDataTemplateComponent" {
    namespace: "FerroUI.Controls.Repeater",
    handles: [Rc<dyn IDataTemplateComponent>, Option<Rc<dyn IDataTemplateComponent>>],
    this: Rc<dyn IDataTemplateComponent>,
    methods: [
        fn ProcessBindings(Option<BoxedValue>, i32, i32) -> i32 =>
            |component: &Rc<dyn IDataTemplateComponent>, data: Option<BoxedValue>, index: i32, phase: i32| component.process_bindings(data, index, phase),
        fn Recycle() => |component: &Rc<dyn IDataTemplateComponent>| component.recycle(),
    ],
});

ferro_markup_type!(class ItemTemplateWrapper {
    namespace: "FerroUI.Controls.Repeater",
    handles: [Rc<ItemTemplateWrapper>, Option<Rc<ItemTemplateWrapper>>],
    this: Rc<ItemTemplateWrapper>,
    interfaces: [Rc<dyn IElementFactory>],
    constructors: [
        (Rc<dyn IDataTemplate>) => ItemTemplateWrapper::new,
        (Rc<dyn IDataTemplateSelector>) => ItemTemplateWrapper::with_selector,
    ],
    properties: [
        Template: Option<Rc<dyn IDataTemplate>> { get: ItemTemplateWrapper::template, set: ItemTemplateWrapper::set_template },
        TemplateSelector: Option<Rc<dyn IDataTemplateSelector>> { get: ItemTemplateWrapper::template_selector, set: ItemTemplateWrapper::set_template_selector },
    ],
    methods: [
        try fn GetElement(ElementFactoryGetArgs) -> Ref<Control> =>
            |factory: &Rc<ItemTemplateWrapper>, args: ElementFactoryGetArgs| factory.get_element(&args),
        try fn RecycleElement(ElementFactoryRecycleArgs) =>
            |factory: &Rc<ItemTemplateWrapper>, args: ElementFactoryRecycleArgs| factory.recycle_element(&args),
    ],
});

/// Carries metadata for the native dictionary handle, whose generic type is
/// defined in FerroUI.Base. Markup adds a keyed data template through Add.
#[doc(hidden)]
pub struct ElementFactoryTemplatesMetadata;
ferro_markup_type!(class ElementFactoryTemplatesMetadata as "ElementFactoryTemplates" {
    namespace: "FerroUI.Controls.Repeater",
    handles: [ElementFactoryTemplates, Option<ElementFactoryTemplates>],
    this: ElementFactoryTemplates,
    constructors: [() => ElementFactoryTemplates::new],
    properties: [Count: i32 { get: |templates: &ElementFactoryTemplates| templates.count() as i32 }],
    indexers: [
        (String) -> Rc<dyn IDataTemplate> {
            try_get: |templates: &ElementFactoryTemplates, key: String| templates.try_get_value(&key).ok_or_else(|| format!("No template with key '{key}' exists.")),
            set: |templates: &ElementFactoryTemplates, key: String, template: Rc<dyn IDataTemplate>| templates.set(key, template)
        },
    ],
    methods: [
        try fn Add(String, Rc<dyn IDataTemplate>) => |templates: &ElementFactoryTemplates, key: String, template: Rc<dyn IDataTemplate>| {
            if templates.contains_key(&key) { return Err(format!("A template with key '{key}' already exists.")); }
            templates.add(key, template);
            Ok(())
        },
        fn Clear() => |templates: &ElementFactoryTemplates| templates.clear(),
    ],
});

ferro_markup_enum!(ItemCollectionTransitionOperation { Add, Remove, Move }, { namespace: "FerroUI.Controls.Repeater" });
ferro_markup_enum!(flags ItemCollectionTransitionTriggers {
    None = ItemCollectionTransitionTriggers::empty(),
    CollectionChangeAdd = ItemCollectionTransitionTriggers::COLLECTION_CHANGE_ADD,
    CollectionChangeRemove = ItemCollectionTransitionTriggers::COLLECTION_CHANGE_REMOVE,
    CollectionChangeReset = ItemCollectionTransitionTriggers::COLLECTION_CHANGE_RESET,
    LayoutTransition = ItemCollectionTransitionTriggers::LAYOUT_TRANSITION,
}, { namespace: "FerroUI.Controls.Repeater" });
ferro_markup_enum!(ElementOwner { ElementFactory, Layout, PinnedPool, UniqueIdResetPool, Animator }, { namespace: "FerroUI.Controls.Repeater" });

ferro_markup_type!(class ItemCollectionTransition {
    namespace: "FerroUI.Controls.Repeater",
    handles: [Rc<ItemCollectionTransition>, Option<Rc<ItemCollectionTransition>>],
    this: Rc<ItemCollectionTransition>,
    constructors: [
        (Ref<ItemCollectionTransitionProvider>, Ref<Control>, ItemCollectionTransitionOperation, ItemCollectionTransitionTriggers) => ItemCollectionTransition::new,
        (Ref<ItemCollectionTransitionProvider>, Ref<Control>, ItemCollectionTransitionTriggers, Rect, Rect) => ItemCollectionTransition::with_bounds,
    ],
    properties: [
        Element: Ref<Control> { get: ItemCollectionTransition::element },
        Operation: ItemCollectionTransitionOperation { get: ItemCollectionTransition::operation },
        Triggers: ItemCollectionTransitionTriggers { get: ItemCollectionTransition::triggers },
        OldBounds: Rect { get: ItemCollectionTransition::old_bounds },
        NewBounds: Rect { get: ItemCollectionTransition::new_bounds },
        HasStarted: bool { get: ItemCollectionTransition::has_started },
    ],
    methods: [fn Start() -> Rc<ItemCollectionTransitionProgress> => |transition: &Rc<ItemCollectionTransition>| transition.start()],
});

ferro_markup_type!(class ItemCollectionTransitionProgress {
    namespace: "FerroUI.Controls.Repeater",
    handles: [Rc<ItemCollectionTransitionProgress>, Option<Rc<ItemCollectionTransitionProgress>>],
    this: Rc<ItemCollectionTransitionProgress>,
    properties: [
        Transition: Option<Rc<ItemCollectionTransition>> { get: ItemCollectionTransitionProgress::transition },
        Element: Option<Ref<Control>> { get: ItemCollectionTransitionProgress::element },
    ],
    methods: [fn Complete() => ItemCollectionTransitionProgress::complete],
});

ferro_markup_type!(class ItemCollectionTransitionCompletedEventArgs {
    namespace: "FerroUI.Controls.Repeater",
    handles: [ItemCollectionTransitionCompletedEventArgs, Option<ItemCollectionTransitionCompletedEventArgs>],
    properties: [
        Transition: Rc<ItemCollectionTransition> { get: ItemCollectionTransitionCompletedEventArgs::transition },
        Element: Ref<Control> { get: ItemCollectionTransitionCompletedEventArgs::element },
    ],
});

ferro_markup_type!(static RepeaterBindingHelper {
    namespace: "FerroUI.Controls.Repeater",
    type_info: RepeaterBindingHelper,
    methods: [
        static fn GetDataTemplateComponent(Ref<Control>) -> Option<Rc<dyn IDataTemplateComponent>> =>
            |element: Ref<Control>| Phaser::get_data_template_component(&element),
        static fn SetDataTemplateComponent(Ref<Control>, Option<Rc<dyn IDataTemplateComponent>>) =>
            |element: Ref<Control>, component: Option<Rc<dyn IDataTemplateComponent>>| Phaser::set_data_template_component(&element, component),
    ],
});

// The test hooks are registered for markup only with the feature `testing` (WinUI keeps them
// in a private namespace).
#[cfg(any(test, feature = "testing"))]
use super::RepeaterTestHooks;
#[cfg(any(test, feature = "testing"))]
ferro_markup_type!(static RepeaterTestHooks {
    namespace: "FerroUI.Controls.Repeater",
    type_info: RepeaterTestHooks,
    methods: [
        static fn GetLayoutId(Ref<FerroObject>) -> String => |layout: Ref<FerroObject>| RepeaterTestHooks::get_layout_id(&layout),
        static fn SetLayoutId(Ref<FerroObject>, String) => |layout: Ref<FerroObject>, id: String| RepeaterTestHooks::set_layout_id(&layout, id),
        static fn GetElementFactoryElementIndex() -> i32 => RepeaterTestHooks::get_element_factory_element_index,
        static fn SetElementFactoryElementIndex(i32) => RepeaterTestHooks::set_element_factory_element_index,
        static fn GetLogItemIndex() -> i32 => RepeaterTestHooks::get_log_item_index,
        static fn SetLogItemIndex(i32) => RepeaterTestHooks::set_log_item_index,
        // Static-event syntax is not part of FerroUI's markup contract. The
        // equivalent method returns the ordinary disposable subscription.
        static fn SubscribeBuildTreeCompleted(MarkupDelegate) -> Rc<dyn IDisposable> => |handler: MarkupDelegate| {
            RepeaterTestHooks::build_tree_completed(move || { handler.invoke(&[None, None]); })
        },
    ],
});

ferro_markup_type!(static BuildTreeScheduler {
    namespace: "FerroUI.Controls.Repeater",
    static_properties: [
        BudgetMilliseconds: f64 { get: BuildTreeScheduler::budget_milliseconds, set: BuildTreeScheduler::set_budget_milliseconds },
        PendingWorkCount: i32 { get: || BuildTreeScheduler::pending_work_count() as i32 },
    ],
    methods: [static fn ShouldYield() -> bool => BuildTreeScheduler::should_yield],
});

const TYPES: &[&MarkupType] = &[
    <ElementFactoryGetArgs as MarkupTyped>::MARKUP,
    <ElementFactoryRecycleArgs as MarkupTyped>::MARKUP,
    <SelectTemplateEventArgs as MarkupTyped>::MARKUP,
    <dyn IElementFactory as MarkupTyped>::MARKUP,
    <dyn IDataTemplateSelector as MarkupTyped>::MARKUP,
    <dyn IDataTemplateComponent as MarkupTyped>::MARKUP,
    <ItemTemplateWrapper as MarkupTyped>::MARKUP,
    <ElementFactoryTemplatesMetadata as MarkupTyped>::MARKUP,
    <ItemCollectionTransitionOperation as MarkupTyped>::MARKUP,
    <ItemCollectionTransitionTriggers as MarkupTyped>::MARKUP,
    <ElementOwner as MarkupTyped>::MARKUP,
    <ItemCollectionTransition as MarkupTyped>::MARKUP,
    <ItemCollectionTransitionProgress as MarkupTyped>::MARKUP,
    <ItemCollectionTransitionCompletedEventArgs as MarkupTyped>::MARKUP,
    <RepeaterBindingHelper as MarkupTyped>::MARKUP,
    <BuildTreeScheduler as MarkupTyped>::MARKUP,
];

pub(crate) fn register_types() {
    MarkupType::register_all(TYPES);
    #[cfg(any(test, feature = "testing"))]
    MarkupType::register_all(&[<RepeaterTestHooks as MarkupTyped>::MARKUP]);
    ValueTypes::register_global(register_value_types);
}

// Controls metadata is registered once for the process, whereas value
// conversions belong to each UI thread. Keep the registration callback global
// so loading templates or binding services also works on later UI threads.
fn register_value_types() {
    ValueTypes::register_nullable::<ElementFactoryGetArgs>();
    ValueTypes::register_nullable::<ElementFactoryRecycleArgs>();
    ValueTypes::register_nullable::<SelectTemplateEventArgs>();
    ValueTypes::register_nullable::<Rc<dyn IElementFactory>>();
    ValueTypes::register_nullable::<Rc<dyn IDataTemplateSelector>>();
    ValueTypes::register_nullable::<Rc<dyn IDataTemplateComponent>>();
    ValueTypes::register_nullable::<Rc<ItemTemplateWrapper>>();
    ValueTypes::register_nullable::<ElementFactoryTemplates>();
    ValueTypes::register_nullable::<Rc<ItemCollectionTransition>>();
    ValueTypes::register_nullable::<Rc<ItemCollectionTransitionProgress>>();
    ValueTypes::register_nullable::<ItemCollectionTransitionCompletedEventArgs>();
    ValueTypes::register_nullable::<Rc<ItemsSourceView>>();
    ItemsSource::register_binding_conversion_with::<Rc<ItemsSourceView>>(|view| {
        view.clone().into()
    });
    ValueTypes::register_cast::<Rc<ItemTemplateWrapper>, Rc<dyn IElementFactory>>(|wrapper| {
        wrapper.clone()
    });
}
