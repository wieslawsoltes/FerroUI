//! The runtime effects of the Skia backend: SkSL programs compiled once and
//! given their parameters as uniforms on each draw.
//!
//! This is the layer the GPU forms of WinUI's composition effects are built
//! on (the lights and the pixel operations, docs/porting/winui-composition.md,
//! "How the effects are drawn"), and the one user-defined effects are to be
//! built on:
//!
//! - [`compile`] compiles an SkSL program as a shader, a blender or a colour
//!   filter ([`ProgramKind`]) once per thread: the compiled effects are kept
//!   in a cache keyed by the kind and the source text, a failure included
//!   (it is logged once and not compiled again). [`RuntimeProgram`] is a
//!   program of the backend itself, a static source with a name.
//! - [`Uniforms`] builds the uniform block of an effect by name and checks
//!   each value against the uniforms the program declares: the name, the
//!   type (`float` to `float4`, `float2x2` to `float4x4`, `int` to `int4`)
//!   and the number of values (arrays included). A uniform left unset is an
//!   error too, so a program and its caller cannot drift apart silently.
//! - [`make_shader`], [`make_blender`] and [`make_color_filter`] make the
//!   Skia object of a compiled effect with its uniforms and its children
//!   (shaders, colour filters and blenders, checked against the children
//!   the program declares).
//!
//! The fallback is the caller's: every step returns `None` (or an error)
//! when the program does not compile, a uniform or child does not match, or
//! runtime effects are switched off ([`set_enabled`], used by the tests to
//! draw the CPU reference with the same drawing context). The callers in
//! the drawing context then report that they did not draw, and the
//! compositor draws the same on the CPU. Runtime effects run on every
//! surface of the backend: raster, Ganesh (OpenGL, WebGL, Vulkan, Metal) and
//! Graphite; they are recorded into pictures as well.
//!
//! The programs use `float` (full precision) for every value they compute:
//! `half` may be 16 bits on mobile GPUs.

use ferroui_base::logging::{LogEventLevel, Logger};
use skia_safe as sk;
use skia_safe::runtime_effect::{uniform, ChildPtr, ChildType};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fmt;

/// What an SkSL program is compiled as.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum ProgramKind {
    /// `half4 main(float2 coord)`: a shader, given the local coordinates of
    /// the point it colours.
    Shader,
    /// `half4 main(half4 src, half4 dst)`: a blender.
    Blender,
    /// `half4 main(half4 color)`: a colour filter.
    #[allow(dead_code)]
    ColorFilter,
}

/// An SkSL program of the backend: a static source, compiled on first use.
pub(crate) struct RuntimeProgram {
    /// A name for the log.
    pub(crate) name: &'static str,
    /// What the program is.
    pub(crate) kind: ProgramKind,
    /// The SkSL source.
    pub(crate) source: &'static str,
}

impl RuntimeProgram {
    /// The compiled program; `None` when it does not compile or runtime
    /// effects are off. Found by the address of its static source, without
    /// hashing the text.
    pub(crate) fn effect(&self) -> Option<sk::RuntimeEffect> {
        if !is_enabled() {
            return None;
        }
        let key = (self.kind, self.source.as_ptr() as usize, self.source.len());
        if let Some(compiled) = STATIC_CACHE.with(|cache| cache.borrow().get(&key).cloned()) {
            return compiled;
        }
        let compiled = compile_named(self.kind, self.source, self.name);
        STATIC_CACHE.with(|cache| cache.borrow_mut().insert(key, compiled.clone()));
        compiled
    }
}

ferroui_base::ferro_thread_local! {
    /// The compiled effects of this thread, by kind and source, failures
    /// included.
    static CACHE: RefCell<[HashMap<String, Option<sk::RuntimeEffect>>; 3]> =
        RefCell::new([HashMap::new(), HashMap::new(), HashMap::new()]);
    /// The compiled programs of the backend, by kind and the address and
    /// length of their static source.
    static STATIC_CACHE: RefCell<HashMap<(ProgramKind, usize, usize), Option<sk::RuntimeEffect>>> =
        RefCell::new(HashMap::new());
    static ENABLED: Cell<bool> = Cell::new(true);
}

/// Switches the runtime effects of this thread on or off; off, nothing is
/// compiled and every caller falls back to the CPU path. Returns the
/// previous state.
#[allow(dead_code)]
pub(crate) fn set_enabled(enabled: bool) -> bool {
    ENABLED.with(|cell| cell.replace(enabled))
}

/// Whether the runtime effects of this thread are on.
pub(crate) fn is_enabled() -> bool {
    ENABLED.with(Cell::get)
}

/// Compiles `source` as `kind`, once per thread; `None` when it does not
/// compile (logged once) or runtime effects are off.
#[allow(dead_code)]
pub(crate) fn compile(kind: ProgramKind, source: &str) -> Option<sk::RuntimeEffect> {
    compile_named(kind, source, "runtime effect")
}

fn compile_named(kind: ProgramKind, source: &str, name: &str) -> Option<sk::RuntimeEffect> {
    if !is_enabled() {
        return None;
    }
    let index = kind as usize;
    if let Some(compiled) = CACHE.with(|cache| cache.borrow()[index].get(source).cloned()) {
        return compiled;
    }
    let result = match kind {
        ProgramKind::Shader => sk::RuntimeEffect::make_for_shader(source, None),
        ProgramKind::Blender => sk::RuntimeEffect::make_for_blender(source, None),
        ProgramKind::ColorFilter => sk::RuntimeEffect::make_for_color_filter(source, None),
    };
    let compiled = match result {
        Ok(effect) => Some(effect),
        Err(error) => {
            if let Some(logger) = Logger::try_get(LogEventLevel::Error, "Skia") {
                logger.log(None, &format!("The SkSL program {name} does not compile: {error}"));
            }
            None
        }
    };
    CACHE.with(|cache| cache.borrow_mut()[index].insert(source.to_owned(), compiled.clone()));
    compiled
}

/// Why a uniform block or the children of an effect do not match its
/// program.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum UniformError {
    /// The program declares no uniform of the name.
    Unknown(String),
    /// The uniform is of another kind (float or int) than the value.
    WrongType(String),
    /// The number of values is not that of the uniform.
    WrongCount { name: String, expected: usize, given: usize },
    /// A uniform was given no value.
    Missing(String),
    /// The children are not those of the program.
    Children,
}

impl fmt::Display for UniformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UniformError::Unknown(name) => write!(f, "no uniform {name}"),
            UniformError::WrongType(name) => write!(f, "the uniform {name} is of another type"),
            UniformError::WrongCount { name, expected, given } => {
                write!(f, "the uniform {name} takes {expected} values, {given} given")
            }
            UniformError::Missing(name) => write!(f, "the uniform {name} has no value"),
            UniformError::Children => write!(f, "the children are not those of the program"),
        }
    }
}

/// The number of scalars of a uniform type and whether they are floats.
fn components(ty: uniform::Type) -> (usize, bool) {
    use uniform::Type;
    match ty {
        Type::Float => (1, true),
        Type::Float2 => (2, true),
        Type::Float3 => (3, true),
        Type::Float4 => (4, true),
        Type::Float2x2 => (4, true),
        Type::Float3x3 => (9, true),
        Type::Float4x4 => (16, true),
        Type::Int => (1, false),
        Type::Int2 => (2, false),
        Type::Int3 => (3, false),
        Type::Int4 => (4, false),
    }
}

/// The uniform block of a runtime effect, built by name and checked against
/// the uniforms its program declares. The values of a uniform are given in
/// the order of its components (a matrix column by column, as SkSL stores
/// it), an array element after element.
pub(crate) struct Uniforms {
    effect: sk::RuntimeEffect,
    data: Vec<u8>,
    set: Vec<bool>,
    error: Option<UniformError>,
}

impl Uniforms {
    /// An empty block for `effect`.
    pub(crate) fn new(effect: &sk::RuntimeEffect) -> Uniforms {
        Uniforms {
            effect: effect.clone(),
            data: vec![0; effect.uniform_size()],
            set: vec![false; effect.uniforms().len()],
            error: None,
        }
    }

    fn write(&mut self, name: &str, float: bool, values: &[[u8; 4]]) -> &mut Self {
        if self.error.is_some() {
            return self;
        }
        let Some(index) = self.effect.uniforms().iter().position(|uniform| uniform.name() == name) else {
            self.error = Some(UniformError::Unknown(name.to_owned()));
            return self;
        };
        let uniform = &self.effect.uniforms()[index];
        let (per_element, is_float) = components(uniform.ty());
        if is_float != float {
            self.error = Some(UniformError::WrongType(name.to_owned()));
            return self;
        }
        let expected = per_element * uniform.count().max(1) as usize;
        if values.len() != expected {
            self.error = Some(UniformError::WrongCount { name: name.to_owned(), expected, given: values.len() });
            return self;
        }
        let offset = uniform.offset();
        for (at, value) in values.iter().enumerate() {
            self.data[offset + at * 4..offset + at * 4 + 4].copy_from_slice(value);
        }
        self.set[index] = true;
        self
    }

    /// Sets a `float` uniform.
    pub(crate) fn float(&mut self, name: &str, value: f32) -> &mut Self {
        self.floats(name, &[value])
    }

    /// Sets a `float2` uniform.
    #[allow(dead_code)]
    pub(crate) fn float2(&mut self, name: &str, value: [f32; 2]) -> &mut Self {
        self.floats(name, &value)
    }

    /// Sets a `float3` uniform.
    #[allow(dead_code)]
    pub(crate) fn float3(&mut self, name: &str, value: [f32; 3]) -> &mut Self {
        self.floats(name, &value)
    }

    /// Sets a `float4` uniform.
    pub(crate) fn float4(&mut self, name: &str, value: [f32; 4]) -> &mut Self {
        self.floats(name, &value)
    }

    /// Sets a float uniform of any type and count: a vector, a matrix
    /// (column by column) or an array of them (element after element).
    pub(crate) fn floats(&mut self, name: &str, values: &[f32]) -> &mut Self {
        let bytes: Vec<[u8; 4]> = values.iter().map(|value| value.to_ne_bytes()).collect();
        self.write(name, true, &bytes)
    }

    /// Sets an `int` uniform.
    pub(crate) fn int(&mut self, name: &str, value: i32) -> &mut Self {
        self.ints(name, &[value])
    }

    /// Sets an int uniform of any type and count.
    pub(crate) fn ints(&mut self, name: &str, values: &[i32]) -> &mut Self {
        let bytes: Vec<[u8; 4]> = values.iter().map(|value| value.to_ne_bytes()).collect();
        self.write(name, false, &bytes)
    }

    /// The block, once every uniform has its value.
    pub(crate) fn finish(&self) -> Result<sk::Data, UniformError> {
        if let Some(error) = &self.error {
            return Err(error.clone());
        }
        if let Some(index) = self.set.iter().position(|set| !set) {
            return Err(UniformError::Missing(self.effect.uniforms()[index].name().to_owned()));
        }
        Ok(sk::Data::new_copy(&self.data))
    }
}

/// Whether `children` are the children `effect` declares, in order.
fn children_match(effect: &sk::RuntimeEffect, children: &[ChildPtr]) -> bool {
    let declared = effect.children();
    declared.len() == children.len()
        && declared.iter().zip(children).all(|(declared, child)| match declared.ty() {
            ChildType::Shader => matches!(child, ChildPtr::Shader(_)),
            ChildType::ColorFilter => matches!(child, ChildPtr::ColorFilter(_)),
            ChildType::Blender => matches!(child, ChildPtr::Blender(_)),
        })
}

fn log_mismatch(error: &UniformError) {
    if let Some(logger) = Logger::try_get(LogEventLevel::Error, "Skia") {
        logger.log(None, &format!("A runtime effect was given parameters it does not declare: {error}"));
    }
}

/// The shader of a shader effect with its uniforms and children; `None`
/// when they do not match its program.
pub(crate) fn make_shader(
    effect: &sk::RuntimeEffect,
    uniforms: &Uniforms,
    children: &[ChildPtr],
    local_matrix: Option<&sk::Matrix>,
) -> Option<sk::Shader> {
    let data = uniforms.finish().map_err(|error| log_mismatch(&error)).ok()?;
    if !children_match(effect, children) {
        log_mismatch(&UniformError::Children);
        return None;
    }
    effect.make_shader(data, children, local_matrix)
}

/// The blender of a blender effect with its uniforms and children; `None`
/// when they do not match its program.
pub(crate) fn make_blender(effect: &sk::RuntimeEffect, uniforms: &Uniforms, children: &[ChildPtr]) -> Option<sk::Blender> {
    let data = uniforms.finish().map_err(|error| log_mismatch(&error)).ok()?;
    if !children_match(effect, children) {
        log_mismatch(&UniformError::Children);
        return None;
    }
    effect.make_blender(data, children)
}

/// The colour filter of a colour filter effect with its uniforms and
/// children; `None` when they do not match its program.
#[allow(dead_code)]
pub(crate) fn make_color_filter(
    effect: &sk::RuntimeEffect,
    uniforms: &Uniforms,
    children: &[ChildPtr],
) -> Option<sk::ColorFilter> {
    let data = uniforms.finish().map_err(|error| log_mismatch(&error)).ok()?;
    if !children_match(effect, children) {
        log_mismatch(&UniformError::Children);
        return None;
    }
    effect.make_color_filter(data, children)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TINT: RuntimeProgram = RuntimeProgram {
        name: "tint",
        kind: ProgramKind::Shader,
        source: "uniform float4 color; uniform float amounts[3]; uniform int count;\n\
                 half4 main(float2 p) { return half4(color * amounts[2] * float(count)); }",
    };

    #[test]
    fn a_program_is_compiled_once_and_its_failure_is_kept() {
        let first = TINT.effect().expect("compiles");
        let second = TINT.effect().expect("compiles");
        assert_eq!(first.uniform_size(), second.uniform_size());
        assert!(compile(ProgramKind::Shader, "half4 main(float2 p) { return undefined; }").is_none());
        assert!(compile(ProgramKind::Shader, "half4 main(float2 p) { return undefined; }").is_none());
        // A shader is not a blender.
        assert!(compile(ProgramKind::Blender, TINT.source).is_none());
    }

    #[test]
    fn uniforms_are_checked_against_the_program() {
        let effect = TINT.effect().unwrap();
        let mut uniforms = Uniforms::new(&effect);
        uniforms.float4("color", [1.0, 0.5, 0.25, 1.0]).floats("amounts", &[0.0, 0.5, 1.0]).int("count", 2);
        let data = uniforms.finish().expect("every uniform set");
        assert_eq!(effect.uniform_size(), data.len());
        assert!(make_shader(&effect, &uniforms, &[], None).is_some());

        let mut missing = Uniforms::new(&effect);
        missing.float4("color", [1.0; 4]).int("count", 0);
        assert_eq!(Err(UniformError::Missing("amounts".into())), missing.finish());

        let mut wrong = Uniforms::new(&effect);
        wrong.floats("amounts", &[1.0, 2.0]);
        assert_eq!(Err(UniformError::WrongCount { name: "amounts".into(), expected: 3, given: 2 }), wrong.finish());

        let mut wrong = Uniforms::new(&effect);
        wrong.float("count", 1.0);
        assert_eq!(Err(UniformError::WrongType("count".into())), wrong.finish());

        let mut wrong = Uniforms::new(&effect);
        wrong.float("nothing", 1.0);
        assert_eq!(Err(UniformError::Unknown("nothing".into())), wrong.finish());
        // A child the program does not declare.
        let child = sk::shaders::color(sk::Color::RED);
        assert!(make_shader(&effect, &uniforms, &[ChildPtr::Shader(child)], None).is_none());
    }

    #[test]
    fn switched_off_nothing_compiles() {
        let previous = set_enabled(false);
        assert!(TINT.effect().is_none());
        set_enabled(previous);
        assert!(TINT.effect().is_some());
    }
}
