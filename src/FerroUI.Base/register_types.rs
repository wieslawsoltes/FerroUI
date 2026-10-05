//! The type table of this crate: its namespaces and its classes.
//!
//! Types become known by name when they are initialised on a thread; this
//! list makes the ones that were never used known too (markup can name any
//! of them). Registration is explicit: an application (or the markup
//! loader) calls the `register_types()` of each crate it uses. There is no
//! life-before-main registration.
//!
//! Keep the list complete: every `ferro_class!`, `ferro_static_type!` and
//! `impl StaticType` of the crate has an entry (a test checks it against the
//! sources), under the namespace of the upstream type.

use crate::metadata::{MarkupAssembly, XmlnsDefinition, FERRO_XML_NAMESPACE};
use crate::{StaticType, TypeInfo};

/// The dotted namespaces of the modules of this crate. A type belongs to the
/// namespace of the longest module path that is a prefix of the path of its
/// declaring module, so a file that differs from its directory gets an entry
/// of its own.
const NAMESPACES: &[(&str, &str)] = &[
    ("ferroui_base", "FerroUI"),
    ("ferroui_base::combined_geometry", "FerroUI.Media"),
    ("ferroui_base::rotate_3d_transform", "FerroUI.Media"),
    ("ferroui_base::animation", "FerroUI.Animation"),
    ("ferroui_base::animation::easings", "FerroUI.Animation.Easings"),
    ("ferroui_base::collections", "FerroUI.Collections"),
    ("ferroui_base::controls", "FerroUI.Controls"),
    ("ferroui_base::data", "FerroUI.Data"),
    ("ferroui_base::input", "FerroUI.Input"),
    ("ferroui_base::input::gesture_recognizers", "FerroUI.Input.GestureRecognizers"),
    ("ferroui_base::input::text_input", "FerroUI.Input.TextInput"),
    ("ferroui_base::interactivity", "FerroUI.Interactivity"),
    ("ferroui_base::layout", "FerroUI.Layout"),
    ("ferroui_base::logical_tree", "FerroUI.LogicalTree"),
    ("ferroui_base::media", "FerroUI.Media"),
    ("ferroui_base::media::imaging", "FerroUI.Media.Imaging"),
    ("ferroui_base::media::immutable", "FerroUI.Media.Immutable"),
    ("ferroui_base::media::transformation", "FerroUI.Media.Transformation"),
    ("ferroui_base::platform", "FerroUI.Platform"),
    ("ferroui_base::rendering", "FerroUI.Rendering"),
    ("ferroui_base::styling", "FerroUI.Styling"),
    ("ferroui_base::threading", "FerroUI.Threading"),
    ("ferroui_base::visual_tree", "FerroUI.VisualTree"),
];

/// What this crate states about itself for markup: its assembly name and
/// the namespaces the XML namespace of the framework maps to.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroUI.Base",
    crate_name: "ferroui_base",
    xmlns_definitions: &[
        XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI" },
        XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.Animation" },
        XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.Animation.Easings" },
        XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.Controls" },
        XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.Data" },
        XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.Data.Converters" },
        XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.Input" },
        XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.Input.GestureRecognizers" },
        XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.Input.TextInput" },
        XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.Layout" },
        XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.LogicalTree" },
        XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.Media" },
        XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.Media.Imaging" },
        XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.Media.Transformation" },
        XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.Styling" },
    ],
    xmlns_prefixes: &[],
    metadata: &[],
};

macro_rules! types {
    ($($type_:ty),* $(,)?) => {
        &[$(<$type_ as StaticType>::TYPE),*]
    };
}

/// Registers the namespaces and the types of this crate with the table of
/// known types ([`TypeInfo::find`]). Cheap and idempotent; nothing is
/// initialised until a type is used or looked into.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        TypeInfo::register_namespaces(NAMESPACES);
        TypeInfo::register_all(TYPES);
        crate::rust_paths::register_rust_paths();
        MarkupAssembly::register(&ASSEMBLY);
        crate::markup_types::register();
    });
}

const TYPES: &[&TypeInfo] = types![
    // FerroUI
    crate::FerroObject,
    crate::StyledElement,
    crate::Visual,
    // FerroUI.Animation
    crate::animation::Animatable,
    crate::animation::Animation,
    crate::animation::AnimatorKeyFrame,
    crate::animation::KeyFrame,
    crate::animation::KeySpline,
    crate::animation::TransitionBase,
    crate::animation::BoolTransition,
    crate::animation::BoxShadowsTransition,
    crate::animation::BrushTransition,
    crate::animation::ColorTransition,
    crate::animation::CornerRadiusTransition,
    crate::animation::DoubleTransition,
    crate::animation::EffectTransition,
    crate::animation::FloatTransition,
    crate::animation::IntegerTransition,
    crate::animation::PointTransition,
    crate::animation::RelativePointTransition,
    crate::animation::SizeTransition,
    crate::animation::ThicknessTransition,
    crate::animation::TransformOperationsTransition,
    crate::animation::VectorTransition,
    // FerroUI.Controls
    crate::controls::NameScope,
    crate::controls::ResourceDictionary,
    crate::controls::ResourceProvider,
    // FerroUI.Input
    crate::input::AccessKeyHandler,
    crate::input::DragDrop,
    crate::input::FocusManager,
    crate::input::Gestures,
    crate::input::InputElement,
    crate::input::InputMethod,
    crate::input::KeyBinding,
    crate::input::KeyboardNavigation,
    crate::input::navigation::XYFocus,
    // FerroUI.Input.GestureRecognizers
    crate::input::gesture_recognizers::GestureRecognizer,
    crate::input::gesture_recognizers::PinchGestureRecognizer,
    crate::input::gesture_recognizers::PullGestureRecognizer,
    crate::input::gesture_recognizers::ScrollGestureRecognizer,
    crate::input::gesture_recognizers::SwipeGestureRecognizer,
    // FerroUI.Input.TextInput
    crate::input::text_input::TextInputOptions,
    // FerroUI.Interactivity
    crate::interactivity::Interactive,
    // FerroUI.Layout
    crate::layout::Layoutable,
    // FerroUI.Media
    crate::media::ArcSegment,
    crate::media::BezierSegment,
    crate::media::BitmapCache,
    crate::media::Brush,
    crate::media::CacheMode,
    crate::media::CombinedGeometry,
    crate::media::ConicGradientBrush,
    crate::media::DashStyle,
    crate::media::Drawing,
    crate::media::DrawingBrush,
    crate::media::DrawingGroup,
    crate::media::DrawingImage,
    crate::media::EllipseGeometry,
    crate::media::ExperimentalAcrylicMaterial,
    crate::media::Geometry,
    crate::media::GeometryDrawing,
    crate::media::GeometryGroup,
    crate::media::GradientBrush,
    crate::media::GradientStop,
    crate::media::ImageBrush,
    crate::media::ImageDrawing,
    crate::media::ImmutableGeometry,
    crate::media::LineGeometry,
    crate::media::LineSegment,
    crate::media::LinearGradientBrush,
    crate::media::MatrixTransform,
    crate::media::PathFigure,
    crate::media::PathGeometry,
    crate::media::PathSegment,
    crate::media::Pen,
    crate::media::PlatformGeometry,
    crate::media::PolyBezierSegment,
    crate::media::PolyLineSegment,
    crate::media::PolylineGeometry,
    crate::media::QuadraticBezierSegment,
    crate::media::RadialGradientBrush,
    crate::media::RectangleGeometry,
    crate::media::Rotate3DTransform,
    crate::media::RotateTransform,
    crate::media::ScaleTransform,
    crate::media::SkewTransform,
    crate::media::SolidColorBrush,
    crate::media::StreamGeometry,
    crate::media::TextDecoration,
    crate::media::TileBrush,
    crate::media::Transform,
    crate::media::TransformGroup,
    crate::media::TranslateTransform,
    crate::media::VisualBrush,
    crate::media::effects::BlurEffect,
    crate::media::effects::DropShadowDirectionEffect,
    crate::media::effects::DropShadowEffect,
    crate::media::effects::DropShadowEffectBase,
    crate::media::effects::Effect,
    // FerroUI.Media.Imaging
    crate::media::imaging::CroppedBitmap,
    // FerroUI.Styling
    crate::styling::Container,
    crate::styling::ContainerQuery,
    crate::styling::ControlTheme,
    crate::styling::Style,
    crate::styling::StyleBase,
    crate::styling::Styles,
];
