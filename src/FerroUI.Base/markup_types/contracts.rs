//! Markup metadata of the contracts of this crate that are types of
//! members markup can set: each is published under the name of the
//! interface of the managed original, with the handle types the members of
//! this port use for it.
//!
//! Handles compare by identity.

use crate::data::core::ValueTypes;
use crate::ferro_markup_type;
use crate::metadata::{MarkupType, MarkupTyped};
use std::rc::Rc;

ferro_markup_type!(interface dyn crate::media::IBrush as "IBrush" {
    namespace: "FerroUI.Media",
    handles: [Rc<dyn crate::media::IBrush>, Option<Rc<dyn crate::media::IBrush>>],
    this: Rc<dyn crate::media::IBrush>,
    parse: crate::media::Brush::parse,
    properties: [
        Opacity: f64 { get: |brush: &Rc<dyn crate::media::IBrush>| brush.opacity() },
        Transform: Option<Rc<dyn crate::media::ITransform>> {
            get: |brush: &Rc<dyn crate::media::IBrush>| brush.transform()
        },
        TransformOrigin: crate::RelativePoint { get: |brush: &Rc<dyn crate::media::IBrush>| brush.transform_origin() },
    ],
});

// The brush contracts that are the declared types of static values (`Brushes`):
// a value of one of them is a brush, through the casts registered below.

impl PartialEq for dyn crate::media::ISolidColorBrush {
    fn eq(&self, other: &Self) -> bool {
        crate::media::IBrush::equals(self, other as &dyn crate::media::IBrush)
    }
}

impl PartialEq for dyn crate::media::IImmutableBrush {
    fn eq(&self, other: &Self) -> bool {
        crate::media::IBrush::equals(self, other as &dyn crate::media::IBrush)
    }
}

impl PartialEq for dyn crate::media::IImmutableSolidColorBrush {
    fn eq(&self, other: &Self) -> bool {
        crate::media::IBrush::equals(self, other as &dyn crate::media::IBrush)
    }
}

ferro_markup_type!(interface dyn crate::media::ISolidColorBrush as "ISolidColorBrush" {
    namespace: "FerroUI.Media",
    handles: [Rc<dyn crate::media::ISolidColorBrush>, Option<Rc<dyn crate::media::ISolidColorBrush>>],
    this: Rc<dyn crate::media::ISolidColorBrush>,
    interfaces: [Rc<dyn crate::media::IBrush>],
    properties: [
        Color: crate::media::Color { get: |brush: &Rc<dyn crate::media::ISolidColorBrush>| brush.color() },
    ],
});

ferro_markup_type!(interface dyn crate::media::IImmutableBrush as "IImmutableBrush" {
    namespace: "FerroUI.Media",
    handles: [Rc<dyn crate::media::IImmutableBrush>, Option<Rc<dyn crate::media::IImmutableBrush>>],
    interfaces: [Rc<dyn crate::media::IBrush>],
});

ferro_markup_type!(interface dyn crate::media::IImmutableSolidColorBrush as "IImmutableSolidColorBrush" {
    namespace: "FerroUI.Media",
    handles: [
        Rc<dyn crate::media::IImmutableSolidColorBrush>,
        Option<Rc<dyn crate::media::IImmutableSolidColorBrush>>,
    ],
    interfaces: [
        Rc<dyn crate::media::ISolidColorBrush>,
        Rc<dyn crate::media::IImmutableBrush>,
        Rc<dyn crate::media::IBrush>,
    ],
});

ferro_markup_type!(interface dyn crate::media::IImage as "IImage" {
    namespace: "FerroUI.Media",
    handles: [Rc<dyn crate::media::IImage>, Option<Rc<dyn crate::media::IImage>>],
});

ferro_markup_type!(interface dyn crate::media::IImageBrushSource as "IImageBrushSource" {
    namespace: "FerroUI.Media",
    handles: [Rc<dyn crate::media::IImageBrushSource>, Option<Rc<dyn crate::media::IImageBrushSource>>],
});

ferro_markup_type!(interface dyn crate::media::ITransform as "ITransform" {
    namespace: "FerroUI.Media",
    handles: [Rc<dyn crate::media::ITransform>, Option<Rc<dyn crate::media::ITransform>>],
    // The transform converter of the managed original: a list of transform operations.
    parse: |s: &str| {
        crate::media::transformation::TransformOperations::parse(s)
            .map(|operations| -> Rc<dyn crate::media::ITransform> { operations })
    },
});

ferro_markup_type!(interface dyn crate::media::IPen as "IPen" {
    namespace: "FerroUI.Media",
    handles: [Rc<dyn crate::media::IPen>, Option<Rc<dyn crate::media::IPen>>],
});

ferro_markup_type!(interface dyn crate::media::IDashStyle as "IDashStyle" {
    namespace: "FerroUI.Media",
    handles: [Rc<dyn crate::media::IDashStyle>, Option<Rc<dyn crate::media::IDashStyle>>],
});

ferro_markup_type!(interface dyn crate::media::IExperimentalAcrylicMaterial as "IExperimentalAcrylicMaterial" {
    namespace: "FerroUI.Media",
    handles: [Rc<dyn crate::media::IExperimentalAcrylicMaterial>, Option<Rc<dyn crate::media::IExperimentalAcrylicMaterial>>],
});

ferro_markup_type!(interface dyn crate::media::effects::IEffect as "IEffect" {
    namespace: "FerroUI.Media",
    handles: [Rc<dyn crate::media::effects::IEffect>, Option<Rc<dyn crate::media::effects::IEffect>>],
    parse: crate::media::effects::Effect::parse,
});

ferro_markup_type!(interface dyn crate::styling::IStyle as "IStyle" {
    namespace: "FerroUI.Styling",
    handles: [Rc<dyn crate::styling::IStyle>, Option<Rc<dyn crate::styling::IStyle>>],
});

ferro_markup_type!(interface dyn crate::styling::IStyleHost as "IStyleHost" {
    namespace: "FerroUI.Styling",
    handles: [Rc<dyn crate::styling::IStyleHost>, Option<Rc<dyn crate::styling::IStyleHost>>],
});

ferro_markup_type!(interface dyn crate::styling::ITemplate as "ITemplate" {
    namespace: "FerroUI.Styling",
    handles: [Rc<dyn crate::styling::ITemplate>, Option<Rc<dyn crate::styling::ITemplate>>],
});

ferro_markup_type!(interface dyn crate::controls::IResourceNode as "IResourceNode" {
    namespace: "FerroUI.Controls",
    handles: [Rc<dyn crate::controls::IResourceNode>, Option<Rc<dyn crate::controls::IResourceNode>>],
});

ferro_markup_type!(interface dyn crate::controls::IResourceHost as "IResourceHost" {
    namespace: "FerroUI.Controls",
    handles: [Rc<dyn crate::controls::IResourceHost>, Option<Rc<dyn crate::controls::IResourceHost>>],
});

ferro_markup_type!(interface dyn crate::controls::IResourceProvider as "IResourceProvider" {
    namespace: "FerroUI.Controls",
    handles: [Rc<dyn crate::controls::IResourceProvider>, Option<Rc<dyn crate::controls::IResourceProvider>>],
});

/// The resource dictionary object behind the contract: the collections of a
/// dictionary are reached through the class.
fn resource_dictionary(
    dictionary: &Rc<dyn crate::controls::IResourceDictionary>,
) -> Result<crate::Ref<crate::controls::ResourceDictionary>, String> {
    dictionary
        .as_object()
        .and_then(|object| object.to_ref().cast::<crate::controls::ResourceDictionary>())
        .ok_or_else(|| "The resource dictionary is not a ResourceDictionary.".to_string())
}

ferro_markup_type!(interface dyn crate::controls::IResourceDictionary as "IResourceDictionary" {
    namespace: "FerroUI.Controls",
    handles: [Rc<dyn crate::controls::IResourceDictionary>, Option<Rc<dyn crate::controls::IResourceDictionary>>],
    this: Rc<dyn crate::controls::IResourceDictionary>,
    interfaces: [Rc<dyn crate::controls::IResourceProvider>],
    properties: [
        Count: i32 { get: |dictionary: &Rc<dyn crate::controls::IResourceDictionary>| dictionary.count() as i32 },
        MergedDictionaries: crate::collections::FerroList<Rc<dyn crate::controls::IResourceProvider>> {
            try_get: |dictionary: &Rc<dyn crate::controls::IResourceDictionary>| {
                resource_dictionary(dictionary).map(|dictionary| dictionary.merged_dictionaries())
            }
        },
        ThemeDictionaries: crate::collections::FerroDictionary<
            crate::styling::ThemeVariant,
            Rc<dyn crate::controls::IThemeVariantProvider>,
        > {
            try_get: |dictionary: &Rc<dyn crate::controls::IResourceDictionary>| {
                resource_dictionary(dictionary).map(|dictionary| dictionary.theme_dictionaries())
            }
        },
    ],
    methods: [
        // `IDictionary<object, object?>.Add`: adding a key twice is an error.
        try fn Add(Option<crate::BoxedValue>, Option<crate::BoxedValue>) =>
            |dictionary: &Rc<dyn crate::controls::IResourceDictionary>,
             key: Option<crate::BoxedValue>,
             value: Option<crate::BoxedValue>| {
                let key = super::plain::resource_key(key)?;
                if dictionary.contains_key(&key) {
                    return Err(format!("An item with the same key has already been added. Key: {key}"));
                }
                Ok::<(), String>(dictionary.add(key, value))
            },
    ],
});

ferro_markup_type!(interface dyn crate::controls::IThemeVariantProvider as "IThemeVariantProvider" {
    namespace: "FerroUI.Controls",
    handles: [Rc<dyn crate::controls::IThemeVariantProvider>, Option<Rc<dyn crate::controls::IThemeVariantProvider>>],
    this: Rc<dyn crate::controls::IThemeVariantProvider>,
    interfaces: [Rc<dyn crate::controls::IResourceProvider>],
    properties: [
        // The key of the provider in the theme dictionaries it is an entry of.
        Key: Option<crate::styling::ThemeVariant> {
            get: |provider: &Rc<dyn crate::controls::IThemeVariantProvider>| provider.key(),
            set: |provider: &Rc<dyn crate::controls::IThemeVariantProvider>, value: Option<crate::styling::ThemeVariant>| {
                provider.set_key(value)
            }
        },
    ],
});

ferro_markup_type!(interface dyn crate::controls::IDeferredContent as "IDeferredContent" {
    namespace: "FerroUI.Controls",
    handles: [Rc<dyn crate::controls::IDeferredContent>, Option<Rc<dyn crate::controls::IDeferredContent>>],
});

ferro_markup_type!(interface dyn crate::input::ICommand as "ICommand" {
    namespace: "System.Windows.Input",
    handles: [Rc<dyn crate::input::ICommand>, Option<Rc<dyn crate::input::ICommand>>],
});

ferro_markup_type!(interface dyn crate::data::converters::IValueConverter as "IValueConverter" {
    namespace: "FerroUI.Data.Converters",
    handles: [Rc<dyn crate::data::converters::IValueConverter>, Option<Rc<dyn crate::data::converters::IValueConverter>>],
});

ferro_markup_type!(interface dyn crate::data::converters::IMultiValueConverter as "IMultiValueConverter" {
    namespace: "FerroUI.Data.Converters",
    handles: [Rc<dyn crate::data::converters::IMultiValueConverter>, Option<Rc<dyn crate::data::converters::IMultiValueConverter>>],
});

ferro_markup_type!(interface dyn crate::animation::easings::IEasing as "IEasing" {
    namespace: "FerroUI.Animation.Easings",
    handles: [Rc<dyn crate::animation::easings::IEasing>, Option<Rc<dyn crate::animation::easings::IEasing>>],
});

ferro_markup_type!(interface dyn crate::animation::ITransition as "ITransition" {
    namespace: "FerroUI.Animation",
    handles: [Rc<dyn crate::animation::ITransition>, Option<Rc<dyn crate::animation::ITransition>>],
});

ferro_markup_type!(interface dyn crate::animation::IAnimation as "IAnimation" {
    namespace: "FerroUI.Animation",
    handles: [Rc<dyn crate::animation::IAnimation>, Option<Rc<dyn crate::animation::IAnimation>>],
});

ferro_markup_type!(interface dyn crate::animation::IAnimationSetter as "IAnimationSetter" {
    namespace: "FerroUI.Animation",
    handles: [Rc<dyn crate::animation::IAnimationSetter>, Option<Rc<dyn crate::animation::IAnimationSetter>>],
});

ferro_markup_type!(interface dyn crate::animation::IPageTransition as "IPageTransition" {
    namespace: "FerroUI.Animation",
    handles: [Rc<dyn crate::animation::IPageTransition>, Option<Rc<dyn crate::animation::IPageTransition>>],
});

ferro_markup_type!(interface dyn crate::animation::IClock as "IClock" {
    namespace: "FerroUI.Animation",
    handles: [Rc<dyn crate::animation::IClock>, Option<Rc<dyn crate::animation::IClock>>],
});

ferro_markup_type!(interface dyn crate::metadata::IServiceProvider as "IServiceProvider" {
    namespace: "System",
    handles: [Rc<dyn crate::metadata::IServiceProvider>, Option<Rc<dyn crate::metadata::IServiceProvider>>],
});

ferro_markup_type!(interface dyn crate::INamed as "INamed" {
    namespace: "FerroUI",
    handles: [Rc<dyn crate::INamed>, Option<Rc<dyn crate::INamed>>],
    this: Rc<dyn crate::INamed>,
    properties: [
        Name: Option<String> { get: |named: &Rc<dyn crate::INamed>| named.name() },
    ],
});

// A name scope is held through `NameScopeRef` (a handle that compares by
// identity) where it is a property value.
ferro_markup_type!(interface dyn crate::controls::INameScope as "INameScope" {
    this: crate::controls::NameScopeRef,
    namespace: "FerroUI.Controls",
    handles: [
        crate::controls::NameScopeRef,
        Option<crate::controls::NameScopeRef>,
        Rc<dyn crate::controls::INameScope>,
        Option<Rc<dyn crate::controls::INameScope>>,
    ],
    properties: [
        IsCompleted: bool { get: |scope: &crate::controls::NameScopeRef| scope.is_completed() },
    ],
    methods: [
        // The element is any object in the managed original; the port registers objects of the object model.
        try fn Register(String, Option<crate::BoxedValue>) =>
            |scope: &crate::controls::NameScopeRef, name: String, element: Option<crate::BoxedValue>| {
                match element.as_ref().and_then(|element| ValueTypes::as_object(&**element)) {
                    Some(element) => scope.try_register(&name, element).map_err(|error| error.to_string()),
                    None => Err("Only an object of the object model can be registered in a name scope.".to_string()),
                }
            },
        fn Find(String) -> Option<crate::Ref<crate::FerroObject>> =>
            |scope: &crate::controls::NameScopeRef, name: String| scope.find(&name),
        fn Complete() => |scope: &crate::controls::NameScopeRef| scope.complete(),
    ],
});

// The abstract base classes of the managed original that this port
// declares as traits.

ferro_markup_type!(class dyn crate::styling::SetterBase as "SetterBase" {
    namespace: "FerroUI.Styling",
    handles: [Rc<dyn crate::styling::SetterBase>, Option<Rc<dyn crate::styling::SetterBase>>],
});

ferro_markup_type!(class dyn crate::data::BindingBase as "BindingBase" {
    namespace: "FerroUI.Data",
    handles: [Rc<dyn crate::data::BindingBase>, Option<Rc<dyn crate::data::BindingBase>>],
});

ferro_markup_type!(class dyn crate::media::TextTrimming as "TextTrimming" {
    namespace: "FerroUI.Media",
    handles: [Rc<dyn crate::media::TextTrimming>, Option<Rc<dyn crate::media::TextTrimming>>],
    parse: <dyn crate::media::TextTrimming>::parse,
    static_properties: [
        None: Rc<dyn crate::media::TextTrimming> { get: <dyn crate::media::TextTrimming>::none },
        CharacterEllipsis: Rc<dyn crate::media::TextTrimming> { get: <dyn crate::media::TextTrimming>::character_ellipsis },
        WordEllipsis: Rc<dyn crate::media::TextTrimming> { get: <dyn crate::media::TextTrimming>::word_ellipsis },
        PrefixCharacterEllipsis: Rc<dyn crate::media::TextTrimming> { get: <dyn crate::media::TextTrimming>::prefix_character_ellipsis },
        LeadingCharacterEllipsis: Rc<dyn crate::media::TextTrimming> { get: <dyn crate::media::TextTrimming>::leading_character_ellipsis },
        PathSegmentEllipsis: Rc<dyn crate::media::TextTrimming> { get: <dyn crate::media::TextTrimming>::path_segment_ellipsis },
    ],
});

/// The types declared in this file.
pub(super) const TYPES: &[&MarkupType] = &[
    <dyn crate::INamed as MarkupTyped>::MARKUP,
    <dyn crate::media::IBrush as MarkupTyped>::MARKUP,
    <dyn crate::media::ISolidColorBrush as MarkupTyped>::MARKUP,
    <dyn crate::media::IImmutableBrush as MarkupTyped>::MARKUP,
    <dyn crate::media::IImmutableSolidColorBrush as MarkupTyped>::MARKUP,
    <dyn crate::media::IImage as MarkupTyped>::MARKUP,
    <dyn crate::media::IImageBrushSource as MarkupTyped>::MARKUP,
    <dyn crate::media::ITransform as MarkupTyped>::MARKUP,
    <dyn crate::media::IPen as MarkupTyped>::MARKUP,
    <dyn crate::media::IDashStyle as MarkupTyped>::MARKUP,
    <dyn crate::media::IExperimentalAcrylicMaterial as MarkupTyped>::MARKUP,
    <dyn crate::media::effects::IEffect as MarkupTyped>::MARKUP,
    <dyn crate::styling::IStyle as MarkupTyped>::MARKUP,
    <dyn crate::styling::IStyleHost as MarkupTyped>::MARKUP,
    <dyn crate::styling::ITemplate as MarkupTyped>::MARKUP,
    <dyn crate::controls::IResourceNode as MarkupTyped>::MARKUP,
    <dyn crate::controls::IResourceHost as MarkupTyped>::MARKUP,
    <dyn crate::controls::IResourceProvider as MarkupTyped>::MARKUP,
    <dyn crate::controls::IResourceDictionary as MarkupTyped>::MARKUP,
    <dyn crate::controls::IThemeVariantProvider as MarkupTyped>::MARKUP,
    <dyn crate::controls::IDeferredContent as MarkupTyped>::MARKUP,
    <dyn crate::input::ICommand as MarkupTyped>::MARKUP,
    <dyn crate::data::converters::IValueConverter as MarkupTyped>::MARKUP,
    <dyn crate::data::converters::IMultiValueConverter as MarkupTyped>::MARKUP,
    <dyn crate::animation::easings::IEasing as MarkupTyped>::MARKUP,
    <dyn crate::animation::ITransition as MarkupTyped>::MARKUP,
    <dyn crate::animation::IAnimation as MarkupTyped>::MARKUP,
    <dyn crate::animation::IAnimationSetter as MarkupTyped>::MARKUP,
    <dyn crate::animation::IPageTransition as MarkupTyped>::MARKUP,
    <dyn crate::animation::IClock as MarkupTyped>::MARKUP,
    <dyn crate::metadata::IServiceProvider as MarkupTyped>::MARKUP,
    <dyn crate::controls::INameScope as MarkupTyped>::MARKUP,
    <dyn crate::styling::SetterBase as MarkupTyped>::MARKUP,
    <dyn crate::data::BindingBase as MarkupTyped>::MARKUP,
    <dyn crate::media::TextTrimming as MarkupTyped>::MARKUP,
];

/// Registers the nullable forms of the contract handles that can be held in
/// untyped values with the untyped value conversions of the current thread.
/// (The handles of contracts implemented by classes are also registered by
/// those classes when they are initialised.)
pub(super) fn register_value_types() {
    ValueTypes::register_nullable::<Rc<dyn crate::media::IBrush>>();
    ValueTypes::register_nullable::<Rc<dyn crate::media::ISolidColorBrush>>();
    ValueTypes::register_nullable::<Rc<dyn crate::media::IImmutableBrush>>();
    ValueTypes::register_nullable::<Rc<dyn crate::media::IImmutableSolidColorBrush>>();
    // A theme variant provider is a resource provider.
    ValueTypes::register_cast::<Rc<dyn crate::controls::IThemeVariantProvider>, Rc<dyn crate::controls::IResourceProvider>>(
        |provider| provider.clone(),
    );
    // The brush contracts derive from each other.
    ValueTypes::register_cast::<Rc<dyn crate::media::ISolidColorBrush>, Rc<dyn crate::media::IBrush>>(|b| b.clone());
    ValueTypes::register_cast::<Rc<dyn crate::media::IImmutableBrush>, Rc<dyn crate::media::IBrush>>(|b| b.clone());
    ValueTypes::register_cast::<Rc<dyn crate::media::IImmutableSolidColorBrush>, Rc<dyn crate::media::IBrush>>(|b| b.clone());
    ValueTypes::register_cast::<Rc<dyn crate::media::IImmutableSolidColorBrush>, Rc<dyn crate::media::ISolidColorBrush>>(
        |b| b.clone(),
    );
    ValueTypes::register_cast::<Rc<dyn crate::media::IImmutableSolidColorBrush>, Rc<dyn crate::media::IImmutableBrush>>(
        |b| b.clone(),
    );
    ValueTypes::register_nullable::<Rc<dyn crate::media::IImage>>();
    ValueTypes::register_nullable::<Rc<dyn crate::media::IImageBrushSource>>();
    ValueTypes::register_nullable::<Rc<dyn crate::media::ITransform>>();
    ValueTypes::register_nullable::<Rc<dyn crate::media::IPen>>();
    ValueTypes::register_nullable::<Rc<dyn crate::media::IDashStyle>>();
    ValueTypes::register_nullable::<Rc<dyn crate::media::IExperimentalAcrylicMaterial>>();
    ValueTypes::register_nullable::<Rc<dyn crate::media::effects::IEffect>>();
    ValueTypes::register_nullable::<Rc<dyn crate::input::ICommand>>();
    ValueTypes::register_nullable::<Rc<dyn crate::animation::ITransition>>();
    ValueTypes::register_nullable::<Rc<dyn crate::animation::IPageTransition>>();
    ValueTypes::register_nullable::<Rc<dyn crate::animation::IClock>>();
    ValueTypes::register_nullable::<Rc<dyn crate::media::TextTrimming>>();
    ValueTypes::register_nullable::<crate::controls::NameScopeRef>();
    ValueTypes::register_nullable::<Rc<dyn crate::INamed>>();
    ValueTypes::register_nullable::<Rc<dyn crate::styling::IStyle>>();
    ValueTypes::register_nullable::<Rc<dyn crate::styling::SetterBase>>();
    ValueTypes::register_nullable::<Rc<dyn crate::styling::ITemplate>>();
    ValueTypes::register_nullable::<Rc<dyn crate::controls::IResourceProvider>>();
    ValueTypes::register_nullable::<Rc<dyn crate::controls::IResourceDictionary>>();
    ValueTypes::register_nullable::<Rc<dyn crate::controls::IThemeVariantProvider>>();
    ValueTypes::register_nullable::<Rc<dyn crate::controls::IDeferredContent>>();
    ValueTypes::register_nullable::<Rc<dyn crate::data::converters::IValueConverter>>();
    ValueTypes::register_nullable::<Rc<dyn crate::data::converters::IMultiValueConverter>>();
    ValueTypes::register_nullable::<Rc<dyn crate::animation::easings::IEasing>>();
    ValueTypes::register_nullable::<Rc<dyn crate::animation::IAnimation>>();
    ValueTypes::register_nullable::<Rc<dyn crate::animation::IAnimationSetter>>();
    ValueTypes::register_nullable::<Rc<dyn crate::data::BindingBase>>();

}
