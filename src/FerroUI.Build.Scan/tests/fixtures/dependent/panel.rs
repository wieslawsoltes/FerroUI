use ferroui_base::ferro_markup_type;
use fixture::media::{Dock, Thickness};
use fixture::Decorator;

pub struct Margin {
    pub thickness: Thickness,
    pub dock: Option<Dock>,
}

ferro_markup_type!(struct Margin {
    handles: [Margin],
    properties: [
        Thickness: Thickness { get: |margin: &Margin| margin.thickness },
        Dock: Option<Dock> { get: |margin: &Margin| margin.dock },
        Owner: Option<ferroui_base::Ref<Decorator>> { get: Margin::owner },
    ],
    static_properties: [
        Default: Thickness { get: fixture::Border::default_thickness, set: fixture::Border::set_default_thickness },
    ],
    fields: [PressedEvent: ferroui_base::RoutedEvent<ferroui_base::RoutedEventArgs> => fixture::Border::pressed_event],
});
