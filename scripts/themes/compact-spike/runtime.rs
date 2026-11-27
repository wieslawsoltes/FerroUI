// Included in the generated theme module by the experiment.
use super::rt;
use ferroui_base::metadata::MarkupValue;
use ferroui_base::styling::{ControlTheme, Setter, SetterBase, Style, StyleBase};
use ferroui_base::{CornerRadius, FerroObject, Ref, StaticType, Thickness};
use ferroui_markup_xaml::xaml_il::runtime::XamlIlContext;
use ferroui_markup_xaml::{
    markup_extensions::StaticResourceExtension, templates::ControlTemplate, XamlLoadError,
};
use rt::*;
use std::rc::Rc;

pub(super) struct Program {
    pub parent: bool,
    pub words: &'static [Word],
    pub properties: &'static [PropertyFn],
    pub strings: &'static [&'static str],
    pub selectors: &'static [&'static [SelectorStep]],
    pub native: &'static [StyleMember],
}

struct Cursor(&'static [Word]);

impl Cursor {
    fn take(&mut self) -> usize {
        let (value, rest) = self
            .0
            .split_first()
            .expect("compiler emitted a truncated program");
        self.0 = rest;
        usize::from(*value)
    }
}

#[inline(never)]
pub(super) fn build(
    context: &Rc<XamlIlContext>,
    program: &'static Program,
) -> Result<MarkupValue, XamlLoadError> {
    let theme = ControlTheme::new();
    if program.parent {
        context.push_parent(to_value(theme.clone()));
    }
    let mut cursor = Cursor(program.words);
    members(
        context,
        &theme,
        theme.upcast_ref::<StyleBase>(),
        program,
        &mut cursor,
    )?;
    debug_assert!(cursor.0.is_empty());
    if program.parent {
        context.pop_parent();
    }
    Ok(to_value(theme))
}

fn members(
    context: &Rc<XamlIlContext>,
    theme: &Ref<ControlTheme>,
    style: &Ref<StyleBase>,
    program: &Program,
    cursor: &mut Cursor,
) -> Result<(), XamlLoadError> {
    loop {
        match cursor.take() {
            0 => return Ok(()),
            1 => match &program.native[cursor.take()] {
                StyleMember::TargetType(t) => {
                    ControlTheme::__markup_set_TargetType(theme, Some(*t))
                }
                StyleMember::BasedOn {
                    key,
                    line,
                    position,
                } => based_on(context, theme, *key, *line, *position)?,
                StyleMember::Setter(setter) => add_theme_setter(context, style, setter)?,
                StyleMember::Style { .. } => unreachable!(),
            },
            2 => {
                let property = program.properties[cursor.take()];
                let key = program.strings[cursor.take()];
                add_dynamic_resource_setter_with_parent(context, style, property(), key);
            }
            3 => {
                let parent = cursor.take() != 0;
                let steps = program.selectors[cursor.take()];
                let line = cursor.take() as i32;
                let position = cursor.take() as i32;
                let nested = Style::new();
                if parent {
                    context.push_parent(to_value(nested.clone()));
                }
                Style::__markup_set_Selector(&nested, build_selector(steps));
                nested.enable_selector_index();
                members(
                    context,
                    theme,
                    nested.upcast_ref::<StyleBase>(),
                    program,
                    cursor,
                )?;
                if parent {
                    context.pop_parent();
                }
                StyleBase::__markup_Add_1(style, cast(nested, line, position)?);
            }
            4 => {
                let property = program.properties[cursor.take()];
                let key = program.strings[cursor.take()];
                let form = match cursor.take() {
                    0 => SetterForm::Plain,
                    1 => SetterForm::Created,
                    2 => SetterForm::Parented,
                    _ => unreachable!(),
                };
                let line = cursor.take() as i32;
                let position = cursor.take() as i32;
                add_theme_setter(
                    context,
                    style,
                    &ThemeSetter {
                        property,
                        form,
                        value: ThemeValue::Static {
                            key,
                            line,
                            position,
                        },
                    },
                )?;
            }
            _ => unreachable!("unknown style instruction"),
        }
    }
}

pub(super) struct Bindings {
    pub properties: &'static [PropertyFn],
    pub words: &'static [Word],
}

#[inline(never)]
pub(super) fn bind_rows(
    object: &Ref<FerroObject>,
    rows: &'static Bindings,
) -> Result<(), XamlLoadError> {
    let mut cursor = Cursor(rows.words);
    while !cursor.0.is_empty() {
        let target = rows.properties[cursor.take()]();
        let source = rows.properties[cursor.take()]();
        let line = cursor.take() as i32;
        let position = cursor.take() as i32;
        bind_template_binding(object, target, source, line, position)?;
    }
    Ok(())
}

pub(super) struct Aliases {
    pub strings: &'static [&'static str],
    pub namespaces: Option<XmlNamespaceTable>,
    pub words: &'static [Word],
    pub count: usize,
    pub cache: std::sync::OnceLock<Box<[ResourceAliasRow]>>,
}

impl Aliases {
    #[inline(never)]
    pub(super) fn rows(&'static self) -> &'static [ResourceAliasRow] {
        self.cache.get_or_init(|| {
            let mut input = Cursor(self.words);
            let mut rows = Vec::with_capacity(self.count);
            for _ in 0..self.count {
                let key = self.strings[input.take()];
                let target = self.strings[input.take()];
                let line = input.take() as i32;
                let position = input.take() as i32;
                rows.push(ResourceAliasRow {
                    key,
                    target,
                    namespaces: self.namespaces,
                    line,
                    position,
                    value_line: line,
                    value_position: position,
                });
            }
            debug_assert!(input.0.is_empty());
            rows.into_boxed_slice()
        })
    }
}
