// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model_reactiveui_services::lifecycle::IActivationBridge;
use ferrodock_model_reactiveui_services::overlays::hosting::HostTypeRegistry;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObject,
    FerroObjectExtensions, FerroObjectImpl, FerroObjectImplExt, Ref, StyledElementImpl,
    StyledProperty, VisualImpl,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{ContentControlImpl, ControlImpl};
use ferroui_reactive::disposables::{Disposable, SerialDisposable};
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::{register_activatable_view_model, IActivatableViewModel, ViewForMixins};
use ferroui_reactiveui_controls::{
    view_model_of_optional, IReactiveViewClass, ITypedReactiveViewClass, ReactiveUserControlBase,
    ReactiveView, ReactiveViewFor, ViewModelFilter,
};
use ferroui_splat::{Locator, ReadonlyDependencyResolverExt};
use std::rc::Rc;

/// Source generic reactive user control represented by the framework's typed
/// handle over a non-generic class. Deriving markup views use the base class.
pub type DockReactiveUserControl<VM> = ReactiveView<DockReactiveUserControlBase, VM>;
/// Source abstract reactive view base bridging each activated view-model change
/// to IActivationBridge, with the source direct-activator fallback.
#[repr(C)]
pub struct DockReactiveUserControlBase {
    base: ReactiveUserControlBase,
    types: Rc<HostTypeRegistry>,
}
ferro_class!(DockReactiveUserControlBase: ReactiveUserControlBase);
// Abstract in markup: no parameterless constructor is exported in class metadata.
ferro_class_info!(DockReactiveUserControlBase {});
ferro_impl_classes!(DockReactiveUserControlBase: StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl, TemplatedControlImpl, ContentControlImpl);
impl FerroObjectImpl for DockReactiveUserControlBase {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        let view = this.to_ref();
        let weak = view.downgrade();
        let activation_view = ReactiveViewFor::weak(&view);
        let _ = activation_view.when_activated_disposables(move |disposables| {
            let bridge = Locator::current().get_service::<dyn IActivationBridge>();
            let handle = SerialDisposable::new();
            disposables.add(handle.clone());
            let control = weak.upgrade().expect("live activated control");
            let weak = control.downgrade();
            let types = control.types.clone();
            let object: &FerroObject = &control;
            let subscription = object
                .get_observable(ReactiveUserControlBase::view_model_property())
                .subscribe_next(move |value: Option<BoxedValue>| {
                    let activatable = view_model_of_optional(value.as_ref())
                        .and_then(|model| types.reference::<dyn IActivatableViewModel>(&model));
                    let next = match activatable {
                        Some(model) => match &bridge {
                            Some(bridge) => bridge.track(
                                model,
                                ReactiveViewFor::new(
                                    weak.upgrade().expect("live view-model observer"),
                                ),
                            ),
                            None => model.activator().activate(),
                        },
                        None => Disposable::empty(),
                    };
                    handle.set_disposable(Some(next));
                });
            disposables.add(subscription);
        });
    }
}
impl DockReactiveUserControlBase {
    /// Initializes fields for this class or a derived view's composition.
    pub fn construct() -> Self {
        Self {
            base: ReactiveUserControlBase::construct(),
            types: HostTypeRegistry::new(),
        }
    }
    /// Gets the existing native membership registry used at the erased view-model
    /// boundary. Register interface membership before binding or activation.
    pub fn native_types(&self) -> Rc<HostTypeRegistry> {
        self.types.clone()
    }
    /// Registers this same model's native activatable projection and the shared
    /// ReactiveUI type registration, without retaining the model. Automatic
    /// unregistered CLR interface discovery remains an explicit metadata gap.
    pub fn register_activatable_view_model<T: IActivatableViewModel>(&self, model: &Rc<T>) {
        register_activatable_view_model::<T>();
        self.types
            .register_interface(model, model.clone() as Rc<dyn IActivatableViewModel>);
    }
}
impl IReactiveViewClass for DockReactiveUserControlBase {
    fn view_model_property() -> &'static StyledProperty<Option<BoxedValue>> {
        ReactiveUserControlBase::view_model_property()
    }
    fn is_valid_view_model_value(&self, value: Option<&BoxedValue>) -> bool {
        IReactiveViewClass::is_valid_view_model_value(&self.base, value)
    }
}
impl ITypedReactiveViewClass for DockReactiveUserControlBase {
    fn create() -> Ref<Self> {
        instantiate(Self::construct())
    }
    fn set_view_model_filter(&self, filter: Option<ViewModelFilter>) {
        self.base.set_view_model_filter(filter);
    }
}
