include!("catalog_host.rs");
use std::{hint::black_box, time::Instant};
#[cfg(native)]
#[inline(never)]
fn build() -> (Ref<WrapPanel>, Scope) {
    let scope: Scope = Rc::new(NameScope::new());
    include!(concat!(env!("SPIKE_OUT"), "/native.rs"))
}
#[cfg(vm)]
#[inline(never)]
fn build() -> (Ref<WrapPanel>, Scope) {
    let bytes = black_box(include_bytes!(concat!(env!("SPIKE_OUT"), "/template.bin")).as_slice());
    execute(bytes)
}
fn main() {
    let start = Instant::now();
    let (panel, scope) = build();
    let first = start.elapsed().as_nanos();
    assert_eq!((panel.item_spacing(), panel.line_spacing()), (8., 8.));
    let children = panel.children();
    assert_eq!(children.count(), 3);
    let checked_child = |i| children.get(i).cast::<ToggleButton>().unwrap();
    let first_button = checked_child(0);
    assert_eq!(first_button.name().as_deref(), Some("DemoToggle"));
    assert!(scope
        .find("DemoToggle")
        .unwrap()
        .ptr_eq(first_button.upcast_ref::<FerroObject>()));
    for (i, label) in ["Toggle", "Three state", "Disabled"].iter().enumerate() {
        assert_eq!(
            checked_child(i)
                .content()
                .unwrap()
                .downcast_ref::<String>()
                .unwrap(),
            label
        );
    }
    assert!(checked_child(1).is_three_state());
    assert_eq!(checked_child(2).is_checked(), Some(true));
    assert!(!checked_child(2).is_enabled());
    let start = Instant::now();
    for _ in 0..256 {
        black_box(build());
    }
    println!("{} {}", first, start.elapsed().as_nanos() / 256);
}
