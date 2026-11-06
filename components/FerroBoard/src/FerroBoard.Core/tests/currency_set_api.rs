use contract::Driver;
use ferroboard_core::{layouts::*, text::Utf16Text};
use std::rc::Rc;
#[path = "../../../tests/support/currency_set_api_contract.rs"]
mod contract;
#[test]
fn original_currency_defaults_shared_init_slot_rereads_and_registry_keys() {
    assert_eq!(
        contract::verify::<Core>(include_str!(
            "../../../tests/fixtures/currency-set-api.jsonl"
        )),
        102
    );
}

pub struct Core;
pub fn error(e: KeyDataError) -> String {
    match e {
        KeyDataError::NullReference => "NullReferenceException",
        KeyDataError::Collection(ResourceTextListError::IndexOutOfRange) => {
            "ArgumentOutOfRangeException"
        }
        KeyDataError::Collection(ResourceTextListError::Modified) => "InvalidOperationException",
        _ => panic!("unexpected {e:?}"),
    }
    .into()
}
impl Driver for Core {
    fn make() -> Rc<CurrencySet> {
        Rc::new(CurrencySet::default())
    }
    fn set_text(c: &Rc<CurrencySet>, name: &str, value: Option<Utf16Text>) {
        match name {
            "ExtensionId" => c.set_extension_id(value),
            "Id" => c.set_id(value),
            "Label" => c.set_label(value),
            _ => panic!("{name}"),
        }
    }
    fn get_text(c: &Rc<CurrencySet>, name: &str) -> Option<Utf16Text> {
        match name {
            "ExtensionId" => c.extension_id(),
            "Id" => c.id(),
            "Label" => c.label(),
            _ => panic!("{name}"),
        }
    }
    fn set_slots(c: &Rc<CurrencySet>, v: Option<ResourceReadOnlyList<TextKey>>) {
        c.set_slots(v)
    }
    fn slots(c: &Rc<CurrencySet>) -> Option<ResourceReadOnlyList<TextKey>> {
        c.slots()
    }
    fn lookup(c: &Rc<CurrencySet>, code: i32) -> Result<Option<TextKey>, String> {
        c.get_slot(code).map_err(error)
    }
}
