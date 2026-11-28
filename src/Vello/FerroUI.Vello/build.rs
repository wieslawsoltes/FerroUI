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

    pub fn write() {
        use ferroui_build::shader_effects::library_shader_effects;
        let effects = library_shader_effects().unwrap_or_else(|error| panic!("the shader effects: {error}"));
        let mut table = String::from("&[\n");
        for effect in effects {
            let glsl = &effect.glsl;
            let bindings = glsl.bindings(effect.children);
            let _ = writeln!(
                table,
                "    LibraryShader {{ name: {:?}, module_hash: {}, vertex: {:?}, fragment: {:?}, \
                 uniforms_block: {:?}, pass_block: {:?}, content: {:?}, children: &{:?}, uniforms_size: {}, \
                 offsets: &{:?} }},",
                effect.name,
                effect.module_hash,
                glsl.vertex,
                glsl.fragment,
                bindings.uniforms_block,
                bindings.pass_block,
                bindings.content,
                bindings.children,
                glsl.uniforms_size,
                glsl.uniform_offsets,
            );
        }
        table.push_str("]\n");
        let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR is set by cargo"));
        std::fs::write(out.join("web_gl_shader_effects.rs"), table).expect("OUT_DIR is writable");
    }
}
