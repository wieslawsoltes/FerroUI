#pragma once

#include "com.h"
#include "comimpl.h"
#include "ferro-native.h"

@protocol IRenderTarget

-(void) resize: (FrnPixelSize) size withScale: (float) scale;
-(CALayer*) layer;
// Asks for the layer to be displayed again, where the view's own request does
// not reach it.
-(void) setNeedsDisplay;
// The frames the target has begun on the main thread so far, each of the size
// and the scale of the last `resize:withScale:` before it and shown with the
// transaction it was drawn in; 0 for a target that does not count them.
-(uint64_t) mainThreadFrames;

@end

@interface IOSurfaceRenderTarget : NSObject<IRenderTarget>
-(IOSurfaceRenderTarget*) initWithOpenGlContext: (IFrnGlContext*) context;
-(IFrnGlSurfaceRenderTarget*) createSurfaceRenderTarget;
-(IFrnSoftwareRenderTarget*) createSoftwareRenderTarget;
-(HRESULT) setSwFrame: (FrnFramebuffer*) fb;
-(void)consumeSurfaces;
@end

@interface MetalRenderTarget : NSObject<IRenderTarget>
-(MetalRenderTarget*) initWithDevice: (IFrnMetalDevice*) device;
-(void) getRenderTarget: (IFrnMetalRenderTarget**) ppv;
@end