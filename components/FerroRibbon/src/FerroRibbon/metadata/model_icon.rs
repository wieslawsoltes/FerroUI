//! Native descriptors for Core Model/RibbonIcon.cs. Copyright Wiesław Šoltés, MIT.
use ferroribbon_core::collections::RibbonFixedItems;
use ferroribbon_core::{
    RibbonString,
    model::{RibbonIcon, RibbonIconKind, RibbonIconLayer},
};
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupType, MarkupTyped},
};
use std::rc::Rc;

fn string(value: Option<RibbonString>) -> Result<Option<String>, String> {
    value
        .map(|s| s.to_utf8().map_err(|e| e.to_string()))
        .transpose()
}
pub struct RibbonIconMetadata;
ferroui_base::ferro_markup_type!(class RibbonIconMetadata as "RibbonIcon" {
    namespace: "FerroRibbon.Model", this: Rc<RibbonIcon>,
    handles: [Rc<RibbonIcon>, Option<Rc<RibbonIcon>>, RibbonIcon],
    constructors: [
        (RibbonIconKind, Option<String>) => |kind, value: Option<String>| RibbonIcon::new(kind, value.map(Into::into)),
        (RibbonIconKind, Option<String>, Option<String>) => |kind, value: Option<String>, font: Option<String>| RibbonIcon::new_with_options(kind, value.map(Into::into), font.map(Into::into), None, 24.),
        (RibbonIconKind, Option<String>, Option<String>, Option<String>) => |kind, value: Option<String>, font: Option<String>, color: Option<String>| RibbonIcon::new_with_options(kind, value.map(Into::into), font.map(Into::into), color.map(Into::into), 24.),
        (RibbonIconKind, Option<String>, Option<String>, Option<String>, f64) => |kind, value: Option<String>, font: Option<String>, color: Option<String>, size| RibbonIcon::new_with_options(kind, value.map(Into::into), font.map(Into::into), color.map(Into::into), size)
    ],
    properties: [
        Kind: RibbonIconKind {get: |v: &Rc<RibbonIcon>| v.kind},
        Value: Option<String> {try_get: |v: &Rc<RibbonIcon>| string(v.value.clone())},
        FontFamily: Option<String> {try_get: |v: &Rc<RibbonIcon>| string(v.font_family.clone())},
        Foreground: Option<String> {try_get: |v: &Rc<RibbonIcon>| string(v.foreground.clone())},
        ViewBoxSize: f64 {get: |v: &Rc<RibbonIcon>| v.view_box_size}
    ],
    methods: [
        static fn op_Implicit(Option<String>) -> Rc<RibbonIcon> => |value: Option<String>| RibbonIcon::glyph(value.map(Into::into)),
        static try fn Layers(f64) -> Rc<RibbonIcon> => |size| RibbonIcon::layers(size, Some(&RibbonFixedItems::new(Vec::new()))),
        static try fn Layers(f64, Option<NativeIconLayerArray>) -> Rc<RibbonIcon> => |size, layers: Option<NativeIconLayerArray>| RibbonIcon::layers(size, layers.as_ref()),
        static fn Glyph(Option<String>) -> Rc<RibbonIcon> => |value: Option<String>| RibbonIcon::glyph(value.map(Into::into)),
        static fn Glyph(Option<String>, Option<String>) -> Rc<RibbonIcon> => |value: Option<String>, font: Option<String>| RibbonIcon::glyph_with_options(value.map(Into::into), font.map(Into::into), None),
        static fn Glyph(Option<String>, Option<String>, Option<String>) -> Rc<RibbonIcon> => |value: Option<String>, font: Option<String>, color: Option<String>| RibbonIcon::glyph_with_options(value.map(Into::into), font.map(Into::into), color.map(Into::into)),
        static fn Path(Option<String>) -> Rc<RibbonIcon> => |data: Option<String>| RibbonIcon::path(data.map(Into::into)),
        static fn Path(Option<String>, f64) -> Rc<RibbonIcon> => |data: Option<String>, size| RibbonIcon::path_with_options(data.map(Into::into), size, None),
        static fn Path(Option<String>, f64, Option<String>) -> Rc<RibbonIcon> => |data: Option<String>, size, color: Option<String>| RibbonIcon::path_with_options(data.map(Into::into), size, color.map(Into::into)),
        static fn Stroke(Option<String>) -> Rc<RibbonIcon> => |data: Option<String>| RibbonIcon::stroke(data.map(Into::into)),
        static fn Stroke(Option<String>, f64) -> Rc<RibbonIcon> => |data: Option<String>, thickness| RibbonIcon::stroke_with_options(data.map(Into::into), thickness, 24., None),
        static fn Stroke(Option<String>, f64, f64) -> Rc<RibbonIcon> => |data: Option<String>, thickness, size| RibbonIcon::stroke_with_options(data.map(Into::into), thickness, size, None),
        static fn Stroke(Option<String>, f64, f64, Option<String>) -> Rc<RibbonIcon> => |data: Option<String>, thickness, size, color: Option<String>| RibbonIcon::stroke_with_options(data.map(Into::into), thickness, size, color.map(Into::into)),
        static fn Image(Option<String>) -> Rc<RibbonIcon> => |uri: Option<String>| RibbonIcon::image(uri.map(Into::into)),
        static fn Text(Option<String>) -> Rc<RibbonIcon> => |value: Option<String>| RibbonIcon::text(value.map(Into::into)),
        static fn Text(Option<String>, Option<String>) -> Rc<RibbonIcon> => |value: Option<String>, color: Option<String>| RibbonIcon::text_with_foreground(value.map(Into::into), color.map(Into::into)),
        fn WithForeground(Option<String>) -> Rc<RibbonIcon> => |v: &Rc<RibbonIcon>, color: Option<String>| v.with_foreground(color.map(Into::into))
    ]
});

pub struct RibbonIconLayerMetadata;
ferroui_base::ferro_markup_type!(class RibbonIconLayerMetadata as "RibbonIconLayer" {
    namespace: "FerroRibbon.Model", this: Rc<RibbonIconLayer>,
    handles: [Rc<RibbonIconLayer>, Option<Rc<RibbonIconLayer>>, RibbonIconLayer],
    constructors: [
        (Option<String>) => |data: Option<String>| RibbonIconLayer::new(data.map(Into::into)),
        (Option<String>, f64) => |data: Option<String>, thickness| RibbonIconLayer::with_options(data.map(Into::into), thickness, None, 1.),
        (Option<String>, f64, Option<String>) => |data: Option<String>, thickness, color: Option<String>| RibbonIconLayer::with_options(data.map(Into::into), thickness, color.map(Into::into), 1.),
        (Option<String>, f64, Option<String>, f64) => |data: Option<String>, thickness, color: Option<String>, opacity| RibbonIconLayer::with_options(data.map(Into::into), thickness, color.map(Into::into), opacity)
    ],
    properties: [
        Data: Option<String> {try_get: |v: &Rc<RibbonIconLayer>| string(v.data())},
        StrokeThickness: f64 {get: |v: &Rc<RibbonIconLayer>| v.stroke_thickness()},
        Color: Option<String> {try_get: |v: &Rc<RibbonIconLayer>| string(v.color())},
        Opacity: f64 {get: |v: &Rc<RibbonIconLayer>| v.opacity()}
    ],
    methods: [
        try fn ToString() -> Option<String> => |v: &Rc<RibbonIconLayer>| string(v.to_text()),
        static fn ParseViewBox(Option<String>) -> Option<f64> => |value: Option<String>| RibbonIconLayer::parse_view_box(value.map(RibbonString::from).as_ref()),
        static fn Parse(Option<String>) -> NativeIconLayers => |value: Option<String>| RibbonIconLayer::parse(value.map(RibbonString::from).as_ref())
    ]
});

pub type NativeIconLayers =
    ferroribbon_core::collections::RibbonReadOnlyList<Option<Rc<RibbonIconLayer>>>;
pub type NativeIconLayerArray =
    ferroribbon_core::collections::RibbonArray<Option<Rc<RibbonIconLayer>>>;
pub type NativeIconLayerListItems =
    ferroribbon_core::collections::RibbonItems<Option<Rc<RibbonIconLayer>>>;
pub type NativeIconLayerFixedItems =
    ferroribbon_core::collections::RibbonFixedItems<Option<Rc<RibbonIconLayer>>>;
pub type NativeIconLayerList =
    ferroribbon_core::collections::RibbonList<Option<Rc<RibbonIconLayer>>>;
pub struct RibbonIconLayersMetadata;
ferroui_base::ferro_markup_type!(interface RibbonIconLayersMetadata as "IReadOnlyList`1" {
    namespace: "System.Collections.Generic", this: NativeIconLayers, handles: [NativeIconLayers],
    generic: "IReadOnlyList`1" [Option<Rc<RibbonIconLayer>>],
    properties: [Count: i32 {get: |v: &NativeIconLayers| v.count()}],
    indexers: [(i32) -> Option<Rc<RibbonIconLayer>> {try_get: readonly_index}]
});
fn readonly_index(value: &NativeIconLayers, i: i32) -> Result<Option<Rc<RibbonIconLayer>>, String> {
    if let Ok(list) =
        (value.clone() as Rc<dyn std::any::Any>).downcast::<NativeIconLayerListItems>()
    {
        checked_index(i, list.len(), false).map(|i| list.get(i))
    } else {
        // Interface implementations own their index contract. Do not insert a
        // Count read or a bounds rule ahead of the source virtual getter.
        Ok(value.get(i))
    }
}
fn checked_index(i: i32, length: usize, allow_end: bool) -> Result<usize, String> {
    let i = usize::try_from(i).map_err(|_| "ArgumentOutOfRangeException: index".to_owned())?;
    if i < length || allow_end && i == length {
        Ok(i)
    } else {
        Err("ArgumentOutOfRangeException: index".to_owned())
    }
}
pub struct RibbonIconLayerListMetadata;
ferroui_base::ferro_markup_type!(class RibbonIconLayerListMetadata as "List`1" {
    namespace: "System.Collections.Generic", this: NativeIconLayerList, handles: [NativeIconLayerListItems,NativeIconLayerList],
    interfaces: [NativeIconLayers], generic: "List`1" [Option<Rc<RibbonIconLayer>>],
    constructors: [() => new_icon_layers],
    properties: [Count: i32 {get: |v: &NativeIconLayerList| i32::try_from(v.len()).expect("Int32 Count")}],
    indexers: [(i32) -> Option<Rc<RibbonIconLayer>> {try_get: |v: &NativeIconLayerList, i| checked_index(i, v.len(), false).map(|i| v.get(i)), try_set: |v: &NativeIconLayerList, i, x| checked_index(i, v.len(), false).map(|i| v.set(i, x))}],
    methods: [
        fn Add(Option<Rc<RibbonIconLayer>>) => |v: &NativeIconLayerList, x| v.add(x),
        fn Clear() => |v: &NativeIconLayerList| v.clear(),
        try fn Insert(i32, Option<Rc<RibbonIconLayer>>) => |v: &NativeIconLayerList, i, x| checked_index(i, v.len(), true).map(|i| v.insert(i, x)),
        try fn RemoveAt(i32) => |v: &NativeIconLayerList, i| checked_index(i, v.len(), false).map(|i| v.remove_at(i)),
        fn Remove(Option<Rc<RibbonIconLayer>>) -> bool => |v: &NativeIconLayerList, x| v.remove(&x)
    ]
});

pub(crate) fn register() {
    ValueTypes::register_reference::<RibbonIcon>();
    ValueTypes::register_reference::<RibbonIconLayer>();
    ValueTypes::register_nullable::<NativeIconLayerArray>();
    ValueTypes::register_reference::<NativeIconLayerListItems>();
    ValueTypes::register_nullable::<NativeIconLayerList>();
    ValueTypes::register_reference::<NativeIconLayerFixedItems>();
    ValueTypes::register_reference_view_if::<NativeIconLayers>(|v| {
        let object = v.clone() as Rc<dyn std::any::Any>;
        ValueTypes::is_reference(object.as_ref().type_id()).then_some(object)
    });
    ValueTypes::register_cast::<NativeIconLayerArray, NativeIconLayers>(|v| {
        v.clone() as NativeIconLayers
    });
    ValueTypes::register_boxed_cast::<NativeIconLayers, NativeIconLayerArray>(|v| {
        v.downcast_ref::<NativeIconLayers>().and_then(|v| {
            (v.clone() as Rc<dyn std::any::Any>)
                .downcast::<NativeIconLayerFixedItems>()
                .ok()
        })
    });
    ValueTypes::register_nullable::<NativeIconLayers>();
    ValueTypes::register_cast::<NativeIconLayerList, NativeIconLayers>(|v| {
        v.clone() as NativeIconLayers
    });
    ValueTypes::register_boxed_cast::<NativeIconLayers, NativeIconLayerList>(|v| {
        v.downcast_ref::<NativeIconLayers>().and_then(|v| {
            (v.clone() as Rc<dyn std::any::Any>)
                .downcast::<NativeIconLayerListItems>()
                .ok()
        })
    });
    MarkupType::register_all(&[
        RibbonIconMetadata::MARKUP,
        RibbonIconLayerMetadata::MARKUP,
        RibbonIconLayersMetadata::MARKUP,
        RibbonIconLayerListMetadata::MARKUP,
    ]);
}

fn new_icon_layers() -> NativeIconLayerList {
    NativeIconLayerListItems::new()
}
