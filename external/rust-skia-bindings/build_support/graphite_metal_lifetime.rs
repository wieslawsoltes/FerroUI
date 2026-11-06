//! FerroUI hotfix for the pinned, manually reference-counted Graphite Metal pipeline.
//! Keep the patch strict and idempotent: a dependency upgrade must be reviewed.
use std::{fs, io, path::Path};
const CHANGES: &[(&str, &str)] = &[
    (r#"    (*psoDescriptor).vertexFunction = [vsLibrary newFunctionWithName: vsFuncName];
    (*psoDescriptor).fragmentFunction = [fsLibrary newFunctionWithName: fsFuncName];

    // TODO: I *think* this gets cleaned up by the pipelineDescriptor?
    (*psoDescriptor).vertexDescriptor = create_vertex_descriptor(appendStepFunc,
                                                                 staticAttrs,
                                                                 appendAttrs);"#, r#"    sk_cfp<id<MTLFunction>> vertexFunction([vsLibrary newFunctionWithName: vsFuncName]);
    sk_cfp<id<MTLFunction>> fragmentFunction([fsLibrary newFunctionWithName: fsFuncName]);
    (*psoDescriptor).vertexFunction = vertexFunction.get();
    (*psoDescriptor).fragmentFunction = fragmentFunction.get();

    sk_cfp<MTLVertexDescriptor*> vertexDescriptor(create_vertex_descriptor(appendStepFunc,
                                                                        staticAttrs,
                                                                        appendAttrs));
    (*psoDescriptor).vertexDescriptor = vertexDescriptor.get();"#),
    (r#"    auto mtlColorAttachment =
            create_color_attachment(TextureFormatToMTLPixelFormat(colorFormat), blendInfo);
    (*psoDescriptor).colorAttachments[0] = mtlColorAttachment;"#, r#"    sk_cfp<MTLRenderPipelineColorAttachmentDescriptor*> mtlColorAttachment(
            create_color_attachment(TextureFormatToMTLPixelFormat(colorFormat), blendInfo));
    (*psoDescriptor).colorAttachments[0] = mtlColorAttachment.get();"#),
 ];
pub fn apply(root: &Path) -> io::Result<()> {
    let path = root.join("src/gpu/graphite/mtl/MtlGraphicsPipeline.mm");
    let mut source = fs::read_to_string(&path)?;
    let original = CHANGES.iter().all(|(old, new)| source.matches(old).count()==1 && !source.contains(new));
    let patched = CHANGES.iter().all(|(old, new)| !source.contains(old) && source.matches(new).count()==1);
    if patched { return Ok(()); }
    if !original { return Err(io::Error::new(io::ErrorKind::InvalidData,
        "Pinned Graphite Metal ownership patch does not match Skia; review the dependency revision")); }
    for (old, new) in CHANGES { source = source.replace(old, new); }
    fs::write(path, source)
}
