//! The GLSL ES 3.0 of the runtime shader effects (#455, E5; decision 2 of
//! docs/porting/custom-effects.md): the WebGL2 renderer of the browser has no
//! wgpu device, so the WGSL module of the pass of a shader effect
//! (`ShaderEffectDescription::wgsl_module` of `ferroui-base`, the module
//! Vello's hybrid and GPU modes compile) is translated by naga on the build
//! host, at build time, and written to `OUT_DIR`. The author writes no third
//! program, and the browser module carries no translator.
//!
//! ```ignore
//! // build.rs of a crate whose effects run in the browser
//! fn main() {
//!     ferroui_build::write_shader_effect_glsl::<my_effects::RippleEffect>().unwrap();
//! }
//!
//! // the crate
//! const RIPPLE_FRAGMENT: &str = include_str!(concat!(env!("OUT_DIR"), "/shader_effects/ripple.frag.glsl"));
//! ```
//!
//! Each stage is a program of its own (`#version 300 es`): the vertex stage
//! draws a triangle over the target, the fragment stage is the effect. The
//! names of the uniform blocks and of the texture of the content in the
//! fragment stage are naga's (they are listed in [`ShaderEffectGlsl`]), with
//! the bindings of the WGSL module (`SHADER_WGSL_*_BINDING`). The
//! coordinates are those of naga's GLSL back end with its default flags (the
//! vertex output adjusted to the clip space of OpenGL).
//!
//! The browser renderer does not draw these programs yet (step 3 of the
//! Vello route of docs/porting/gpu-rendering.md): it draws the shader effects
//! by their CPU references until its sink paints with framebuffer textures.

use ferroui_base::media::effects::{
    CustomEffect, ImmutableCustomEffect, SHADER_WGSL_FRAGMENT_ENTRY, SHADER_WGSL_VERTEX_ENTRY,
};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

/// The two stages of the pass of a shader effect in GLSL ES 3.0 (WebGL2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShaderEffectGlsl {
    /// The vertex stage: a triangle over the target.
    pub vertex: String,
    /// The fragment stage: the program of the effect and the pass.
    pub fragment: String,
    /// The uniform blocks of the fragment stage: the binding of the WGSL
    /// module and the name of the block in the GLSL.
    pub uniform_blocks: Vec<(u32, String)>,
    /// The textures of the fragment stage (the content): the binding of the
    /// WGSL module and the name of the sampler in the GLSL.
    pub textures: Vec<(u32, String)>,
}

/// Why the WGSL module of a shader effect was not translated or written.
#[derive(Debug)]
#[non_exhaustive]
pub enum ShaderEffectGlslError {
    /// The module does not parse (naga's report).
    Parse(String),
    /// The module is not valid (naga's report).
    Validation(String),
    /// naga's GLSL back end refused a stage.
    Translation(String),
    /// `OUT_DIR` is not set: not in a build script.
    NoOutDir,
    /// A file could not be written.
    Io(std::io::Error),
}

impl fmt::Display for ShaderEffectGlslError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(report) => write!(f, "the WGSL of the shader effect does not parse: {report}"),
            Self::Validation(report) => write!(f, "the WGSL of the shader effect is not valid: {report}"),
            Self::Translation(report) => write!(f, "the WGSL of the shader effect has no GLSL ES 3.0: {report}"),
            Self::NoOutDir => f.write_str("OUT_DIR is not set (the GLSL of a shader effect is written by a build script)"),
            Self::Io(error) => write!(f, "the GLSL of the shader effect was not written: {error}"),
        }
    }
}

impl std::error::Error for ShaderEffectGlslError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<std::io::Error> for ShaderEffectGlslError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

/// The version of the output: GLSL ES 3.0 for WebGL2.
const VERSION: naga::back::glsl::Version = naga::back::glsl::Version::Embedded { version: 300, is_webgl: true };

/// Translates the WGSL module of the pass of a shader effect
/// (`ShaderEffectDescription::wgsl_module`) into GLSL ES 3.0: parsed and
/// validated by naga (without optional capabilities, as Vello checks it),
/// then each entry point written by naga's GLSL back end.
pub fn translate_shader_effect_module(wgsl_module: &str) -> Result<ShaderEffectGlsl, ShaderEffectGlslError> {
    let module = naga::front::wgsl::parse_str(wgsl_module)
        .map_err(|error| ShaderEffectGlslError::Parse(error.emit_to_string(wgsl_module)))?;
    let mut validator =
        naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::empty());
    let info = validator
        .validate(&module)
        .map_err(|error| ShaderEffectGlslError::Validation(error.emit_to_string(wgsl_module)))?;
    let (vertex, _) = write_stage(&module, &info, naga::ShaderStage::Vertex, SHADER_WGSL_VERTEX_ENTRY)?;
    let (fragment, reflection) = write_stage(&module, &info, naga::ShaderStage::Fragment, SHADER_WGSL_FRAGMENT_ENTRY)?;

    let binding = |handle: naga::Handle<naga::GlobalVariable>| module.global_variables[handle].binding.map(|binding| binding.binding);
    let mut uniform_blocks: Vec<(u32, String)> = reflection
        .uniforms
        .iter()
        .filter_map(|(handle, name)| Some((binding(*handle)?, name.clone())))
        .collect();
    uniform_blocks.sort();
    let mut textures: Vec<(u32, String)> = reflection
        .texture_mapping
        .iter()
        .filter_map(|(name, mapping)| Some((binding(mapping.texture)?, name.clone())))
        .collect();
    textures.sort();
    Ok(ShaderEffectGlsl { vertex, fragment, uniform_blocks, textures })
}

/// One entry point of `module` in GLSL ES 3.0, with naga's reflection.
fn write_stage(
    module: &naga::Module,
    info: &naga::valid::ModuleInfo,
    stage: naga::ShaderStage,
    entry_point: &str,
) -> Result<(String, naga::back::glsl::ReflectionInfo), ShaderEffectGlslError> {
    let options = naga::back::glsl::Options { version: VERSION, ..naga::back::glsl::Options::default() };
    let pipeline_options =
        naga::back::glsl::PipelineOptions { shader_stage: stage, entry_point: entry_point.to_owned(), multiview: None };
    let mut output = String::new();
    let reflection = {
        let mut writer = naga::back::glsl::Writer::new(
            &mut output,
            module,
            info,
            &options,
            &pipeline_options,
            naga::proc::BoundsCheckPolicies::default(),
        )
        .map_err(|error| ShaderEffectGlslError::Translation(format!("{entry_point}: {error}")))?;
        writer.write().map_err(|error| ShaderEffectGlslError::Translation(format!("{entry_point}: {error}")))?
    };
    Ok((output, reflection))
}

/// The GLSL ES 3.0 of the shader of the custom effect type `T`; `None` for a
/// type without a shader (or with a graph, which takes precedence).
pub fn shader_effect_glsl<T: CustomEffect>() -> Option<Result<ShaderEffectGlsl, ShaderEffectGlslError>> {
    let description = ImmutableCustomEffect::new(T::default()).shader()?;
    Some(translate_shader_effect_module(&description.wgsl_module()))
}

/// The files of the GLSL of a shader effect.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShaderEffectGlslFiles {
    /// `<directory>/<name>.vert.glsl`.
    pub vertex: PathBuf,
    /// `<directory>/<name>.frag.glsl`.
    pub fragment: PathBuf,
}

/// The name of the files of an effect: its name, with every character but
/// ASCII letters, digits, `-` and `_` written `_`.
fn file_stem(name: &str) -> String {
    name.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect()
}

/// Translates the WGSL module `wgsl_module` and writes its stages to
/// `<directory>/<name>.vert.glsl` and `<directory>/<name>.frag.glsl`
/// (creating the directory); a file whose text is the same is not written
/// again.
pub fn write_shader_effect_glsl_to(
    directory: &Path,
    name: &str,
    wgsl_module: &str,
) -> Result<ShaderEffectGlslFiles, ShaderEffectGlslError> {
    let glsl = translate_shader_effect_module(wgsl_module)?;
    fs::create_dir_all(directory)?;
    let stem = file_stem(name);
    let files = ShaderEffectGlslFiles {
        vertex: directory.join(format!("{stem}.vert.glsl")),
        fragment: directory.join(format!("{stem}.frag.glsl")),
    };
    for (path, text) in [(&files.vertex, &glsl.vertex), (&files.fragment, &glsl.fragment)] {
        if fs::read_to_string(path).ok().as_deref() != Some(text.as_str()) {
            fs::write(path, text)?;
        }
    }
    Ok(files)
}

/// Writes the GLSL ES 3.0 of the shader of the custom effect type `T` to
/// `$OUT_DIR/shader_effects/<NAME>.vert.glsl` and `.frag.glsl`, named after
/// [`CustomEffect::NAME`]; `Ok(None)` for a type without a shader. For a
/// build script.
pub fn write_shader_effect_glsl<T: CustomEffect>() -> Result<Option<ShaderEffectGlslFiles>, ShaderEffectGlslError> {
    let Some(description) = ImmutableCustomEffect::new(T::default()).shader() else {
        return Ok(None);
    };
    let out_dir = std::env::var_os("OUT_DIR").ok_or(ShaderEffectGlslError::NoOutDir)?;
    let directory = Path::new(&out_dir).join("shader_effects");
    write_shader_effect_glsl_to(&directory, T::NAME, &description.wgsl_module()).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::media::effects::library::{
        BurningFlameShaderEffect, DissolveTransitionEffect, MaskTransitionEffect, PixelateEffect,
        PointerSpotlightShaderEffect, ReactiveGridShaderEffect, ScanlineShaderEffect, SpotlightShaderEffect, TintEffect,
        WaterRippleShaderEffect,
    };
    use ferroui_base::media::effects::{
        SHADER_WGSL_CHILDREN_BINDING, SHADER_WGSL_CONTENT_BINDING, SHADER_WGSL_PASS_BINDING,
        SHADER_WGSL_UNIFORMS_BINDING,
    };

    /// A smoke check of a stage of GLSL ES 3.0: the version first, a
    /// precision for the floats of the fragment stage, one `main`, and the
    /// braces balanced.
    fn assert_glsl_es(source: &str, fragment: bool, what: &str) {
        assert!(source.starts_with("#version 300 es"), "{what}: {source}");
        assert_eq!(1, source.matches("void main(").count(), "{what}: {source}");
        if fragment {
            assert!(source.contains("precision highp float;"), "{what}: {source}");
        }
        assert_eq!(source.matches('{').count(), source.matches('}').count(), "{what}: {source}");
        assert_eq!(source.matches('(').count(), source.matches(')').count(), "{what}: {source}");
    }

    #[test]
    fn the_library_shader_effects_translate_to_glsl_es() {
        for (name, glsl) in [
            ("pixelate", shader_effect_glsl::<PixelateEffect>()),
            ("scanline", shader_effect_glsl::<ScanlineShaderEffect>()),
            ("spotlight", shader_effect_glsl::<SpotlightShaderEffect>()),
            // The interactive effects (E7).
            ("pointer-spotlight", shader_effect_glsl::<PointerSpotlightShaderEffect>()),
            ("reactive-grid", shader_effect_glsl::<ReactiveGridShaderEffect>()),
            ("water-ripple", shader_effect_glsl::<WaterRippleShaderEffect>()),
            ("burning-flame", shader_effect_glsl::<BurningFlameShaderEffect>()),
        ] {
            let glsl = glsl.unwrap_or_else(|| panic!("{name}: a shader effect")).unwrap_or_else(|error| panic!("{name}: {error}"));
            assert_glsl_es(&glsl.vertex, false, &format!("{name}, vertex"));
            assert!(glsl.vertex.contains("gl_Position"), "{name}: {}", glsl.vertex);
            assert_glsl_es(&glsl.fragment, true, &format!("{name}, fragment"));
            // The content is fetched by texel, the uniforms are blocks.
            assert!(glsl.fragment.contains("texelFetch("), "{name}: {}", glsl.fragment);
            let blocks: Vec<u32> = glsl.uniform_blocks.iter().map(|(binding, _)| *binding).collect();
            assert_eq!(vec![SHADER_WGSL_UNIFORMS_BINDING, SHADER_WGSL_PASS_BINDING], blocks, "{name}");
            let textures: Vec<u32> = glsl.textures.iter().map(|(binding, _)| *binding).collect();
            assert_eq!(vec![SHADER_WGSL_CONTENT_BINDING], textures, "{name}");
            for (_, block) in glsl.uniform_blocks.iter().chain(&glsl.textures) {
                assert!(glsl.fragment.contains(block.as_str()), "{name}: {block} in {}", glsl.fragment);
            }
        }
        // A type without a shader has none.
        assert!(shader_effect_glsl::<TintEffect>().is_none());
    }

    /// The page transitions (#455, E6): the textures of their images follow
    /// the content, at the bindings of their children.
    #[test]
    fn the_images_of_the_page_transitions_are_textures_of_the_glsl_es() {
        for (name, children, glsl) in [
            ("dissolve-transition", 3, shader_effect_glsl::<DissolveTransitionEffect>()),
            ("mask-transition", 2, shader_effect_glsl::<MaskTransitionEffect>()),
        ] {
            let glsl = glsl.unwrap_or_else(|| panic!("{name}: a shader effect")).unwrap_or_else(|error| panic!("{name}: {error}"));
            assert_glsl_es(&glsl.fragment, true, &format!("{name}, fragment"));
            let mut textures: Vec<u32> = glsl.textures.iter().map(|(binding, _)| *binding).collect();
            textures.sort_unstable();
            let expected: Vec<u32> = std::iter::once(SHADER_WGSL_CONTENT_BINDING)
                .chain((0..children).map(|child| SHADER_WGSL_CHILDREN_BINDING + child))
                .collect();
            assert_eq!(expected, textures, "{name}");
        }
    }

    #[test]
    fn the_glsl_is_written_to_files_named_after_the_effect() {
        let directory = std::env::temp_dir().join(format!("ferroui-build-shader-effects-{}", std::process::id()));
        let module = ImmutableCustomEffect::new(SpotlightShaderEffect::default()).shader().unwrap().wgsl_module();
        let files = write_shader_effect_glsl_to(&directory, "spot light", &module).unwrap();
        assert_eq!(directory.join("spot_light.vert.glsl"), files.vertex);
        assert_eq!(directory.join("spot_light.frag.glsl"), files.fragment);
        let glsl = translate_shader_effect_module(&module).unwrap();
        assert_eq!(glsl.vertex, fs::read_to_string(&files.vertex).unwrap());
        assert_eq!(glsl.fragment, fs::read_to_string(&files.fragment).unwrap());
        // Written again with the same text: nothing changes.
        assert_eq!(files, write_shader_effect_glsl_to(&directory, "spot light", &module).unwrap());
        let _ = fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_module_that_does_not_parse_is_an_error() {
        let error = translate_shader_effect_module("fn effect(coord: vec2<f32>) -> vec4<f32> { return undefined; }");
        assert!(matches!(error, Err(ShaderEffectGlslError::Parse(_))), "{error:?}");
    }
}
