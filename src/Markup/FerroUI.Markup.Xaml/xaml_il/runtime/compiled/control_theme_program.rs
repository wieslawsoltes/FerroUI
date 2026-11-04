//! Construction instructions for the emitter's closed ControlTheme grammar.
//!
//! The emitter keeps each resource's imports separate, evaluates every import at
//! its original call site, and declines the complete factory if any statement is
//! outside the grammar. This is construction only: property storage, getters,
//! setters and the objects built by a template still use their typed APIs.
//!
//! A generated caller owns the local array. Declaration number `i` occupies
//! `len - 1 - i`, so ordinary array drop glue has precisely the original reverse
//! declaration order, including cleanup after one destructor panics. The caller
//! declares its context before this array. No explicit cleanup loop, erased
//! object storage, allocation for the instruction stream, or extra Rc is needed.
//!
//! Every accepted factory already constructs all six local roles below. This is
//! intentional: the shared enum's drop glue must not retain a type absent from
//! the resource. Optional operations and all leaf callbacks are imported by the
//! individual factory, never by a global opcode dispatch table.

use super::{DeferredBuild, DeferredContent, MarkupValue, XamlIlContext, XamlLoadError};
use crate::templates::ControlTemplate;
use ferroui_base::styling::{ControlTheme, Selector, Selectors, Setter, Style, StyleBase};
use ferroui_base::{FerroProperty, Ref, TypeInfo};
use std::rc::Rc;

/// Local roles already present in every factory accepted by the emitter.
pub enum Local {
    Empty,
    Theme(Ref<ControlTheme>),
    Style(Ref<Style>),
    Selector(Selector),
    Setter(Rc<Setter>),
    Template(Rc<ControlTemplate>),
    Deferred(Rc<DeferredContent>),
}

impl Local {
    fn theme(&self) -> &Ref<ControlTheme> {
        match self {
            Self::Theme(value) => value,
            _ => unreachable!("validated ControlTheme local"),
        }
    }

    fn style(&self) -> &Ref<Style> {
        match self {
            Self::Style(value) => value,
            _ => unreachable!("validated Style local"),
        }
    }

    fn style_base(&self) -> &Ref<StyleBase> {
        match self {
            Self::Theme(value) => value.upcast_ref(),
            Self::Style(value) => value.upcast_ref(),
            _ => unreachable!("validated StyleBase local"),
        }
    }

    fn selector(&self) -> &Selector {
        match self {
            Self::Selector(value) => value,
            _ => unreachable!("validated Selector local"),
        }
    }

    fn setter(&self) -> &Rc<Setter> {
        match self {
            Self::Setter(value) => value,
            _ => unreachable!("validated Setter local"),
        }
    }

    fn template(&self) -> &Rc<ControlTemplate> {
        match self {
            Self::Template(value) => value,
            _ => unreachable!("validated ControlTemplate local"),
        }
    }

    fn deferred(&self) -> &Rc<DeferredContent> {
        match self {
            Self::Deferred(value) => value,
            _ => unreachable!("validated DeferredContent local"),
        }
    }

    fn take_selector(&mut self) -> Selector {
        match std::mem::replace(self, Self::Empty) {
            Self::Selector(value) => value,
            _ => unreachable!("validated move of Selector local"),
        }
    }
}

pub type Operation = fn(
    &Rc<XamlIlContext>,
    &mut [Local],
    &Imports,
    &[u16; 4],
) -> Result<(), XamlLoadError>;

pub struct Instruction {
    pub operation: u16,
    pub args: [u16; 4],
}

pub struct DeferredSite {
    pub result_type: &'static TypeInfo,
    pub build: DeferredBuild,
    pub line: i32,
    pub column: i32,
}

pub struct Imports {
    pub properties: &'static [fn() -> &'static FerroProperty],
    pub values: &'static [fn() -> MarkupValue],
    pub selectors: &'static [fn() -> Selector],
    pub types: &'static [&'static TypeInfo],
    pub texts: &'static [&'static str],
    pub positions: &'static [(i32, i32)],
    pub deferred: &'static [DeferredSite],
    pub dynamic_runs: &'static [&'static [[u16; 2]]],
}

pub struct Program {
    pub operations: &'static [Operation],
    pub instructions: &'static [Instruction],
    pub imports: Imports,
}

#[inline(never)]
pub fn run(
    context: &Rc<XamlIlContext>,
    program: &Program,
    locals: &mut [Local],
    returned: u16,
) -> Result<MarkupValue, XamlLoadError> {
    for instruction in program.instructions {
        (program.operations[instruction.operation as usize])(
            context,
            locals,
            &program.imports,
            &instruction.args,
        )?;
    }
    // The accepted source returns a clone. A moved result is a different grammar
    // and must not silently acquire an extra owner or a different drop boundary.
    Ok(super::to_value(locals[returned as usize].theme().clone()))
}

pub fn new_theme(_: &Rc<XamlIlContext>, locals: &mut [Local], _: &Imports, a: &[u16; 4]) -> Result<(), XamlLoadError> {
    locals[a[0] as usize] = Local::Theme(ControlTheme::new());
    Ok(())
}

pub fn target_type(_: &Rc<XamlIlContext>, locals: &mut [Local], imports: &Imports, a: &[u16; 4]) -> Result<(), XamlLoadError> {
    ControlTheme::__markup_set_TargetType(locals[a[0] as usize].theme(), Some(imports.types[a[1] as usize]));
    Ok(())
}

pub fn push_theme(context: &Rc<XamlIlContext>, locals: &mut [Local], _: &Imports, a: &[u16; 4]) -> Result<(), XamlLoadError> {
    super::push_parent(context, locals[a[0] as usize].theme());
    Ok(())
}

pub fn push_style(context: &Rc<XamlIlContext>, locals: &mut [Local], _: &Imports, a: &[u16; 4]) -> Result<(), XamlLoadError> {
    super::push_parent(context, locals[a[0] as usize].style());
    Ok(())
}

pub fn push_template(context: &Rc<XamlIlContext>, locals: &mut [Local], _: &Imports, a: &[u16; 4]) -> Result<(), XamlLoadError> {
    super::push_parent(context, locals[a[0] as usize].template());
    Ok(())
}

pub fn pop_parent(context: &Rc<XamlIlContext>, _: &mut [Local], _: &Imports, _: &[u16; 4]) -> Result<(), XamlLoadError> {
    context.pop_parent();
    Ok(())
}

pub fn new_style(_: &Rc<XamlIlContext>, locals: &mut [Local], _: &Imports, a: &[u16; 4]) -> Result<(), XamlLoadError> {
    locals[a[0] as usize] = Local::Style(Style::new());
    Ok(())
}

pub fn begin_style(context: &Rc<XamlIlContext>, locals: &mut [Local], _: &Imports, a: &[u16; 4]) -> Result<(), XamlLoadError> {
    locals[a[0] as usize] = Local::Style(super::style_program::begin(context));
    Ok(())
}

pub fn selected_style(context: &Rc<XamlIlContext>, locals: &mut [Local], imports: &Imports, a: &[u16; 4]) -> Result<(), XamlLoadError> {
    // The existing fused helper creates/pushes the style before building the
    // selector, and moves that selector directly into the property accessor.
    let style = super::style_program::begin(context);
    Style::__markup_set_Selector(&style, Some((imports.selectors[a[1] as usize])()));
    locals[a[0] as usize] = Local::Style(style);
    Ok(())
}

pub fn new_selector(_: &Rc<XamlIlContext>, locals: &mut [Local], imports: &Imports, a: &[u16; 4]) -> Result<(), XamlLoadError> {
    locals[a[0] as usize] = Local::Selector((imports.selectors[a[1] as usize])());
    Ok(())
}

pub fn or_two(_: &Rc<XamlIlContext>, locals: &mut [Local], _: &Imports, a: &[u16; 4]) -> Result<(), XamlLoadError> {
    let first = locals[a[1] as usize].take_selector();
    let second = locals[a[2] as usize].take_selector();
    locals[a[0] as usize] = Local::Selector(Selectors::or([first, second]));
    Ok(())
}

pub fn or_three(_: &Rc<XamlIlContext>, locals: &mut [Local], _: &Imports, a: &[u16; 4]) -> Result<(), XamlLoadError> {
    let first = locals[a[1] as usize].take_selector();
    let second = locals[a[2] as usize].take_selector();
    let third = locals[a[3] as usize].take_selector();
    locals[a[0] as usize] = Local::Selector(Selectors::or([first, second, third]));
    Ok(())
}

pub fn set_selector(_: &Rc<XamlIlContext>, locals: &mut [Local], _: &Imports, a: &[u16; 4]) -> Result<(), XamlLoadError> {
    Style::__markup_set_Selector(locals[a[0] as usize].style(), Some(locals[a[1] as usize].selector().clone()));
    Ok(())
}

pub fn attach(_: &Rc<XamlIlContext>, locals: &mut [Local], imports: &Imports, a: &[u16; 4]) -> Result<(), XamlLoadError> {
    let (line, column) = imports.positions[a[2] as usize];
    super::style_program::attach(locals[a[0] as usize].style_base(), locals[a[1] as usize].style(), line, column)
}

fn dynamic_run(context: &Rc<XamlIlContext>, style: &Ref<Style>, imports: &Imports, run: u16) {
    for &[property, key] in imports.dynamic_runs[run as usize] {
        super::add_dynamic_resource_setter_with_parent(
            context,
            style.upcast_ref(),
            (imports.properties[property as usize])(),
            imports.texts[key as usize],
        );
    }
}

pub fn finish_dynamic(context: &Rc<XamlIlContext>, locals: &mut [Local], imports: &Imports, a: &[u16; 4]) -> Result<(), XamlLoadError> {
    let parent = locals[a[0] as usize].style_base();
    let child = locals[a[1] as usize].style();
    dynamic_run(context, child, imports, a[2]);
    context.pop_parent();
    let (line, column) = imports.positions[a[3] as usize];
    super::style_program::attach(parent, child, line, column)
}

pub fn add_selected_style(context: &Rc<XamlIlContext>, locals: &mut [Local], imports: &Imports, a: &[u16; 4]) -> Result<(), XamlLoadError> {
    let parent = locals[a[0] as usize].style_base();
    // This child belongs to the fused helper's scope, not the caller's local
    // array. It must drop before the next instruction, including on error.
    let style = super::style_program::begin(context);
    Style::__markup_set_Selector(&style, Some((imports.selectors[a[1] as usize])()));
    dynamic_run(context, &style, imports, a[2]);
    context.pop_parent();
    let (line, column) = imports.positions[a[3] as usize];
    super::style_program::attach(parent, &style, line, column)
}

pub fn literal_setter(_: &Rc<XamlIlContext>, locals: &mut [Local], imports: &Imports, a: &[u16; 4]) -> Result<(), XamlLoadError> {
    super::add_setter(locals[a[0] as usize].style_base(), (imports.properties[a[1] as usize])(), (imports.values[a[2] as usize])());
    Ok(())
}

pub fn dynamic_setter(context: &Rc<XamlIlContext>, locals: &mut [Local], imports: &Imports, a: &[u16; 4]) -> Result<(), XamlLoadError> {
    super::add_dynamic_resource_setter_with_parent(context, locals[a[0] as usize].style_base(), (imports.properties[a[1] as usize])(), imports.texts[a[2] as usize]);
    Ok(())
}

pub fn new_setter(_: &Rc<XamlIlContext>, locals: &mut [Local], imports: &Imports, a: &[u16; 4]) -> Result<(), XamlLoadError> {
    locals[a[0] as usize] = Local::Setter(super::new_setter((imports.properties[a[1] as usize])()));
    Ok(())
}

pub fn new_setter_with_parent(context: &Rc<XamlIlContext>, locals: &mut [Local], imports: &Imports, a: &[u16; 4]) -> Result<(), XamlLoadError> {
    locals[a[0] as usize] = Local::Setter(super::new_setter_with_parent(context, (imports.properties[a[1] as usize])()));
    Ok(())
}

pub fn new_template(_: &Rc<XamlIlContext>, locals: &mut [Local], _: &Imports, a: &[u16; 4]) -> Result<(), XamlLoadError> {
    locals[a[0] as usize] = Local::Template(ControlTemplate::__markup_new_0());
    Ok(())
}

pub fn defer(context: &Rc<XamlIlContext>, locals: &mut [Local], imports: &Imports, a: &[u16; 4]) -> Result<(), XamlLoadError> {
    let site = &imports.deferred[a[1] as usize];
    locals[a[0] as usize] = Local::Deferred(super::defer(super::class_handle(site.result_type), context, site.build, site.line, site.column)?);
    Ok(())
}

pub fn template_content(_: &Rc<XamlIlContext>, locals: &mut [Local], _: &Imports, a: &[u16; 4]) -> Result<(), XamlLoadError> {
    ControlTemplate::__markup_set_Content(locals[a[0] as usize].template(), super::to_object(Clone::clone(locals[a[1] as usize].deferred())));
    Ok(())
}

pub fn setter_template(_: &Rc<XamlIlContext>, locals: &mut [Local], _: &Imports, a: &[u16; 4]) -> Result<(), XamlLoadError> {
    super::add_setter_value(locals[a[0] as usize].style_base(), locals[a[1] as usize].setter(), super::to_object(Clone::clone(locals[a[2] as usize].template())));
    Ok(())
}

pub fn setter_template_with_parent(context: &Rc<XamlIlContext>, locals: &mut [Local], _: &Imports, a: &[u16; 4]) -> Result<(), XamlLoadError> {
    super::add_setter_value_with_parent(context, locals[a[0] as usize].style_base(), locals[a[1] as usize].setter(), super::to_object(Clone::clone(locals[a[2] as usize].template())));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::styling::SetterValue;
    use std::cell::RefCell;
    use std::panic::{catch_unwind, AssertUnwindSafe};

    struct Probe {
        id: u8,
        panic: bool,
        events: Rc<RefCell<Vec<u8>>>,
    }

    impl PartialEq for Probe {
        fn eq(&self, other: &Self) -> bool { std::ptr::eq(self, other) }
    }

    impl Drop for Probe {
        fn drop(&mut self) {
            self.events.borrow_mut().push(self.id);
            assert!(!self.panic, "payload destructor");
        }
    }

    fn setter(id: u8, panic_at: Option<u8>, events: &Rc<RefCell<Vec<u8>>>) -> Rc<Setter> {
        let setter = Rc::new(Setter::default());
        setter.set_value(Some(SetterValue::Value(Rc::new(Probe {
            id, panic: panic_at == Some(id), events: events.clone(),
        }))));
        setter
    }

    fn fail(_: &Rc<XamlIlContext>, _: &mut [Local], _: &Imports, _: &[u16; 4]) -> Result<(), XamlLoadError> {
        Err(super::super::at("TestException", "construction failed", 7, 9))
    }

    fn panic_step(_: &Rc<XamlIlContext>, _: &mut [Local], _: &Imports, _: &[u16; 4]) -> Result<(), XamlLoadError> {
        panic!("construction callback")
    }

    const IMPORTS: Imports = Imports {
        properties: &[], values: &[], selectors: &[], types: &[], texts: &[],
        positions: &[], deferred: &[], dynamic_runs: &[],
    };
    const SUCCESS: Program = Program { operations: &[], instructions: &[], imports: IMPORTS };
    const ERROR: Program = Program {
        operations: &[fail],
        instructions: &[Instruction { operation: 0, args: [0; 4] }],
        imports: IMPORTS,
    };
    const PANIC: Program = Program {
        operations: &[panic_step],
        instructions: &[Instruction { operation: 0, args: [0; 4] }],
        imports: IMPORTS,
    };

    fn original(mode: u8, panic_at: Option<u8>, events: &Rc<RefCell<Vec<u8>>>) -> Result<MarkupValue, XamlLoadError> {
        let _context_lifetime = Probe { id: 0, panic: false, events: events.clone() };
        let _context = super::super::create_context(None, None, &[]);
        let theme = ControlTheme::new();
        let _first = setter(1, panic_at, events);
        let _second = setter(2, panic_at, events);
        let _third = setter(3, panic_at, events);
        match mode {
            1 => Err(super::super::at("TestException", "construction failed", 7, 9)),
            2 => panic!("construction callback"),
            _ => Ok(super::super::to_value(theme.clone())),
        }
    }

    fn programmed(mode: u8, panic_at: Option<u8>, events: &Rc<RefCell<Vec<u8>>>) -> Result<MarkupValue, XamlLoadError> {
        let _context_lifetime = Probe { id: 0, panic: false, events: events.clone() };
        let context = super::super::create_context(None, None, &[]);
        let mut locals = [const { Local::Empty }; 4];
        locals[3] = Local::Theme(ControlTheme::new());
        locals[2] = Local::Setter(setter(1, panic_at, events));
        locals[1] = Local::Setter(setter(2, panic_at, events));
        locals[0] = Local::Setter(setter(3, panic_at, events));
        run(&context, match mode { 1 => &ERROR, 2 => &PANIC, _ => &SUCCESS }, &mut locals, 3)
    }

    #[test]
    fn caller_owned_locals_match_original_cleanup_on_error_unwind_and_panicking_drop() {
        for (mode, panic_at) in [(0, None), (1, None), (2, None), (0, Some(2)), (1, Some(2))] {
            let expected = Rc::new(RefCell::new(Vec::new()));
            let actual = Rc::new(RefCell::new(Vec::new()));
            let original = catch_unwind(AssertUnwindSafe(|| original(mode, panic_at, &expected)));
            let programmed = catch_unwind(AssertUnwindSafe(|| programmed(mode, panic_at, &actual)));
            assert_eq!(programmed.is_err(), original.is_err());
            if let (Ok(original), Ok(programmed)) = (&original, &programmed) {
                assert_eq!(programmed.is_ok(), original.is_ok());
            }
            assert_eq!(*actual.borrow(), *expected.borrow());
            assert_eq!(*actual.borrow(), [3, 2, 1, 0]);
        }
    }

    fn theme_with_result_payload(events: &Rc<RefCell<Vec<u8>>>) -> Ref<ControlTheme> {
        let theme = ControlTheme::new();
        // The actual theme's Setter collection is this payload's sole owner.
        // Neither this helper nor the oracle keeps a second Setter/Rc owner.
        theme.upcast_ref::<StyleBase>().add_setter(setter(4, None, events));
        theme
    }

    fn original_result_payload(panic_at: Option<u8>, events: &Rc<RefCell<Vec<u8>>>) -> Result<MarkupValue, XamlLoadError> {
        let _context_lifetime = Probe { id: 0, panic: false, events: events.clone() };
        let _context = super::super::create_context(None, None, &[]);
        let theme = theme_with_result_payload(events);
        let _first = setter(1, panic_at, events);
        let _second = setter(2, panic_at, events);
        let _third = setter(3, panic_at, events);
        Ok(super::super::to_value(theme.clone()))
    }

    fn programmed_result_payload(panic_at: Option<u8>, events: &Rc<RefCell<Vec<u8>>>) -> Result<MarkupValue, XamlLoadError> {
        let _context_lifetime = Probe { id: 0, panic: false, events: events.clone() };
        let context = super::super::create_context(None, None, &[]);
        let mut locals = [const { Local::Empty }; 4];
        locals[3] = Local::Theme(theme_with_result_payload(events));
        locals[2] = Local::Setter(setter(1, panic_at, events));
        locals[1] = Local::Setter(setter(2, panic_at, events));
        locals[0] = Local::Setter(setter(3, panic_at, events));
        run(&context, &SUCCESS, &mut locals, 3)
    }

    #[test]
    fn returned_theme_payload_matches_original_return_place_cleanup() {
        for panic_at in [None, Some(2)] {
            let expected = Rc::new(RefCell::new(Vec::new()));
            let actual = Rc::new(RefCell::new(Vec::new()));
            let original = catch_unwind(AssertUnwindSafe(|| original_result_payload(panic_at, &expected)));
            let programmed = catch_unwind(AssertUnwindSafe(|| programmed_result_payload(panic_at, &actual)));
            assert_eq!(programmed.is_err(), original.is_err());
            assert_eq!(*actual.borrow(), *expected.borrow());
            if panic_at.is_none() {
                // The output is still alive; local cleanup must not destroy the
                // unique payload reached only through its actual theme setters.
                assert!(!expected.borrow().contains(&4));
                assert!(matches!(&original, Ok(Ok(Some(_)))));
                assert!(matches!(&programmed, Ok(Ok(Some(_)))));
            }
            drop(original);
            drop(programmed);
            assert_eq!(*actual.borrow(), *expected.borrow());
            if panic_at.is_none() {
                assert_eq!(*actual.borrow(), [3, 2, 1, 0, 4]);
            }
            // On a destructor panic the original expression is the oracle; do
            // not assume whether Rust destroys an initialized return place.
        }
    }
}
