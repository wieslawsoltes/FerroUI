#import <Foundation/Foundation.h>
#import <Metal/Metal.h>
@interface PortableMediaPlane:NSObject
@property(nonatomic,strong) id<MTLRenderPipelineState> pipeline;
@end
@implementation PortableMediaPlane
@end
void *portable_media_plane_create(void *pointer){
    @autoreleasepool{
        id<MTLDevice> device=(__bridge id<MTLDevice>)pointer;
        NSString *source=@"#include <metal_stdlib>\nusing namespace metal;\n"
        "struct Params{float4x4 projection;float4 uv[3];int textured;int rec709;};"
        "struct Vertex{float4 position[[position]];float2 uv;};"
        "vertex Vertex vertex_main(uint id[[vertex_id]],constant Params &p[[buffer(0)]]){"
        "float2 uv=float2(id&1,(id>>1)&1);Vertex v;v.position=p.projection*float4(uv,0,1);"
        "v.uv=p.uv[0].xy+uv.x*p.uv[1].xy+uv.y*p.uv[2].xy;return v;}"
        "fragment float4 fragment_main(Vertex v[[stage_in]],constant Params &p[[buffer(0)]],texture2d<float> image[[texture(0)]]){"
        "constexpr sampler s(filter::linear,address::clamp_to_edge);float3 rgb=p.textured?image.sample(s,v.uv).rgb:float3(v.uv,.5);"
        "if(p.rec709){float3 linear=select(pow((rgb+.099)/1.099,float3(1/.45)),rgb/4.5,rgb<.081);"
        "rgb=select(1.055*pow(max(linear,float3(0)),float3(1/2.4))-.055,12.92*linear,linear<.0031308);}"
        "return float4(rgb*.85,.85);}";
        NSError *error=nil;id<MTLLibrary> library=[device newLibraryWithSource:source options:nil error:&error];
        if(!library){NSLog(@"Sample plane shader: %@",error);return NULL;}
        MTLRenderPipelineDescriptor *description=[MTLRenderPipelineDescriptor new];
        description.vertexFunction=[library newFunctionWithName:@"vertex_main"];
        description.fragmentFunction=[library newFunctionWithName:@"fragment_main"];
        description.colorAttachments[0].pixelFormat=MTLPixelFormatBGRA8Unorm;
        id<MTLRenderPipelineState> pipeline=[device newRenderPipelineStateWithDescriptor:description error:&error];
        if(!pipeline){NSLog(@"Sample plane pipeline: %@",error);return NULL;}
        PortableMediaPlane *owner=[PortableMediaPlane new];owner.pipeline=pipeline;return (__bridge_retained void*)owner;
    }
}
// Uniform layout follows Metal's float4 alignment; Rust supplies 28 floats
// followed by two int32 values, with zero padding to the full 16-byte boundary.
int portable_media_plane_encode(void *p,void *commands,void *output,void *input,const void *parameters,size_t length){
    @autoreleasepool{
        MTLRenderPassDescriptor *pass=[MTLRenderPassDescriptor renderPassDescriptor];
        pass.colorAttachments[0].texture=(__bridge id<MTLTexture>)output;
        pass.colorAttachments[0].loadAction=MTLLoadActionClear;pass.colorAttachments[0].storeAction=MTLStoreActionStore;
        pass.colorAttachments[0].clearColor=MTLClearColorMake(0,0,0,0);
        id<MTLRenderCommandEncoder> encoder=[(__bridge id<MTLCommandBuffer>)commands renderCommandEncoderWithDescriptor:pass];
        if(!encoder)return 0;
        [encoder setRenderPipelineState:((__bridge PortableMediaPlane*)p).pipeline];
        [encoder setVertexBytes:parameters length:length atIndex:0];[encoder setFragmentBytes:parameters length:length atIndex:0];
        if(input)[encoder setFragmentTexture:(__bridge id<MTLTexture>)input atIndex:0];
        [encoder drawPrimitives:MTLPrimitiveTypeTriangleStrip vertexStart:0 vertexCount:4];[encoder endEncoding];return 1;
    }
}
void portable_media_plane_release(void *p){id owner=(__bridge_transfer id)p;(void)owner;}

#import <CoreVideo/CoreVideo.h>
// Explicit diagnostic only, after the renderer host publishes GPU-complete
// output. Production presentation never locks/maps pixel data.
int portable_media_plane_probe(void *pointer,unsigned char *alpha){
    CVPixelBufferRef buffer=pointer;
    if(CVPixelBufferGetPixelFormatType(buffer)!=kCVPixelFormatType_32BGRA)return 0;
    if(CVPixelBufferLockBaseAddress(buffer,kCVPixelBufferLock_ReadOnly)!=kCVReturnSuccess)return 0;
    const unsigned char *pixels=CVPixelBufferGetBaseAddress(buffer);size_t stride=CVPixelBufferGetBytesPerRow(buffer);
    size_t width=CVPixelBufferGetWidth(buffer),height=CVPixelBufferGetHeight(buffer);
    const int x[]={2,98,98,2,50,78,78,18,18},y[]={2,2,98,98,50,30,65,70,20};
    for(int i=0;i<9;i++)alpha[i]=pixels[(height*y[i]/100)*stride+(width*x[i]/100)*4+3];
    // Projected quarter-cell centres derived independently from PLANE.
    const int qx[]={3833,6683,3385,6136},qy[]={3389,3606,6302,6136};
    for(int i=0;i<4;i++){
        const unsigned char *pixel=pixels+(height*qy[i]/10000)*stride+(width*qx[i]/10000)*4;
        alpha[9+i*4]=pixel[2];alpha[10+i*4]=pixel[1];alpha[11+i*4]=pixel[0];alpha[12+i*4]=pixel[3];
    }
    CVPixelBufferUnlockBaseAddress(buffer,kCVPixelBufferLock_ReadOnly);return 1;
}
