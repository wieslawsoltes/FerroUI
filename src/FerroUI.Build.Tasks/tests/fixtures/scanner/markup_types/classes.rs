use crate::media::{Brush, Dock, Thickness};
use crate::{Border, Decorator};
use ferroui_base::metadata::MarkupDelegate;
use ferroui_base::{ferro_class_info, BoxedValue, EventArgs, Ref, RoutedEvent, RoutedEventArgs};

ferro_class_info!(Border {
    markup: {
        namespace: "Fixture.Controls",
        content: Child,
        constructors: [
            (String) => Border::with_name,
            (property: &'static FerroProperty [InheritDataTypeFrom(2)], dock: Dock) => |property: &'static FerroProperty, dock: Dock| Border::docked(property, dock),
        ],
        properties: [
            Tag: Option<BoxedValue> { get: Border::tag, set: Border::set_tag } [DependsOn("Child"), AssignBinding],
            Brushes: Vec<Ref<Brush>> { try_get: |border: &Ref<Border>| border.brushes() },
        ],
        static_properties: [
            Default: Thickness { get: Border::default_thickness, try_set: Border::set_default_thickness },
        ],
        indexers: [
            (i32) -> Ref<Brush> { get: Border::brush_at } [Indexed],
        ],
        property_attributes: [Child: [Content, DependsOn("Tag")], Background: []],
        methods: [
            fn Add(Ref<Brush>) => Border::add_brush,
            static try fn Parse(String) -> Ref<Border> => Border::parse [Browsable(false)],
            fn Count() -> i32 => (|border: &Ref<Border>| border.count()) [Obsolete],
        ],
        fields: [PressedEvent: RoutedEvent<RoutedEventArgs> => Border::pressed_event],
        events: [
            Closed(Option<BoxedValue>, EventArgs) => |border: &Ref<Border>, handler: MarkupDelegate| border.on_closed(handler),
            try Opened() => Border::on_opened,
        ],
        attributes: [
            UsableDuringInitialization,
            TemplatePart("PART_Bar", type(Ref<Decorator>), IsRequired = true, Weight = -1.5, Initial = 'x'),
            Separators([",", " "], null),
        ],
    },
});
