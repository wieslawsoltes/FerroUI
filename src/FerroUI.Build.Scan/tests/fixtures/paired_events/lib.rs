pub struct Events;
impl Events {
    pub fn add(handler: Option<MarkupDelegate>) {}
    pub fn remove(handler: Option<MarkupDelegate>) -> Result<(), String> {Ok(())}
}
ferro_markup_type!(static Events {namespace:"Fixture.PairedEvents",
    events:[static Changed(i32) {add:Events::add,try_remove:Events::remove}],
});
pub struct Instance;
impl Instance {
    pub fn add(&self,handler:Option<MarkupDelegate>) {}
    pub fn remove(&self,handler:Option<MarkupDelegate>) {}
}
ferro_markup_type!(class Instance {namespace:"Fixture.PairedEvents",
    events:[Changed() {add:Instance::add,remove:Instance::remove}],
});
