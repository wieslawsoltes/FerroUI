//! The GLSL ES 3.0 of the shader effects of the library for the WebGL2 route (feature
//! `hybrid-webgl`, `web_gl/shader_programs.rs`): translated from their WGSL by naga on the build
//! host, so the browser module carries no translator (decision 2 of
//! docs/porting/custom-effects.md).

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    #[cfg(feature = "hybrid-webgl")]
    web_gl::write();
}

#[cfg(feature = "hybrid-webgl")]
mod web_gl {
    use std::fmt::Write as _;
    use std::path::PathBuf;

    /// The name in the GLSL of what the WGSL module binds at `binding`.
    fn named(names: &[(u32, String)], binding: u32, what: &str, effect: &str) -> String {
        match names.iter().find(|(bound, _)| *bound == binding) {
            Some((_, name)) => name.clone(),
            None => panic!("{effect}: its GLSL has no {what} at binding {binding}"),
        }
    }

    pub fn write() {
        use ferroui_build::shader_effects::library_shader_effects;
        const UNIFORMS: u32 = 0;
        const PASS: u32 = 1;
        const CONTENT: u32 = 2;
        let effects = library_shader_effects().unwrap_or_else(|error| panic!("the shader effects: {error}"));
        let mut table = String::from("&[\n");
        for effect in effects {
            let glsl = &effect.glsl;
            let _ = writeln!(
                table,
                "    LibraryShader {{ name: {:?}, module_hash: {}, vertex: {:?}, fragment: {:?}, \
                 uniforms_block: {:?}, pass_block: {:?}, content: {:?}, uniforms_size: {}, offsets: &{:?} }},",
                effect.name,
                effect.module_hash,
                glsl.vertex,
                glsl.fragment,
                named(&glsl.uniform_blocks, UNIFORMS, "uniform block", effect.name),
                named(&glsl.uniform_blocks, PASS, "pass block", effect.name),
                named(&glsl.textures, CONTENT, "content texture", effect.name),
                glsl.uniforms_size,
                glsl.uniform_offsets,
            );
        }
        table.push_str("]\n");
        let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR is set by cargo"));
        std::fs::write(out.join("web_gl_shader_effects.rs"), table).expect("OUT_DIR is writable");
    }
}
