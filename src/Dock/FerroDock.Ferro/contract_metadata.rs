//! UI-owned descriptors of the original foreign service and lifecycle handles.
//! The empty carriers have no behavioral objects; domain traits remain UI independent.
use ferrodock_model::collections::ReadOnlyObservableCollection;
use ferrodock_model::services::*;
use ferroui_reactiveui::ICommand;
use std::rc::Rc;
/// Descriptor carrier for the original IDockBusyService handles.
#[doc(hidden)]
pub trait IDockBusyServiceMetadata {}
ferroui_base::ferro_markup_type!(interface dyn IDockBusyServiceMetadata as "IDockBusyService" {
    namespace:"FerroDock.Model.Services",handles:[Rc<dyn IDockBusyService>,Option<Rc<dyn IDockBusyService>>]
});
/// Descriptor carrier for the original IDockGlobalBusyService handles.
#[doc(hidden)]
pub trait IDockGlobalBusyServiceMetadata {}
ferroui_base::ferro_markup_type!(interface dyn IDockGlobalBusyServiceMetadata as "IDockGlobalBusyService" {
    namespace:"FerroDock.Model.Services",handles:[Rc<dyn IDockGlobalBusyService>,Option<Rc<dyn IDockGlobalBusyService>>]
});
/// Descriptor carrier for the original IDockDialogService handles.
#[doc(hidden)]
pub trait IDockDialogServiceMetadata {}
ferroui_base::ferro_markup_type!(interface dyn IDockDialogServiceMetadata as "IDockDialogService" {
    namespace:"FerroDock.Model.Services",handles:[Rc<dyn IDockDialogService>,Option<Rc<dyn IDockDialogService>>]
});
/// Descriptor carrier for the original IDockGlobalDialogService handles.
#[doc(hidden)]
pub trait IDockGlobalDialogServiceMetadata {}
ferroui_base::ferro_markup_type!(interface dyn IDockGlobalDialogServiceMetadata as "IDockGlobalDialogService" {
    namespace:"FerroDock.Model.Services",handles:[Rc<dyn IDockGlobalDialogService>,Option<Rc<dyn IDockGlobalDialogService>>]
});
/// Descriptor carrier for the original IDockConfirmationService handles.
#[doc(hidden)]
pub trait IDockConfirmationServiceMetadata {}
ferroui_base::ferro_markup_type!(interface dyn IDockConfirmationServiceMetadata as "IDockConfirmationService" {
    namespace:"FerroDock.Model.Services",handles:[Rc<dyn IDockConfirmationService>,Option<Rc<dyn IDockConfirmationService>>]
});
/// Descriptor carrier for the original IDockGlobalConfirmationService handles.
#[doc(hidden)]
pub trait IDockGlobalConfirmationServiceMetadata {}
ferroui_base::ferro_markup_type!(interface dyn IDockGlobalConfirmationServiceMetadata as "IDockGlobalConfirmationService" {
    namespace:"FerroDock.Model.Services",handles:[Rc<dyn IDockGlobalConfirmationService>,Option<Rc<dyn IDockGlobalConfirmationService>>]
});
/// Descriptor carrier for the original IVisualTreeLifecycle handles.
#[doc(hidden)]
pub trait IVisualTreeLifecycleMetadata {}
ferroui_base::ferro_markup_type!(interface dyn IVisualTreeLifecycleMetadata as "IVisualTreeLifecycle" {
    namespace:"FerroDock.Model.Services",handles:[Rc<dyn IVisualTreeLifecycle>,Option<Rc<dyn IVisualTreeLifecycle>>]
});
/// Descriptor carrier for the original ICommand handles.
#[doc(hidden)]
pub trait ICommandMetadata {}
ferroui_base::ferro_markup_type!(interface dyn ICommandMetadata as "ICommand" {
    namespace:"ReactiveUI",handles:[Rc<dyn ICommand>,Option<Rc<dyn ICommand>>]
});
/// Specialized descriptor carrier for the original read-only DialogRequest collection.
#[doc(hidden)]
pub trait DialogRequestCollectionMetadata {}
ferroui_base::ferro_markup_type!(interface dyn DialogRequestCollectionMetadata as "ReadOnlyObservableCollection(DialogRequest)" {
    namespace:"System.Collections.ObjectModel",handles:[ReadOnlyObservableCollection<Rc<DialogRequest>>,Option<ReadOnlyObservableCollection<Rc<DialogRequest>>>]
});
/// Specialized descriptor carrier for the original read-only ConfirmationRequest collection.
#[doc(hidden)]
pub trait ConfirmationRequestCollectionMetadata {}
ferroui_base::ferro_markup_type!(interface dyn ConfirmationRequestCollectionMetadata as "ReadOnlyObservableCollection(ConfirmationRequest)" {
    namespace:"System.Collections.ObjectModel",handles:[ReadOnlyObservableCollection<Rc<ConfirmationRequest>>,Option<ReadOnlyObservableCollection<Rc<ConfirmationRequest>>>]
});
pub(crate) fn register_command_bridge() {
    use ferroui_base::data::core::ValueTypes;
    use ferroui_base::input::ICommand as NativeCommand;
    use ferroui_reactiveui_controls::ReactiveCommandAdapter;
    ValueTypes::register_nullable::<Rc<dyn ICommand>>();
    ValueTypes::register_conversion::<Rc<dyn ICommand>, Rc<dyn NativeCommand>>(|command| {
        Some(ReactiveCommandAdapter::command_of(command.clone()))
    });
    ValueTypes::register_conversion::<Rc<dyn ICommand>, Option<Rc<dyn NativeCommand>>>(|command| {
        Some(Some(ReactiveCommandAdapter::command_of(command.clone())))
    });
    ValueTypes::register_conversion::<Option<Rc<dyn ICommand>>, Option<Rc<dyn NativeCommand>>>(
        |command| Some(command.clone().map(ReactiveCommandAdapter::command_of)),
    );
}

/// Descriptor of the original per-host aggregate, used by the default layer bindings.
#[doc(hidden)]
pub trait IHostOverlayServicesMetadata {}
ferroui_base::ferro_markup_type!(interface dyn IHostOverlayServicesMetadata as "IHostOverlayServices" {
    namespace:"FerroDock.Model.Services",this:Rc<dyn IHostOverlayServices>,handles:[Rc<dyn IHostOverlayServices>,Option<Rc<dyn IHostOverlayServices>>],
    properties:[
        Busy:Rc<dyn IDockBusyService>{get:|this:&Rc<dyn IHostOverlayServices>|this.busy()},
        Dialogs:Rc<dyn IDockDialogService>{get:|this:&Rc<dyn IHostOverlayServices>|this.dialogs()},
        Confirmations:Rc<dyn IDockConfirmationService>{get:|this:&Rc<dyn IHostOverlayServices>|this.confirmations()},
        GlobalBusyService:Rc<dyn IDockGlobalBusyService>{get:|this:&Rc<dyn IHostOverlayServices>|this.global_busy_service()},
        GlobalDialogService:Rc<dyn IDockGlobalDialogService>{get:|this:&Rc<dyn IHostOverlayServices>|this.global_dialog_service()},
        GlobalConfirmationService:Rc<dyn IDockGlobalConfirmationService>{get:|this:&Rc<dyn IHostOverlayServices>|this.global_confirmation_service()}
    ]
});

pub(crate) fn register_service_handles() {
    use ferroui_base::data::core::ValueTypes;
    ValueTypes::register_nullable::<Rc<dyn IDockBusyService>>();
    ValueTypes::register_nullable::<Rc<dyn IDockGlobalBusyService>>();
    ValueTypes::register_nullable::<Rc<dyn IDockDialogService>>();
    ValueTypes::register_nullable::<Rc<dyn IDockGlobalDialogService>>();
    ValueTypes::register_nullable::<Rc<dyn IDockConfirmationService>>();
    ValueTypes::register_nullable::<Rc<dyn IDockGlobalConfirmationService>>();
    ValueTypes::register_nullable::<Rc<dyn IHostOverlayServices>>();
}
