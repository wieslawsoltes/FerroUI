//! Optional WGSL -> HLSL adapter. No wgpu device or CPU canvas is involved.
use crate::{bitmap::Bitmap, ffi};
use ferroui_base::{
    media::effects::{ImmutableCustomEffect, ShaderUniformType, ShaderUniformValue},
    Rect,
};
use std::{any::TypeId, cell::RefCell, collections::HashMap, ffi::CString, rc::Rc};
struct Module {
    pipeline: ffi::Owned,
    offsets: Vec<u32>,
    size: usize,
}
thread_local! {
    static MODULES: RefCell<HashMap<(TypeId,u64), Result<Rc<Module>, String>>> = RefCell::new(HashMap::new());
}
fn compile(effect: &ImmutableCustomEffect) -> Result<Rc<Module>, String> {
    effect.check_shader_children()?;
    let source = effect.shader().ok_or("No WGSL shader")?.wgsl_module();
    let module = naga::front::wgsl::parse_str(&source).map_err(|e| e.emit_to_string(&source))?;
    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .map_err(|e| e.emit_to_string(&source))?;
    let variable = module
        .global_variables
        .iter()
        .find(|(_, v)| {
            v.name.as_deref() == Some("uniforms") && v.space == naga::AddressSpace::Uniform
        })
        .ok_or("Missing uniforms block")?
        .1;
    let naga::TypeInner::Struct { members, span } = &module.types[variable.ty].inner else {
        return Err("Uniforms must be a struct".into());
    };
    let layout = effect.shader_uniform_layout();
    if members.len() != layout.len() {
        return Err("Shader uniform count differs from effect parameters".into());
    }
    for (m, (name, ty)) in members.iter().zip(&layout) {
        use naga::{Scalar as S, TypeInner as T, VectorSize as V};
        let matches = match (&module.types[m.ty].inner, ty) {
            (T::Scalar(s), ShaderUniformType::Float) => *s == S::F32,
            (T::Scalar(s), ShaderUniformType::Int) => *s == S::I32,
            (
                T::Vector {
                    size: V::Bi,
                    scalar,
                },
                ShaderUniformType::Float2,
            ) => *scalar == S::F32,
            (
                T::Vector {
                    size: V::Quad,
                    scalar,
                },
                ShaderUniformType::Float4,
            ) => *scalar == S::F32,
            _ => false,
        };
        if m.name.as_deref() != Some(name.as_str()) || !matches {
            return Err(format!("Invalid shader uniform {name}"));
        }
    }
    let offsets = members.iter().map(|m| m.offset).collect();
    let size = (*span as usize).max(16).div_ceil(16) * 16;
    let mut options = naga::back::hlsl::Options {
        shader_model: naga::back::hlsl::ShaderModel::V5_0,
        fake_missing_bindings: false,
        ..Default::default()
    };
    for binding in 0..3 + effect.shader().unwrap().children.len() as u32 {
        options.binding_map.insert(
            naga::ResourceBinding { group: 0, binding },
            naga::back::hlsl::BindTarget {
                space: 0,
                register: if binding < 2 { binding } else { binding - 2 },
                ..Default::default()
            },
        );
    }
    let mut hlsl = String::new();
    let pipeline = naga::back::hlsl::PipelineOptions::default();
    let translation = naga::back::hlsl::Writer::new(&mut hlsl, &options, &pipeline)
        .write(&module, &info, None)
        .map_err(|e| e.to_string())?;
    if let Some(directory) = std::env::var_os("FERROUI_D3D_SHADER_DUMP") {
        let name: String = effect
            .name()
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
            .take(64)
            .collect();
        let _ = std::fs::write(
            std::path::PathBuf::from(directory).join(format!("{name}.hlsl")),
            &hlsl,
        );
    }
    let entry = |name: &str| -> Result<CString, String> {
        let index = module
            .entry_points
            .iter()
            .position(|e| e.name == name)
            .ok_or("Missing shader entry point")?;
        CString::new(
            translation.entry_point_names[index]
                .as_ref()
                .map_err(|e| e.to_string())?
                .as_str(),
        )
        .map_err(|e| e.to_string())
    };
    let vertex = entry("vs_main")?;
    let fragment = entry("fs_main")?;
    let source = CString::new(hlsl).map_err(|e| e.to_string())?;
    let mut error = [0u8; 4096];
    let pipeline = unsafe {
        ffi::d2_gpu_shader_compile(
            source.as_ptr(),
            vertex.as_ptr(),
            fragment.as_ptr(),
            error.as_mut_ptr(),
            error.len(),
        )
    };
    if pipeline.is_null() {
        return Err(String::from_utf8_lossy(
            &error[..error.iter().position(|b| *b == 0).unwrap_or(error.len())],
        )
        .into_owned());
    }
    Ok(Rc::new(Module {
        pipeline: unsafe { ffi::Owned::new(pipeline) },
        offsets,
        size,
    }))
}
pub(crate) fn apply(surface: ffi::Ptr, effect: &ImmutableCustomEffect, area: Rect, scale: f32) {
    let module = MODULES.with(|m| {
        let generation = crate::device_generation();
        let mut cache = m.borrow_mut();
        cache.retain(|(_, g), _| *g == generation);
        cache
            .entry((effect.value_type_id(), generation))
            .or_insert_with(|| {
                let result = compile(effect);
                if let Err(e) = &result {
                    eprintln!(
                        "Direct2D: shader {} failed ({e}); drawing content unchanged",
                        effect.name()
                    );
                }
                result
            })
            .clone()
    });
    let Ok(module) = module else { return };
    let mut bytes = vec![0u8; module.size];
    let mut index = 0;
    effect.for_each_shader_uniform(
        [area.width as f32, area.height as f32],
        scale,
        &mut |_, value| {
            let values: Vec<u8> = match value {
                ShaderUniformValue::Float(v) => v.to_ne_bytes().to_vec(),
                ShaderUniformValue::Int(v) => v.to_ne_bytes().to_vec(),
                ShaderUniformValue::Float2(v) => v.into_iter().flat_map(f32::to_ne_bytes).collect(),
                ShaderUniformValue::Float4(v) => v.into_iter().flat_map(f32::to_ne_bytes).collect(),
            };
            let offset = module.offsets[index] as usize;
            bytes[offset..offset + values.len()].copy_from_slice(&values);
            index += 1;
        },
    );
    let mut children = Vec::new();
    effect.for_each_shader_child(&mut |_, image| {
        children.push(image.bitmap().and_then(|b| {
            Bitmap::from_impl(&*b).map(Bitmap::retained_surface)
        }));
    });
    let handles: Vec<_> = children
        .iter()
        .map(|b| b.as_ref().map_or(std::ptr::null_mut(), |b| b.0))
        .collect();
    unsafe {
        ffi::d2_gpu_shader_apply(
            surface,
            module.pipeline.0,
            bytes.as_ptr(),
            bytes.len(),
            handles.as_ptr(),
            handles.len(),
            area.x,
            area.y,
            area.width,
            area.height,
        )
    };
}
#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::media::effects::library::*;
    #[test]
    fn all_library_shader_programs_compile_to_native_direct3d_pipelines() {
        let effects = [
            ImmutableCustomEffect::new(PixelateEffect::default()),
            ImmutableCustomEffect::new(ScanlineShaderEffect::default()),
            ImmutableCustomEffect::new(SpotlightShaderEffect::default()),
            ImmutableCustomEffect::new(DissolveTransitionEffect::default()),
            ImmutableCustomEffect::new(MaskTransitionEffect::default()),
            ImmutableCustomEffect::new(PointerSpotlightShaderEffect::default()),
            ImmutableCustomEffect::new(ReactiveGridShaderEffect::default()),
            ImmutableCustomEffect::new(WaterRippleShaderEffect::default()),
            ImmutableCustomEffect::new(BurningFlameShaderEffect::default()),
        ];
        for effect in &effects {
            assert!(
                compile(effect).is_ok(),
                "{}: {:?}",
                effect.name(),
                compile(effect).err()
            );
        }
    }
}
