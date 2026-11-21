//! Native descriptors for Core Model/RibbonScreenTip.cs. Copyright Wiesław Šoltés, MIT.
use ferroribbon_core::{
    ObservableObject, RibbonString,
    model::{RibbonIcon, RibbonScreenTip},
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

pub struct RibbonScreenTipMetadata;
ferroui_base::ferro_markup_type!(class RibbonScreenTipMetadata as "RibbonScreenTip" {
    namespace: "FerroRibbon.Model", this: Rc<RibbonScreenTip>,
    handles: [Rc<RibbonScreenTip>, Option<Rc<RibbonScreenTip>>, RibbonScreenTip],
    base: ObservableObject,
    notify_property_changed: RibbonScreenTip,
    constructors: [
        () => RibbonScreenTip::new,
        (Option<String>) => |title: Option<String>| RibbonScreenTip::with_title(title.map(Into::into)),
        (Option<String>, Option<String>) => |title: Option<String>, description: Option<String>| RibbonScreenTip::with_description(title.map(Into::into), description.map(Into::into)),
        (Option<String>, Option<String>, Option<String>) => |title: Option<String>, description: Option<String>, shortcut: Option<String>| RibbonScreenTip::with_text(title.map(Into::into), description.map(Into::into), shortcut.map(Into::into))
    ],
    properties: [
        Title: Option<String> {try_get: |v: &Rc<RibbonScreenTip>| string(v.title()), set: |v: &Rc<RibbonScreenTip>, x: Option<String>| v.set_title(x.map(Into::into))},
        Description: Option<String> {try_get: |v: &Rc<RibbonScreenTip>| string(v.description()), set: |v: &Rc<RibbonScreenTip>, x: Option<String>| v.set_description(x.map(Into::into))},
        Shortcut: Option<String> {try_get: |v: &Rc<RibbonScreenTip>| string(v.shortcut()), set: |v: &Rc<RibbonScreenTip>, x: Option<String>| v.set_shortcut(x.map(Into::into))},
        HelpText: Option<String> {try_get: |v: &Rc<RibbonScreenTip>| string(v.help_text()), set: |v: &Rc<RibbonScreenTip>, x: Option<String>| v.set_help_text(x.map(Into::into))},
        DisabledReason: Option<String> {try_get: |v: &Rc<RibbonScreenTip>| string(v.disabled_reason()), set: |v: &Rc<RibbonScreenTip>, x: Option<String>| v.set_disabled_reason(x.map(Into::into))},
        Image: Option<Rc<RibbonIcon>> {get: |v: &Rc<RibbonScreenTip>| v.image(), set: |v: &Rc<RibbonScreenTip>, x| v.set_image(x)},
        ExtendedDescription: Option<String> {try_get: |v: &Rc<RibbonScreenTip>| string(v.extended_description()), set: |v: &Rc<RibbonScreenTip>, x: Option<String>| v.set_extended_description(x.map(Into::into))},
        ExtendedImage: Option<Rc<RibbonIcon>> {get: |v: &Rc<RibbonScreenTip>| v.extended_image(), set: |v: &Rc<RibbonScreenTip>, x| v.set_extended_image(x)}
    ],
    methods: [static fn FormatTitle(Option<String>, Option<String>) -> String => |title: Option<String>, shortcut: Option<String>| {
        RibbonScreenTip::format_title(title.map(RibbonString::from).as_ref(), shortcut.map(RibbonString::from).as_ref()).to_utf8().expect("native UTF-8 inputs")
    }]
});

pub(crate) fn register() {
    ValueTypes::register_reference::<RibbonScreenTip>();
    MarkupType::register_all(&[RibbonScreenTipMetadata::MARKUP]);
}
