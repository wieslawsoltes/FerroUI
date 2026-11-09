// Experimental GPU renderer: public Metal API, Core Graphics path preparation,
// and Core Text glyph outlines. No CPU framebuffer is used for presentation.
#import "metal.h"
#include "metal_source.h"
#import <CoreText/CoreText.h>
#import <Foundation/Foundation.h>
#import <Metal/Metal.h>
#import <TargetConditionals.h>
#if !TARGET_OS_SIMULATOR
#import <MetalPerformanceShaders/MetalPerformanceShaders.h>
#endif
#include <simd/simd.h>
#include <stdatomic.h>
#include <stdlib.h>
#include <string.h>

typedef struct {
  vector_uint2 origin, extent;
  uint32_t edges, evenodd, mode, blend;
  vector_float4 color, gradient, inverse, translation, src, dst, clip, shadow,
      target;
  uint32_t stops, clipped, flipped, reserved;
  uint32_t edgeRows, edgeOriginY, edgeStride, edgePad;
  // The analytic clips, in canvas pixels: a rectangle (left, top, right,
  // bottom) with corner radii (x, y) (`clipped & 2`), and a rectangle
  // (`clipped & 4`), the intersection of the rectangle clips in effect.
  vector_float4 rclip, rradius, sclip;
} QParams;
typedef struct {
  vector_float4 color, location;
} QStop;
typedef struct {
  vector_float4 bounds, uv, limits, tint;
  vector_uint4 flags;
} QImageParams;
typedef struct {
  // `radii`: the corner radii of a rounded box (`x`, `y`), zero for a box.
  vector_float4 bounds, color, radii;
} QSolidParams;
typedef struct {
  QImageParams image;
  vector_float4 clip;
} QRectImageParams;
static QRectImageParams rect_image_params(QParams p, id<MTLTexture> image);
static QImageParams image_params(QParams p, id<MTLTexture> image) {
  vector_float2 size = {image.width, image.height};
  vector_float2 lo = {(float)p.origin.x, (float)p.origin.y};
  vector_float2 hi = lo + (vector_float2){p.extent.x, p.extent.y};
  // Fractional axis-aligned image quads retain their exact geometry. The
  // vertex shader expands to pixel bounds and the fragment shader evaluates
  // the same four coverage samples as the general path pipeline.
  if (p.reserved & 8) {
    lo = p.gradient.xy;
    hi = p.gradient.zw;
  }
  vector_float2 first = {
      p.inverse.x * lo.x + p.inverse.z * lo.y + p.translation.x,
      p.inverse.y * lo.x + p.inverse.w * lo.y + p.translation.y};
  vector_float2 last = {
      p.inverse.x * hi.x + p.inverse.z * hi.y + p.translation.x,
      p.inverse.y * hi.x + p.inverse.w * hi.y + p.translation.y};
  vector_float2 src = {p.src.x, p.src.y}, extent = {p.src.z, p.src.w};
  vector_float2 dst = {p.dst.x, p.dst.y}, span = {p.dst.z, p.dst.w};
  vector_float2 a = (src + (first - dst) / span * extent) / size;
  vector_float2 b = (src + (last - dst) / span * extent) / size;
  vector_float2 low = simd_min(src, src + extent) / size,
                high = simd_max(src, src + extent) / size;
  vector_float4 tint =
      (p.reserved & 3)
          ? (vector_float4){p.color.x * p.color.w, p.color.y * p.color.w,
                            p.color.z * p.color.w, p.color.w}
          : (vector_float4){1, 1, 1, 1};
  return (QImageParams){
      {lo.x - p.target.x, lo.y - p.target.y, hi.x - lo.x, hi.y - lo.y},
      {a.x, a.y, b.x - a.x, b.y - a.y},
      {low.x, low.y, high.x, high.y},
      tint * p.translation.w,
      {p.flipped & 2, (p.reserved & 3) == 2, (p.reserved & 4) != 0,
       (p.reserved & 8) ? (1 | ((p.flipped & 1) << 1)) : 0}};
}

static QRectImageParams rect_image_params(QParams p, id<MTLTexture> image) {
  return (QRectImageParams){image_params(p, image),
                            {p.origin.x - p.target.x, p.origin.y - p.target.y,
                             p.extent.x, p.extent.y}};
}

// Dictionary equality still compares the complete owned key on collisions.
static uint64_t content_hash(const void *bytes, NSUInteger length) {
  const uint8_t *p = bytes;
  uint64_t hash = 0x9e3779b97f4a7c15ULL ^ length;
  while (length >= 8) {
    uint64_t word;
    memcpy(&word, p, 8);
    hash = (hash ^ word) * 0x9ddfea08eb382d69ULL;
    hash ^= hash >> 32;
    p += 8;
    length -= 8;
  }
  uint64_t tail = 0;
  memcpy(&tail, p, length);
  hash = (hash ^ tail) * 0x9ddfea08eb382d69ULL;
  return hash ^ (hash >> 33);
}
// Borrow fixed-size keys on lookup and copy only on insertion. Their hash
// covers every byte; equality still compares every byte on collisions. The
// dictionary owns malloc-backed copies, never a pointer into a caller stack.
typedef struct {
  uint64_t hash;
  size_t length;
  uint8_t bytes[48];
} QSmallKey;
_Static_assert(sizeof(CGAffineTransform) <= 48, "transform key capacity");
#ifdef QUARTZ_MEDIA_TEST_FIXTURES
static _Atomic bool fixtureKeyCollisions;
#endif
static QSmallKey small_key(const void *bytes, size_t length) {
  QSmallKey key = {0};
  key.length = length;
  memcpy(key.bytes, bytes, length);
  key.hash = content_hash(bytes, length);
  return key;
}
static CFHashCode small_hash(const void *raw) {
#ifdef QUARTZ_MEDIA_TEST_FIXTURES
  if (atomic_load(&fixtureKeyCollisions))
    return 0;
#endif
  return ((const QSmallKey *)raw)->hash;
}
static Boolean small_equal(const void *a, const void *b) {
  const QSmallKey *x = a, *y = b;
  return x->length == y->length && !memcmp(x->bytes, y->bytes, x->length);
}
static void small_release(CFAllocatorRef allocator, const void *key) {
  (void)allocator;
  free((void *)key);
}
static CFMutableDictionaryRef small_dictionary(void) {
  const CFDictionaryKeyCallBacks keys = {0,    NULL,        small_release,
                                         NULL, small_equal, small_hash};
  return CFDictionaryCreateMutable(NULL, 0, &keys,
                                   &kCFTypeDictionaryValueCallBacks);
}
static const void *small_get(CFDictionaryRef dictionary, const QSmallKey *key) {
  return dictionary ? CFDictionaryGetValue(dictionary, key) : NULL;
}
static BOOL small_set(CFMutableDictionaryRef dictionary, const QSmallKey *key,
                      const void *value) {
  if (!dictionary || !value)
    return NO;
  if (CFDictionaryContainsKey(dictionary, key)) {
    CFDictionaryReplaceValue(dictionary, key, value);
    return YES;
  }
  QSmallKey *owned = malloc(sizeof(*owned));
  if (!owned)
    return NO; // This cache is optional; keep drawing after allocation failure.
  *owned = *key;
  CFDictionarySetValue(dictionary, owned, value);
  return YES;
}

static _Atomic uint64_t counters[9];
#ifdef QUARTZ_MEDIA_TEST_FIXTURES
static _Atomic bool fixtureFailNextTexture;
static _Atomic bool fixtureDisableGlyphAtlas;
static _Atomic bool fixtureDisableRectImages;
static _Atomic bool fixtureDisableDeferredMerge, fixtureDisableLayerAtlas;
static uint64_t fixtureAtlasRegionClears;
static uint64_t fixtureEncodedBatches;
static _Atomic NSUInteger fixtureGlyphBudget = 16 * 1024 * 1024;
void qm_fixture_rect_images(int enabled) {
  atomic_store(&fixtureDisableRectImages, !enabled);
}
void qm_fixture_fail_allocation(void) {
  atomic_store(&fixtureFailNextTexture, true);
}
#endif
static NSUInteger glyph_atlas_limit(void) {
#ifdef QUARTZ_MEDIA_TEST_FIXTURES
  return atomic_load(&fixtureGlyphBudget);
#else
  return 16 * 1024 * 1024;
#endif
}
static BOOL glyph_atlas_enabled(void) {
#ifdef QUARTZ_MEDIA_TEST_FIXTURES
  return !atomic_load(&fixtureDisableGlyphAtlas);
#else
  return YES;
#endif
}
static _Atomic uint64_t rendererGeneration = 1;
static _Atomic bool rendererLost = false;
uint64_t qm_generation(void) { return atomic_load(&rendererGeneration); }
int qm_is_lost(uint64_t generation) {
  return atomic_load(&rendererLost) || generation != qm_generation();
}
// Also used by isolated fault-injection tests: no invalid GPU command is
// needed.
void qm_record_fault(uint64_t generation) {
  atomic_fetch_add(&counters[6], 1);
  if (generation == qm_generation())
    atomic_store(&rendererLost, true);
}
// Renderer mutation is serialized by quartz_lock; completion handlers touch
// only atomic fault counters. These private properties need no extra atomic
// Objective-C getter/setter locks. Published resource handles stay retained.
@interface QState : NSObject
@property(nonatomic) CGAffineTransform transform;
@property(nonatomic) id<MTLTexture> clip;
@property(nonatomic) CGRect clipRect;
// The analytic clip: a rounded rectangle in canvas pixels (`rclipped`).
@property(nonatomic) vector_float4 rclip, rradius, sclip;
@property(nonatomic) int rclipped, sclipped;
@property(nonatomic) vector_float4 shadow, shadowColor, gradientOptions;
@property(nonatomic) int blendMode, aliased, interpolation, hinting;
@end
@implementation QState
@end
static void recycle_texture(id<MTLTexture> texture, uint64_t generation);
// Sharing this owner, rather than only the Metal handle, prevents snapshots and
// nested layers from returning aliased storage to the pool prematurely.
@class QLayerAtlas;
@interface QTexture : NSObject
@property(nonatomic) id<MTLTexture> handle, mipmap;
@property(nonatomic) uint64_t generation;
@property(nonatomic) CGRect dirty, region;
@property(nonatomic) QLayerAtlas *atlasOwner;
@property(nonatomic) BOOL external, cropped, initialized;
@property(nonatomic) CGPoint origin;
@property(nonatomic) CGSize canvas;
@end
@implementation QTexture
- (void)dealloc {
  if (!_external && !_atlasOwner)
    recycle_texture(_handle, _generation);
}
@end
@interface QLayerAtlas : NSObject
@property(nonatomic) NSMutableArray<id<MTLTexture>> *pages;
@property(nonatomic) id<MTLTexture> current;
@property(nonatomic) NSUInteger x, y, row;
@property(nonatomic) uint64_t generation;
@end
@implementation QLayerAtlas
- (instancetype)init {
  if ((self = [super init])) {
    _pages = [NSMutableArray new];
    _generation = qm_generation();
  }
  return self;
}
- (void)dealloc {
  for (id<MTLTexture> page in _pages)
    recycle_texture(page, _generation);
}
@end
@interface QLayer : NSObject
@property(nonatomic) QTexture *parent;
@property(nonatomic) double opacity;
@property(nonatomic) int blend;
@end
@implementation QLayer
@end
@interface QMetalSurface : NSObject
@property(nonatomic) QTexture *storage;
@property(nonatomic) QLayerAtlas *layerAtlas;
@property(nonatomic) id<MTLTexture> texture;
@property(nonatomic) QState *state;
@property(nonatomic) NSMutableArray<QState *> *states;
@property(nonatomic) NSMutableArray<QLayer *> *layers;
@end
@implementation QMetalSurface
- (id<MTLTexture>)texture {
  return _storage.handle;
}
- (void)setTexture:(id<MTLTexture>)texture {
  QTexture *owner = [QTexture new];
  owner.handle = texture;
  owner.canvas = CGSizeMake(texture.width, texture.height);
  owner.region = CGRectMake(0, 0, texture.width, texture.height);
  owner.dirty = CGRectMake(0, 0, texture.width, texture.height);
  owner.generation = qm_generation();
  self.storage = owner;
}
@end
@interface QGlyphImage : NSObject
@property(nonatomic) QMetalSurface *surface;
@property(nonatomic) CGRect bounds, atlasRect;
@property(nonatomic) id<MTLTexture> atlas;
@property(nonatomic) BOOL monochrome;
@end
@implementation QGlyphImage
@end
@interface QPreparedDraw : NSObject
@property(nonatomic) NSMutableData *params, *edges, *images;
@property(nonatomic) id<MTLTexture> image;
@property(nonatomic) NSUInteger kind;
@property(nonatomic) CGRect bounds;
@end
@implementation QPreparedDraw
@end
@interface QPathEdges : NSObject
@property(nonatomic) NSData *edges;
@property(nonatomic) CGRect box;
@end
@implementation QPathEdges
@end
@interface QCoverageImage : NSObject
@property(nonatomic) id<MTLTexture> texture;
@property(nonatomic) CGRect rect;
@end
@implementation QCoverageImage
@end
@interface QDrawBatch : NSObject
@property(nonatomic) NSMutableData *params, *edges, *images;
@property(nonatomic) QTexture *storage;
@property(nonatomic) id<MTLTexture> image, clip;
@property(nonatomic) NSUInteger kind, count;
@property(nonatomic) BOOL clear;
@property(nonatomic) MTLClearColor clearColor;
@end
@implementation QDrawBatch
@end
@interface QUploadBuffer : NSObject
@property(nonatomic) id<MTLBuffer> buffer;
@property(nonatomic) id<MTLCommandBuffer> lastUse;
@property(nonatomic) uint64_t generation;
@end
@implementation QUploadBuffer
@end
@interface QMetalRenderer : NSObject
@property(nonatomic) id<MTLDevice> device;
@property(nonatomic) MTLPixelFormat canvasFormat;
@property(nonatomic) id<MTLCommandQueue> queue;
@property(nonatomic) id<MTLCommandBuffer> pending;
@property(nonatomic) NSUInteger pendingOperations, pendingBytes, poolBytes;
@property(nonatomic)
    NSMutableDictionary<NSString *, NSMutableArray<id<MTLTexture>> *>
        *texturePool;
@property(nonatomic) id<MTLLibrary> library;
@property(nonatomic) id<MTLComputePipelineState> paint;
@property(nonatomic) id<MTLRenderPipelineState> draw, pathPipeline,
    imagePipeline, clearPipeline, rectImagePipeline, solidPipeline;
@property(nonatomic) NSUInteger drawKind, drawCount;
@property(nonatomic) QTexture *drawStorage;
@property(nonatomic) NSMutableData *drawParams, *drawEdges, *drawImages;
@property(nonatomic) id<MTLTexture> drawTarget, drawClip, drawImage, glyphAtlas;
@property(nonatomic) NSUInteger atlasX, atlasY, atlasRow, atlasBytes;
@property(nonatomic) id<MTLTexture> monoAtlas;
@property(nonatomic) NSUInteger monoX, monoY, monoRow;
@property(nonatomic) BOOL drawClear;
@property(nonatomic) MTLClearColor drawClearColor;
@property(nonatomic) BOOL separateDestination, traceTiming;
@property(nonatomic) id<MTLComputePipelineState> convertFrame;
@property(nonatomic) id<MTLComputePipelineState> layerFilter, gaussian;
@property(nonatomic) id<MTLSharedEvent> event;
@property(nonatomic) NSMutableDictionary<NSData *, NSData *> *edgeCache;
@property(nonatomic) NSUInteger edgeCacheBytes;
@property(nonatomic) CFMutableDictionaryRef glyphCache, glyphImages,
    preparedRuns, pathCache;
@property(nonatomic) QPreparedDraw *capture;
@property(nonatomic) NSUInteger preparedBytes, pathBytes, pathEntries,
    coverageBytes, coveragePathBytes;
@property(nonatomic)
    NSMutableDictionary<NSData *, QCoverageImage *> *coverageImages;
@property(nonatomic) id<MTLTexture> coverageAtlas;
@property(nonatomic) NSUInteger coverageX, coverageY, coverageRow;
@property(nonatomic) CFMutableDictionaryRef coveragePaths;
@property(nonatomic) NSMutableArray<QUploadBuffer *> *buffers;
@property(nonatomic) CFMutableDictionaryRef rectanglePaths;
@property(nonatomic) NSUInteger bufferBytes;
@property(nonatomic) id<MTLRenderCommandEncoder> encoder;
@property(nonatomic) id<MTLTexture> encoderTarget;
@property(nonatomic) NSMutableSet<id<MTLTexture>> *encoderInputs;
@property(nonatomic) NSMutableArray<QDrawBatch *> *deferred;
@property(nonatomic) NSMutableArray<QTexture *> *deferredSources;
@property(nonatomic) NSUInteger deferredBytes, deferredSourceBytes;
@property(nonatomic) BOOL encodingDeferred;
@property(nonatomic) NSUInteger glyphImageBytes;
@property(nonatomic) uint64_t eventValue;
@property(nonatomic) id<MTLTexture> white, transparent;
@property(nonatomic) NSRecursiveLock *lock;
@property(nonatomic) NSMutableDictionary<NSNumber *, id<MTLRenderPipelineState>>
    *presentPipelines;
@end
@implementation QMetalRenderer
@end
static void flush_draws(void);
static void flush_batch(void);
static void flush_deferred(void);
static void end_render_encoder(void);
static void finish_operation(void);
static id<MTLTexture> texture(size_t w, size_t h);
static void clear_texture(QMetalSurface *s);
static void copy_texture(id<MTLTexture> dst, id<MTLTexture> src);
static NSRecursiveLock *quartz_lock(void) {
  static NSRecursiveLock *lock;
  static dispatch_once_t once;
  dispatch_once(&once, ^{
    lock = [NSRecursiveLock new];
  });
  return lock;
}
static QMetalRenderer *renderer(void) {
  static QMetalRenderer *r;
  static dispatch_once_t once;
  dispatch_once(&once, ^{
    r = [QMetalRenderer new];
    r.lock = quartz_lock();
    @try {
      r.traceTiming = getenv("FERROUI_QUARTZ_TRACE_TIMING") != NULL;
      r.device = MTLCreateSystemDefaultDevice();
      if (!r.device) {
        r = nil;
        return;
      }
      // Apple GPUs support BGRA8 shader writes. Intel/AMD retain the existing
      // RGBA working format; unsupported drawables use the conversion pass.
      r.canvasFormat = [r.device supportsFamily:MTLGPUFamilyApple1]
                           ? MTLPixelFormatBGRA8Unorm
                           : MTLPixelFormatRGBA8Unorm;
      const char *forceSeparate =
          getenv("FERROUI_QUARTZ_FORCE_SEPARATE_DESTINATION");
      r.separateDestination =
          r.device.readWriteTextureSupport < MTLReadWriteTextureTier2 ||
          (forceSeparate && !strcmp(forceSeparate, "1"));
      r.queue = [r.device newCommandQueueWithMaxCommandBufferCount:4096];
      NSError *error = nil;
      MTLCompileOptions *options = [MTLCompileOptions new];
      if (r.separateDestination)
        options.preprocessorMacros = @{@"QUARTZ_SEPARATE_DESTINATION" : @1};
      r.library = [r.device newLibraryWithSource:@(q_metal_source)
                                         options:options
                                           error:&error];
      if (!r.library) {
        NSLog(@"Quartz shader compilation failed: %@", error);
        r = nil;
        return;
      }
      r.paint = [r.device
          newComputePipelineStateWithFunction:[r.library
                                                  newFunctionWithName:@"paint"]
                                        error:&error];
      if (!r.paint) {
        r = nil;
        return;
      }
      MTLRenderPipelineDescriptor *draw = [MTLRenderPipelineDescriptor new];
      draw.vertexFunction = [r.library newFunctionWithName:@"draw_vertex"];
      draw.fragmentFunction = [r.library newFunctionWithName:@"draw_fragment"];
      draw.colorAttachments[0].pixelFormat = r.canvasFormat;
      draw.colorAttachments[0].blendingEnabled = YES;
      draw.colorAttachments[0].sourceRGBBlendFactor = MTLBlendFactorOne;
      draw.colorAttachments[0].sourceAlphaBlendFactor = MTLBlendFactorOne;
      draw.colorAttachments[0].destinationRGBBlendFactor =
          MTLBlendFactorOneMinusSourceAlpha;
      draw.colorAttachments[0].destinationAlphaBlendFactor =
          MTLBlendFactorOneMinusSourceAlpha;
      r.draw = [r.device newRenderPipelineStateWithDescriptor:draw
                                                        error:&error];
      if (!r.draw) {
        NSLog(@"Quartz draw pipeline failed: %@", error);
        r = nil;
        return;
      }
      draw.fragmentFunction = [r.library newFunctionWithName:@"path_fragment"];
      r.pathPipeline = [r.device newRenderPipelineStateWithDescriptor:draw
                                                                error:&error];
      draw.vertexFunction = [r.library newFunctionWithName:@"image_vertex"];
      draw.fragmentFunction = [r.library newFunctionWithName:@"image_fragment"];
      r.imagePipeline = [r.device newRenderPipelineStateWithDescriptor:draw
                                                                 error:&error];
      draw.vertexFunction =
          [r.library newFunctionWithName:@"rect_image_vertex"];
      draw.fragmentFunction =
          [r.library newFunctionWithName:@"rect_image_fragment"];
      r.rectImagePipeline =
          [r.device newRenderPipelineStateWithDescriptor:draw error:&error];
      draw.vertexFunction = [r.library newFunctionWithName:@"solid_vertex"];
      draw.fragmentFunction = [r.library newFunctionWithName:@"solid_fragment"];
      r.solidPipeline = [r.device newRenderPipelineStateWithDescriptor:draw
                                                                 error:&error];
      if (!r.pathPipeline || !r.imagePipeline || !r.rectImagePipeline ||
          !r.solidPipeline) {
        NSLog(@"Quartz specialized pipeline failed: %@", error);
        r = nil;
        return;
      }
      draw.vertexFunction = [r.library newFunctionWithName:@"solid_vertex"];
      draw.fragmentFunction = [r.library newFunctionWithName:@"clear_fragment"];
      draw.colorAttachments[0].blendingEnabled = NO;
      r.clearPipeline = [r.device newRenderPipelineStateWithDescriptor:draw
                                                                 error:&error];
      if (!r.clearPipeline) {
        r = nil;
        return;
      }
      r.drawParams = [NSMutableData new];
      r.drawEdges = [NSMutableData new];
      r.drawImages = [NSMutableData new];
      r.convertFrame =
          [r.device newComputePipelineStateWithFunction:
                        [r.library newFunctionWithName:@"convert_frame"]
                                                  error:&error];
      if (!r.convertFrame) {
        r = nil;
        return;
      }
      r.layerFilter =
          [r.device newComputePipelineStateWithFunction:
                        [r.library newFunctionWithName:@"layer_filter"]
                                                  error:&error];
      r.gaussian =
          [r.device newComputePipelineStateWithFunction:
                        [r.library newFunctionWithName:@"gaussian_pass"]
                                                  error:&error];
      if (!r.layerFilter || !r.gaussian) {
        r = nil;
        return;
      }
      r.texturePool = [NSMutableDictionary new];
      r.edgeCache = [NSMutableDictionary new];
      r.coverageImages = [NSMutableDictionary new];
      r.glyphCache =
          CFDictionaryCreateMutable(NULL, 0, &kCFTypeDictionaryKeyCallBacks,
                                    &kCFTypeDictionaryValueCallBacks);
      r.glyphImages =
          CFDictionaryCreateMutable(NULL, 0, &kCFTypeDictionaryKeyCallBacks,
                                    &kCFTypeDictionaryValueCallBacks);
      r.preparedRuns =
          CFDictionaryCreateMutable(NULL, 0, &kCFTypeDictionaryKeyCallBacks,
                                    &kCFTypeDictionaryValueCallBacks);
      // CGPath hashes collide for distinct paths on current OS releases. The
      // Rust path cache retains immutable paths, so retained pointer identity
      // gives constant-time lookup without geometric equality scans.
      CFDictionaryKeyCallBacks pathKeys = kCFTypeDictionaryKeyCallBacks;
      pathKeys.equal = NULL;
      pathKeys.hash = NULL;
      r.coveragePaths = CFDictionaryCreateMutable(
          NULL, 0, &pathKeys, &kCFTypeDictionaryValueCallBacks);
      r.rectanglePaths = small_dictionary();
      r.buffers = [NSMutableArray new];
      r.deferred = [NSMutableArray new];
      r.deferredSources = [NSMutableArray new];
      r.encoderInputs = [NSMutableSet new];
      r.pathCache = CFDictionaryCreateMutable(NULL, 0, &pathKeys,
                                              &kCFTypeDictionaryValueCallBacks);
      r.event = [r.device newSharedEvent];
      if (!r.queue || !r.event || !r.glyphCache || !r.glyphImages ||
          !r.preparedRuns || !r.pathCache || !r.coveragePaths ||
          !r.rectanglePaths) {
        r = nil;
        return;
      }
      r.presentPipelines = [NSMutableDictionary new];
      MTLTextureDescriptor *d = [MTLTextureDescriptor
          texture2DDescriptorWithPixelFormat:MTLPixelFormatRGBA8Unorm
                                       width:1
                                      height:1
                                   mipmapped:NO];
      d.usage = MTLTextureUsageShaderRead;
      r.white = [r.device newTextureWithDescriptor:d];
      r.transparent = [r.device newTextureWithDescriptor:d];
      if (!r.white || !r.transparent) {
        r = nil;
        return;
      }
      uint32_t zero = 0;
      [r.transparent replaceRegion:MTLRegionMake2D(0, 0, 1, 1)
                       mipmapLevel:0
                         withBytes:&zero
                       bytesPerRow:4];
      uint32_t white = 0xffffffff;
      [r.white replaceRegion:MTLRegionMake2D(0, 0, 1, 1)
                 mipmapLevel:0
                   withBytes:&white
                 bytesPerRow:4];
    } @catch (NSException *error) {
      NSLog(@"Quartz initialization failed: %@", error);
      r = nil;
    }
  });
  return r;
}
static void commit(id<MTLCommandBuffer> cb) {
  if (!cb) {
    qm_record_fault(qm_generation());
    return;
  }
  uint64_t generation = qm_generation();
  static _Atomic uint64_t sequence;
  uint64_t serial = atomic_fetch_add(&sequence, 1);
  BOOL trace = renderer().traceTiming;
  [cb addCompletedHandler:^(id<MTLCommandBuffer> done) {
    if (trace && serial % 25 == 0)
      fprintf(stderr, "Quartz GPU buffer %llu: gpu=%.4f ms kernel=%.4f ms\n",
              (unsigned long long)serial,
              (done.GPUEndTime - done.GPUStartTime) * 1000,
              (done.kernelEndTime - done.kernelStartTime) * 1000);
    if (done.status == MTLCommandBufferStatusError) {
      qm_record_fault(generation);
      NSLog(@"Quartz Metal error: %@", done.error);
    }
  }];
  [cb commit];
  atomic_fetch_add(&counters[7], 1);
}
// One ordered batch can contain draws, copies and system MPS kernels. Separate
// encoders keep Metal's tracked-resource hazards explicit.
// Presentation/readback and producer-frame completion flush; an operation bound
// prevents unbounded batches.
static id<MTLCommandBuffer> raw_batch_buffer(void) {
  QMetalRenderer *r = renderer();
  if (!r.pending) {
    r.pending = [r.queue commandBuffer];
    if (!r.pending) {
      qm_record_fault(qm_generation());
      return nil;
    }
    r.pending.label = @"FerroUI Quartz GPU batch";
  }
  return r.pending;
}
// Poll completion at reuse instead of scheduling a callback per upload. The
// completed command buffers are released on the next pool access. All access
// is serialized by the renderer lock.
static id<MTLBuffer> upload_buffer(const void *bytes, NSUInteger length,
                                   id<MTLCommandBuffer> cb) {
  if (!cb)
    return nil;
  QMetalRenderer *r = renderer();
  NSUInteger capacity = 4096;
  while (capacity < length)
    capacity *= 2;
  id<MTLBuffer> buffer = nil;
  for (QUploadBuffer *entry in r.buffers) {
    id<MTLCommandBuffer> previous = entry.lastUse;
    if (entry.generation != qm_generation())
      continue;
    if (previous.status == MTLCommandBufferStatusError) {
      qm_record_fault(entry.generation);
      return nil;
    }
    BOOL complete =
        !previous || previous.status == MTLCommandBufferStatusCompleted;
    if (complete)
      entry.lastUse = nil;
    if (entry.buffer.length == capacity && complete) {
      buffer = entry.buffer;
      entry.lastUse = cb;
      break;
    }
  }
  if (!buffer) {
    buffer = [r.device newBufferWithLength:capacity
                                   options:MTLResourceStorageModeShared];
    if (!buffer)
      return nil;
    if (r.buffers.count < 32 && r.bufferBytes + capacity <= 16 * 1024 * 1024) {
      QUploadBuffer *entry = [QUploadBuffer new];
      entry.buffer = buffer;
      entry.lastUse = cb;
      entry.generation = qm_generation();
      [r.buffers addObject:entry];
      r.bufferBytes += capacity;
    }
  }
  memcpy(buffer.contents, bytes, length);
  return buffer;
}
static void end_render_encoder(void) {
  QMetalRenderer *r = renderer();
  [r.encoder endEncoding];
  r.encoder = nil;
  r.encoderTarget = nil;
  [r.encoderInputs removeAllObjects];
}
// Flush before any compute/blit or texture reuse. The command buffer retains
// all resources, and the queue orders their use against subsequent operations.
static void flush_draws(void) {
  QMetalRenderer *r = renderer();
  if (!r.drawCount && !r.drawClear)
    return;
  // Independent offscreen layers can be drawn before the drawable. Keeping
  // drawable commands in order lets all its batches share one render pass.
  // Any compute/blit/snapshot dependency drains this list first.
  if (r.drawStorage.external && !r.encodingDeferred) {
    QDrawBatch *previous = r.deferred.lastObject;
    BOOL merge = previous && !r.drawClear && r.drawKind >= 2 &&
                 previous.kind == r.drawKind &&
                 previous.storage == r.drawStorage &&
                 previous.image == r.drawImage && previous.clip == r.drawClip &&
                 previous.count + r.drawCount <= 4096;
#ifdef QUARTZ_MEDIA_TEST_FIXTURES
    merge = merge && !atomic_load(&fixtureDisableDeferredMerge);
#endif
    r.deferredBytes +=
        r.drawParams.length + r.drawEdges.length + r.drawImages.length;
    if (merge) {
      // These are consecutive commands in the ordered drawable stream.
      // Independent offscreen work may be interleaved, but every dependency
      // drains this stream. Never regroup across a texture/clip/pipeline
      // change.
      previous.count += r.drawCount;
      [previous.params appendData:r.drawParams];
      [previous.images appendData:r.drawImages];
      [previous.edges appendData:r.drawEdges];
      [r.drawParams setLength:0];
      [r.drawImages setLength:0];
      [r.drawEdges setLength:0];
    } else {
      QDrawBatch *batch = [QDrawBatch new];
      batch.params = r.drawParams;
      batch.edges = r.drawEdges;
      batch.images = r.drawImages;
      batch.storage = r.drawStorage;
      batch.image = r.drawImage;
      batch.clip = r.drawClip;
      batch.kind = r.drawKind;
      batch.count = r.drawCount;
      batch.clear = r.drawClear;
      batch.clearColor = r.drawClearColor;
      [r.deferred addObject:batch];
      r.drawParams = [NSMutableData new];
      r.drawEdges = [NSMutableData new];
      r.drawImages = [NSMutableData new];
    }
    r.drawStorage = nil;
    r.drawTarget = r.drawImage = r.drawClip = nil;
    r.drawClear = NO;
    r.drawCount = 0;
    if (r.deferred.count >= 256 || r.deferredBytes >= 16 * 1024 * 1024)
      flush_deferred();
    return;
  }
  BOOL atlasClear = r.drawClear && r.drawStorage.atlasOwner;
  if (r.encoder &&
      (r.encoderTarget != r.drawTarget || (r.drawClear && !atlasClear)))
    end_render_encoder();
  id<MTLCommandBuffer> cb = raw_batch_buffer();
  NSMutableData *vertexData = r.drawKind >= 2 ? r.drawImages : r.drawParams;
  BOOL inlineDraws = vertexData.length <= 4096;
  id<MTLBuffer> draws =
      inlineDraws ? nil
                  : upload_buffer(vertexData.bytes, vertexData.length, cb);
  id<MTLBuffer> edges =
      (r.drawKind < 2 && r.drawEdges.length > 4096)
          ? upload_buffer(r.drawEdges.bytes, r.drawEdges.length, cb)
          : nil;
  id<MTLRenderCommandEncoder> enc = r.encoder;
  if (!enc) {
    MTLRenderPassDescriptor *pass =
        [MTLRenderPassDescriptor renderPassDescriptor];
    pass.colorAttachments[0].texture = r.drawTarget;
    pass.colorAttachments[0].loadAction =
        (r.drawClear && !atlasClear) ? MTLLoadActionClear : MTLLoadActionLoad;
    pass.colorAttachments[0].clearColor = r.drawClearColor;
    pass.colorAttachments[0].storeAction = MTLStoreActionStore;
    enc = [cb renderCommandEncoderWithDescriptor:pass];
    r.encoder = enc;
    r.encoderTarget = r.drawTarget;
  }
  if ((!inlineDraws && !draws) ||
      (r.drawKind < 2 && r.drawEdges.length > 4096 && !edges) || !enc) {
    end_render_encoder();
    qm_record_fault(qm_generation());
  } else {
    vector_float2 size = {r.drawTarget.width, r.drawTarget.height};
    if (atlasClear) {
      CGRect rect = r.drawStorage.region;
      QSolidParams clear = {
          {rect.origin.x, rect.origin.y, rect.size.width, rect.size.height},
          {0, 0, 0, 0}};
      [enc setRenderPipelineState:r.clearPipeline];
      [enc setVertexBytes:&clear length:sizeof(clear) atIndex:0];
      [enc setVertexBytes:&size length:sizeof(size) atIndex:1];
      [enc drawPrimitives:MTLPrimitiveTypeTriangle vertexStart:0 vertexCount:6];
#ifdef QUARTZ_MEDIA_TEST_FIXTURES
      fixtureAtlasRegionClears++;
#endif
    }
    if (r.drawImage)
      [r.encoderInputs addObject:r.drawImage];
    if (r.drawClip)
      [r.encoderInputs addObject:r.drawClip];
    [enc setRenderPipelineState:r.drawKind == 0   ? r.pathPipeline
                                : r.drawKind == 2 ? r.imagePipeline
                                : r.drawKind == 3 ? r.rectImagePipeline
                                : r.drawKind == 4 ? r.solidPipeline
                                                  : r.draw];
    if (r.drawCount) {
      if (inlineDraws)
        [enc setVertexBytes:vertexData.bytes
                     length:vertexData.length
                    atIndex:0];
      else
        [enc setVertexBuffer:draws offset:0 atIndex:0];
    }
    [enc setVertexBytes:&size length:sizeof(size) atIndex:1];
    if (r.drawKind < 2 && r.drawParams.length) {
      if (inlineDraws)
        [enc setFragmentBytes:r.drawParams.bytes
                       length:r.drawParams.length
                      atIndex:0];
      else
        [enc setFragmentBuffer:draws offset:0 atIndex:0];
      if (edges)
        [enc setFragmentBuffer:edges offset:0 atIndex:1];
      else {
        vector_float4 zero = {0};
        [enc setFragmentBytes:r.drawEdges.length ? r.drawEdges.bytes : &zero
                       length:MAX(r.drawEdges.length, sizeof(zero))
                      atIndex:1];
      }
      [enc setFragmentTexture:r.drawClip ?: r.white atIndex:0];
    }
    [enc setFragmentTexture:r.drawImage ?: r.white atIndex:1];
    if (r.drawCount) {
      [enc drawPrimitives:MTLPrimitiveTypeTriangle
              vertexStart:0
              vertexCount:6 * r.drawCount];
#ifdef QUARTZ_MEDIA_TEST_FIXTURES
      fixtureEncodedBatches++;
#endif
    }
  }
  [r.drawParams setLength:0];
  [r.drawEdges setLength:0];
  [r.drawImages setLength:0];
  r.drawTarget = r.drawClip = r.drawImage = nil;
  r.drawClear = NO;
  r.drawCount = 0;
  r.drawStorage = nil;
  if (++r.pendingOperations >= 256)
    flush_batch();
}
static void flush_deferred(void) {
  QMetalRenderer *r = renderer();
  if (r.encodingDeferred)
    return;
  flush_draws();
  if (!r.deferred.count)
    return;
  NSArray<QDrawBatch *> *batches = r.deferred;
  r.deferred = [NSMutableArray new];
  r.deferredBytes = 0;
  r.encodingDeferred = YES;
  for (QDrawBatch *batch in batches) {
    r.drawParams = batch.params;
    r.drawEdges = batch.edges;
    r.drawImages = batch.images;
    r.drawStorage = batch.storage;
    r.drawTarget = batch.storage.handle;
    r.drawClip = batch.clip;
    r.drawImage = batch.image;
    r.drawKind = batch.kind;
    r.drawCount = batch.count;
    r.drawClear = batch.clear;
    r.drawClearColor = batch.clearColor;
    flush_draws();
  }
  r.encodingDeferred = NO;
  end_render_encoder();
  [r.deferredSources removeAllObjects];
  r.deferredSourceBytes = 0;
}
static id<MTLCommandBuffer> batch_buffer(void) {
  flush_draws();
  flush_deferred();
  end_render_encoder();
  return raw_batch_buffer();
}
// Bounds are discovered from actual draws, not trusted caller hints. Grow an
// isolated layer as needed, preserving earlier pixels and the global CTM.
// Filters and snapshots promote it to the complete logical canvas before reads.
static void grow_layer(QMetalSurface *s, CGRect affected, BOOL full) {
  QMetalRenderer *r = renderer();
  QTexture *__attribute__((objc_precise_lifetime)) old = s.storage;
  if (!old.cropped)
    return;
  CGRect canvas = CGRectMake(0, 0, old.canvas.width, old.canvas.height);
  affected = CGRectIntersection(
      CGRectIntegral(full ? affected : CGRectInset(affected, -2, -2)), canvas);
  CGRect previous = CGRectMake(old.origin.x, old.origin.y,
                               old.region.size.width, old.region.size.height);
  if (!full && old.initialized && CGRectContainsRect(previous, affected))
    return;
  CGRect area = full                      ? canvas
                : CGRectIsNull(old.dirty) ? affected
                                          : CGRectUnion(previous, affected);
  if (CGRectIsEmpty(area) || CGRectIsNull(area))
    return;
  flush_draws();
  id<MTLTexture> next = texture(area.size.width, area.size.height);
  if (!next)
    return;
  s.texture = next;
  if (!CGRectIsNull(old.dirty)) {
    clear_texture(s);
    id<MTLBlitCommandEncoder> copy = [batch_buffer() blitCommandEncoder];
    [copy copyFromTexture:old.handle
              sourceSlice:0
              sourceLevel:0
             sourceOrigin:MTLOriginMake(old.region.origin.x,
                                        old.region.origin.y, 0)
               sourceSize:MTLSizeMake(old.region.size.width,
                                      old.region.size.height, 1)
                toTexture:next
         destinationSlice:0
         destinationLevel:0
        destinationOrigin:MTLOriginMake(old.origin.x - area.origin.x,
                                        old.origin.y - area.origin.y, 0)];
    [copy endEncoding];
    finish_operation();
  }
  s.storage.origin = area.origin;
  s.storage.canvas = old.canvas;
  s.storage.cropped = !full;
  s.storage.dirty = old.dirty;
  s.storage.initialized = YES;
  if (CGRectIsNull(old.dirty)) {
    // Fuse an empty layer's clear with its first batch of real draws.
    r.drawTarget = next;
    r.drawStorage = s.storage;
    r.drawClear = YES;
    r.drawClearColor = MTLClearColorMake(0, 0, 0, 0);
  }
}
// Small, top-level groups share a render attachment. Their regions have
// transparent padding for filtering, and grow_layer detaches a region if real
// drawing exceeds the allocation hint. Nested groups use independent storage
// so no pass samples the texture it is rendering into.
static BOOL atlas_layer(QMetalSurface *s, CGRect hint) {
  QMetalRenderer *r = renderer();
#ifdef QUARTZ_MEDIA_TEST_FIXTURES
  if (atomic_load(&fixtureDisableLayerAtlas))
    return NO;
#endif
  hint = CGRectIntersection(
      CGRectIntegral(CGRectInset(hint, -2, -2)),
      CGRectMake(0, 0, s.storage.canvas.width, s.storage.canvas.height));
  if (CGRectIsEmpty(hint) || CGRectIsNull(hint) || hint.size.width > 512 ||
      hint.size.height > 512)
    return NO;
  flush_draws();
  if (!s.layerAtlas)
    s.layerAtlas = [QLayerAtlas new];
  QLayerAtlas *atlas = s.layerAtlas;
  if (atlas.x + hint.size.width > 1024) {
    atlas.x = 0;
    atlas.y += atlas.row;
    atlas.row = 0;
  }
  if (!atlas.current || atlas.y + hint.size.height > 1024) {
    if (atlas.pages.count >= 4)
      return NO;
    atlas.current = texture(1024, 1024);
    if (!atlas.current)
      return NO;
    [atlas.pages addObject:atlas.current];
    atlas.x = atlas.y = atlas.row = 0;
  }
  QTexture *view = [QTexture new];
  view.handle = atlas.current;
  view.generation = qm_generation();
  view.canvas = s.storage.canvas;
  view.origin = hint.origin;
  view.region = CGRectMake(atlas.x, atlas.y, hint.size.width, hint.size.height);
  view.atlasOwner = atlas;
  view.cropped = view.initialized = YES;
  view.dirty = CGRectNull;
  s.storage = view;
  atlas.x += hint.size.width;
  atlas.row = MAX(atlas.row, hint.size.height);
  r.drawTarget = view.handle;
  r.drawStorage = view;
  r.drawClear = YES;
  r.drawClearColor = MTLClearColorMake(0, 0, 0, 0);
  return YES;
}
static void initialize_surface(QMetalSurface *s) {
  if (s.storage.cropped)
    grow_layer(
        s, CGRectMake(0, 0, s.storage.canvas.width, s.storage.canvas.height),
        YES);
}
static void flush_batch(void) {
  flush_draws();
  flush_deferred();
  end_render_encoder();
  QMetalRenderer *r = renderer();
  id<MTLCommandBuffer> cb = r.pending;
  r.pending = nil;
  r.pendingOperations = 0;
  r.pendingBytes = 0;
  if (cb)
    commit(cb);
}
static void finish_operation(void) {
  QMetalRenderer *r = renderer();
  if (++r.pendingOperations >= 256)
    flush_batch();
}
// Never allow Objective-C exceptions to unwind through Rust's C ABI.
// Recoverable native exceptions invalidate this generation; the compositor
// recreates it.
#define LOCK                                                                   \
  @autoreleasepool {                                                           \
    NSRecursiveLock *lock_ = quartz_lock();                                    \
    [lock_ lock];                                                              \
    @try {                                                                     \
      QMetalRenderer *r = renderer();
#define UNLOCK                                                                 \
  }                                                                            \
  @catch (NSException * error) {                                               \
    NSLog(@"Quartz Metal operation failed: %@", error);                        \
    qm_record_fault(qm_generation());                                          \
  }                                                                            \
  @finally {                                                                   \
    [lock_ unlock];                                                            \
  }                                                                            \
  }
static NSString *texture_key(NSUInteger w, NSUInteger h) {
  return [NSString
      stringWithFormat:@"%lux%lu", (unsigned long)w, (unsigned long)h];
}
static void recycle_texture(id<MTLTexture> t, uint64_t generation) {
  if (!t)
    return;
  @autoreleasepool {
    QMetalRenderer *r = renderer();
    [r.lock lock];
    if (generation != qm_generation() || atomic_load(&rendererLost)) {
      [r.lock unlock];
      return;
    }
    if (r.drawTarget == t || r.drawImage == t || r.drawClip == t)
      flush_draws();
    for (QDrawBatch *batch in r.deferred) {
      if (batch.image == t || batch.clip == t || batch.storage.handle == t) {
        flush_deferred();
        break;
      }
    }
    if (r.encoderTarget == t || [r.encoderInputs containsObject:t])
      end_render_encoder();
    // The ordered queue makes reuse safe even if an earlier encoder is still
    // executing. All command buffers retain their resources until completion.
    NSUInteger bytes = t.width * t.height * 4;
    if (generation == qm_generation() && !atomic_load(&rendererLost) &&
        t.device == r.device && bytes <= 64 * 1024 * 1024) {
      NSString *key = texture_key(t.width, t.height);
      if (r.poolBytes + bytes > 64 * 1024 * 1024) {
        [r.texturePool removeAllObjects];
        r.poolBytes = 0;
      }
      NSMutableArray *bucket = r.texturePool[key];
      if (!bucket)
        r.texturePool[key] = bucket = [NSMutableArray new];
      if (bucket.count < 32) {
        [bucket addObject:t];
        r.poolBytes += bytes;
      }
    }
    [r.lock unlock];
  }
}
static id<MTLTexture> texture(size_t w, size_t h) {
  QMetalRenderer *r = renderer();
  if (!r || !w || !h || w > 16384 || h > 16384)
    return nil;
#ifdef QUARTZ_MEDIA_TEST_FIXTURES
  if (atomic_exchange(&fixtureFailNextTexture, false)) {
    qm_record_fault(qm_generation());
    return nil;
  }
#endif
  NSUInteger bytes = w * h * 4;
  NSMutableArray *bucket = r.texturePool[texture_key(w, h)];
  id<MTLTexture> pooled = bucket.lastObject;
  if (pooled) {
    [bucket removeLastObject];
    r.poolBytes -= bytes;
    return pooled;
  }
  if (r.pendingBytes && r.pendingBytes + bytes > 64 * 1024 * 1024)
    flush_batch();
  r.pendingBytes += bytes;
  MTLTextureDescriptor *d =
      [MTLTextureDescriptor texture2DDescriptorWithPixelFormat:r.canvasFormat
                                                         width:w
                                                        height:h
                                                     mipmapped:NO];
  d.storageMode = MTLStorageModePrivate;
  d.usage = MTLTextureUsageShaderRead | MTLTextureUsageShaderWrite |
            MTLTextureUsageRenderTarget;
  id<MTLTexture> t = [renderer().device newTextureWithDescriptor:d];
  if (!t) {
    qm_record_fault(qm_generation());
    return nil;
  }
  atomic_fetch_add(&counters[8], 1);
  return t;
}
static QState *copy_state(QState *s) {
  QState *n = [QState new];
  n.transform = s.transform;
  n.clip = s.clip;
  n.clipRect = s.clipRect;
  n.shadow = s.shadow;
  n.shadowColor = s.shadowColor;
  n.gradientOptions = s.gradientOptions;
  n.blendMode = s.blendMode;
  n.aliased = s.aliased;
  n.interpolation = s.interpolation;
  n.hinting = s.hinting;
  n.rclip = s.rclip;
  n.rradius = s.rradius;
  n.rclipped = s.rclipped;
  n.sclip = s.sclip;
  n.sclipped = s.sclipped;
  return n;
}
static QParams params(QMetalSurface *s) {
  QParams p = {0};
  QState *state = s.state;
  QTexture *storage = s.storage;
  // BitmapBlendingMode applies to draw_bitmap, not vector or text paint.
  // Image drawing supplies its mode explicitly in draw_texture.
  p.blend = 0;
  p.flipped = (state.aliased ? 1 : 0) | (state.interpolation == 1 ? 2 : 0) |
              (state.interpolation << 2);
  p.extent = (vector_uint2){storage.canvas.width, storage.canvas.height};
  p.target = (vector_float4){storage.origin.x - storage.region.origin.x,
                             storage.origin.y - storage.region.origin.y, 0, 0};
  CGAffineTransform inv = CGAffineTransformInvert(state.transform);
  p.inverse = (vector_float4){inv.a, inv.b, inv.c, inv.d};
  p.translation = (vector_float4){inv.tx, inv.ty, 0, 1};
  if (state.clip) {
    p.clipped |= 1;
    CGRect c = state.clipRect;
    p.clip =
        (vector_float4){c.origin.x, c.origin.y, c.size.width, c.size.height};
  }
  if (state.rclipped) {
    p.clipped |= 2;
    p.rclip = state.rclip;
    p.rradius = state.rradius;
  }
  if (state.sclipped) {
    p.clipped |= 4;
    p.sclip = state.sclip;
  }
  return p;
}
// Path coverage only needs segments crossing the pixel's row. Device-space
// bins preserve winding and save the GPU from scanning a whole text run per
// pixel.
static NSData *binned_edges(QParams *p, NSData *edges) {
  if (p->edges < 32 || p->mode == 6 || !p->extent.y)
    return edges;
  const uint32_t stride = 8;
  uint32_t origin = p->origin.y / stride * stride,
           rows = (p->origin.y + p->extent.y - origin + stride - 1) / stride;
  vector_uint4 header = {origin, rows, stride, 0};
  NSMutableData *key = [NSMutableData dataWithBytes:&header
                                             length:sizeof(header)];
  [key appendData:edges];
  NSData *cached = renderer().edgeCache[key];
  if (cached) {
    p->edgeRows = rows;
    p->edgeOriginY = origin;
    p->edgeStride = stride;
    return cached;
  }
  NSMutableArray<NSMutableData *> *bins =
      [NSMutableArray arrayWithCapacity:rows];
  for (uint32_t row = 0; row < rows; row++)
    [bins addObject:[NSMutableData new]];
  const vector_float4 *segments = edges.bytes;
  size_t bytes = 0;
  for (uint32_t i = 0; i < p->edges; i++) {
    vector_float4 edge = segments[i];
    if (edge.y == edge.w)
      continue;
    double low = fmin(edge.y, edge.w), high = fmax(edge.y, edge.w);
    if (high < origin || low >= origin + rows * stride)
      continue;
    int first = MAX(0, (int)floor((low - origin) / stride));
    int last = MIN((int)rows - 1, (int)floor((high - origin) / stride));
    for (int row = first; row <= last; row++) {
      [bins[row] appendBytes:&edge length:sizeof(edge)];
      bytes += sizeof(edge);
    }
    if (bytes > 32 * 1024 * 1024 || bytes > edges.length * 32)
      return edges;
  }
  NSMutableData *result =
      [NSMutableData dataWithLength:rows * sizeof(vector_uint4)];
  vector_uint4 *starts = result.mutableBytes;
  uint32_t offset = 0;
  for (uint32_t row = 0; row < rows; row++) {
    uint32_t n = (uint32_t)(bins[row].length / 16);
    starts[row] = (vector_uint4){offset, n, 0, 0};
    offset += n;
  }
  for (NSData *row in bins)
    [result appendData:row];
  NSUInteger bytesToKeep = key.length + result.length;
  if (renderer().edgeCache.count >= 2048 ||
      renderer().edgeCacheBytes + bytesToKeep > 16 * 1024 * 1024) {
    [renderer().edgeCache removeAllObjects];
    renderer().edgeCacheBytes = 0;
  }
  if (bytesToKeep <= 16 * 1024 * 1024) {
    renderer().edgeCache[key] = result;
    renderer().edgeCacheBytes += bytesToKeep;
  }
  p->edgeRows = rows;
  p->edgeOriginY = origin;
  p->edgeStride = stride;
  return result;
}
static void copy_texture(id<MTLTexture> dst, id<MTLTexture> src);
static void private_surface(QMetalSurface *s) {
  if (!s.storage.external)
    return;
  flush_draws();
  id<MTLTexture> previous = s.texture;
  CGRect dirty = s.storage.dirty;
  s.texture = texture(previous.width, previous.height);
  copy_texture(s.texture, previous);
  s.storage.dirty = dirty;
}
static void writable_surface(QMetalSurface *s) {
  MTLTextureUsage required =
      MTLTextureUsageShaderRead | MTLTextureUsageShaderWrite;
  if (s.storage.external && (s.texture.usage & required) != required)
    private_surface(s);
}
static void readable_surface(QMetalSurface *s) {
  if (s.storage.external && !(s.texture.usage & MTLTextureUsageShaderRead))
    private_surface(s);
}
static void dispatch(QMetalSurface *s, id<MTLTexture> target, QParams p,
                     NSData *edges, NSData *stops, id<MTLTexture> image);
// Coverage is translation invariant at integral device offsets. Cache the
// GPU-rasterized mask by normalized edges and AA/fill rule, independently of
// brush color and clip. R16Float exactly represents all four-sample coverages.
static void clear_coverage_paths(QMetalRenderer *r) {
  CFDictionaryRemoveAllValues(r.coveragePaths);
  r.coveragePathBytes = 0;
}
static QCoverageImage *path_coverage(QParams p, NSData *edges) {
  QMetalRenderer *r = renderer();
  NSUInteger bytes = (NSUInteger)p.extent.x * p.extent.y * 2;
  if (!bytes || bytes > 1024 * 1024 || p.edges > 4096 || p.extent.x > 2048 ||
      p.extent.y > 2048)
    return nil;
  if (CFDictionaryGetCount(r.coveragePaths) >= 4096 ||
      r.coveragePathBytes + edges.length > 16 * 1024 * 1024)
    clear_coverage_paths(r);
  uint32_t variant[] = {p.origin.x, p.origin.y, p.extent.x,
                        p.extent.y, p.evenodd,  p.flipped & 1};
  QSmallKey variantKey = small_key(variant, sizeof(variant));
  CFMutableDictionaryRef variants =
      (CFMutableDictionaryRef)CFDictionaryGetValue(r.coveragePaths,
                                                   (__bridge CFTypeRef)edges);
  QCoverageImage *ready =
      (__bridge QCoverageImage *)small_get(variants, &variantKey);
  if (ready)
    return ready;
  NSMutableData *normalized = [edges mutableCopy];
  vector_float4 *es = normalized.mutableBytes;
  vector_float4 origin = {p.origin.x, p.origin.y, p.origin.x, p.origin.y};
  for (NSUInteger i = 0; i < p.edges; i++)
    es[i] -= origin;
  vector_uint4 header = {p.extent.x, p.extent.y, p.evenodd, p.flipped & 1};
  NSMutableData *key = [NSMutableData dataWithBytes:&header
                                             length:sizeof(header)];
  [key appendData:normalized];
  QCoverageImage *cached = r.coverageImages[key];
  if (cached) {
    if (!variants) {
      variants = small_dictionary();
      if (!variants)
        return cached;
      CFDictionarySetValue(r.coveragePaths, (__bridge CFTypeRef)edges,
                           variants);
      CFRelease(variants);
      r.coveragePathBytes += edges.length;
    }
    small_set(variants, &variantKey, (__bridge CFTypeRef)cached);
    return cached;
  }
  MTLTextureDescriptor *desc = [MTLTextureDescriptor
      texture2DDescriptorWithPixelFormat:MTLPixelFormatR16Float
                                   width:p.extent.x
                                  height:p.extent.y
                               mipmapped:NO];
  desc.storageMode = MTLStorageModePrivate;
  desc.usage = MTLTextureUsageShaderRead | MTLTextureUsageShaderWrite;
  id<MTLTexture> mask = [r.device newTextureWithDescriptor:desc];
  if (!mask)
    return nil; // Optional cache allocation must not invalidate the renderer.
  QParams raster = {0};
  raster.extent = p.extent;
  raster.edges = p.edges;
  raster.evenodd = p.evenodd;
  raster.flipped = p.flipped & 1;
  raster.mode = 1;
  // Clip and source are unused by mask generation. A standalone surface keeps
  // it independent of the destination's clip and deferred clear state.
  QMetalSurface *surface = [QMetalSurface new];
  surface.state = [QState new];
  dispatch(surface, mask, raster, normalized, nil, nil);
  if (r.coverageImages.count >= 4096 ||
      r.coverageBytes + key.length > 64 * 1024 * 1024) {
    [r.coverageImages removeAllObjects];
    clear_coverage_paths(r);
    variants = NULL;
    r.coverageAtlas = nil;
    r.coverageBytes = r.coverageX = r.coverageY = r.coverageRow = 0;
  }
  if (r.coverageX + p.extent.x > 2048) {
    r.coverageX = 0;
    r.coverageY += r.coverageRow;
    r.coverageRow = 0;
  }
  if (!r.coverageAtlas || r.coverageY + p.extent.y > 2048) {
    if (r.coverageImages.count >= 4096 ||
        r.coverageBytes + 8 * 1024 * 1024 > 64 * 1024 * 1024) {
      [r.coverageImages removeAllObjects];
      clear_coverage_paths(r);
      variants = NULL;
      r.coverageBytes = 0;
    }
    MTLTextureDescriptor *atlas = [MTLTextureDescriptor
        texture2DDescriptorWithPixelFormat:MTLPixelFormatR16Float
                                     width:2048
                                    height:2048
                                 mipmapped:NO];
    atlas.storageMode = MTLStorageModePrivate;
    atlas.usage = MTLTextureUsageShaderRead;
    r.coverageAtlas = [r.device newTextureWithDescriptor:atlas];
    r.coverageX = r.coverageY = r.coverageRow = 0;
    if (!r.coverageAtlas)
      return nil;
    r.coverageBytes += 8 * 1024 * 1024;
  }
  id<MTLBlitCommandEncoder> copy = [batch_buffer() blitCommandEncoder];
  [copy copyFromTexture:mask
            sourceSlice:0
            sourceLevel:0
           sourceOrigin:MTLOriginMake(0, 0, 0)
             sourceSize:MTLSizeMake(p.extent.x, p.extent.y, 1)
              toTexture:r.coverageAtlas
       destinationSlice:0
       destinationLevel:0
      destinationOrigin:MTLOriginMake(r.coverageX, r.coverageY, 0)];
  [copy endEncoding];
  finish_operation();
  cached = [QCoverageImage new];
  cached.texture = r.coverageAtlas;
  cached.rect = CGRectMake(r.coverageX, r.coverageY, p.extent.x, p.extent.y);
  r.coverageX += p.extent.x;
  r.coverageRow = MAX(r.coverageRow, p.extent.y);
  r.coverageImages[key] = cached;
  r.coverageBytes += key.length;
  if (!variants) {
    variants = small_dictionary();
    if (!variants)
      return cached;
    CFDictionarySetValue(r.coveragePaths, (__bridge CFTypeRef)edges, variants);
    CFRelease(variants);
    r.coveragePathBytes += edges.length;
  }
  small_set(variants, &variantKey, (__bridge CFTypeRef)cached);

  return cached;
}
static void dispatch(QMetalSurface *s, id<MTLTexture> target, QParams p,
                     NSData *edges, NSData *stops, id<MTLTexture> image) {
  QTexture *storage = s.storage;
  QState *state = s.state;
  id<MTLTexture> destination = storage.handle;
  if (!target && storage.cropped && p.mode != 1 && p.extent.x && p.extent.y &&
      !atomic_load(&rendererLost)) {
    // Unhinted groups allocate only when real pixels are painted. Hinted
    // groups normally acquire an atlas view before reaching this path.
    grow_layer(s, CGRectMake(p.origin.x, p.origin.y, p.extent.x, p.extent.y),
               !(p.blend == 0 && (p.mode == 0 || p.mode == 4)));
    storage = s.storage;
    target = destination = storage.handle;
    p.target =
        (vector_float4){storage.origin.x - storage.region.origin.x,
                        storage.origin.y - storage.region.origin.y, 0, 0};
  }
  if (!target || (!destination && p.mode != 1) || !p.extent.x || !p.extent.y)
    return;
  QMetalRenderer *r = renderer();
  if (target == destination &&
      !(p.blend == 0 && (p.mode == 0 || p.mode == 4))) {
    writable_surface(s);
    storage = s.storage;
    target = destination = storage.handle;
  }
  if (target == destination && storage.cropped) {
    BOOL full = !(p.blend == 0 && (p.mode == 0 || p.mode == 4));
    grow_layer(s, CGRectMake(p.origin.x, p.origin.y, p.extent.x, p.extent.y),
               full);
    storage = s.storage;
    target = destination = storage.handle;
    if (!target || atomic_load(&rendererLost))
      return;
    p.target =
        (vector_float4){storage.origin.x - storage.region.origin.x,
                        storage.origin.y - storage.region.origin.y, 0, 0};
  }
  if (target == destination && p.mode != 1 && p.extent.x && p.extent.y) {
    CGRect affected =
        CGRectMake(p.origin.x, p.origin.y, p.extent.x, p.extent.y);
    storage.dirty = CGRectUnion(storage.dirty, affected);
  }
  if (p.mode == 0 && p.blend == 0 && p.edges && target == destination &&
      !((p.reserved & 8) && !p.clipped)) {
    QCoverageImage *coverage = path_coverage(p, edges);
    if (coverage) {
      image = coverage.texture;
      p.mode = 4;
      p.reserved = 2; // Single-channel exact-coverage mask, tinted like glyphs.
      p.src = (vector_float4){coverage.rect.origin.x, coverage.rect.origin.y,
                              p.extent.x, p.extent.y};
      p.dst = (vector_float4){p.origin.x, p.origin.y, p.extent.x, p.extent.y};
      p.inverse = (vector_float4){1, 0, 0, 1};
      p.translation = (vector_float4){0, 0, 0, 1};
      p.flipped = 2 | (1 << 2); // one device texel per pixel, nearest
      p.edges = 0;
      edges = nil;
    }
  }
  edges = binned_edges(&p, edges);
  if (!p.extent.x || !p.extent.y)
    return;
  if (p.blend == 0 && (p.mode == 0 || p.mode == 4) && image != target) {
    if (r.capture) {
      NSUInteger capturedKind =
          ((!p.edges || (p.reserved & 8)) && !p.clipped && (p.flipped >> 2) < 3)
              ? ((p.reserved & 8) ? 3 : 2)
              : 1;
      if (p.mode != 4 ||
          (r.capture.params.length && r.capture.kind != capturedKind) ||
          (r.capture.image && r.capture.image != image))
        r.capture = nil;
      else {
        if (capturedKind == 3) {
          QRectImageParams compact = rect_image_params(p, image);
          [r.capture.images appendBytes:&compact length:sizeof(compact)];
        } else if (capturedKind == 2) {
          QImageParams compact = image_params(p, image);
          [r.capture.images appendBytes:&compact length:sizeof(compact)];
        }
        r.capture.kind = capturedKind;
        r.capture.image = image;
        QParams saved = p;
        saved.edgePad = r.capture.edges.length / sizeof(vector_float4);
        if (edges.length)
          [r.capture.edges appendData:edges];
        r.capture.bounds =
            r.capture.params.length
                ? CGRectUnion(r.capture.bounds,
                              CGRectMake(p.origin.x, p.origin.y, p.extent.x,
                                         p.extent.y))
                : CGRectMake(p.origin.x, p.origin.y, p.extent.x, p.extent.y);
        [r.capture.params appendBytes:&saved length:sizeof(saved)];
      }
    }
    NSUInteger kind =
        p.mode == 0 ? ((!p.clipped && (!p.edges || (p.reserved & 8))) ? 4 : 0)
        : ((!p.edges || (p.reserved & 8)) && !p.clipped && (p.flipped >> 2) < 3)
            ? ((p.reserved & 8) ? 3 : 2)
            : 1;
    NSMutableData *drawParams = r.drawParams;
    if (r.drawTarget != target ||
        (r.drawCount && (r.drawKind != kind || r.drawClip != state.clip ||
                         r.drawImage != image || r.drawCount >= 4096))) {
      flush_draws();
      drawParams = r.drawParams;
    }
    NSMutableData *drawImages = r.drawImages, *drawEdges = r.drawEdges;
    if (kind == 4) {
      vector_float2 low = {(float)p.origin.x, (float)p.origin.y};
      vector_float2 high = low + (vector_float2){p.extent.x, p.extent.y};
      if (p.reserved & 8) {
        low = simd_max(low, p.gradient.xy);
        high = simd_min(high, p.gradient.zw);
        if (p.flipped & 1) {
          // Aliased coverage tests the pixel center, with an inclusive low
          // and exclusive high edge. Snapping preserves that exact decision.
          low = (vector_float2){ceilf(low.x - 0.5f), ceilf(low.y - 0.5f)};
          high = (vector_float2){ceilf(high.x - 0.5f), ceilf(high.y - 0.5f)};
        }
      }
      QSolidParams compact = {
          {low.x - p.target.x, low.y - p.target.y, high.x - low.x,
           high.y - low.y},
          {p.color.x * p.color.w, p.color.y * p.color.w, p.color.z * p.color.w,
           p.color.w},
          (p.reserved & 16) ? (vector_float4){p.src.x, p.src.y, 0, 0}
                            : (vector_float4){0, 0, 0, 0}};
      [drawImages appendBytes:&compact length:sizeof(compact)];
    } else if (kind == 3) {
      QRectImageParams compact = rect_image_params(p, image);
      [drawImages appendBytes:&compact length:sizeof(compact)];
    } else if (kind == 2) {
      QImageParams compact = image_params(p, image);
      [drawImages appendBytes:&compact length:sizeof(compact)];
    }
    r.drawKind = kind;
    r.drawStorage = target == destination ? storage : nil;
    r.drawTarget = target;
    r.drawClip = state.clip;
    r.drawImage = image;
    // Compact formats already contain every vertex input. Generic QParams
    // and edge storage are only consumed by paths/coverage paints.
    if (kind < 2) {
      p.edgePad = drawEdges.length / sizeof(vector_float4);
      if (edges.length)
        [drawEdges appendData:edges];
      [drawParams appendBytes:&p length:sizeof(p)];
    }
    r.drawCount++;
    if (target == destination)
      storage.mipmap = nil;
    atomic_fetch_add(&counters[p.mode == 4 ? 2 : 1], 1);
    return;
  }
  if (target == destination)
    storage.mipmap = nil;
  r.capture = nil;
  if (target == destination) {
    initialize_surface(s);
    storage = s.storage;
    target = destination = storage.handle;
    p.target =
        (vector_float4){storage.origin.x - storage.region.origin.x,
                        storage.origin.y - storage.region.origin.y, 0, 0};
  }
  id<MTLTexture> previous = nil;
  if (r.separateDestination && p.mode != 1 &&
      !(p.mode == 5 && p.blend == 17 && !p.edges && !p.clipped)) {
    // Tier-1/simulator devices cannot bind RGBA8 as read_write. Copy only
    // the affected region on the GPU, then read and write distinct textures.
    previous = texture(target.width, target.height);
    if (!previous) {
      qm_record_fault(qm_generation());
      return;
    }
    id<MTLBlitCommandEncoder> copy = [batch_buffer() blitCommandEncoder];
    MTLOrigin origin =
        MTLOriginMake(p.origin.x - p.target.x, p.origin.y - p.target.y, 0);
    [copy copyFromTexture:target
              sourceSlice:0
              sourceLevel:0
             sourceOrigin:origin
               sourceSize:MTLSizeMake(p.extent.x, p.extent.y, 1)
                toTexture:previous
         destinationSlice:0
         destinationLevel:0
        destinationOrigin:origin];
    [copy endEncoding];
  }
  id<MTLCommandBuffer> cb = batch_buffer();
  id<MTLComputeCommandEncoder> enc = [cb computeCommandEncoder];
  if (!cb || !enc) {
    qm_record_fault(qm_generation());
    return;
  }
  [enc setComputePipelineState:r.paint];
  [enc setTexture:target atIndex:0];
  [enc setTexture:state.clip ?: r.white atIndex:1];
  [enc setTexture:image ?: r.white atIndex:2];
  if (r.separateDestination)
    [enc setTexture:previous ?: r.white atIndex:3];
  vector_float4 zero = {0};
  id<MTLBuffer> e =
      [r.device newBufferWithBytes:edges.length ? edges.bytes : &zero
                            length:MAX(edges.length, sizeof(zero))
                           options:MTLResourceStorageModeShared];
  id<MTLBuffer> g =
      [r.device newBufferWithBytes:stops.length ? stops.bytes : &zero
                            length:MAX(stops.length, sizeof(zero))
                           options:MTLResourceStorageModeShared];
  if (!e || !g) {
    [enc endEncoding];
    qm_record_fault(qm_generation());
    return;
  }
  [enc setBuffer:e offset:0 atIndex:0];
  [enc setBytes:&p length:sizeof(p) atIndex:1];
  [enc setBuffer:g offset:0 atIndex:2];
  [enc dispatchThreads:MTLSizeMake(p.extent.x, p.extent.y, 1)
      threadsPerThreadgroup:MTLSizeMake(8, 8, 1)];
  [enc endEncoding];
  finish_operation();
  if (previous)
    recycle_texture(previous, qm_generation());
  atomic_fetch_add(&counters[(p.mode == 4 || p.mode == 8) ? 2 : 1], 1);
}
static CGRect crop(QMetalSurface *s, CGRect rect) {
  rect = CGRectIntersection(
      CGRectIntegral(rect),
      CGRectMake(0, 0, s.storage.canvas.width, s.storage.canvas.height));
  if (s.state.clip || s.state.rclipped || s.state.sclipped)
    rect = CGRectIntersection(rect, s.state.clipRect);
  return CGRectIsNull(rect) ? CGRectZero : rect;
}
static void bounds(QParams *p, CGRect rect) {
  p->origin = (vector_uint2){MAX(0, rect.origin.x), MAX(0, rect.origin.y)};
  p->extent = (vector_uint2){MAX(0, rect.size.width), MAX(0, rect.size.height)};
}
typedef struct {
  __unsafe_unretained NSMutableData *data;
  CGPoint first, last;
  bool open;
} EdgeSink;
static void edge(EdgeSink *s, CGPoint to) {
  if (!CGPointEqualToPoint(s->last, to)) {
    vector_float4 e = {s->last.x, s->last.y, to.x, to.y};
    [s->data appendBytes:&e length:sizeof(e)];
  }
  s->last = to;
}
static void path_edge(void *info, const CGPathElement *e) {
  EdgeSink *s = info;
  switch (e->type) {
  case kCGPathElementMoveToPoint:
    if (s->open)
      edge(s, s->first);
    s->first = s->last = e->points[0];
    s->open = true;
    break;
  case kCGPathElementAddLineToPoint:
    edge(s, e->points[0]);
    break;
  case kCGPathElementCloseSubpath:
    edge(s, s->first);
    s->open = false;
    break;
  default:
    break; // Non-flattened curves contribute no edge.
  }
}
static NSData *path_edges(QMetalSurface *s, CGPathRef path, CGRect *box) {
  CGAffineTransform t = s.state.transform;
  QMetalRenderer *r = renderer();
  QSmallKey key = small_key(&t, sizeof(t));
  CFMutableDictionaryRef paths =
      (CFMutableDictionaryRef)CFDictionaryGetValue(r.pathCache, path);
  QPathEdges *cached = (__bridge QPathEdges *)small_get(paths, &key);
  if (cached) {
    *box = cached.box;
    return cached.edges;
  }
  CGPathRef transformed = CGPathCreateCopyByTransformingPath(path, &t);
  *box = CGPathGetPathBoundingBox(transformed);
  CGPathRef flat = CGPathCreateCopyByFlattening(transformed, 0.25);
  NSMutableData *data = [NSMutableData new];
  EdgeSink sink = {data, {0, 0}, {0, 0}, false};
  CGPathApply(flat, &sink, path_edge);
  if (sink.open)
    edge(&sink, sink.first);
  CGPathRelease(flat);
  CGPathRelease(transformed);
  if (r.pathBytes + data.length < 16 * 1024 * 1024) {
    if (r.pathEntries >= 2048) {
      CFDictionaryRemoveAllValues(r.pathCache);
      r.pathBytes = r.pathEntries = 0;
      paths = NULL;
    }
    if (!paths) {
      paths = small_dictionary();
      if (!paths)
        return data;
      CFDictionarySetValue(r.pathCache, path, paths);
      CFRelease(paths);
    }
    QPathEdges *value = [QPathEdges new];
    value.box = *box;
    value.edges = data;
    if (small_set(paths, &key, (__bridge CFTypeRef)value)) {
      r.pathEntries++;
      r.pathBytes += data.length;
    } else if (!CFDictionaryGetCount(paths)) {
      CFDictionaryRemoveValue(r.pathCache, path);
    }
  }
  return data;
}
static NSData *rect_edges(QMetalSurface *s, CGRect rect, CGRect *box) {
  CGAffineTransform transform = s.state.transform;
  CGPoint a = CGPointApplyAffineTransform(rect.origin, transform);
  CGPoint b = CGPointApplyAffineTransform(
      CGPointMake(CGRectGetMaxX(rect), rect.origin.y), transform);
  CGPoint c = CGPointApplyAffineTransform(
      CGPointMake(CGRectGetMaxX(rect), CGRectGetMaxY(rect)), transform);
  CGPoint d = CGPointApplyAffineTransform(
      CGPointMake(rect.origin.x, CGRectGetMaxY(rect)), transform);
  vector_float4 e[] = {{a.x, a.y, b.x, b.y},
                       {b.x, b.y, c.x, c.y},
                       {c.x, c.y, d.x, d.y},
                       {d.x, d.y, a.x, a.y}};
  CGFloat x = MIN(MIN(a.x, b.x), MIN(c.x, d.x)),
          y = MIN(MIN(a.y, b.y), MIN(c.y, d.y));
  *box = CGRectMake(x, y, MAX(MAX(a.x, b.x), MAX(c.x, d.x)) - x,
                    MAX(MAX(a.y, b.y), MAX(c.y, d.y)) - y);
  if (transform.b == 0 && transform.c == 0 && x == floor(x) && y == floor(y) &&
      CGRectGetMaxX(*box) == floor(CGRectGetMaxX(*box)) &&
      CGRectGetMaxY(*box) == floor(CGRectGetMaxY(*box)))
    return nil;
  return [NSData dataWithBytes:e length:sizeof(e)];
}
static void clear_texture(QMetalSurface *s) {
  if (!s.texture)
    return;
  id<MTLCommandBuffer> cb = batch_buffer();
  MTLRenderPassDescriptor *pass =
      [MTLRenderPassDescriptor renderPassDescriptor];
  pass.colorAttachments[0].texture = s.texture;
  pass.colorAttachments[0].loadAction = MTLLoadActionClear;
  pass.colorAttachments[0].storeAction = MTLStoreActionStore;
  pass.colorAttachments[0].clearColor = MTLClearColorMake(0, 0, 0, 0);
  id<MTLRenderCommandEncoder> enc =
      [cb renderCommandEncoderWithDescriptor:pass];
  if (!enc)
    qm_record_fault(qm_generation());
  [enc endEncoding];
  s.storage.mipmap = nil;
  s.storage.dirty = CGRectNull;
  finish_operation();
  atomic_fetch_add(&counters[1], 1);
}

void *qm_surface(size_t w, size_t h) {
  LOCK if (!r) return NULL;
  QMetalSurface *s = [QMetalSurface new];
  s.texture = texture(w, h);
  if (!s.texture)
    return NULL;
  s.states = [NSMutableArray new];
  s.layers = [NSMutableArray new];
  s.state = [QState new];
  s.state.transform = CGAffineTransformIdentity;
  s.state.gradientOptions = (vector_float4){0, 1, 0, 0};
  clear_texture(s);
  atomic_fetch_add(&counters[0], 1);
  return (__bridge_retained void *)s;
  UNLOCK
  return NULL;
}
void *qm_surface_external(void *raw) {
  LOCK if (!r || !raw) return NULL;
  id<MTLTexture> image = (__bridge id<MTLTexture>)raw;
  if (image.device.registryID != r.device.registryID || image.framebufferOnly ||
      image.pixelFormat != r.canvasFormat ||
      image.textureType != MTLTextureType2D || image.sampleCount != 1 ||
      !(image.usage & MTLTextureUsageRenderTarget) || !image.width ||
      !image.height || image.width > 16384 || image.height > 16384)
    return NULL;
  QMetalSurface *s = [QMetalSurface new];
  s.texture = image;
  s.storage.external = YES;
  s.states = [NSMutableArray new];
  s.layers = [NSMutableArray new];
  s.state = [QState new];
  s.state.transform = CGAffineTransformIdentity;
  s.state.gradientOptions = (vector_float4){0, 1, 0, 0};
  return (__bridge_retained void *)s;
  UNLOCK
  return NULL;
}
uint64_t qm_configure(void *device) {
  LOCK if (!r) return 0;
  // A different platform device is a recoverable incompatibility, never an
  // exception.
  if (!r || !device ||
      ((__bridge id<MTLDevice>)device).registryID != r.device.registryID)
    return 0;
  if (atomic_load(&rendererLost)) {
    // A failed queue is not a completion proof. Frame conversions submit with
    // their own callbacks; indeterminate producer leases remain quarantined.
    // Drop unsubmitted UI commands and start a new, independently ordered
    // queue.
    end_render_encoder();
    [r.deferred removeAllObjects];
    [r.deferredSources removeAllObjects];
    r.deferredBytes = r.deferredSourceBytes = 0;
    r.encodingDeferred = NO;
    [r.drawParams setLength:0];
    [r.drawEdges setLength:0];
    [r.drawImages setLength:0];
    r.drawClear = NO;
    r.drawCount = 0;
    r.drawStorage = nil;
    r.drawTarget = r.drawClip = r.drawImage = r.glyphAtlas = nil;
    r.atlasX = r.atlasY = r.atlasRow = r.atlasBytes = 0;
    r.monoAtlas = nil;
    r.monoX = r.monoY = r.monoRow = 0;
    r.pending = nil;
    r.pendingOperations = r.pendingBytes = 0;
    r.queue = [r.device newCommandQueueWithMaxCommandBufferCount:4096];
    r.event = [r.device newSharedEvent];
    r.eventValue = 0;
    [r.texturePool removeAllObjects];
    r.poolBytes = 0;
    [r.edgeCache removeAllObjects];
    r.edgeCacheBytes = 0;
    CFDictionaryRemoveAllValues(r.glyphCache);
    CFDictionaryRemoveAllValues(r.glyphImages);
    r.glyphImageBytes = 0;
    CFDictionaryRemoveAllValues(r.preparedRuns);
    CFDictionaryRemoveAllValues(r.pathCache);
    clear_coverage_paths(r);
    [r.buffers removeAllObjects];
    r.bufferBytes = 0;
    r.preparedBytes = r.pathBytes = r.pathEntries = 0;
    r.capture = nil;
    [r.coverageImages removeAllObjects];
    r.coverageBytes = 0;
    r.coverageAtlas = nil;
    r.coverageX = r.coverageY = r.coverageRow = 0;
    if (!r.queue || !r.event)
      return 0;
    atomic_fetch_add(&rendererGeneration, 1);
    atomic_store(&rendererLost, false);
  }
  return qm_generation();
  UNLOCK
  return 0;
}
int qm_attach_queue(void *rawQueue) {
  LOCK if (!r || !rawQueue) return 0;
  id<MTLCommandQueue> queue = (__bridge id<MTLCommandQueue>)rawQueue;
  if (queue.device.registryID != r.device.registryID)
    return 0;
  if (queue != r.queue) {
    uint64_t value = ++r.eventValue;
    [batch_buffer() encodeSignalEvent:r.event value:value];
    flush_batch();
    id<MTLCommandBuffer> wait = [queue commandBuffer];
    [wait encodeWaitForEvent:r.event value:value];
    commit(wait);
    r.queue = queue;
  }
  return 1;
  UNLOCK
  return 0;
}
void qm_end_context(void *surface) {
  LOCK if (!r || !surface) return;
  ((__bridge QMetalSurface *)surface).layerAtlas = nil;
  UNLOCK
}
void qm_save(void *surface) {
  LOCK if (!r || !surface) return;
  QMetalSurface *s = (__bridge QMetalSurface *)surface;
  [s.states addObject:copy_state(s.state)];
  UNLOCK
}
void qm_restore(void *surface) {
  LOCK if (!r || !surface) return;
  QMetalSurface *s = (__bridge QMetalSurface *)surface;
  if (!s.states.count)
    return;
  s.state = s.states.lastObject;
  [s.states removeLastObject];
  UNLOCK
}
void qm_set_transform(void *surface, double a, double b, double c, double d,
                      double x, double y, double sx, double sy) {
  LOCK if (!r || !surface) return;
  QMetalSurface *s = (__bridge QMetalSurface *)surface;
  s.state.transform =
      CGAffineTransformMake(a * sx, b * sy, c * sx, d * sy, x * sx, y * sy);
  UNLOCK
}
void qm_prepare(void *surface, const double *m, int blend, int aliased,
                int interpolation) {
  LOCK if (!r || !surface || !m) return;
  QState *state = ((__bridge QMetalSurface *)surface).state;
  state.transform =
      CGAffineTransformMake(m[0] * m[6], m[1] * m[7], m[2] * m[6], m[3] * m[7],
                            m[4] * m[6], m[5] * m[7]);
  state.blendMode = blend;
  state.aliased = aliased;
  state.interpolation = interpolation;
  UNLOCK
}
void qm_clear(void *surface, double red, double green, double blue,
              double alpha) {
  LOCK if (!r || !surface) return;
  QMetalSurface *s = (__bridge QMetalSurface *)surface;
  initialize_surface(s);
  QParams p = params(s);
  p.mode = 5;
  p.blend = 17;
  p.color = (vector_float4){red, green, blue, alpha};
  if (!p.clipped && s.texture) {
    flush_draws();
    r.drawStorage = s.storage;
    r.drawTarget = s.texture;
    r.drawClear = YES;
    r.drawClearColor =
        MTLClearColorMake(red * alpha, green * alpha, blue * alpha, alpha);
    s.storage.mipmap = nil;
    s.storage.dirty = CGRectMake(0, 0, s.texture.width, s.texture.height);
    atomic_fetch_add(&counters[1], 1);
  } else
    dispatch(s, s.texture, p, nil, nil, nil);
  UNLOCK
}
void qm_clip(void *surface, CGPathRef path, int evenodd) {
  LOCK if (!r || !surface) return;
  QMetalSurface *s = (__bridge QMetalSurface *)surface;
  CGRect box;
  NSData *edges = path_edges(s, path, &box);
  box = crop(s, box);
  QParams p = params(s);
  p.mode = 1;
  p.edges = (uint32_t)(edges.length / 16);
  p.evenodd = evenodd;
  bounds(&p, box);
  id<MTLTexture> mask = texture(MAX(1, p.extent.x), MAX(1, p.extent.y));
  dispatch(s, mask, p, edges, nil, nil);
  s.state.clip = mask;
  s.state.clipRect = box;
  // The mask was drawn through the analytic clips: it holds them now.
  s.state.rclipped = 0;
  s.state.sclipped = 0;
  UNLOCK
}
// A rectangle clip with equal elliptical corners (x, y, width, height, rx, ry
// in user space), as a coverage function of the shaders instead of a mask:
// no texture, no path and no pass for it. Applied when the transform keeps
// axes and no other clip is in effect; returns 0 otherwise, and the caller
// clips with the path (whose mask then takes the analytic clip in).
int qm_clip_rounded(void *surface, const double *rect) {
  int applied = 0;
  LOCK if (!r || !surface || !rect) return 0;
  QMetalSurface *s = (__bridge QMetalSurface *)surface;
  CGAffineTransform t = s.state.transform;
  double x = rect[0], y = rect[1], w = rect[2], h = rect[3], rx = rect[4],
         ry = rect[5];
  BOOL axes = t.b == 0 && t.c == 0 && t.a != 0 && t.d != 0;
  BOOL rounded = rx > 0 && ry > 0 && isfinite(rx) && isfinite(ry);
  // A mask holds every clip before it; a rounded clip in a rounded clip is
  // not one rounded rectangle. Those clip with the path.
  if (axes && !s.state.clip && !(rounded && s.state.rclipped) && w >= 0 &&
      h >= 0 && isfinite(x) && isfinite(y) && isfinite(w) && isfinite(h)) {
    BOOL clipped = s.state.rclipped || s.state.sclipped;
    CGPoint a = CGPointApplyAffineTransform(CGPointMake(x, y), t);
    CGPoint b = CGPointApplyAffineTransform(CGPointMake(x + w, y + h), t);
    double left = MIN(a.x, b.x), top = MIN(a.y, b.y), right = MAX(a.x, b.x),
           bottom = MAX(a.y, b.y);
    if (rounded) {
      double radiusX = MIN(rx * fabs(t.a), (right - left) / 2);
      double radiusY = MIN(ry * fabs(t.d), (bottom - top) / 2);
      s.state.rclip = (vector_float4){left, top, right, bottom};
      s.state.rradius = (vector_float4){radiusX, radiusY, 0, 0};
      s.state.rclipped = 1;
    } else if (s.state.sclipped) {
      vector_float4 old = s.state.sclip;
      left = MAX(left, old.x);
      top = MAX(top, old.y);
      right = MAX(left, MIN(right, old.z));
      bottom = MAX(top, MIN(bottom, old.w));
      s.state.sclip = (vector_float4){left, top, right, bottom};
    } else {
      s.state.sclip = (vector_float4){left, top, right, bottom};
      s.state.sclipped = 1;
    }
    CGRect box = CGRectIntersection(
        CGRectIntegral(CGRectMake(left, top, right - left, bottom - top)),
        CGRectMake(0, 0, s.storage.canvas.width, s.storage.canvas.height));
    // The pixels any clip in effect leaves: the work of a draw is cropped to them.
    if (clipped)
      box = CGRectIntersection(box, s.state.clipRect);
    s.state.clipRect = CGRectIsNull(box) ? CGRectZero : box;
    applied = 1;
  }
  UNLOCK
  return applied;
}
void qm_fill(void *surface, CGPathRef path, int evenodd, double red,
             double green, double blue, double alpha) {
  LOCK if (!r || !surface) return;
  QMetalSurface *s = (__bridge QMetalSurface *)surface;
  CGRect box;
  NSData *edges = path_edges(s, path, &box);
  if (!edges.length)
    return;
  QParams p = params(s);
  p.edges = (uint32_t)(edges.length / 16);
  p.evenodd = evenodd;
  p.color = (vector_float4){red, green, blue, alpha};
  if (s.state.shadowColor.w > 0) {
    QParams shadow = p;
    shadow.mode = 6;
    shadow.shadow = s.state.shadow;
    shadow.color = s.state.shadowColor;
    CGRect b = CGRectOffset(box, shadow.shadow.x, shadow.shadow.y);
    b = CGRectInset(b, -shadow.shadow.z * 2 - 1, -shadow.shadow.z * 2 - 1);
    bounds(&shadow, crop(s, b));
    dispatch(s, s.texture, shadow, edges, nil, nil);
  }
  bounds(&p, crop(s, box));
  dispatch(s, s.texture, p, edges, nil, nil);
  UNLOCK
}
void qm_rectangle(void *surface, const double *rect, double red, double green,
                  double blue, double alpha) {
  LOCK if (!r || !surface || !rect || !(rect[2] > 0) || !(rect[3] > 0)) return;
  double x = rect[0], y = rect[1], w = rect[2], h = rect[3], rx = rect[4],
         ry = rect[5];
  if (!(rx > 0) || !(ry > 0) || !isfinite(rx) || !isfinite(ry))
    rx = ry = 0;
  QMetalSurface *s = (__bridge QMetalSurface *)surface;
  double scale =
      MIN(1, MIN(rx > 0 ? w / (2 * rx) : 1, ry > 0 ? h / (2 * ry) : 1));
  rx *= scale;
  ry *= scale;
  CGAffineTransform transform = s.state.transform;
  if (s.state.shadowColor.w <= 0 && transform.b == 0 && transform.c == 0 &&
      !s.state.clip && (rx == 0 || !s.state.aliased)) {
    // Axis-aligned solid boxes, rounded or not, use exact compact coverage:
    // the shaders test the coverage samples against the box and its
    // elliptical corners, where a path would test them against the edges of
    // its flattened curves. Preparing those edges and their coverage image
    // cannot change the samples' outcome beyond the flattening tolerance, and
    // costs a path, its edges and a coverage cache entry per position.
    CGPoint a = CGPointApplyAffineTransform(CGPointMake(x, y), transform);
    CGPoint b =
        CGPointApplyAffineTransform(CGPointMake(x + w, y + h), transform);
    CGRect box = CGRectMake(MIN(a.x, b.x), MIN(a.y, b.y), fabs(b.x - a.x),
                            fabs(b.y - a.y));
    QParams p = params(s);
    p.reserved = rx > 0 ? 8 | 16 : 8;
    p.gradient = (vector_float4){box.origin.x, box.origin.y, CGRectGetMaxX(box),
                                 CGRectGetMaxY(box)};
    p.src = (vector_float4){rx * fabs(transform.a), ry * fabs(transform.d), 0,
                            0};
    p.color = (vector_float4){red, green, blue, alpha};
    bounds(&p, crop(s, box));
    dispatch(s, s.texture, p, nil, nil, nil);
    return;
  }
  double keyBytes[] = {x, y, w, h, rx, ry};
  QSmallKey key = small_key(keyBytes, sizeof(keyBytes));
  CGPathRef path = (CGPathRef)small_get(r.rectanglePaths, &key);
  if (!path) {
    CGMutablePathRef made = CGPathCreateMutable();
    double right = x + w, bottom = y + h, k = 0.5522847498307936;
    CGPathMoveToPoint(made, NULL, x + rx, y);
    CGPathAddLineToPoint(made, NULL, right - rx, y);
    CGPathAddCurveToPoint(made, NULL, right - rx + k * rx, y, right,
                          y + ry - k * ry, right, y + ry);
    CGPathAddLineToPoint(made, NULL, right, bottom - ry);
    CGPathAddCurveToPoint(made, NULL, right, bottom - ry + k * ry,
                          right - rx + k * rx, bottom, right - rx, bottom);
    CGPathAddLineToPoint(made, NULL, x + rx, bottom);
    CGPathAddCurveToPoint(made, NULL, x + rx - k * rx, bottom, x,
                          bottom - ry + k * ry, x, bottom - ry);
    CGPathAddLineToPoint(made, NULL, x, y + ry);
    CGPathAddCurveToPoint(made, NULL, x, y + ry - k * ry, x + rx - k * rx, y,
                          x + rx, y);
    CGPathCloseSubpath(made);
    if (CFDictionaryGetCount(r.rectanglePaths) >= 2048)
      CFDictionaryRemoveAllValues(r.rectanglePaths);
    // Keep a local owner if the optional key allocation fails.
    small_set(r.rectanglePaths, &key, made);
    qm_fill(surface, made, 0, red, green, blue, alpha);
    CGPathRelease(made);
    return;
  }
  qm_fill(surface, path, 0, red, green, blue, alpha);
  UNLOCK
}
void qm_gradient(void *surface, const double *colors, const double *locations,
                 size_t n, double x1, double y1, double x2, double y2,
                 double radius, int radial) {
  LOCK if (!r || !surface) return;
  QMetalSurface *s = (__bridge QMetalSurface *)surface;
  QParams p = params(s);
  p.mode = radial == 2 ? 7 : (radial ? 3 : 2);
  p.reserved = (uint32_t)s.state.gradientOptions.x;
  p.shadow = (vector_float4){s.state.gradientOptions.y,
                             s.state.gradientOptions.z, 0, 0};
  p.gradient = (vector_float4){x1, y1, x2, y2};
  p.translation.z = radius;
  p.stops = n;
  bounds(&p, crop(s, CGRectMake(0, 0, s.storage.canvas.width,
                                s.storage.canvas.height)));
  NSMutableData *stops = [NSMutableData dataWithLength:n * sizeof(QStop)];
  QStop *out = stops.mutableBytes;
  for (size_t i = 0; i < n; i++)
    out[i] = (QStop){{colors[i * 4], colors[i * 4 + 1], colors[i * 4 + 2],
                      colors[i * 4 + 3]},
                     {locations[i], 0, 0, 0}};
  dispatch(s, s.texture, p, nil, stops, nil);
  UNLOCK
}
static void draw_texture(QMetalSurface *s, id<MTLTexture> image, double sx,
                         double sy, double sw, double sh, double dx, double dy,
                         double dw, double dh, double alpha, int blend) {
  if (dw <= 0 || dh <= 0)
    return;
  QParams p = params(s);
  p.mode = 4;
  p.blend = blend;
  p.src = (vector_float4){sx, sy, sw, sh};
  p.dst = (vector_float4){dx, dy, dw, dh};
  p.translation.w = alpha;
  CGRect box;
  NSData *edges = rect_edges(s, CGRectMake(dx, dy, dw, dh), &box);
  p.edges = edges.length / 16;
  if (p.edges && p.inverse.y == 0 && p.inverse.z == 0 && p.inverse.x != 0 &&
      p.inverse.w != 0 && sw > 0 && sh > 0 && s.state.interpolation != 1
#ifdef QUARTZ_MEDIA_TEST_FIXTURES
      && !atomic_load(&fixtureDisableRectImages)
#endif
  ) {
    p.reserved |= 8;
    // Carry the original transformed endpoints, rather than reconstructing
    // them from a rounded inverse matrix at quarter-pixel boundaries.
    p.gradient = (vector_float4){box.origin.x, box.origin.y, CGRectGetMaxX(box),
                                 CGRectGetMaxY(box)};
  }
  bounds(&p, crop(s, box));
  dispatch(s, s.texture, p, edges, nil, image);
}
void qm_begin_layer(void *surface, double opacity, int blend) {
  LOCK if (!r || !surface) return;
  QMetalSurface *s = (__bridge QMetalSurface *)surface;
  qm_save(surface);
  QLayer *layer = [QLayer new];
  layer.parent = s.storage;
  layer.opacity = opacity;
  layer.blend = blend;
  [s.layers addObject:layer];
  // An empty group is metadata, not a temporary Metal texture. Avoid the
  // allocate/pool/recycle cycle for a placeholder that a hint replaces.
  QTexture *empty = [QTexture new];
  empty.canvas = layer.parent.canvas;
  empty.cropped = YES;
  empty.dirty = CGRectNull;
  empty.generation = qm_generation();
  s.storage = empty;
  // The parent clip applies when the isolated group is composed. Inheriting
  // it into the child as well squares fractional edge coverage even for an
  // identity opacity group, and unnecessarily truncates filter inputs.
  s.state.clip = nil;
  s.state.clipRect = CGRectZero;
  s.state.rclipped = 0;
  s.state.sclipped = 0;
  s.storage.dirty = CGRectNull;
  atomic_fetch_add(&counters[1], 1);
  UNLOCK
}
void qm_begin_layer_bounds(void *surface, double opacity, int blend,
                           const double *rect) {
  LOCK if (!r || !surface || !rect) return;
  qm_begin_layer(surface, opacity, blend);
  QMetalSurface *s = (__bridge QMetalSurface *)surface;
  CGRect hint = CGRectApplyAffineTransform(
      CGRectMake(rect[0], rect[1], rect[2], rect[3]), s.state.transform);
  if (s.layers.lastObject.parent.cropped || !atlas_layer(s, hint))
    grow_layer(s, hint, NO);
  UNLOCK
}
void qm_end_layer(void *surface) {
  LOCK if (!r || !surface) return;
  QMetalSurface *s = (__bridge QMetalSurface *)surface;
  QLayer *layer = s.layers.lastObject;
  if (!layer)
    return;
  if (layer.blend != 0)
    initialize_surface(s);
  QTexture *__attribute__((objc_precise_lifetime)) imageOwner = s.storage;
  id<MTLTexture> image = imageOwner.handle;
  s.storage = layer.parent;
  [s.layers removeLastObject];
  qm_restore(surface);
  CGAffineTransform transform = s.state.transform;
  s.state.transform = CGAffineTransformIdentity;
  // Only source-over can omit transparent pixels outside the dirty region.
  // Other Porter-Duff modes can change the destination there.
  CGRect area = layer.blend == 0 ? imageOwner.dirty
                                 : CGRectMake(0, 0, image.width, image.height);
  if (s.storage.external && layer.blend == 0 && !CGRectIsNull(area) &&
      !CGRectIsEmpty(area)) {
    NSUInteger bytes =
        imageOwner.region.size.width * imageOwner.region.size.height * 4;
    if (r.deferredSourceBytes + bytes > 32 * 1024 * 1024)
      flush_deferred();
    [r.deferredSources addObject:imageOwner];
    r.deferredSourceBytes += bytes;
  }
  if (!CGRectIsNull(area) && !CGRectIsEmpty(area))
    draw_texture(
        s, image,
        area.origin.x - imageOwner.origin.x + imageOwner.region.origin.x,
        area.origin.y - imageOwner.origin.y + imageOwner.region.origin.y,
        area.size.width, area.size.height, area.origin.x, area.origin.y,
        area.size.width, area.size.height, layer.opacity, layer.blend);
  s.state.transform = transform;
  (void)imageOwner; // Keep the owner alive through encoding.
  UNLOCK
}
static void copy_texture(id<MTLTexture> dst, id<MTLTexture> src) {
  if (!dst || !src)
    return;
  id<MTLCommandBuffer> cb = batch_buffer();
  id<MTLBlitCommandEncoder> e = [cb blitCommandEncoder];
  [e copyFromTexture:src
            sourceSlice:0
            sourceLevel:0
           sourceOrigin:MTLOriginMake(0, 0, 0)
             sourceSize:MTLSizeMake(src.width, src.height, 1)
              toTexture:dst
       destinationSlice:0
       destinationLevel:0
      destinationOrigin:MTLOriginMake(0, 0, 0)];
  [e endEncoding];
  finish_operation();
}
// Cache a GPU-generated mip chain until the source is next written.
static id<MTLTexture> sampled_texture(QMetalSurface *source,
                                      int interpolation) {
  initialize_surface(source);
  readable_surface(source);
  if (interpolation < 3)
    return source.texture;
  if (source.storage.mipmap)
    return source.storage.mipmap;
  MTLTextureDescriptor *d = [MTLTextureDescriptor
      texture2DDescriptorWithPixelFormat:renderer().canvasFormat
                                   width:source.texture.width
                                  height:source.texture.height
                               mipmapped:YES];
  d.storageMode = MTLStorageModePrivate;
  d.usage = MTLTextureUsageShaderRead;
  id<MTLTexture> mip = [renderer().device newTextureWithDescriptor:d];
  if (!mip)
    return source.texture;
  copy_texture(mip, source.texture);
  id<MTLBlitCommandEncoder> e = [batch_buffer() blitCommandEncoder];
  [e generateMipmapsForTexture:mip];
  [e endEncoding];
  finish_operation();
  source.storage.mipmap = mip;
  return mip;
}
void qm_copy_surface(void *surface, void *source) {
  LOCK if (!r || !surface || !source) return;
  QMetalSurface *s = (__bridge QMetalSurface *)surface;
  initialize_surface(s);
  s.storage.mipmap = nil;
  s.storage.dirty = CGRectMake(0, 0, s.texture.width, s.texture.height);
  initialize_surface((__bridge QMetalSurface *)source);
  copy_texture(((__bridge QMetalSurface *)surface).texture,
               ((__bridge QMetalSurface *)source).texture);
  UNLOCK
}
void qm_draw_surface(void *surface, void *source, double sx, double sy,
                     double sw, double sh, double dx, double dy, double dw,
                     double dh, double alpha) {
  LOCK if (!r || !surface || !source) return;
  QMetalSurface *s = (__bridge QMetalSurface *)surface;
  id<MTLTexture> image =
      sampled_texture((__bridge QMetalSurface *)source, s.state.interpolation);
  if (image == s.texture) {
    id<MTLTexture> copy = texture(image.width, image.height);
    copy_texture(copy, image);
    image = copy;
  }
  draw_texture(s, image, sx, sy, sw, sh, dx, dy, dw, dh, alpha,
               s.state.blendMode);
  UNLOCK
}
static void transfer(QMetalSurface *s, uint8_t *bytes, bool upload) {
  if (!upload)
    initialize_surface(s);
  if (upload)
    s.storage.dirty = CGRectMake(0, 0, s.texture.width, s.texture.height);
  if (upload)
    s.storage.mipmap = nil;
  size_t w = s.texture.width, h = s.texture.height,
         pitch = (w * 4 + 255) & ~255;
  id<MTLBuffer> buffer =
      [renderer().device newBufferWithLength:pitch * h
                                     options:MTLResourceStorageModeShared];
  uint8_t *data = buffer.contents;
  if (!s.texture || !bytes || !buffer || !data)
    return;
  BOOL bgra = s.texture.pixelFormat == MTLPixelFormatBGRA8Unorm;
  if (upload)
    for (size_t y = 0; y < h; y++) {
      if (bgra) {
        memcpy(data + y * pitch, bytes + y * w * 4, w * 4);
        continue;
      }
      for (size_t x = 0; x < w; x++) {
        const uint8_t *in = bytes + (y * w + x) * 4;
        uint8_t *out = data + y * pitch + x * 4;
        out[0] = in[2];
        out[1] = in[1];
        out[2] = in[0];
        out[3] = in[3];
      }
    }
  id<MTLCommandBuffer> cb = batch_buffer();
  id<MTLBlitCommandEncoder> e = [cb blitCommandEncoder];
  if (upload)
    [e copyFromBuffer:buffer
               sourceOffset:0
          sourceBytesPerRow:pitch
        sourceBytesPerImage:pitch * h
                 sourceSize:MTLSizeMake(w, h, 1)
                  toTexture:s.texture
           destinationSlice:0
           destinationLevel:0
          destinationOrigin:MTLOriginMake(0, 0, 0)];
  else
    [e copyFromTexture:s.texture
                     sourceSlice:0
                     sourceLevel:0
                    sourceOrigin:MTLOriginMake(0, 0, 0)
                      sourceSize:MTLSizeMake(w, h, 1)
                        toBuffer:buffer
               destinationOffset:0
          destinationBytesPerRow:pitch
        destinationBytesPerImage:pitch * h];
  [e endEncoding];
  finish_operation();
  if (!upload) {
    flush_batch();
    [cb waitUntilCompleted];
    if (cb.status != MTLCommandBufferStatusCompleted)
      return;
    for (size_t y = 0; y < h; y++) {
      if (bgra) {
        memcpy(bytes + y * w * 4, data + y * pitch, w * 4);
        continue;
      }
      for (size_t x = 0; x < w; x++) {
        const uint8_t *in = data + y * pitch + x * 4;
        uint8_t *out = bytes + (y * w + x) * 4;
        out[0] = in[2];
        out[1] = in[1];
        out[2] = in[0];
        out[3] = in[3];
      }
    }
  }
  atomic_fetch_add(&counters[upload ? 5 : 4], 1);
}
void qm_read_pixels(void *surface, uint8_t *out, size_t length) {
  LOCK if (!r || !surface) return;
  QMetalSurface *s = (__bridge QMetalSurface *)surface;
  initialize_surface(s);
  if (!out || length != s.texture.width * s.texture.height * 4)
    return;
  transfer(s, out, false);
  UNLOCK
}
void qm_write_pixels(void *surface, const uint8_t *bytes, size_t length) {
  LOCK if (!r || !surface) return;
  QMetalSurface *s = (__bridge QMetalSurface *)surface;
  initialize_surface(s);
  if (!bytes || length != s.texture.width * s.texture.height * 4)
    return;
  transfer(s, (uint8_t *)bytes, true);
  UNLOCK
}
void *qm_snapshot(void *surface) {
  LOCK if (!r || !surface) return NULL;
  QMetalSurface *s = (__bridge QMetalSurface *)surface;
  initialize_surface(s);
  size_t w = s.texture.width, h = s.texture.height;
  NSMutableData *data = [NSMutableData dataWithLength:w * h * 4];
  transfer(s, data.mutableBytes, false);
  CGDataProviderRef provider =
      CGDataProviderCreateWithCFData((__bridge CFDataRef)data);
  CGColorSpaceRef cs = CGColorSpaceCreateWithName(kCGColorSpaceSRGB);
  CGImageRef image = CGImageCreate(
      w, h, 8, 32, w * 4, cs,
      kCGImageAlphaPremultipliedFirst | kCGBitmapByteOrder32Little, provider,
      NULL, false, kCGRenderingIntentDefault);
  CGColorSpaceRelease(cs);
  CGDataProviderRelease(provider);
  return (void *)image;
  UNLOCK
  return NULL;
}
void qm_draw_image(void *surface, CGImageRef image, double sx, double sy,
                   double sw, double sh, double dx, double dy, double dw,
                   double dh, double alpha) {
  LOCK if (!r || !surface || !image) return;
  size_t w = CGImageGetWidth(image), h = CGImageGetHeight(image);
  NSMutableData *data = [NSMutableData dataWithLength:w * h * 4];
  CGColorSpaceRef cs = CGColorSpaceCreateWithName(kCGColorSpaceSRGB);
  CGContextRef c = CGBitmapContextCreate(data.mutableBytes, w, h, 8, w * 4, cs,
                                         kCGImageAlphaPremultipliedFirst |
                                             kCGBitmapByteOrder32Little);
  if (!c) {
    CGColorSpaceRelease(cs);
    return;
  }
  CGContextDrawImage(c, CGRectMake(0, 0, w, h), image);
  CGContextRelease(c);
  CGColorSpaceRelease(cs);
  void *tmp = qm_surface(w, h);
  if (!tmp)
    return;
  qm_write_pixels(tmp, data.bytes, data.length);
  qm_draw_surface(surface, tmp, sx, sy, sw, sh, dx, dy, dw, dh, alpha);
  CFRelease(tmp);
  UNLOCK
}
// Core Text's public rasterizer supplies native hinted and color glyph inputs.
// The canvas, clipping, tint and composition remain GPU operations. Cached
// masks are bounded by total pixel bytes and are independent of the brush
// color.
static BOOL alpha_glyph(QGlyphImage *image, const uint8_t *pixels, NSUInteger w,
                        NSUInteger h, NSMutableDictionary *images) {
  if (!glyph_atlas_enabled() || !pixels || w > 1024 || h > 1024)
    return NO;
  // Preserve color/subpixel rasterization verbatim. Only exact white masks
  // can be reconstructed losslessly from one channel.
  for (NSUInteger i = 0; i < w * h; i++) {
    const uint8_t *p = pixels + i * 4;
    if (p[0] != p[3] || p[1] != p[3] || p[2] != p[3])
      return NO;
  }
  QMetalRenderer *r = renderer();
  if (r.monoX + w > 1024) {
    r.monoX = 0;
    r.monoY += r.monoRow;
    r.monoRow = 0;
  }
  if (!r.monoAtlas || r.monoY + h > 1024) {
    if (r.atlasBytes + 1024 * 1024 > glyph_atlas_limit()) {
      flush_draws();
      [images removeAllObjects];
      CFDictionaryRemoveAllValues(r.glyphImages);
      CFDictionaryRemoveAllValues(r.preparedRuns);
      r.glyphImageBytes = r.preparedBytes = r.atlasBytes = 0;
      r.glyphAtlas = r.monoAtlas = nil;
      r.atlasX = r.atlasY = r.atlasRow = 0;
      r.capture = nil;
    }
    MTLTextureDescriptor *desc = [MTLTextureDescriptor
        texture2DDescriptorWithPixelFormat:MTLPixelFormatR8Unorm
                                     width:1024
                                    height:1024
                                 mipmapped:NO];
    desc.storageMode = MTLStorageModePrivate;
    desc.usage = MTLTextureUsageShaderRead;
    r.monoAtlas = [r.device newTextureWithDescriptor:desc];
    if (!r.monoAtlas)
      return NO;
    r.monoX = r.monoY = r.monoRow = 0;
    r.atlasBytes += 1024 * 1024;
  }
  NSUInteger pitch = (w + 255) & ~255, bytes = pitch * h;
  id<MTLBuffer> upload =
      [r.device newBufferWithLength:bytes options:MTLResourceStorageModeShared];
  if (!upload)
    return NO;
  uint8_t *out = upload.contents;
  for (NSUInteger y = 0; y < h; y++)
    for (NSUInteger x = 0; x < w; x++)
      out[y * pitch + x] = pixels[(y * w + x) * 4 + 3];
  if (r.pendingBytes && r.pendingBytes + bytes > 64 * 1024 * 1024)
    flush_batch();
  r.pendingBytes += bytes;
  id<MTLBlitCommandEncoder> copy = [batch_buffer() blitCommandEncoder];
  [copy copyFromBuffer:upload
             sourceOffset:0
        sourceBytesPerRow:pitch
      sourceBytesPerImage:bytes
               sourceSize:MTLSizeMake(w, h, 1)
                toTexture:r.monoAtlas
         destinationSlice:0
         destinationLevel:0
        destinationOrigin:MTLOriginMake(r.monoX, r.monoY, 0)];
  [copy endEncoding];
  image.monochrome = YES;
  image.atlas = r.monoAtlas;
  image.atlasRect = CGRectMake(r.monoX, r.monoY, w, h);
  r.monoX += w;
  r.monoRow = MAX(r.monoRow, h);
  finish_operation();
  atomic_fetch_add(&counters[5], 1);
  return YES;
}
static QGlyphImage *glyph_image(QMetalSurface *target, CTFontRef font,
                                CGGlyph glyph, NSMutableDictionary *images) {
  CTFontRef f = font;
  QMetalRenderer *r = renderer();
  NSNumber *key = @(glyph | ((target.state.hinting > 1) << 16) |
                    (target.state.aliased << 17));
  QGlyphImage *image = images[key];
  if (!image) {
    CGRect b = CTFontGetBoundingRectsForGlyphs(f, kCTFontOrientationHorizontal,
                                               &glyph, NULL, 1);
    b = CGRectIntegral(CGRectInset(b, -2, -2));
    if (b.size.width > 0 && b.size.height > 0 && b.size.width <= 4096 &&
        b.size.height <= 4096) {
      NSUInteger w = b.size.width, h = b.size.height;
      CGColorSpaceRef cs = CGColorSpaceCreateWithName(kCGColorSpaceSRGB);
      CGContextRef c = CGBitmapContextCreate(NULL, w, h, 8, w * 4, cs,
                                             kCGImageAlphaPremultipliedFirst |
                                                 kCGBitmapByteOrder32Little);
      CGColorSpaceRelease(cs);
      if (c) {
        // This standalone bitmap uses Core Text's native coordinate space.
        // Applying the drawing surface's Y flip here inverts glyph images.
        CGContextTranslateCTM(c, -b.origin.x, -b.origin.y);
        CGContextSetRGBFillColor(c, 1, 1, 1, 1);
        CGContextSetShouldAntialias(c, !target.state.aliased);
        CGContextSetShouldSmoothFonts(c, target.state.hinting > 1);
        CGContextSetShouldSubpixelPositionFonts(c, true);
        CGContextSetShouldSubpixelQuantizeFonts(c, false);
        CGPoint origin = CGPointZero;
        CTFontDrawGlyphs(f, &glyph, &origin, 1, c);
        image = [QGlyphImage new];
        image.bounds = b;
        BOOL packed =
            alpha_glyph(image, CGBitmapContextGetData(c), w, h, images);
        if (!packed) {
          void *surface = qm_surface(w, h);
          image.surface = CFBridgingRelease(surface);
          qm_write_pixels((__bridge void *)image.surface,
                          CGBitmapContextGetData(c), w * h * 4);
          CGContextRelease(c);
          if (glyph_atlas_enabled() && w <= 1024 && h <= 1024) {
            if (r.atlasX + w > 1024) {
              r.atlasX = 0;
              r.atlasY += r.atlasRow;
              r.atlasRow = 0;
            }
            if (!r.glyphAtlas || r.atlasY + h > 1024) {
              if (r.atlasBytes + 4 * 1024 * 1024 > glyph_atlas_limit()) {
                flush_draws();
                [images removeAllObjects];
                CFDictionaryRemoveAllValues(r.glyphImages);
                CFDictionaryRemoveAllValues(r.preparedRuns);
                r.glyphImageBytes = r.preparedBytes = r.atlasBytes = 0;
                r.monoAtlas = nil;
                r.monoX = r.monoY = r.monoRow = 0;
                r.capture = nil;
              }
              r.atlasBytes += 4 * 1024 * 1024;
              r.glyphAtlas = texture(1024, 1024);
              r.atlasX = r.atlasY = r.atlasRow = 0;
            }
            if (r.glyphAtlas) {
              id<MTLBlitCommandEncoder> copy =
                  [batch_buffer() blitCommandEncoder];
              [copy copyFromTexture:image.surface.texture
                        sourceSlice:0
                        sourceLevel:0
                       sourceOrigin:MTLOriginMake(0, 0, 0)
                         sourceSize:MTLSizeMake(w, h, 1)
                          toTexture:r.glyphAtlas
                   destinationSlice:0
                   destinationLevel:0
                  destinationOrigin:MTLOriginMake(r.atlasX, r.atlasY, 0)];
              [copy endEncoding];
              image.atlas = r.glyphAtlas;
              image.atlasRect = CGRectMake(r.atlasX, r.atlasY, w, h);
              r.atlasX += w;
              r.atlasRow = MAX(r.atlasRow, h);
              finish_operation();
            }
          }
        }
        if (packed)
          CGContextRelease(c);
        if (image.atlas)
          image.surface = nil;
        NSUInteger cost = w * h * (packed ? 1 : 4);
        if (r.glyphImageBytes + cost <= 16 * 1024 * 1024) {
          images[key] = image;
          r.glyphImageBytes += cost;
        }
      }
    }
  }
  return image;
}
static void draw_glyph_image(QMetalSurface *s, QGlyphImage *image, double scale,
                             double x, double y, bool colored,
                             vector_float4 tint) {
  if (!image.atlas && !image.surface.texture)
    return;
  // Non-atlased images can return to the texture pool after this draw. They
  // must not be referenced by a persistent prepared-run cache.
  if (!image.atlas)
    renderer().capture = nil;
  CGRect b = image.bounds;
  double dx = x + b.origin.x / scale, dy = y - CGRectGetMaxY(b) / scale,
         w = b.size.width / scale, h = b.size.height / scale;
  QParams p = params(s);
  p.mode = 4;
  p.color = colored ? (vector_float4){1, 1, 1, 1} : tint;
  p.reserved = (image.monochrome ? 2 : !colored) | (image.atlas ? 4 : 0);
  p.src = (vector_float4){image.atlasRect.origin.x, image.atlasRect.origin.y,
                          b.size.width, b.size.height};
  p.dst = (vector_float4){dx, dy, w, h};
  p.translation.w = colored ? tint.a : 1;
  CGRect box;
  NSData *edges = rect_edges(s, CGRectMake(dx, dy, w, h), &box);
  // Native raster inputs carry two transparent padding pixels. For an
  // axis-aligned, linearly/nearest sampled mask the outer quad cannot affect
  // coverage; keep every interior Core Text sample and omit redundant edges.
  if (s.state.transform.b == 0 && s.state.transform.c == 0 &&
      s.state.interpolation < 3)
    edges = nil;
  p.edges = edges.length / 16;
  bounds(&p, crop(s, box));
  dispatch(s, s.texture, p, edges, nil, image.atlas ?: image.surface.texture);
}
void qm_text_options(void *surface, int hinting) {
  LOCK if (!r || !surface) return;
  ((__bridge QMetalSurface *)surface).state.hinting = hinting;
  UNLOCK
}
// Foundation may hash only a prefix of NSData. Prepared glyph runs share
// that prefix, so include a whole-content discriminator before those bytes.
void qm_glyphs(void *surface, void *font, double size, const uint16_t *glyphs,
               const double *positions, size_t n, double red, double green,
               double blue, double alpha) {
  LOCK if (!r || !surface) return;
  QMetalSurface *s = (__bridge QMetalSurface *)surface;
  CGAffineTransform ctm = s.state.transform;
  double scale = MAX(hypot(ctm.a, ctm.b), hypot(ctm.c, ctm.d));
  if (!(scale > 0) || !isfinite(scale))
    return;
  // Cache prepared placement/coverage, never shaped text. The key owns the
  // complete glyph and position bytes, face, transform, paint and clip bounds;
  // pointer reuse and changing DPI/AA/viewport cannot reuse stale geometry.
  NSMutableData *runKey = nil;
  NSMutableDictionary *runs = nil;
  if (font && !s.storage.cropped && s.state.hinting >= 2 && n && n <= 4096) {
    if (r.preparedBytes >= 16 * 1024 * 1024 ||
        CFDictionaryGetCount(r.preparedRuns) >= 32) {
      CFDictionaryRemoveAllValues(r.preparedRuns);
      r.preparedBytes = 0;
    }
    QParams keyParams = params(s);
    keyParams.color = (vector_float4){red, green, blue, alpha};
    uint64_t discriminator = 0;
    runKey = [NSMutableData dataWithBytes:&discriminator
                                   length:sizeof(discriminator)];
    [runKey appendBytes:&keyParams length:sizeof(keyParams)];
    [runKey appendBytes:&ctm length:sizeof(ctm)];
    [runKey appendBytes:&size length:sizeof(size)];
    [runKey appendBytes:glyphs length:n * sizeof(*glyphs)];
    [runKey appendBytes:positions length:n * 2 * sizeof(*positions)];
    discriminator =
        content_hash((const uint8_t *)runKey.bytes + 8, runKey.length - 8);
    memcpy(runKey.mutableBytes, &discriminator, 8);
    runs = (__bridge NSMutableDictionary *)CFDictionaryGetValue(r.preparedRuns,
                                                                font);
    QPreparedDraw *ready = runs[runKey];
    if (ready) {
      const QParams *first = ready.params.bytes;
      NSUInteger kind =
          (!first->edges && !first->clipped && (first->flipped >> 2) < 3) ? 2
                                                                          : 1;
      if (r.drawTarget != s.texture ||
          (r.drawCount &&
           (r.drawKind != kind || r.drawClip != s.state.clip ||
            r.drawImage != ready.image ||
            r.drawCount + ready.params.length / sizeof(QParams) > 4096)))
        flush_draws();
      r.drawKind = kind;
      r.drawStorage = s.storage;
      r.drawTarget = s.texture;
      r.drawClip = s.state.clip;
      r.drawImage = ready.image;
      NSUInteger offset = r.drawEdges.length / sizeof(vector_float4);
      NSUInteger start = r.drawParams.length;
      if (kind < 2)
        [r.drawParams appendBytes:ready.params.bytes
                           length:ready.params.length];
      r.drawCount += ready.params.length / sizeof(QParams);
      [r.drawImages appendBytes:ready.images.bytes length:ready.images.length];
      if (kind < 2 && offset && ready.edges.length) {
        QParams *out =
            (QParams *)((uint8_t *)r.drawParams.mutableBytes + start);
        for (NSUInteger i = 0; i < ready.params.length / sizeof(QParams); i++)
          out[i].edgePad += offset;
      }
      s.storage.dirty = CGRectUnion(s.storage.dirty, ready.bounds);
      [r.drawEdges appendBytes:ready.edges.bytes length:ready.edges.length];
      s.storage.mipmap = nil;
      atomic_fetch_add(&counters[2], ready.params.length / sizeof(QParams));
      return;
    }
  }
  CTFontRef f =
      font ? CTFontCreateCopyWithAttributes(font, size, NULL, NULL) : NULL;
  if (!f)
    return;
  CFMutableDictionaryRef glyphsOfFont =
      (CFMutableDictionaryRef)CFDictionaryGetValue(r.glyphCache, f);
  if (!glyphsOfFont) {
    if (CFDictionaryGetCount(r.glyphCache) >= 32)
      CFDictionaryRemoveAllValues(r.glyphCache);
    glyphsOfFont =
        CFDictionaryCreateMutable(NULL, 0, &kCFTypeDictionaryKeyCallBacks,
                                  &kCFTypeDictionaryValueCallBacks);
    if (!glyphsOfFont) {
      CFRelease(f);
      return;
    }
    CFDictionarySetValue(r.glyphCache, f, glyphsOfFont);
    CFRelease(glyphsOfFont);
  }
  CTFontRef scaledFont =
      CTFontCreateCopyWithAttributes(f, size * scale, NULL, NULL);
  if (!scaledFont) {
    CFRelease(f);
    return;
  }
  if (CFDictionaryGetCount(r.glyphImages) >= 32 ||
      r.glyphImageBytes >= 16 * 1024 * 1024) {
    flush_draws();
    CFDictionaryRemoveAllValues(r.glyphImages);
    r.glyphImageBytes = 0;
    CFDictionaryRemoveAllValues(r.preparedRuns);
    r.preparedBytes = 0;
    runs = nil;
    r.glyphAtlas = nil;
    r.atlasX = r.atlasY = r.atlasRow = r.atlasBytes = 0;
    r.monoAtlas = nil;
    r.monoX = r.monoY = r.monoRow = 0;
  }
  NSMutableDictionary *images =
      (__bridge NSMutableDictionary *)CFDictionaryGetValue(r.glyphImages,
                                                           scaledFont);
  if (!images) {
    images = [NSMutableDictionary new];
    CFDictionarySetValue(r.glyphImages, scaledFont, (__bridge CFTypeRef)images);
  }
  if (runKey) {
    r.capture = [QPreparedDraw new];
    r.capture.params = [NSMutableData new];
    r.capture.edges = [NSMutableData new];
    r.capture.images = [NSMutableData new];
  }
  CGMutablePathRef combined = CGPathCreateMutable();
  for (size_t i = 0; i < n; i++) {
    CGAffineTransform transform = CGAffineTransformMake(
        1, 0, 0, -1, positions[i * 2], positions[i * 2 + 1]);
    NSNumber *key = @(glyphs[i]);
    CFTypeRef value =
        CFDictionaryGetValue(glyphsOfFont, (__bridge CFNumberRef)key);
    if (!value) {
      CGPathRef outline = CTFontCreatePathForGlyph(f, glyphs[i], NULL);
      if (CFDictionaryGetCount(glyphsOfFont) >= 1024)
        CFDictionaryRemoveAllValues(glyphsOfFont);
      value = outline ? (CFTypeRef)outline : kCFNull;
      CFDictionarySetValue(glyphsOfFont, (__bridge CFNumberRef)key, value);
      if (outline)
        CGPathRelease(outline);
    }
    if (value != kCFNull && s.state.hinting < 2) {
      CGPathAddPath(combined, &transform, (CGPathRef)value);
    } else {
      QGlyphImage *__attribute__((objc_precise_lifetime)) image =
          glyph_image(s, scaledFont, glyphs[i], images);
      draw_glyph_image(s, image, scale, positions[i * 2], positions[i * 2 + 1],
                       value == kCFNull,
                       (vector_float4){red, green, blue, alpha});
    }
  }
  QPreparedDraw *captured = r.capture;
  r.capture = nil;
  if (runKey && captured.params.length) {
    if (!runs) {
      runs = [NSMutableDictionary new];
      CFDictionarySetValue(r.preparedRuns, font, (__bridge CFTypeRef)runs);
    }
    runs[runKey] = captured;
    r.preparedBytes += runKey.length + captured.params.length +
                       captured.edges.length + captured.images.length;
  }
  qm_fill(surface, combined, 0, red, green, blue, alpha);
  CGPathRelease(combined);
  CFRelease(scaledFont);
  CFRelease(f);
  UNLOCK
}
void qm_glyphs_options(void *surface, void *font, double size,
                       const uint16_t *glyphs, const double *positions,
                       size_t n, double red, double green, double blue,
                       double alpha, int hinting, int aliased,
                       int interpolation) {
  LOCK if (!r || !surface) return;
  QState *state = ((__bridge QMetalSurface *)surface).state;
  int oldHint = state.hinting, oldAlias = state.aliased,
      oldInterpolation = state.interpolation;
  @try {
    state.hinting = hinting;
    state.aliased = aliased;
    state.interpolation = interpolation;
    qm_glyphs(surface, font, size, glyphs, positions, n, red, green, blue,
              alpha);
  } @finally {
    state.hinting = oldHint;
    state.aliased = oldAlias;
    state.interpolation = oldInterpolation;
  }
  UNLOCK
}
void qm_shadow(void *surface, double x, double y, double blur, double red,
               double green, double blue, double alpha) {
  LOCK if (!r || !surface) return;
  QMetalSurface *s = (__bridge QMetalSurface *)surface;
  CGAffineTransform t = s.state.transform;
  s.state.shadow =
      (vector_float4){t.a * x + t.c * y, t.b * x + t.d * y,
                      blur * MAX(hypot(t.a, t.b), hypot(t.c, t.d)), 0};
  s.state.shadowColor = (vector_float4){red, green, blue, alpha};
  UNLOCK
}
int qm_present(void *surface, void *rawTexture, void *platformQueue,
               int flipped) {
  LOCK if (!r || !surface) return 0;
  QMetalSurface *s = (__bridge QMetalSurface *)surface;
  initialize_surface(s);
  id<MTLTexture> dst = (__bridge id<MTLTexture>)rawTexture;
  id<MTLCommandQueue> platform = (__bridge id<MTLCommandQueue>)platformQueue;
  if (!dst || !platform || dst.device.registryID != r.device.registryID ||
      platform.device.registryID != r.device.registryID)
    return 0;
  id<MTLCommandBuffer> cb = nil;
  if (dst == s.texture) {
    cb = batch_buffer();
  } else if (!flipped && dst.pixelFormat == s.texture.pixelFormat &&
             dst.width == s.texture.width && dst.height == s.texture.height &&
             !dst.framebufferOnly) {
    copy_texture(dst, s.texture);
    cb = batch_buffer();
  } else {
    NSNumber *key = @(dst.pixelFormat);
    id<MTLRenderPipelineState> pipeline = r.presentPipelines[key];
    if (!pipeline) {
      MTLRenderPipelineDescriptor *d = [MTLRenderPipelineDescriptor new];
      d.vertexFunction = [r.library newFunctionWithName:@"present_vertex"];
      d.fragmentFunction = [r.library newFunctionWithName:@"present_fragment"];
      d.colorAttachments[0].pixelFormat = dst.pixelFormat;
      NSError *error = nil;
      pipeline = [r.device newRenderPipelineStateWithDescriptor:d error:&error];
      if (!pipeline)
        return 0;
      r.presentPipelines[key] = pipeline;
    }
    MTLRenderPassDescriptor *pass =
        [MTLRenderPassDescriptor renderPassDescriptor];
    pass.colorAttachments[0].texture = dst;
    pass.colorAttachments[0].loadAction = MTLLoadActionDontCare;
    pass.colorAttachments[0].storeAction = MTLStoreActionStore;
    cb = batch_buffer();
    id<MTLRenderCommandEncoder> e =
        [cb renderCommandEncoderWithDescriptor:pass];
    [e setRenderPipelineState:pipeline];
    [e setFragmentTexture:s.texture atIndex:0];
    uint32_t flip = flipped;
    [e setFragmentBytes:&flip length:4 atIndex:0];
    [e drawPrimitives:MTLPrimitiveTypeTriangle vertexStart:0 vertexCount:3];
    [e endEncoding];
  }
  if (platform == r.queue) {
    flush_batch();
    atomic_fetch_add(&counters[3], 1);
    return 1;
  }
  uint64_t value = ++r.eventValue;
  [cb encodeSignalEvent:r.event value:value];
  flush_batch();
  id<MTLCommandBuffer> wait =
      [(__bridge id<MTLCommandQueue>)platformQueue commandBuffer];
  [wait encodeWaitForEvent:r.event value:value];
  commit(wait);
  atomic_fetch_add(&counters[3], 1);
  return 1;
  UNLOCK
  return 0;
}
void qm_statistics(uint64_t *out, size_t count) {
  // Diagnostics include completion errors from everything previously submitted.
  LOCK if (!r) return;
  flush_batch();
  id<MTLCommandBuffer> done = [r.queue commandBuffer];
  commit(done);
  [done waitUntilCompleted];
  for (size_t i = 0; i < MIN(count, 9); i++)
    out[i] = atomic_load(&counters[i]);
  UNLOCK
}

int qm_convert_frame(void *surface, void *rawTexture, int rec709, int flipped,
                     int opaque, void *context,
                     void (*completed)(void *, int)) {
  LOCK if (!r || !surface) return 0;
  QMetalSurface *s = (__bridge QMetalSurface *)surface;
  id<MTLTexture> source = (__bridge id<MTLTexture>)rawTexture;
  if (!source || source.device.registryID != r.device.registryID ||
      source.textureType != MTLTextureType2D || source.sampleCount != 1 ||
      source.width != s.texture.width || source.height != s.texture.height ||
      !(source.usage & MTLTextureUsageShaderRead) ||
      (source.pixelFormat != MTLPixelFormatBGRA8Unorm &&
       source.pixelFormat != MTLPixelFormatRGBA8Unorm))
    return 0;
  writable_surface(s);
  id<MTLCommandBuffer> cb = batch_buffer();
  id<MTLComputeCommandEncoder> e = [cb computeCommandEncoder];
  if (!cb || !e)
    return 0;
  s.storage.dirty = CGRectMake(0, 0, s.texture.width, s.texture.height);
  [e setComputePipelineState:r.convertFrame];
  [e setTexture:s.texture atIndex:0];
  [e setTexture:source atIndex:1];
  vector_uint4 mode = {rec709, flipped, opaque, 0};
  [e setBytes:&mode length:sizeof(mode) atIndex:0];
  [e dispatchThreads:MTLSizeMake(source.width, source.height, 1)
      threadsPerThreadgroup:MTLSizeMake(8, 8, 1)];
  [e endEncoding];
  [cb addCompletedHandler:^(id<MTLCommandBuffer> done) {
    if (completed)
      completed(context, done.status == MTLCommandBufferStatusCompleted);
  }];
  flush_batch();
  return 1;
  UNLOCK
  return 0;
}

// Wait/copy/signal all use the renderer queue, not the platform presentation
// queue. The returned bitmap owns independent pixels before signaling reuse.
int qm_snapshot_external(void *surface, void *source, int flipped,
                         void *waitEvent, uint64_t waitValue, void *signalEvent,
                         uint64_t signalValue) {
  LOCK if (!r || !surface) return 0;
  id<MTLSharedEvent> wait = (__bridge id<MTLSharedEvent>)waitEvent;
  id<MTLSharedEvent> signal = (__bridge id<MTLSharedEvent>)signalEvent;
  if (!wait || !signal ||
      (wait.device && wait.device.registryID != r.device.registryID) ||
      (signal.device && signal.device.registryID != r.device.registryID))
    return 0;
  [batch_buffer() encodeWaitForEvent:wait value:waitValue];
  int result = qm_convert_frame(surface, source, 0, flipped, 0, NULL, NULL);
  // Always release the producer's slot after a valid wait/signal pair, even if
  // copying fails.
  [batch_buffer() encodeSignalEvent:signal value:signalValue];
  flush_batch();
  return result;
  UNLOCK
  return 0;
}
#ifdef QUARTZ_MEDIA_TEST_FIXTURES
void qm_fixture_layer_atlas(int enabled) {
  LOCK if (!r) return;
  flush_batch();
  atomic_store(&fixtureDisableLayerAtlas, !enabled);
  UNLOCK
}
uint64_t qm_fixture_atlas_region_clears(void) {
  LOCK if (!r) return 0;
  flush_batch();
  return fixtureAtlasRegionClears;
  UNLOCK
  return 0;
}
void qm_fixture_deferred_merge(int enabled) {
  LOCK if (!r) return;
  flush_batch();
  atomic_store(&fixtureDisableDeferredMerge, !enabled);
  UNLOCK
}
uint64_t qm_fixture_encoded_batches(void) {
  LOCK if (!r) return 0;
  flush_batch();
  return fixtureEncodedBatches;
  UNLOCK
  return 0;
}
void qm_fixture_key_collisions(int enabled) {
  LOCK if (!r) return;
  CFDictionaryRemoveAllValues(r.pathCache);
  clear_coverage_paths(r);
  CFDictionaryRemoveAllValues(r.rectanglePaths);
  r.pathEntries = r.pathBytes = 0;
  atomic_store(&fixtureKeyCollisions, enabled != 0);
  UNLOCK
}
void qm_fixture_glyph_atlas(int enabled, size_t budget) {
  LOCK if (!r) return;
  flush_batch();
  CFDictionaryRemoveAllValues(r.glyphImages);
  CFDictionaryRemoveAllValues(r.preparedRuns);
  r.glyphAtlas = r.monoAtlas = nil;
  r.atlasX = r.atlasY = r.atlasRow = r.monoX = r.monoY = r.monoRow = 0;
  r.glyphImageBytes = r.preparedBytes = r.atlasBytes = 0;
  r.capture = nil;
  atomic_store(&fixtureDisableGlyphAtlas, !enabled);
  atomic_store(&fixtureGlyphBudget, MAX(budget, 1024 * 1024));
  UNLOCK
}
void *qm_fixture_render_target(size_t w, size_t h) {
  LOCK if (!r) return NULL;
  MTLTextureDescriptor *desc =
      [MTLTextureDescriptor texture2DDescriptorWithPixelFormat:r.canvasFormat
                                                         width:w
                                                        height:h
                                                     mipmapped:NO];
  desc.storageMode = MTLStorageModePrivate;
  desc.usage = MTLTextureUsageRenderTarget | MTLTextureUsageShaderRead;
  id<MTLTexture> image = [r.device newTextureWithDescriptor:desc];
  return (__bridge_retained void *)image;
  UNLOCK
  return NULL;
}
// Explicit development fixture: exercise the very same ABI exception boundary.
int qm_fixture_exception(void) {
  LOCK if (!r) return 0;
  @throw [NSException exceptionWithName:@"QuartzFixture"
                                 reason:@"recoverable native failure"
                               userInfo:nil];
  UNLOCK
  return 0;
}
#import <CoreVideo/CoreVideo.h>
#import <IOSurface/IOSurfaceRef.h>
// Synthetic camera input only. GPU-generated immutable CoreVideo storage, with
// the same IOSurface import path as AVFoundation. No CPU pixel mapping/upload.
void *qm_fixture_buffer(uint64_t sequence) {
  LOCK if (!r) return NULL;
  CVPixelBufferRef buffer = NULL;
  NSDictionary *attributes = @{
    (__bridge id)kCVPixelBufferIOSurfacePropertiesKey : @{},
    (__bridge id)kCVPixelBufferMetalCompatibilityKey : @YES
  };
  if (CVPixelBufferCreate(NULL, 640, 480, kCVPixelFormatType_32BGRA,
                          (__bridge CFDictionaryRef)attributes,
                          &buffer) != kCVReturnSuccess)
    return NULL;
  MTLTextureDescriptor *d = [MTLTextureDescriptor
      texture2DDescriptorWithPixelFormat:MTLPixelFormatBGRA8Unorm
                                   width:640
                                  height:480
                               mipmapped:NO];
  d.storageMode = MTLStorageModeShared;
  d.usage = MTLTextureUsageRenderTarget | MTLTextureUsageShaderRead;
  id<MTLTexture> image =
      [r.device newTextureWithDescriptor:d
                               iosurface:CVPixelBufferGetIOSurface(buffer)
                                   plane:0];
  if (!image) {
    CVPixelBufferRelease(buffer);
    return NULL;
  }
  MTLRenderPassDescriptor *pass =
      [MTLRenderPassDescriptor renderPassDescriptor];
  pass.colorAttachments[0].texture = image;
  pass.colorAttachments[0].loadAction = MTLLoadActionClear;
  pass.colorAttachments[0].storeAction = MTLStoreActionStore;
  pass.colorAttachments[0].clearColor =
      MTLClearColorMake((sequence % 32) / 31., 0.25, 0.75, 1);
  id<MTLCommandBuffer> cb = batch_buffer();
  id<MTLRenderCommandEncoder> e = [cb renderCommandEncoderWithDescriptor:pass];
  [e endEncoding];
  flush_batch();
  [cb waitUntilCompleted];
  if (cb.status != MTLCommandBufferStatusCompleted) {
    CVPixelBufferRelease(buffer);
    return NULL;
  }
  CVBufferSetAttachment(buffer, kCVImageBufferColorPrimariesKey,
                        kCVImageBufferColorPrimaries_ITU_R_709_2,
                        kCVAttachmentMode_ShouldPropagate);
  CVBufferSetAttachment(buffer, kCVImageBufferTransferFunctionKey,
                        kCVImageBufferTransferFunction_sRGB,
                        kCVAttachmentMode_ShouldPropagate);
  return buffer;
  UNLOCK
  return NULL;
}
#endif

static void filter_compute(QMetalSurface *s,
                           id<MTLComputePipelineState> pipeline,
                           const void *data, size_t length, uint32_t kind,
                           id<MTLTexture> secondary) {
  QMetalRenderer *r = renderer();
  initialize_surface(s);
  id<MTLTexture> out = texture(s.texture.width, s.texture.height);
  if (!out || (length && !data))
    return;
  vector_float4 zero = {0};
  // The ABI data are scalars padded to float4 alignment; no shader reads past
  // the documented parameters for its operation.
  size_t size = MAX(16, (length + 15) & ~15);
  id<MTLBuffer> parameters =
      [r.device newBufferWithLength:size options:MTLResourceStorageModeShared];
  if (!parameters || !parameters.contents) {
    qm_record_fault(qm_generation());
    return;
  }
  memcpy(parameters.contents, data ?: &zero, length);
  id<MTLCommandBuffer> cb = batch_buffer();
  id<MTLComputeCommandEncoder> e = [cb computeCommandEncoder];
  if (!e) {
    qm_record_fault(qm_generation());
    return;
  }
  [e setComputePipelineState:pipeline];
  [e setTexture:out atIndex:0];
  [e setTexture:s.texture atIndex:1];
  [e setTexture:secondary ?: r.white atIndex:2];
  [e setBuffer:parameters offset:0 atIndex:0];
  [e setBytes:&kind length:4 atIndex:1];
  [e dispatchThreads:MTLSizeMake(out.width, out.height, 1)
      threadsPerThreadgroup:MTLSizeMake(8, 8, 1)];
  [e endEncoding];
  finish_operation();
  s.texture = out;
}
void *qm_layer_snapshot(void *surface) {
  LOCK if (!r || !surface) return NULL;
  QMetalSurface *s = (__bridge QMetalSurface *)surface;
  initialize_surface(s);
  readable_surface(s);
  if (s.storage.external)
    flush_deferred();
  QMetalSurface *copy = [QMetalSurface new];
  copy.storage = s.storage;
  return (__bridge_retained void *)copy;
  UNLOCK
  return NULL;
}
void qm_blur(void *surface, double sigma, double ax, double ay, double bx,
             double by) {
  LOCK if (!r || !surface) return;
  QMetalSurface *s = (__bridge QMetalSurface *)surface;
  if (!(sigma > 0) || !isfinite(sigma))
    return;
  initialize_surface(s);
#if !TARGET_OS_SIMULATOR
  double x = hypot(ax, ay), y = hypot(bx, by);
  if (MPSSupportsMTLDevice(r.device) && fabs(ay) < 1e-6 && fabs(bx) < 1e-6 &&
      fabs(x - y) < 1e-6 && x > 0) {
    id<MTLTexture> out = texture(s.texture.width, s.texture.height);
    if (!out)
      return;
    MPSImageGaussianBlur *blur =
        [[MPSImageGaussianBlur alloc] initWithDevice:r.device sigma:sigma * x];
    blur.edgeMode = MPSImageEdgeModeZero;
    id<MTLCommandBuffer> cb = batch_buffer();
    [blur encodeToCommandBuffer:cb
                  sourceTexture:s.texture
             destinationTexture:out];
    finish_operation();
    s.texture = out;
  } else
#endif
  {
    // General affine covariance: convolve on each transformed local axis.
    vector_float4 axis = {ax, ay, sigma, 0};
    filter_compute(s, r.gaussian, &axis, sizeof(axis), 0, nil);
    axis = (vector_float4){bx, by, sigma, 0};
    filter_compute(s, r.gaussian, &axis, sizeof(axis), 0, nil);
  }
  UNLOCK
}
void qm_filter(void *surface, int kind, const float *data, size_t length,
               void *secondary) {
  LOCK if (!r || !surface) return;
  QMetalSurface *s = (__bridge QMetalSurface *)surface;
  id<MTLTexture> other =
      secondary ? ((__bridge QMetalSurface *)secondary).texture : nil;
  filter_compute(s, r.layerFilter, data, length * sizeof(float), kind, other);
  UNLOCK
}

void qm_gradient_options(void *surface, int spread, double aspect,
                         double angle) {
  LOCK if (!r || !surface) return;
  QMetalSurface *s = (__bridge QMetalSurface *)surface;
  s.state.gradientOptions = (vector_float4){spread, aspect, angle, 0};
  UNLOCK
}

void qm_draw_tile(void *surface, void *source, double x, double y, double w,
                  double h, double alpha, int mode) {
  LOCK if (!r || !surface || !source) return;
  QMetalSurface *s = (__bridge QMetalSurface *)surface;
  QParams p = params(s);
  p.mode = 8;
  p.dst = (vector_float4){x, y, w, h};
  p.translation.w = alpha;
  p.reserved = mode;
  bounds(&p, crop(s, CGRectMake(0, 0, s.storage.canvas.width,
                                s.storage.canvas.height)));
  dispatch(
      s, s.texture, p, nil, nil,
      sampled_texture((__bridge QMetalSurface *)source, s.state.interpolation));
  UNLOCK
}

void qm_render_options(void *surface, int blend, int aliased,
                       int interpolation) {
  LOCK if (!r || !surface) return;
  QMetalSurface *s = (__bridge QMetalSurface *)surface;
  s.state.blendMode = blend;
  s.state.aliased = aliased;
  s.state.interpolation = interpolation;
  UNLOCK
}

void *qm_take_layer(void *surface) {
  LOCK if (!r || !surface) return NULL;
  QMetalSurface *s = (__bridge QMetalSurface *)surface;
  void *snapshot = qm_layer_snapshot(surface);
  s.texture = texture(s.texture.width, s.texture.height);
  clear_texture(s);
  return snapshot;
  UNLOCK
  return NULL;
}

void qm_acrylic(void *surface, const double *colors, void *noise, int replace) {
  LOCK if (!r || !surface) return;
  QMetalSurface *s = (__bridge QMetalSurface *)surface;
  QParams p = params(s);
  p.mode = 9;
  if (replace)
    p.blend = 17;
  p.color = (vector_float4){colors[0], colors[1], colors[2], colors[3]};
  p.gradient = (vector_float4){colors[4], colors[5], colors[6], colors[7]};
  bounds(&p, crop(s, CGRectMake(0, 0, s.storage.canvas.width,
                                s.storage.canvas.height)));
  dispatch(s, s.texture, p, nil, nil,
           ((__bridge QMetalSurface *)noise).texture);
  UNLOCK
}

void *qm_shader_compile(const char *source, const char *vertex,
                        const char *fragment, char *message, size_t length) {
  LOCK if (!r) return NULL;
  NSError *error = nil;
  id<MTLLibrary> library = [r.device newLibraryWithSource:@(source)
                                                  options:nil
                                                    error:&error];
  MTLRenderPipelineDescriptor *d = [MTLRenderPipelineDescriptor new];
  d.vertexFunction = [library newFunctionWithName:@(vertex)];
  d.fragmentFunction = [library newFunctionWithName:@(fragment)];
  d.colorAttachments[0].pixelFormat = r.canvasFormat;
  id<MTLRenderPipelineState> pipeline = nil;
  if (library && d.vertexFunction && d.fragmentFunction)
    pipeline = [r.device newRenderPipelineStateWithDescriptor:d error:&error];
  if (!pipeline && length)
    snprintf(message, length, "%s",
             error.localizedDescription.UTF8String
                 ?: "Missing shader entry point");
  return (__bridge_retained void *)pipeline;
  UNLOCK
  return NULL;
}
void qm_shader_apply(void *surface, void *rawPipeline, const uint8_t *uniforms,
                     size_t size, void *const *children, size_t count, double x,
                     double y, double w, double h) {
  LOCK if (!r || !surface) return;
  QMetalSurface *s = (__bridge QMetalSurface *)surface;
  initialize_surface(s);
  writable_surface(s);
  for (NSUInteger i = 0; i < count; i++) {
    if (!children || !children[i])
      continue;
    QMetalSurface *child = (__bridge QMetalSurface *)children[i];
    initialize_surface(child);
    readable_surface(child);
  }
  NSInteger left = MAX(0, floor(x)), top = MAX(0, floor(y));
  NSInteger right = MIN(s.texture.width, ceil(x + w)),
            bottom = MIN(s.texture.height, ceil(y + h));
  if (right <= left || bottom <= top)
    return;
  NSUInteger width = right - left, height = bottom - top;
  id<MTLTexture> input = texture(width, height),
                 output = texture(width, height);
  if (!input || !output || !rawPipeline || (size && !uniforms))
    return;
  vector_float4 zero = {0};
  id<MTLBuffer> block = [r.device
      newBufferWithBytes:size ? (const void *)uniforms : (const void *)&zero
                  length:size ?: sizeof(zero)
                 options:MTLResourceStorageModeShared];
  if (!block) {
    qm_record_fault(qm_generation());
    return;
  }
  id<MTLCommandBuffer> cb = batch_buffer();
  id<MTLBlitCommandEncoder> copy = [cb blitCommandEncoder];
  [copy copyFromTexture:s.texture
            sourceSlice:0
            sourceLevel:0
           sourceOrigin:MTLOriginMake(left, top, 0)
             sourceSize:MTLSizeMake(width, height, 1)
              toTexture:input
       destinationSlice:0
       destinationLevel:0
      destinationOrigin:MTLOriginMake(0, 0, 0)];
  [copy endEncoding];
  MTLRenderPassDescriptor *pass =
      [MTLRenderPassDescriptor renderPassDescriptor];
  pass.colorAttachments[0].texture = output;
  pass.colorAttachments[0].loadAction = MTLLoadActionDontCare;
  pass.colorAttachments[0].storeAction = MTLStoreActionStore;
  id<MTLRenderCommandEncoder> e = [cb renderCommandEncoderWithDescriptor:pass];
  [e setRenderPipelineState:(__bridge id<MTLRenderPipelineState>)rawPipeline];
  [e setFragmentBuffer:block offset:0 atIndex:0];
  vector_float4 area = {left - x, top - y, width, height};
  [e setFragmentBytes:&area length:sizeof(area) atIndex:1];
  [e setFragmentTexture:input atIndex:0];
  for (NSUInteger i = 0; i < count; i++) {
    QMetalSurface *child = (__bridge QMetalSurface *)children[i];
    [e setFragmentTexture:child ? child.texture : r.transparent atIndex:i + 1];
  }
  [e drawPrimitives:MTLPrimitiveTypeTriangle vertexStart:0 vertexCount:3];
  [e endEncoding];
  copy = [cb blitCommandEncoder];
  [copy copyFromTexture:output
            sourceSlice:0
            sourceLevel:0
           sourceOrigin:MTLOriginMake(0, 0, 0)
             sourceSize:MTLSizeMake(width, height, 1)
              toTexture:s.texture
       destinationSlice:0
       destinationLevel:0
      destinationOrigin:MTLOriginMake(left, top, 0)];
  [copy endEncoding];
  s.storage.dirty =
      CGRectUnion(s.storage.dirty, CGRectMake(left, top, width, height));
  s.storage.mipmap = nil;
  finish_operation();
  UNLOCK
}

// Result: 1 valid, -1 wrong device, -2 dimensions, -3 unsupported sampling.
int qm_validate_texture(void *raw, int width, int height, int bgra) {
  LOCK if (!r) return 0;
  id<MTLTexture> t = (__bridge id<MTLTexture>)raw;
  if (!t)
    return -3;
  if (t.device.registryID != r.device.registryID)
    return -1;
  if (width <= 0 || height <= 0 || t.width != (NSUInteger)width ||
      t.height != (NSUInteger)height)
    return -2;
  if (t.textureType != MTLTextureType2D || t.sampleCount != 1 ||
      !(t.usage & MTLTextureUsageShaderRead) ||
      t.pixelFormat !=
          (bgra ? MTLPixelFormatBGRA8Unorm : MTLPixelFormatRGBA8Unorm))
    return -3;
  return 1;
  UNLOCK
  return 0;
}

void qm_finish_context(uint64_t generation) {
  LOCK if (!r) return;
  if (generation != qm_generation())
    return;
  if (atomic_load(&rendererLost)) {
    end_render_encoder();
    [r.deferred removeAllObjects];
    [r.deferredSources removeAllObjects];
    r.deferredBytes = r.deferredSourceBytes = 0;
    r.encodingDeferred = NO;
    [r.drawParams setLength:0];
    [r.drawEdges setLength:0];
    [r.drawImages setLength:0];
    r.drawClear = NO;
    r.drawCount = 0;
    r.drawStorage = nil;
    r.drawTarget = r.drawClip = r.drawImage = r.glyphAtlas = nil;
    r.atlasX = r.atlasY = r.atlasRow = r.atlasBytes = 0;
    r.monoAtlas = nil;
    r.monoX = r.monoY = r.monoRow = 0;
    r.pending = nil;
    r.pendingOperations = r.pendingBytes = 0;
  } else
    flush_batch();
  [r.buffers removeAllObjects];
  r.bufferBytes = 0;
  UNLOCK
}

// Effect input may extend beyond the parent's output clip. The saved layer
// state reapplies the parent's exact mask when the filtered result is composed.
void qm_reset_effect_input_clip(void *surface) {
  LOCK if (!r || !surface) return;
  QMetalSurface *s = (__bridge QMetalSurface *)surface;
  s.state.clip = nil;
  s.state.clipRect = CGRectZero;
  s.state.rclipped = 0;
  s.state.sclipped = 0;
  UNLOCK
}
