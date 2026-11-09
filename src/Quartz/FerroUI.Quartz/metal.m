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
  vector_float4 color, gradient, inverse, translation, src, dst, clip, shadow;
  uint32_t stops, clipped, flipped, reserved;
  uint32_t edgeRows, edgeOriginY, edgeStride, edgePad;
} QParams;
typedef struct {
  vector_float4 color, location;
} QStop;
static _Atomic uint64_t counters[9];
#ifdef QUARTZ_MEDIA_TEST_FIXTURES
static _Atomic bool fixtureFailNextTexture;
void qm_fixture_fail_allocation(void) {
  atomic_store(&fixtureFailNextTexture, true);
}
#endif
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
@interface QState : NSObject
@property CGAffineTransform transform;
@property id<MTLTexture> clip;
@property CGRect clipRect;
@property vector_float4 shadow, shadowColor, gradientOptions;
@property int blendMode, aliased, interpolation, hinting;
@end
@implementation QState
@end
static void recycle_texture(id<MTLTexture> texture, uint64_t generation);
// Sharing this owner, rather than only the Metal handle, prevents snapshots and
// nested layers from returning aliased storage to the pool prematurely.
@interface QTexture : NSObject
@property id<MTLTexture> handle, mipmap;
@property uint64_t generation;
@end
@implementation QTexture
- (void)dealloc {
  recycle_texture(_handle, _generation);
}
@end
@interface QLayer : NSObject
@property QTexture *parent;
@property double opacity;
@property int blend;
@end
@implementation QLayer
@end
@interface QMetalSurface : NSObject
@property QTexture *storage;
@property id<MTLTexture> texture;
@property QState *state;
@property NSMutableArray<QState *> *states;
@property NSMutableArray<QLayer *> *layers;
@end
@implementation QMetalSurface
- (id<MTLTexture>)texture {
  return _storage.handle;
}
- (void)setTexture:(id<MTLTexture>)texture {
  QTexture *owner = [QTexture new];
  owner.handle = texture;
  owner.generation = qm_generation();
  self.storage = owner;
}
@end
@interface QGlyphImage : NSObject
@property QMetalSurface *surface;
@property CGRect bounds;
@end
@implementation QGlyphImage
@end
@interface QMetalRenderer : NSObject
@property id<MTLDevice> device;
@property id<MTLCommandQueue> queue;
@property id<MTLCommandBuffer> pending;
@property NSUInteger pendingOperations, pendingBytes, poolBytes;
@property NSMutableDictionary<NSString *, NSMutableArray<id<MTLTexture>> *>
    *texturePool;
@property id<MTLLibrary> library;
@property id<MTLComputePipelineState> paint;
@property BOOL separateDestination;
@property id<MTLComputePipelineState> convertFrame;
@property id<MTLComputePipelineState> layerFilter, gaussian;
@property id<MTLSharedEvent> event;
@property NSMutableDictionary<NSData *, NSData *> *edgeCache;
@property NSUInteger edgeCacheBytes;
@property CFMutableDictionaryRef glyphCache, glyphImages;
@property NSUInteger glyphImageBytes;
@property uint64_t eventValue;
@property id<MTLTexture> white, transparent;
@property NSRecursiveLock *lock;
@property NSMutableDictionary<NSNumber *, id<MTLRenderPipelineState>>
    *presentPipelines;
@end
@implementation QMetalRenderer
@end
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
      r.device = MTLCreateSystemDefaultDevice();
      if (!r.device) {
        r = nil;
        return;
      }
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
      r.glyphCache =
          CFDictionaryCreateMutable(NULL, 0, &kCFTypeDictionaryKeyCallBacks,
                                    &kCFTypeDictionaryValueCallBacks);
      r.glyphImages =
          CFDictionaryCreateMutable(NULL, 0, &kCFTypeDictionaryKeyCallBacks,
                                    &kCFTypeDictionaryValueCallBacks);
      r.event = [r.device newSharedEvent];
      if (!r.queue || !r.event || !r.glyphCache || !r.glyphImages) {
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
  [cb addCompletedHandler:^(id<MTLCommandBuffer> done) {
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
static id<MTLCommandBuffer> batch_buffer(void) {
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
static void flush_batch(void) {
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
      if (bucket.count < 8) {
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
  if (r.pendingBytes && r.pendingBytes + bytes > 64 * 1024 * 1024)
    flush_batch();
  r.pendingBytes += bytes;
  NSMutableArray *bucket = r.texturePool[texture_key(w, h)];
  id<MTLTexture> pooled = bucket.lastObject;
  if (pooled) {
    [bucket removeLastObject];
    r.poolBytes -= bytes;
    return pooled;
  }
  MTLTextureDescriptor *d = [MTLTextureDescriptor
      texture2DDescriptorWithPixelFormat:MTLPixelFormatRGBA8Unorm
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
  return n;
}
static QParams params(QMetalSurface *s) {
  QParams p = {0};
  // BitmapBlendingMode applies to draw_bitmap, not vector or text paint.
  // Image drawing supplies its mode explicitly in draw_texture.
  p.blend = 0;
  p.flipped = (s.state.aliased ? 1 : 0) | (s.state.interpolation == 1 ? 2 : 0) |
              (s.state.interpolation << 2);
  p.extent = (vector_uint2){s.texture.width, s.texture.height};
  CGAffineTransform inv = CGAffineTransformInvert(s.state.transform);
  p.inverse = (vector_float4){inv.a, inv.b, inv.c, inv.d};
  p.translation = (vector_float4){inv.tx, inv.ty, 0, 1};
  if (s.state.clip) {
    p.clipped = 1;
    CGRect c = s.state.clipRect;
    p.clip =
        (vector_float4){c.origin.x, c.origin.y, c.size.width, c.size.height};
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
  if (renderer().edgeCache.count >= 128 ||
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
static void dispatch(QMetalSurface *s, id<MTLTexture> target, QParams p,
                     NSData *edges, NSData *stops, id<MTLTexture> image) {
  if (!target || !s.texture || !p.extent.x || !p.extent.y)
    return;
  QMetalRenderer *r = renderer();
  edges = binned_edges(&p, edges);
  if (target == s.texture)
    s.storage.mipmap = nil;
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
    MTLOrigin origin = MTLOriginMake(p.origin.x, p.origin.y, 0);
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
  [enc setTexture:s.state.clip ?: r.white atIndex:1];
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
  rect =
      CGRectIntersection(CGRectIntegral(rect),
                         CGRectMake(0, 0, s.texture.width, s.texture.height));
  if (s.state.clip)
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
  return data;
}
static void clear_texture(QMetalSurface *s) {
  QParams p = params(s);
  p.clipped = 0;
  p.mode = 5;
  p.blend = 17;
  dispatch(s, s.texture, p, nil, nil, nil);
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
    if (!r.queue || !r.event)
      return 0;
    atomic_fetch_add(&rendererGeneration, 1);
    atomic_store(&rendererLost, false);
  }
  return qm_generation();
  UNLOCK
  return 0;
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
void qm_clear(void *surface, double red, double green, double blue,
              double alpha) {
  LOCK if (!r || !surface) return;
  QMetalSurface *s = (__bridge QMetalSurface *)surface;
  QParams p = params(s);
  p.mode = 5;
  p.blend = 17;
  p.color = (vector_float4){red, green, blue, alpha};
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
  UNLOCK
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
  bounds(&p, crop(s, CGRectMake(0, 0, s.texture.width, s.texture.height)));
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
  CGPathRef rect = CGPathCreateWithRect(CGRectMake(dx, dy, dw, dh), NULL);
  CGRect box;
  NSData *edges = path_edges(s, rect, &box);
  CGPathRelease(rect);
  p.edges = edges.length / 16;
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
  s.texture = texture(layer.parent.handle.width, layer.parent.handle.height);
  // The parent clip applies when the isolated group is composed. Inheriting
  // it into the child as well squares fractional edge coverage even for an
  // identity opacity group, and unnecessarily truncates filter inputs.
  s.state.clip = nil;
  s.state.clipRect = CGRectZero;
  clear_texture(s);
  UNLOCK
}
void qm_end_layer(void *surface) {
  LOCK if (!r || !surface) return;
  QMetalSurface *s = (__bridge QMetalSurface *)surface;
  QLayer *layer = s.layers.lastObject;
  if (!layer)
    return;
  QTexture *__attribute__((objc_precise_lifetime)) imageOwner = s.storage;
  id<MTLTexture> image = imageOwner.handle;
  s.storage = layer.parent;
  [s.layers removeLastObject];
  qm_restore(surface);
  CGAffineTransform transform = s.state.transform;
  s.state.transform = CGAffineTransformIdentity;
  draw_texture(s, image, 0, 0, image.width, image.height, 0, 0, image.width,
               image.height, layer.opacity, layer.blend);
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
  if (interpolation < 3)
    return source.texture;
  if (source.storage.mipmap)
    return source.storage.mipmap;
  MTLTextureDescriptor *d = [MTLTextureDescriptor
      texture2DDescriptorWithPixelFormat:MTLPixelFormatRGBA8Unorm
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
  ((__bridge QMetalSurface *)surface).storage.mipmap = nil;
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
  if (upload)
    for (size_t y = 0; y < h; y++)
      for (size_t x = 0; x < w; x++) {
        const uint8_t *in = bytes + (y * w + x) * 4;
        uint8_t *out = data + y * pitch + x * 4;
        out[0] = in[2];
        out[1] = in[1];
        out[2] = in[0];
        out[3] = in[3];
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
    for (size_t y = 0; y < h; y++)
      for (size_t x = 0; x < w; x++) {
        const uint8_t *in = data + y * pitch + x * 4;
        uint8_t *out = bytes + (y * w + x) * 4;
        out[0] = in[2];
        out[1] = in[1];
        out[2] = in[0];
        out[3] = in[3];
      }
  }
  atomic_fetch_add(&counters[upload ? 5 : 4], 1);
}
void qm_read_pixels(void *surface, uint8_t *out, size_t length) {
  LOCK if (!r || !surface) return;
  QMetalSurface *s = (__bridge QMetalSurface *)surface;
  if (!out || length != s.texture.width * s.texture.height * 4)
    return;
  transfer(s, out, false);
  UNLOCK
}
void qm_write_pixels(void *surface, const uint8_t *bytes, size_t length) {
  LOCK if (!r || !surface) return;
  QMetalSurface *s = (__bridge QMetalSurface *)surface;
  if (!bytes || length != s.texture.width * s.texture.height * 4)
    return;
  transfer(s, (uint8_t *)bytes, true);
  UNLOCK
}
void *qm_snapshot(void *surface) {
  LOCK if (!r || !surface) return NULL;
  QMetalSurface *s = (__bridge QMetalSurface *)surface;
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
static QGlyphImage *glyph_image(QMetalSurface *target, CTFontRef font,
                                CGGlyph glyph, double scale) {
  QMetalRenderer *r = renderer();
  CTFontRef f = CTFontCreateCopyWithAttributes(
      font, CTFontGetSize(font) * scale, NULL, NULL);
  if (!f)
    return nil;
  if (CFDictionaryGetCount(r.glyphImages) >= 32 ||
      r.glyphImageBytes >= 16 * 1024 * 1024) {
    CFDictionaryRemoveAllValues(r.glyphImages);
    r.glyphImageBytes = 0;
  }
  NSMutableDictionary *images =
      (__bridge NSMutableDictionary *)CFDictionaryGetValue(r.glyphImages, f);
  if (!images) {
    images = [NSMutableDictionary new];
    CFDictionarySetValue(r.glyphImages, f, (__bridge CFTypeRef)images);
  }
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
        void *surface = qm_surface(w, h);
        image.surface = CFBridgingRelease(surface);
        qm_write_pixels((__bridge void *)image.surface,
                        CGBitmapContextGetData(c), w * h * 4);
        CGContextRelease(c);
        images[key] = image;
        r.glyphImageBytes += w * h * 4;
      }
    }
  }
  CFRelease(f);
  return image;
}
static void draw_glyph_image(QMetalSurface *s, QGlyphImage *image, double scale,
                             double x, double y, bool colored,
                             vector_float4 tint) {
  if (!image.surface.texture)
    return;
  CGRect b = image.bounds;
  double dx = x + b.origin.x / scale, dy = y - CGRectGetMaxY(b) / scale,
         w = b.size.width / scale, h = b.size.height / scale;
  QParams p = params(s);
  p.mode = 4;
  p.color = tint;
  p.reserved = !colored;
  p.src = (vector_float4){0, 0, b.size.width, b.size.height};
  p.dst = (vector_float4){dx, dy, w, h};
  p.translation.w = colored ? tint.a : 1;
  CGPathRef rect = CGPathCreateWithRect(CGRectMake(dx, dy, w, h), NULL);
  CGRect box;
  NSData *edges = path_edges(s, rect, &box);
  CGPathRelease(rect);
  p.edges = edges.length / 16;
  bounds(&p, crop(s, box));
  dispatch(s, s.texture, p, edges, nil, image.surface.texture);
}
void qm_text_options(void *surface, int hinting) {
  LOCK if (!r || !surface) return;
  ((__bridge QMetalSurface *)surface).state.hinting = hinting;
  UNLOCK
}
void qm_glyphs(void *surface, void *font, double size, const uint16_t *glyphs,
               const double *positions, size_t n, double red, double green,
               double blue, double alpha) {
  LOCK if (!r || !surface) return;
  QMetalSurface *s = (__bridge QMetalSurface *)surface;
  CGAffineTransform ctm = s.state.transform;
  double scale = MAX(hypot(ctm.a, ctm.b), hypot(ctm.c, ctm.d));
  if (!(scale > 0) || !isfinite(scale))
    return;
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
          glyph_image(s, f, glyphs[i], scale);
      draw_glyph_image(s, image, scale, positions[i * 2], positions[i * 2 + 1],
                       value == kCFNull,
                       (vector_float4){red, green, blue, alpha});
    }
  }
  qm_fill(surface, combined, 0, red, green, blue, alpha);
  CGPathRelease(combined);
  CFRelease(f);
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
  id<MTLTexture> dst = (__bridge id<MTLTexture>)rawTexture;
  id<MTLCommandQueue> platform = (__bridge id<MTLCommandQueue>)platformQueue;
  if (!dst || !platform || dst.device.registryID != r.device.registryID ||
      platform.device.registryID != r.device.registryID)
    return 0;
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
  id<MTLCommandBuffer> cb = batch_buffer();
  id<MTLRenderCommandEncoder> e = [cb renderCommandEncoderWithDescriptor:pass];
  [e setRenderPipelineState:pipeline];
  [e setFragmentTexture:s.texture atIndex:0];
  uint32_t flip = flipped;
  [e setFragmentBytes:&flip length:4 atIndex:0];
  [e drawPrimitives:MTLPrimitiveTypeTriangle vertexStart:0 vertexCount:3];
  [e endEncoding];
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
  id<MTLCommandBuffer> cb = batch_buffer();
  id<MTLComputeCommandEncoder> e = [cb computeCommandEncoder];
  if (!cb || !e)
    return 0;
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
  bounds(&p, crop(s, CGRectMake(0, 0, s.texture.width, s.texture.height)));
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
  bounds(&p, crop(s, CGRectMake(0, 0, s.texture.width, s.texture.height)));
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
  d.colorAttachments[0].pixelFormat = MTLPixelFormatRGBA8Unorm;
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
    r.pending = nil;
    r.pendingOperations = r.pendingBytes = 0;
  } else
    flush_batch();
  UNLOCK
}

// Effect input may extend beyond the parent's output clip. The saved layer
// state reapplies the parent's exact mask when the filtered result is composed.
void qm_reset_effect_input_clip(void *surface) {
  LOCK if (!r || !surface) return;
  QMetalSurface *s = (__bridge QMetalSurface *)surface;
  s.state.clip = nil;
  s.state.clipRect = CGRectZero;
  UNLOCK
}
