use crate::controls::grid::Layout;
use ferroui_base::collections::FerroList;
use ferroui_base::{ferro_markup_type, Ref};
use std::rc::Rc;

pub struct Setter;

ferro_markup_type!(class Setter {
    namespace: "Fixture.Styling",
    handles: [Setter, Rc<Setter>, Option<Rc<Setter>>],
    this: Rc<Setter>,
    base: FerroList<Ref<crate::Border>>,
    interfaces: [Rc<dyn crate::media::IBrush>],
    generic: "FerroList`1" [Ref<crate::Border>],
    constructors: [() => Setter::empty],
    content: Value,
    notify_property_changed: Setter,
});

ferro_markup_type!(static Layout {
    type_info: Layout,
    property_attributes: [Spacing: [ResolveByName]],
    methods: [static fn Reset() => Layout::reset],
});

ferro_markup_type!(static Colors as "Colors" {
    namespace: "Fixture.Media",
    static_properties: [Red: u32 { get: Colors::red }],
});

// Two forms the macro does not have: the scanner reports each and reads nothing of it.
ferro_markup_type!(record Odd { handles: [Odd] });
ferro_markup_type!(class Other {
    handles: [Other],
    surprise: 1,
});
