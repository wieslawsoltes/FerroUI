//! Construction instructions for a closed region of nested styles.
//!
//! Each accepted region already has both Style and Selector locals. Its caller
//! owns an array at the original declaration position, with declaration `i` in
//! slot `len - 1 - i`. Ordinary Rust array cleanup therefore preserves reverse
//! declaration order, including cleanup after a destructor panics. Unrelated
//! locals between regions keep their original scopes. The borrowed external
//! parent, context and local array acquire no additional owning handles here.
//!
//! A factory imports only the operations and lazy callbacks used by its accepted
//! regions. There is no global operation table. The emitter rejects a complete
//! candidate region when a statement, later local use or move is unsupported;
//! these handlers consequently never replace a live slot or move a selector.

use super::{MarkupValue, XamlIlContext, XamlLoadError};
use ferroui_base::styling::{Selector, Style, StyleBase};
use ferroui_base::{FerroProperty, Ref};
use std::rc::Rc;

pub const EXTERNAL_PARENT: u16 = u16::MAX;

/// Both stored roles must already occur in each accepted source region.
pub enum Local {
    Empty,
    Style(Ref<Style>),
    Selector(Selector),
}

impl Local {
    fn style(&self) -> &Ref<Style> {
        match self {
            Self::Style(value) => value,
            _ => unreachable!("validated Style region local"),
        }
    }

    fn selector(&self) -> &Selector {
        match self {
            Self::Selector(value) => value,
            _ => unreachable!("validated Selector region local"),
        }
    }
}

/// Borrows held for one execution, without an owning cleanup scope.
pub struct Frame<'a> {
    context: &'a Rc<XamlIlContext>,
    parent: &'a Ref<StyleBase>,
    locals: &'a mut [Local],
}

impl Frame<'_> {
    fn parent(&self, slot: u16) -> &Ref<StyleBase> {
        if slot == EXTERNAL_PARENT {
            self.parent
        } else {
            self.locals[slot as usize].style().upcast_ref()
        }
    }
}

pub type Operation = fn(&mut Frame<'_>, &Imports, &[u16; 4]) -> Result<(), XamlLoadError>;

pub struct Instruction {
    pub operation: u16,
    pub args: [u16; 4],
}

pub struct Imports {
    pub properties: &'static [fn() -> &'static FerroProperty],
    pub values: &'static [fn() -> MarkupValue],
    pub selectors: &'static [fn() -> Selector],
    pub texts: &'static [&'static str],
    pub positions: &'static [(i32, i32)],
    pub dynamic_runs: &'static [&'static [[u16; 2]]],
}

pub struct Program {
    pub operations: &'static [Operation],
    pub imports: Imports,
}

#[inline(never)]
pub fn run(
    context: &Rc<XamlIlContext>,
    parent: &Ref<StyleBase>,
    program: &Program,
    instructions: &[Instruction],
    locals: &mut [Local],
) -> Result<(), XamlLoadError> {
    let mut frame = Frame { context, parent, locals };
    for instruction in instructions {
        (program.operations[instruction.operation as usize])(&mut frame, &program.imports, &instruction.args)?;
    }
    // No return value or implicit PopParent: the caller retains its original
    // tail expression and cleanup, including an unbalanced stack on failure.
    Ok(())
}

pub fn new_style(frame: &mut Frame<'_>, _: &Imports, a: &[u16; 4]) -> Result<(), XamlLoadError> {
    frame.locals[a[0] as usize] = Local::Style(Style::new());
    Ok(())
}

pub fn begin_style(frame: &mut Frame<'_>, _: &Imports, a: &[u16; 4]) -> Result<(), XamlLoadError> {
    frame.locals[a[0] as usize] = Local::Style(super::style_program::begin(frame.context));
    Ok(())
}

pub fn selected_style(frame: &mut Frame<'_>, imports: &Imports, a: &[u16; 4]) -> Result<(), XamlLoadError> {
    // As in the original fused expression, create and push the style before
    // evaluating the selector. A panicking callback leaves that push intact.
    let style = super::style_program::begin(frame.context);
    Style::__markup_set_Selector(&style, Some((imports.selectors[a[1] as usize])()));
    frame.locals[a[0] as usize] = Local::Style(style);
    Ok(())
}

pub fn new_selector(frame: &mut Frame<'_>, imports: &Imports, a: &[u16; 4]) -> Result<(), XamlLoadError> {
    frame.locals[a[0] as usize] = Local::Selector((imports.selectors[a[1] as usize])());
    Ok(())
}

pub fn push_style(frame: &mut Frame<'_>, _: &Imports, a: &[u16; 4]) -> Result<(), XamlLoadError> {
    super::push_parent(frame.context, frame.locals[a[0] as usize].style());
    Ok(())
}

pub fn pop_parent(frame: &mut Frame<'_>, _: &Imports, _: &[u16; 4]) -> Result<(), XamlLoadError> {
    frame.context.pop_parent();
    Ok(())
}

pub fn set_selector(frame: &mut Frame<'_>, _: &Imports, a: &[u16; 4]) -> Result<(), XamlLoadError> {
    Style::__markup_set_Selector(
        frame.locals[a[0] as usize].style(),
        Some(frame.locals[a[1] as usize].selector().clone()),
    );
    Ok(())
}

pub fn literal_setter(frame: &mut Frame<'_>, imports: &Imports, a: &[u16; 4]) -> Result<(), XamlLoadError> {
    super::add_setter(frame.parent(a[0]), (imports.properties[a[1] as usize])(), (imports.values[a[2] as usize])());
    Ok(())
}

pub fn dynamic_setter(frame: &mut Frame<'_>, imports: &Imports, a: &[u16; 4]) -> Result<(), XamlLoadError> {
    super::add_dynamic_resource_setter_with_parent(
        frame.context,
        frame.parent(a[0]),
        (imports.properties[a[1] as usize])(),
        imports.texts[a[2] as usize],
    );
    Ok(())
}

pub fn attach(frame: &mut Frame<'_>, imports: &Imports, a: &[u16; 4]) -> Result<(), XamlLoadError> {
    let (line, column) = imports.positions[a[2] as usize];
    super::style_program::attach(frame.parent(a[0]), frame.locals[a[1] as usize].style(), line, column)
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

pub fn finish_dynamic(frame: &mut Frame<'_>, imports: &Imports, a: &[u16; 4]) -> Result<(), XamlLoadError> {
    let parent = frame.parent(a[0]);
    let child = frame.locals[a[1] as usize].style();
    dynamic_run(frame.context, child, imports, a[2]);
    frame.context.pop_parent();
    let (line, column) = imports.positions[a[3] as usize];
    super::style_program::attach(parent, child, line, column)
}

pub fn add_selected_style(frame: &mut Frame<'_>, imports: &Imports, a: &[u16; 4]) -> Result<(), XamlLoadError> {
    let parent = frame.parent(a[0]);
    // This fused helper owns its child. Keep its destructor inside this call,
    // before the next instruction, rather than retaining it in a caller slot.
    let style = super::style_program::begin(frame.context);
    Style::__markup_set_Selector(&style, Some((imports.selectors[a[1] as usize])()));
    dynamic_run(frame.context, &style, imports, a[2]);
    frame.context.pop_parent();
    let (line, column) = imports.positions[a[3] as usize];
    super::style_program::attach(parent, &style, line, column)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::data::core::ValueTypes;
    use ferroui_base::styling::{ControlTheme, Selectors, Setter, SetterValue};
    use ferroui_base::{StyledElement, WeakRef};
    use std::cell::RefCell;
    use std::panic::{catch_unwind, AssertUnwindSafe};
    use std::rc::Weak;

    use super::super as rt;

    struct Probe {
        id: u8,
        panic: bool,
        drops: Rc<RefCell<Vec<u8>>>,
    }

    impl PartialEq for Probe {
        fn eq(&self, other: &Self) -> bool { std::ptr::eq(self, other) }
    }

    impl Drop for Probe {
        fn drop(&mut self) {
            self.drops.borrow_mut().push(self.id);
            assert!(!self.panic, "region payload destructor");
        }
    }

    #[derive(Debug, PartialEq, Eq)]
    struct Observation {
        step: &'static str,
        parents: Vec<&'static str>,
        outer_is_parent: bool,
        target_is_last: bool,
    }

    struct State {
        context: Weak<XamlIlContext>,
        parent: WeakRef<StyleBase>,
        drops: Rc<RefCell<Vec<u8>>>,
        observations: Rc<RefCell<Vec<Observation>>>,
        panic_at_drop: Option<u8>,
        panic_in_value: bool,
    }

    ferroui_base::ferro_thread_local! {
        static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
    }

    fn probe(id: u8) -> Probe {
        STATE.with(|state| {
            let state = state.borrow();
            let state = state.as_ref().expect("region test state");
            Probe { id, panic: state.panic_at_drop == Some(id), drops: state.drops.clone() }
        })
    }

    fn property() -> &'static FerroProperty { StyledElement::data_context_property().as_property() }
    fn first_value() -> MarkupValue { Some(Rc::new(probe(1))) }
    fn last_value() -> MarkupValue { Some(Rc::new(probe(5))) }
    fn first_selector() -> Selector { Selectors::property_equals_untyped(None, property(), Rc::new(probe(2))) }
    fn last_selector() -> Selector { Selectors::property_equals_untyped(None, property(), Rc::new(probe(4))) }

    fn result_parent() -> Ref<ControlTheme> {
        let parent = ControlTheme::new();
        let setter = Rc::new(Setter::default());
        setter.set_value(Some(SetterValue::Value(Rc::new(probe(9)))));
        parent.upcast_ref::<StyleBase>().add_setter(setter);
        parent
    }

    fn fail(_: &mut Frame<'_>, _: &Imports, _: &[u16; 4]) -> Result<(), XamlLoadError> {
        Err(rt::at("TestException", "region construction failed", 7, 9))
    }

    fn panic_step(_: &mut Frame<'_>, _: &Imports, _: &[u16; 4]) -> Result<(), XamlLoadError> {
        panic!("region construction callback")
    }

    const CLEANUP: Program = Program {
        operations: &[new_style, new_selector, literal_setter, fail, panic_step],
        imports: Imports {
            properties: &[property], values: &[first_value, last_value], selectors: &[first_selector, last_selector],
            texts: &[], positions: &[], dynamic_runs: &[],
        },
    };
    const FIRST: &[Instruction] = &[
        Instruction { operation: 0, args: [1, 0, 0, 0] },
        Instruction { operation: 2, args: [1, 0, 0, 0] },
        Instruction { operation: 1, args: [0, 0, 0, 0] },
    ];
    const SECOND: &[Instruction] = &[
        Instruction { operation: 1, args: [1, 1, 0, 0] },
        Instruction { operation: 0, args: [0, 0, 0, 0] },
        Instruction { operation: 2, args: [0, 0, 1, 0] },
    ];
    const ERROR: &[Instruction] = &[Instruction { operation: 3, args: [0; 4] }];
    const PANIC: &[Instruction] = &[Instruction { operation: 4, args: [0; 4] }];

    fn original_cleanup(mode: u8) -> Result<MarkupValue, XamlLoadError> {
        let _before = probe(0);
        let _context = rt::create_context(None, None, &[]);
        let parent = result_parent();
        let first_style = Style::new();
        rt::add_setter(first_style.upcast_ref(), property(), first_value());
        let _first_selector = first_selector();
        let _between = probe(3);
        let _last_selector = last_selector();
        let last_style = Style::new();
        rt::add_setter(last_style.upcast_ref(), property(), last_value());
        let _after = probe(6);
        match mode {
            1 => return Err(rt::at("TestException", "region construction failed", 7, 9)),
            2 => panic!("region construction callback"),
            _ => {},
        }
        Ok(rt::to_value(parent.clone()))
    }

    fn programmed_cleanup(mode: u8) -> Result<MarkupValue, XamlLoadError> {
        let _before = probe(0);
        let context = rt::create_context(None, None, &[]);
        let parent = result_parent();
        let mut first = [const { Local::Empty }; 2];
        run(&context, parent.upcast_ref(), &CLEANUP, FIRST, &mut first)?;
        let _between = probe(3);
        let mut second = [const { Local::Empty }; 2];
        run(&context, parent.upcast_ref(), &CLEANUP, SECOND, &mut second)?;
        let _after = probe(6);
        run(&context, parent.upcast_ref(), &CLEANUP, match mode { 1 => ERROR, 2 => PANIC, _ => &[] }, &mut second)?;
        Ok(rt::to_value(parent.clone()))
    }

    fn set_state(
        context: &Rc<XamlIlContext>,
        parent: &Ref<ControlTheme>,
        drops: &Rc<RefCell<Vec<u8>>>,
        observations: &Rc<RefCell<Vec<Observation>>>,
        panic_at_drop: Option<u8>,
        panic_in_value: bool,
    ) {
        STATE.with(|state| *state.borrow_mut() = Some(State {
            context: Rc::downgrade(context),
            parent: parent.upcast_ref::<StyleBase>().downgrade(),
            drops: drops.clone(),
            observations: observations.clone(),
            panic_at_drop,
            panic_in_value,
        }));
    }

    #[test]
    fn regions_preserve_interleaved_local_and_return_payload_cleanup() {
        for (mode, panic_at_drop) in [(0, None), (1, None), (2, None), (0, Some(2)), (1, Some(2)), (0, Some(5)), (1, Some(5))] {
            let context = rt::create_context(None, None, &[]);
            let parent = ControlTheme::new();
            let expected = Rc::new(RefCell::new(Vec::new()));
            let actual = Rc::new(RefCell::new(Vec::new()));
            let observations = Rc::new(RefCell::new(Vec::new()));
            set_state(&context, &parent, &expected, &observations, panic_at_drop, false);
            let original = catch_unwind(AssertUnwindSafe(|| original_cleanup(mode)));
            set_state(&context, &parent, &actual, &observations, panic_at_drop, false);
            let programmed = catch_unwind(AssertUnwindSafe(|| programmed_cleanup(mode)));
            assert_eq!(programmed.is_err(), original.is_err(), "mode {mode}, drop {panic_at_drop:?}");
            if let (Ok(original), Ok(programmed)) = (&original, &programmed) {
                assert_eq!(programmed.is_ok(), original.is_ok());
            }
            assert_eq!(*actual.borrow(), *expected.borrow(), "before result drop: mode {mode}, drop {panic_at_drop:?}");
            if mode == 0 && panic_at_drop.is_none() {
                assert_eq!(*expected.borrow(), [6, 5, 4, 3, 2, 1, 0]);
                assert!(matches!(&original, Ok(Ok(Some(_)))));
                assert!(matches!(&programmed, Ok(Ok(Some(_)))));
            }
            drop(original);
            drop(programmed);
            assert_eq!(*actual.borrow(), *expected.borrow(), "after result drop: mode {mode}, drop {panic_at_drop:?}");
            if mode == 0 && panic_at_drop.is_none() {
                assert_eq!(*expected.borrow(), [6, 5, 4, 3, 2, 1, 0, 9]);
            }
            // A panicking destructor can affect an initialized return place;
            // the exact original Rust expression, not an assumed order, is the
            // oracle. The state holds no strong context/style/selector owner.
            STATE.with(|state| state.borrow_mut().take());
        }
    }

    fn observe(step: &'static str) {
        STATE.with(|state| {
            let state = state.borrow();
            let state = state.as_ref().expect("region test state");
            let context = state.context.upgrade().expect("live test context");
            let parent = state.parent.upgrade().expect("live external parent");
            let stack = context.parents_stack();
            let objects: Vec<_> = stack.iter().map(|value| {
                ValueTypes::as_object(value.as_deref().expect("object parent")).expect("class parent")
            }).collect();
            let target = context.target_object().as_deref().and_then(ValueTypes::as_object);
            let observation = Observation {
                step,
                parents: objects.iter().map(|object| {
                    if object.is::<ControlTheme>() { "theme" } else if object.is::<Style>() { "style" } else { "other" }
                }).collect(),
                outer_is_parent: objects.first().is_some_and(|object| object.ptr_eq(&parent)),
                target_is_last: objects.last().zip(target.as_ref()).is_some_and(|(last, target)| last.ptr_eq(target)),
            };
            state.observations.borrow_mut().push(observation);
            // All snapshot handles die before the lazy import returns. They
            // must not hide a changed lifetime in the construction sequence.
        });
    }

    fn observed_property() -> &'static FerroProperty {
        observe("property");
        property()
    }

    fn observed_second_property() -> &'static FerroProperty {
        observe("second property");
        property()
    }

    fn observed_value() -> MarkupValue {
        observe("value");
        let panic = STATE.with(|state| state.borrow().as_ref().unwrap().panic_in_value);
        assert!(!panic, "lazy value callback");
        Some(Rc::new(31_i32))
    }

    fn observed_selector() -> Selector {
        observe("selector");
        Selectors::nesting(None)
    }

    const OBSERVED: Program = Program {
        operations: &[begin_style, new_selector, set_selector, literal_setter, pop_parent, attach],
        imports: Imports {
            properties: &[observed_property], values: &[observed_value], selectors: &[observed_selector],
            texts: &[], positions: &[(7, 9)], dynamic_runs: &[],
        },
    };
    const OBSERVED_STEPS: &[Instruction] = &[
        Instruction { operation: 0, args: [1, 0, 0, 0] },
        Instruction { operation: 1, args: [0, 0, 0, 0] },
        Instruction { operation: 2, args: [1, 0, 0, 0] },
        Instruction { operation: 3, args: [1, 0, 0, 0] },
        Instruction { operation: 3, args: [EXTERNAL_PARENT, 0, 0, 0] },
        Instruction { operation: 4, args: [0; 4] },
        Instruction { operation: 5, args: [EXTERNAL_PARENT, 1, 0, 0] },
    ];

    fn original_observed(context: &Rc<XamlIlContext>, parent: &Ref<StyleBase>) -> Result<(), XamlLoadError> {
        let style = rt::style_program::begin(context);
        let selector = observed_selector();
        Style::__markup_set_Selector(&style, Some(selector.clone()));
        rt::add_setter(style.upcast_ref(), observed_property(), observed_value());
        rt::add_setter(parent, observed_property(), observed_value());
        context.pop_parent();
        rt::style_program::attach(parent, &style, 7, 9)?;
        Ok(())
    }

    fn programmed_observed(context: &Rc<XamlIlContext>, parent: &Ref<StyleBase>) -> Result<(), XamlLoadError> {
        let mut locals = [const { Local::Empty }; 2];
        run(context, parent, &OBSERVED, OBSERVED_STEPS, &mut locals)?;
        Ok(())
    }

    fn observation_case(
        build: fn(&Rc<XamlIlContext>, &Ref<StyleBase>) -> Result<(), XamlLoadError>,
        panic_in_value: bool,
    ) -> (bool, Vec<Observation>) {
        let context = rt::create_context(None, None, &[]);
        let parent = ControlTheme::new();
        let observations = Rc::new(RefCell::new(Vec::new()));
        set_state(&context, &parent, &Rc::new(RefCell::new(Vec::new())), &observations, None, panic_in_value);
        rt::push_parent(&context, &parent);
        let result = catch_unwind(AssertUnwindSafe(|| build(&context, parent.upcast_ref())));
        if let Ok(result) = &result { assert!(result.is_ok()); }
        observe("after");
        let panicked = result.is_err();
        drop(result);
        // Test cleanup follows observation. Neither executor compensates for a
        // failed construction by popping a parent that original Rust retained.
        while !context.parents_stack().is_empty() { context.pop_parent(); }
        STATE.with(|state| state.borrow_mut().take());
        let observations = std::mem::take(&mut *observations.borrow_mut());
        (panicked, observations)
    }

    #[test]
    fn regions_preserve_parent_stack_and_lazy_import_order_on_value_panic() {
        for panic_in_value in [false, true] {
            let expected = observation_case(original_observed, panic_in_value);
            let actual = observation_case(programmed_observed, panic_in_value);
            assert_eq!(actual, expected);
            assert_eq!(actual.0, panic_in_value);
            let steps: Vec<_> = actual.1.iter().map(|observation| observation.step).collect();
            assert_eq!(steps, if panic_in_value {
                vec!["selector", "property", "value", "after"]
            } else {
                vec!["selector", "property", "value", "property", "value", "after"]
            });
            for observation in &actual.1 {
                assert!(observation.outer_is_parent && observation.target_is_last);
                assert_eq!(observation.parents, if observation.step == "after" && !panic_in_value {
                    vec!["theme"]
                } else {
                    vec!["theme", "style"]
                });
            }
        }
    }

    const DYNAMIC: Program = Program {
        operations: &[selected_style, new_selector, finish_dynamic, add_selected_style],
        imports: Imports {
            properties: &[observed_property, observed_second_property], values: &[], selectors: &[observed_selector],
            texts: &["FirstKey", "SecondKey"], positions: &[(7, 9)], dynamic_runs: &[&[[0, 0], [1, 1]]],
        },
    };
    const DYNAMIC_STEPS: &[Instruction] = &[
        Instruction { operation: 0, args: [1, 0, 0, 0] },
        Instruction { operation: 1, args: [0, 0, 0, 0] },
        Instruction { operation: 2, args: [EXTERNAL_PARENT, 1, 0, 0] },
        Instruction { operation: 3, args: [EXTERNAL_PARENT, 0, 0, 0] },
    ];

    fn original_dynamic(context: &Rc<XamlIlContext>, parent: &Ref<StyleBase>) -> Result<(), XamlLoadError> {
        let style = {
            let style = rt::style_program::begin(context);
            Style::__markup_set_Selector(&style, Some(observed_selector()));
            style
        };
        let _selector = observed_selector();
        rt::style_program::finish_dynamic(
            context, parent, &style,
            &[(observed_property, "FirstKey"), (observed_second_property, "SecondKey")], 7, 9,
        )?;
        {
            let style = rt::style_program::begin(context);
            Style::__markup_set_Selector(&style, Some(observed_selector()));
            rt::style_program::finish_dynamic(
                context, parent, &style,
                &[(observed_property, "FirstKey"), (observed_second_property, "SecondKey")], 7, 9,
            )
        }?;
        Ok(())
    }

    fn programmed_dynamic(context: &Rc<XamlIlContext>, parent: &Ref<StyleBase>) -> Result<(), XamlLoadError> {
        let mut locals = [const { Local::Empty }; 2];
        run(context, parent, &DYNAMIC, DYNAMIC_STEPS, &mut locals)?;
        Ok(())
    }

    #[test]
    fn region_dynamic_runs_preserve_getter_order_and_parent_visibility() {
        let expected = observation_case(original_dynamic, false);
        let actual = observation_case(programmed_dynamic, false);
        assert_eq!(actual, expected);
        assert!(!actual.0);
        let steps: Vec<_> = actual.1.iter().map(|observation| observation.step).collect();
        assert_eq!(steps, ["selector", "selector", "property", "second property", "selector", "property", "second property", "after"]);
        for observation in &actual.1 {
            assert!(observation.outer_is_parent && observation.target_is_last);
            assert_eq!(observation.parents, if observation.step == "after" { vec!["theme"] } else { vec!["theme", "style"] });
        }
    }
}
