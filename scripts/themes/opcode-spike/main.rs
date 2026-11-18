use ferroui_base::animation::{
    Animatable, SignedDuration, SolidColorBrushTransition, TransitionBase, Transitions,
};
use ferroui_base::controls::{INameScope, NameScope};
use ferroui_base::data::BindingPriority;
use ferroui_base::{FerroObject, FerroProperty, Ref, StyledElement, Thickness};
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::{Button, ContentControl};
use ferroui_markup_xaml::xaml_il::runtime::compiled as rt;
use std::{hint::black_box, rc::Rc, time::Instant};

type Presenter = Ref<ContentPresenter>;
type Scope = Rc<dyn INameScope>;

// IDs belong to the host schema. The document has no pointers or Rust layout dependencies.
macro_rules! properties {
    ($(($source:ident, $getter:ident)),*) => {
        fn property(id: u8, source: bool) -> &'static FerroProperty {
            let table: &[fn(bool) -> &'static FerroProperty] = &[$(
                |source| if source { rt::property($source::$getter()) }
                else { rt::property(ContentPresenter::$getter()) }
            ),*];
            table[id as usize](source)
        }
    };
}
properties!(
    (TemplatedControl, background_property),
    (TemplatedControl, foreground_property),
    (TemplatedControl, background_sizing_property),
    (TemplatedControl, border_brush_property),
    (TemplatedControl, border_thickness_property),
    (ContentControl, content_property),
    (ContentControl, content_template_property),
    (TemplatedControl, corner_radius_property),
    (TemplatedControl, padding_property),
    (ContentControl, horizontal_content_alignment_property),
    (ContentControl, vertical_content_alignment_property)
);

fn presenter() -> Presenter {
    let p = ContentPresenter::new();
    p.begin_init();
    p
}
fn name(p: &Presenter, scope: &Scope, name: &str) {
    p.set_direct_value(StyledElement::name_property(), Some(name.to_owned()));
    scope.try_register_value(name, Rc::new(p.clone())).unwrap();
}
fn bind(p: &Presenter, target: u8, source: u8) {
    rt::bind_template_binding(
        p.upcast_ref::<FerroObject>(),
        property(target, false),
        property(source, true),
        0,
        0,
    )
    .unwrap();
}
fn access(p: &Presenter, value: bool) {
    p.set_value_with_priority(
        ContentPresenter::recognizes_access_key_property(),
        value,
        BindingPriority::Template,
    );
}
fn transition_property(item: &Ref<SolidColorBrushTransition>, id: u8) {
    item.set_property(Some(property(id, false)));
}
fn duration(item: &Ref<SolidColorBrushTransition>, ticks: i64) {
    item.set_duration(SignedDuration::from_ticks(ticks));
}
fn append(list: &Transitions, item: &Ref<SolidColorBrushTransition>) {
    list.add(item.into());
}
fn install(p: &Presenter, list: Transitions) {
    p.set_value_with_priority(
        Animatable::transitions_property(),
        Some(list),
        BindingPriority::Template,
    );
}

#[cfg(native)]
#[inline(never)]
fn build() -> (Presenter, Scope) {
    let scope: Scope = Rc::new(NameScope::new());
    include!(concat!(env!("SPIKE_OUT"), "/native.rs"))
}

#[cfg(vm)]
#[inline(never)]
fn build() -> (Presenter, Scope) {
    // Only trusted bytes from this compiler. A public loader needs a verifier before execution.
    let bytes = black_box(include_bytes!(concat!(env!("SPIKE_OUT"), "/template.bin")).as_slice());
    let (magic, mut code) = bytes.split_at(5);
    assert_eq!(magic, b"FXOP\x01");
    fn byte(code: &mut &[u8]) -> u8 {
        let (&value, rest) = code.split_first().unwrap();
        *code = rest;
        value
    }
    assert_eq!(byte(&mut code), 1);
    let p = presenter();
    let scope: Scope = Rc::new(NameScope::new());
    let mut list = None;
    let mut item = None;
    loop {
        match byte(&mut code) {
            0 => {
                assert!(code.is_empty());
                p.try_end_init().unwrap();
                return (p, scope);
            }
            2 => {
                let n = byte(&mut code) as usize;
                let (s, rest) = code.split_at(n);
                code = rest;
                name(&p, &scope, std::str::from_utf8(s).unwrap());
            }
            3 => bind(&p, byte(&mut code), byte(&mut code)),
            4 => access(&p, byte(&mut code) != 0),
            5 => list = Some(Transitions::new()),
            6 => item = Some(SolidColorBrushTransition::new()),
            7 => transition_property(item.as_ref().unwrap(), byte(&mut code)),
            8 => {
                let (ticks, rest) = code.split_at(8);
                code = rest;
                duration(
                    item.as_ref().unwrap(),
                    i64::from_le_bytes(ticks.try_into().unwrap()),
                );
            }
            9 => append(list.as_ref().unwrap(), item.as_ref().unwrap()),
            10 => install(&p, list.take().unwrap()),
            op => panic!("unknown opcode {op}"),
        }
    }
}

fn main() {
    let now = Instant::now();
    let (p, scope) = build();
    let first = now.elapsed().as_nanos();
    assert_eq!(p.name().as_deref(), Some("PART_ContentPresenter"));
    assert!(scope
        .find("PART_ContentPresenter")
        .unwrap()
        .ptr_eq(p.upcast_ref::<FerroObject>()));
    assert!(p.recognizes_access_key());
    let list = p.transitions().unwrap();
    assert_eq!(list.count(), 1);
    let transition = list.get(0);
    assert_eq!(transition.property().name(), "Background");
    assert_eq!(
        transition
            .as_object()
            .unwrap()
            .cast::<TransitionBase>()
            .unwrap()
            .duration()
            .ticks(),
        830000
    );
    let button = Button::new();
    p.set_templated_parent(button.clone().upcast::<FerroObject>());
    for n in [7., 13.] {
        button.set_padding(Thickness::uniform(n));
        assert_eq!(p.padding(), Thickness::uniform(n));
    }
    let now = Instant::now();
    for _ in 0..256 {
        black_box(build());
    }
    println!("{} {}", first, now.elapsed().as_nanos() / 256);
}
