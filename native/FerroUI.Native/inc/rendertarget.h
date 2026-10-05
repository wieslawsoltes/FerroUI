#pragma once

#include "com.h"
#include "comimpl.h"
#include "ferro-native.h"

@protocol IRenderTarget

-(void) resize: (FrnPixelSize) size withScale: (float) scale;
-(CALayer*) layer;

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