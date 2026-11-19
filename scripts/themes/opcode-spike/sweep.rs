include!("catalog_host.rs");
use std::hint::black_box;
include!(concat!(env!("SPIKE_OUT"), "/documents.rs"));
fn main() {
    for id in 0..DOCUMENTS {
        let (panel, scope) = build(black_box(id));
        assert_eq!(panel.item_spacing(), (1 + id % 19) as f64);
        assert_eq!(panel.line_spacing(), (2 + id % 7) as f64);
        let children = panel.children();
        assert_eq!(children.count(), 2 + id % 4);
        for i in 0..children.count() {
            let child = children.get(i).cast::<ToggleButton>().unwrap();
            let text = format!("Choice {id}.{i}");
            assert_eq!(
                child.content().unwrap().downcast_ref::<String>().unwrap(),
                &text
            );
            let name = format!("Toggle{id}_{i}");
            if i == 0 || (id + i) % 5 == 0 {
                assert!(scope
                    .find(&name)
                    .unwrap()
                    .ptr_eq(child.upcast_ref::<FerroObject>()));
                assert_eq!(child.name().as_ref(), Some(&name));
            } else {
                assert!(child.name().is_none());
            }
            assert_eq!(child.is_three_state(), (id + i) % 3 == 0);
            assert_eq!(child.is_checked(), Some((id + 2 * i) % 4 == 0));
            assert_eq!(child.is_enabled(), (id + 3 * i) % 7 != 0);
        }
    }
}
