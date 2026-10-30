//! Build-only declaration projections of the original application payload fixtures.
//! The shared scanner reads these declarations. The JSON emitter compiles their
//! selected typed paths against payload_models.rs; no markup runtime is linked.
#[path = "payload_models.rs"]
pub mod payload_models;
use payload_models::{Payload, Holder, CustomHolder, ReadonlyHolder, TypeNamed, WriteOnlyHolder, ArrayPayload, Generic, StringConverter, ArrayConverter};
use ferrodock_model::BoxedValue;
use std::rc::Rc;
use ferrodock_serializer_system_text_json::{
    dock_json_source_generation as generate_json,
    dock_json_serializable as register_json,
};
ferro_markup_type!(class Payload as "GeneratedPayload" {
    this: Rc<Payload>, handles: [Rc<Payload>], constructors: [() => Payload::new],
    properties: [
        Name: Option<String> {try_get: Payload::try_name, try_set: Payload::try_set_name},
        Self: Option<Rc<Payload>> {get: Payload::own, try_set: Payload::try_set_own},
    ],
});
ferro_markup_type!(class Holder as "GeneratedHolder" {
    this: Rc<Holder>, handles: [Rc<Holder>], constructors: [() => Holder::new],
    properties: [
        Content: Option<BoxedValue> {try_get: Holder::try_content, set: Holder::set_content},
        Other: Option<BoxedValue> {get: Holder::other, set: Holder::set_other},
        Typed: Option<Rc<Payload>> {get: Holder::typed, set: Holder::set_typed},
    ],
});
ferro_markup_type!(class CustomHolder as "GeneratedCustomHolder" {
    this: Rc<CustomHolder>, handles: [Rc<CustomHolder>], constructors: [() => CustomHolder::new],
    properties: [Content: Option<BoxedValue> {try_get: CustomHolder::try_content, try_set: CustomHolder::try_set_content} [JsonConverter(type(StringConverter))]],
});
ferro_markup_type!(class ReadonlyHolder as "GeneratedReadonlyHolder" {
    this: Rc<ReadonlyHolder>, handles: [Rc<ReadonlyHolder>], constructors: [() => ReadonlyHolder::new],
    properties: [Content: Option<BoxedValue> {try_get: ReadonlyHolder::try_content}],
});
ferro_markup_type!(class TypeNamed as "GeneratedTypeNamed" {
    this: Rc<TypeNamed>, handles: [Rc<TypeNamed>], constructors: [() => TypeNamed::new],
    properties: [
        Kind: Option<String> {try_get: TypeNamed::try_kind, set: TypeNamed::set_kind} [JsonPropertyName("$type")],
        Name: Option<String> {try_get: TypeNamed::try_name, set: TypeNamed::set_name},
    ],
});
ferro_markup_type!(class WriteOnlyHolder as "GeneratedWriteOnlyHolder" {
    this: Rc<WriteOnlyHolder>, handles: [Rc<WriteOnlyHolder>], constructors: [() => WriteOnlyHolder::new],
    properties: [
        Content: Option<BoxedValue> {try_set: WriteOnlyHolder::try_set_content},
        Number: i32 {try_set: WriteOnlyHolder::try_set_number},
    ],
});
ferro_markup_type!(class ArrayPayload as "GeneratedArrayPayload" {
    this: Rc<ArrayPayload>, handles: [Rc<ArrayPayload>], attributes: [JsonConverter(type(ArrayConverter))],
});

ferro_markup_type!(class Generic<T> as "GeneratedGeneric`1" {
    this: Rc<Generic<T>>, handles: [Rc<Generic<T>>],
    generic: "GeneratedGeneric`1" [T], constructors: [() => Generic::new],
    properties: [Value: T {get: Generic::value, set: Generic::set_value}],
});
