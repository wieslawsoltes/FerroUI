use ferroui_base::controls::{INameScope, NameScope};
use ferroui_base::{FerroObject, Ref};
use ferroui_controls::primitives::ToggleButton;
use ferroui_controls::{Control, WrapPanel};
use std::rc::Rc;
type Scope = Rc<dyn INameScope>;
fn panel() -> Ref<WrapPanel> {
    let panel = WrapPanel::new();
    panel.begin_init();
    panel
}
fn toggle() -> Ref<ToggleButton> {
    let c = ToggleButton::new();
    c.begin_init();
    c
}
fn name(c: &Ref<ToggleButton>, scope: &Scope, name: &str) {
    c.set_name(Some(name.to_owned()));
    scope.try_register_value(name, Rc::new(c.clone())).unwrap();
}
fn content(c: &Ref<ToggleButton>, text: &str) {
    c.set_content(Some(Rc::new(text.to_owned())));
}
fn attach(p: &Ref<WrapPanel>, c: Ref<ToggleButton>) {
    c.end_init();
    p.children().add(c.upcast::<Control>());
}
#[cfg(vm)]
#[inline(never)]
fn execute(bytes: &[u8]) -> (Ref<WrapPanel>, Scope) {
    let (magic, mut code) = bytes.split_at(5);
    assert_eq!(magic, b"FXCP\x01");
    fn read<'a>(code: &mut &'a [u8], n: usize) -> &'a [u8] {
        let (arg, rest) = code.split_at(n);
        *code = rest;
        arg
    }
    fn text<'a>(code: &mut &'a [u8]) -> &'a str {
        let n = read(code, 1)[0] as usize;
        std::str::from_utf8(read(code, n)).unwrap()
    }
    assert_eq!(read(&mut code, 1)[0], 1);
    let p = panel();
    let scope: Scope = Rc::new(NameScope::new());
    let mut c = None;
    loop {
        let op = read(&mut code, 1)[0];
        match op {
            0 => {
                assert!(code.is_empty());
                p.end_init();
                return (p, scope);
            }
            2 | 3 => {
                let n = f64::from_le_bytes(read(&mut code, 8).try_into().unwrap());
                if op == 2 {
                    p.set_item_spacing(n)
                } else {
                    p.set_line_spacing(n)
                }
            }
            4 => c = Some(toggle()),
            5 => name(c.as_ref().unwrap(), &scope, text(&mut code)),
            6 => content(c.as_ref().unwrap(), text(&mut code)),
            7 => c
                .as_ref()
                .unwrap()
                .set_is_three_state(read(&mut code, 1)[0] != 0),
            8 => c
                .as_ref()
                .unwrap()
                .set_is_enabled(read(&mut code, 1)[0] != 0),
            9 => c
                .as_ref()
                .unwrap()
                .set_is_checked(Some(read(&mut code, 1)[0] != 0)),
            10 => attach(&p, c.take().unwrap()),
            _ => panic!("unknown opcode {op}"),
        }
    }
}
