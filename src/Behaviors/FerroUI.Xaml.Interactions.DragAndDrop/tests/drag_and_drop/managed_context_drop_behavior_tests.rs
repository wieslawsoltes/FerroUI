//! Ported from
//! `tests/Xaml.Behaviors.Interactions.UnitTests/DragAndDrop/ManagedContextDropBehaviorTests.cs` of
//! Xaml.Behaviors.
//!
//! Adapted: `Assert.Same` is the identity of the shared payload (`Rc::ptr_eq`); the `byte[]` of the
//! reference is the shared bytes of the byte formats of the framework (`Rc<[u8]>`).

use crate::managed_drag_drop::managed_data_transfer::ManagedPayloadDataTransfer;
use crate::{ManagedContextDropBehavior, ManagedDataTransferExtensions, ManagedDataTransferItemExtensions};
use ferroui_base::input::{DataFormat, DataTransferExtensions, IDataTransfer};
use ferroui_base::BoxedValue;
use std::any::Any;
use std::rc::Rc;

/// The payload of the tests (`sealed record ManagedPayload(string Value)`).
#[derive(Debug, PartialEq)]
struct ManagedPayload {
    value: String,
}

impl ManagedPayload {
    fn new(value: &str) -> Rc<Self> {
        Rc::new(Self { value: value.to_string() })
    }
}

/// The address of a shared value, for identity comparisons.
fn address<T: ?Sized>(value: &Rc<T>) -> *const () {
    Rc::as_ptr(value) as *const ()
}

#[test]
fn managed_data_transfer_extensions_return_original_payload() {
    let payload = ManagedPayload::new("payload");
    let data_transfer: Rc<dyn IDataTransfer> =
        ManagedPayloadDataTransfer::create("Managed.Payload", payload.clone() as BoxedValue);

    let value = data_transfer.try_get_managed_value_as::<ManagedPayload>("Managed.Payload").expect("the payload");
    assert_eq!(address(&payload), address(&value));
    let items = data_transfer.items();
    assert_eq!(1, items.len());
    let value = items[0].try_get_managed_value_as::<ManagedPayload>("Managed.Payload").expect("the payload");
    assert_eq!(address(&payload), address(&value));
}

#[test]
fn managed_data_transfer_extensions_keep_text_payload_compatible_with_text_apis() {
    let data_transfer: Rc<dyn IDataTransfer> =
        ManagedPayloadDataTransfer::create("Managed.Payload", Rc::new("payload".to_string()));

    assert_eq!(Some("payload".to_string()), data_transfer.try_get_text());
    assert_eq!(
        Some("payload".to_string()),
        data_transfer.try_get_value(&DataFormat::create_string_application_format("Managed.Payload"))
    );
}

#[test]
fn managed_context_drop_behavior_create_data_transfer_preserves_payload_in_drag_event_args_data_transfer() {
    let format = "Managed.Payload";
    let payload = ManagedPayload::new("payload");

    let data_transfer = ManagedContextDropBehavior::create_data_transfer(Some(payload.clone()), Some(format));

    let value = data_transfer.try_get_managed_value_as::<ManagedPayload>(format).expect("the payload");
    assert_eq!(address(&payload), address(&value));
}

#[test]
fn managed_data_transfer_extensions_keep_byte_array_payload_compatible_with_byte_apis() {
    let format = "Managed.Bytes";
    let payload: Rc<[u8]> = Rc::from([0x01u8, 0x02, 0x03]);
    let boxed: BoxedValue = Rc::new(payload.clone());

    let data_transfer = ManagedPayloadDataTransfer::create(format, boxed.clone());

    let value: Rc<dyn Any> = data_transfer.try_get_managed_value(format).expect("the payload");
    assert_eq!(address(&boxed), address(&value));
    let bytes = data_transfer.try_get_value(&DataFormat::create_bytes_application_format(format)).expect("the bytes");
    assert_eq!(address(&payload), address(&bytes));
}
