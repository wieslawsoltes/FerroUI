use ferroui_base::metadata::MarkupDelegate;
pub struct Events;
ferroui_base::ferro_markup_type!(class Events {
    namespace: "Fixture.StaticEvents",
    events: [
        static Changed() => Events::subscribe,
        static try Failed() => Events::subscribe_fallible,
        Instance() => Events::instance,
    ],
});
impl Events {
    pub fn subscribe(_: MarkupDelegate) {}
    pub fn subscribe_fallible(_: MarkupDelegate) -> Result<(), String> { Err("Failed".into()) }
    pub fn instance(&self, _: MarkupDelegate) {}
}
