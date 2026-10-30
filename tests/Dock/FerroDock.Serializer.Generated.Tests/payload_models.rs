//! Reference application payloads with the original simple class/property behavior.
use ferrodock_model::{core::SerializationResult, BoxedValue};
use ferrodock_serializer_system_text_json::{JsonTypeInfo, JsonValue};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
#[derive(Default)]
/// Original application payload class.
pub struct Payload {
    pub(crate) name: RefCell<Option<String>>,
    pub(crate) own: RefCell<Option<Rc<Payload>>>,
}
#[derive(Default)]
/// Original application holder class.
pub struct Holder {
    pub(crate) content: RefCell<Option<BoxedValue>>,
    pub(crate) other: RefCell<Option<BoxedValue>>,
    pub(crate) typed: RefCell<Option<Rc<Payload>>>,
}
#[derive(Default)]
/// Original explicitly converted property holder.
pub struct CustomHolder {
    pub(crate) content: RefCell<Option<BoxedValue>>,
}
/// Original get-only payload holder.
pub struct ReadonlyHolder {
    pub(crate) content: Option<BoxedValue>,
}
impl Default for ReadonlyHolder {
    fn default() -> Self {
        let value = Payload::new();
        value.name.replace(Some("readonly".into()));
        Self {
            content: Some(Rc::new(value)),
        }
    }
}
#[derive(Default)]
/// Original class with a renamed JSON property.
pub struct TypeNamed {
    pub(crate) kind: RefCell<Option<String>>,
    pub(crate) name: RefCell<Option<String>>,
}
#[derive(Default)]
/// Original set-only payload holder.
pub struct WriteOnlyHolder {
    pub(crate) content: RefCell<Option<BoxedValue>>,
    pub(crate) number: Cell<i32>,
}
#[derive(Default)]
/// Original class with a type-level array converter.
pub struct ArrayPayload;
#[derive(Default)]
/// Original generic application payload.
pub struct Generic<T> {
    pub(crate) value: RefCell<T>,
}
impl Payload {
    pub(crate) fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }
}
impl Holder {
    pub(crate) fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }
}
impl CustomHolder {
    pub(crate) fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }
}
impl ReadonlyHolder {
    pub(crate) fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }
}
impl TypeNamed {
    pub(crate) fn try_name(this: &Rc<Self>) -> SerializationResult<Option<String>> {
        Ok(Self::name(this))
    }
    pub(crate) fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }
}
impl WriteOnlyHolder {
    pub(crate) fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }
}
impl<T: Default> Generic<T> {
    pub(crate) fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }
}
pub(crate) fn string_info() -> Rc<JsonTypeInfo> {
    StringConverter::info()
}
impl StringConverter {
    pub(crate) fn info() -> Rc<JsonTypeInfo> {
        JsonTypeInfo::optional_with::<BoxedValue>(JsonTypeInfo::converter(
            |v: &BoxedValue, ctx| {
                ctx.value(v.downcast_ref::<String>().expect("source string converter"))
            },
            |v, ctx| Ok(ctx.value::<String>(v)?.map(|v| Rc::new(v) as BoxedValue)),
        ))
    }
}
pub(crate) fn array_info() -> Rc<JsonTypeInfo> {
    ArrayConverter::info()
}
impl ArrayConverter {
    pub(crate) fn info() -> Rc<JsonTypeInfo> {
        JsonTypeInfo::converter(
            |_: &Rc<ArrayPayload>, _| Ok(JsonValue::Array(vec![JsonValue::number(1)])),
            |v, _| -> SerializationResult<_> {
                Ok(if matches!(v, JsonValue::Null) {
                    None
                } else {
                    Some(Rc::new(ArrayPayload))
                })
            },
        )
    }
}
// Typed source property accessors used by the declaration-model binding consumer.
impl Payload {
    pub(crate) fn name(this: &Rc<Self>) -> Option<String> {
        this.name.borrow().clone()
    }
    pub(crate) fn set_name(this: &Rc<Self>, value: Option<String>) {
        this.name.replace(value);
    }
    pub(crate) fn own(this: &Rc<Self>) -> Option<Rc<Self>> {
        this.own.borrow().clone()
    }
    pub(crate) fn set_own(this: &Rc<Self>, value: Option<Rc<Self>>) {
        this.own.replace(value);
    }
}
impl Holder {
    pub(crate) fn content(this: &Rc<Self>) -> Option<BoxedValue> {
        this.content.borrow().clone()
    }
    pub(crate) fn set_content(this: &Rc<Self>, value: Option<BoxedValue>) {
        this.content.replace(value);
    }
    pub(crate) fn other(this: &Rc<Self>) -> Option<BoxedValue> {
        this.other.borrow().clone()
    }
    pub(crate) fn set_other(this: &Rc<Self>, value: Option<BoxedValue>) {
        this.other.replace(value);
    }
    pub(crate) fn typed(this: &Rc<Self>) -> Option<Rc<Payload>> {
        this.typed.borrow().clone()
    }
    pub(crate) fn set_typed(this: &Rc<Self>, value: Option<Rc<Payload>>) {
        this.typed.replace(value);
    }
}
impl CustomHolder {
    pub(crate) fn content(this: &Rc<Self>) -> Option<BoxedValue> {
        this.content.borrow().clone()
    }
    pub(crate) fn set_content(this: &Rc<Self>, value: Option<BoxedValue>) {
        this.content.replace(value);
    }
}
impl ReadonlyHolder {
    pub(crate) fn content(this: &Rc<Self>) -> Option<BoxedValue> {
        this.content.clone()
    }
}
impl TypeNamed {
    pub(crate) fn kind(this: &Rc<Self>) -> Option<String> {
        this.kind.borrow().clone()
    }
    pub(crate) fn set_kind(this: &Rc<Self>, value: Option<String>) {
        this.kind.replace(value);
    }
    pub(crate) fn name(this: &Rc<Self>) -> Option<String> {
        this.name.borrow().clone()
    }
    pub(crate) fn set_name(this: &Rc<Self>, value: Option<String>) {
        this.name.replace(value);
    }
}
impl WriteOnlyHolder {
    pub(crate) fn set_content(this: &Rc<Self>, value: Option<BoxedValue>) {
        this.content.replace(value);
    }
    pub(crate) fn set_number(this: &Rc<Self>, value: i32) {
        this.number.set(value);
    }
}
/// Port of the source application's explicitly selected property converter.
pub struct StringConverter;
/// Port of the source application's explicitly selected type converter.
pub struct ArrayConverter;

impl<T: Clone> Generic<T> {
    pub(crate) fn value(this: &Rc<Self>) -> T {
        this.value.borrow().clone()
    }
    pub(crate) fn set_value(this: &Rc<Self>, value: T) {
        this.value.replace(value);
    }
}

// Fallible native declaration protocol over the same original fixture accessors.
impl TypeNamed {
    pub(crate) fn try_kind(this: &Rc<Self>) -> SerializationResult<Option<String>> {
        Ok(Self::kind(this))
    }
}
impl Payload {
    pub(crate) fn try_name(this: &Rc<Self>) -> SerializationResult<Option<String>> {
        Ok(Self::name(this))
    }
}
impl Holder {
    pub(crate) fn try_content(this: &Rc<Self>) -> SerializationResult<Option<BoxedValue>> {
        Ok(Self::content(this))
    }
}
impl CustomHolder {
    pub(crate) fn try_content(this: &Rc<Self>) -> SerializationResult<Option<BoxedValue>> {
        Ok(Self::content(this))
    }
}
impl ReadonlyHolder {
    pub(crate) fn try_content(this: &Rc<Self>) -> Result<Option<BoxedValue>, std::io::Error> {
        Ok(Self::content(this))
    }
}
impl Payload {
    pub(crate) fn try_set_name(this: &Rc<Self>, value: Option<String>) -> SerializationResult<()> {
        Self::set_name(this, value);
        Ok(())
    }
}
impl Payload {
    pub(crate) fn try_set_own(
        this: &Rc<Self>,
        value: Option<Rc<Payload>>,
    ) -> SerializationResult<()> {
        Self::set_own(this, value);
        Ok(())
    }
}
impl CustomHolder {
    pub(crate) fn try_set_content(
        this: &Rc<Self>,
        value: Option<BoxedValue>,
    ) -> SerializationResult<()> {
        Self::set_content(this, value);
        Ok(())
    }
}
impl WriteOnlyHolder {
    pub(crate) fn try_set_content(
        this: &Rc<Self>,
        value: Option<BoxedValue>,
    ) -> SerializationResult<()> {
        Self::set_content(this, value);
        Ok(())
    }
}
impl WriteOnlyHolder {
    pub(crate) fn try_set_number(this: &Rc<Self>, value: i32) -> SerializationResult<()> {
        Self::set_number(this, value);
        Ok(())
    }
}
