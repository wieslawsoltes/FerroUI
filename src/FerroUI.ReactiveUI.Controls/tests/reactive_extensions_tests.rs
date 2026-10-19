//! `ReactiveExtensionsTests` (the tests of the integration): the public surface of the extension
//! types. Upstream reads the surface by reflection; the tests that ask for a member are ported as
//! uses of the member, which the compiler checks, and the tests of the shape of a type (static,
//! sealed, its constructors) have no counterpart.

use super::binding_support::start_app;
use crate::{FerroActivationForViewFetcher, FerroObjectReactiveExtensions, FerroPropertySignal};
use ferroui_base::data::BindingValue;
use ferroui_base::{BoxedValue, FerroObject};
use ferroui_controls::Button;
use ferroui_reactiveui::{IActivatableView, IActivationForViewFetcher};
use std::rc::Rc;

/// A view that is not a visual.
struct PlainView;

impl IActivatableView for PlainView {}

// Not ported: FerroObjectReactiveExtensions_IsStaticClass - reflection (the extensions are a trait)

#[test]
fn ferro_object_reactive_extensions_has_get_subject_method() {
    let _app = start_app();
    let button = Button::new();
    let object: &FerroObject = &button;

    let _typed: Rc<FerroPropertySignal<bool>> = object.get_subject(Button::is_default_property());
    let _untyped: Rc<FerroPropertySignal<BoxedValue>> = object.get_subject_untyped(Button::is_default_property().as_property());
}

#[test]
fn ferro_object_reactive_extensions_has_get_binding_subject_method() {
    let _app = start_app();
    let button = Button::new();
    let object: &FerroObject = &button;

    let _typed: Rc<FerroPropertySignal<BindingValue<bool>>> = object.get_binding_subject(Button::is_default_property());
    let _untyped: Rc<FerroPropertySignal<BindingValue<BoxedValue>>> =
        object.get_binding_subject_untyped(Button::is_default_property().as_property());
}

// Not ported: AutoDataTemplateBindingHook_TypeExists - reflection
// Not ported: AutoSuspendHelper_TypeExists - reflection

#[test]
fn ferro_activation_for_view_fetcher_type_exists() {
    let fetcher = FerroActivationForViewFetcher::new();

    assert_eq!(fetcher.get_affinity_for_view(&PlainView), 0);
}

// Not ported: AutoDataTemplateBindingHook_HasExpectedMethods - reflection

#[test]
fn ferro_activation_for_view_fetcher_has_expected_methods() {
    let fetcher: Rc<dyn IActivationForViewFetcher> = Rc::new(FerroActivationForViewFetcher::new());
    let view: Rc<dyn IActivatableView> = Rc::new(PlainView);

    assert_eq!(fetcher.get_affinity_for_view(&*view), 0);
    let _activation = fetcher.get_activation_for_view(&view);
}

// Not ported: ReactiveUIFerro_HasPublicExtensionMethods - reflection
// Not ported: AutoSuspendHelper_HasExpectedProperties - reflection
// Not ported: PublicTypes_HaveParameterlessConstructors - reflection
