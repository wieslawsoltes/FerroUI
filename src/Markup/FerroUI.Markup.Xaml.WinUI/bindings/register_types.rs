use super::{WinUIBindExtension, WinUIBinding, WinUIBindingScope, WinUIBindings};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::data::BindingBase;
use ferroui_base::ferro_markup_type;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use std::rc::Rc;

pub(crate) const TYPES: &[&MarkupType] = &[
    <WinUIBindExtension as MarkupTyped>::MARKUP,
    <WinUIBinding as MarkupTyped>::MARKUP,
    <WinUIBindingScope as MarkupTyped>::MARKUP,
    <WinUIBindings as MarkupTyped>::MARKUP,
];

pub(crate) fn register_markup_types() {
    MarkupType::register_all(TYPES);
    ValueTypes::register_global(register_value_types);
}

fn register_value_types() {
    ValueTypes::register_nullable::<Rc<WinUIBindExtension>>();
    ValueTypes::register_nullable::<Rc<WinUIBinding>>();
    ValueTypes::register_nullable::<Rc<WinUIBindings>>();
    ValueTypes::register_nullable::<WinUIBindingScope>();
    ValueTypes::register_cast::<Rc<WinUIBinding>, Rc<dyn BindingBase>>(|binding| binding.clone());
}

ferro_markup_type!(class WinUIBindings {
    namespace: "FerroUI.Markup.Xaml.WinUI",
    this: Rc<WinUIBindings>,
    handles: [Rc<WinUIBindings>, Option<Rc<WinUIBindings>>],
    properties: [
        IsInitialized: bool { get: |this: &Rc<WinUIBindings>| this.is_initialized() },
    ],
    methods: [
        try fn Initialize() => |this: &Rc<WinUIBindings>| this.initialize(),
        try fn Update() => |this: &Rc<WinUIBindings>| this.update(),
        fn StopTracking() => |this: &Rc<WinUIBindings>| this.stop_tracking(),
        fn SetStopOnUnload(bool) -> bool => |this: &Rc<WinUIBindings>, enabled: bool| this.set_stop_on_unload(enabled),
    ],
});
