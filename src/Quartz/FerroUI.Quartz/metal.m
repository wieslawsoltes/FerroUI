// Experimental GPU renderer: public Metal API, Core Graphics path preparation,
// and Core Text glyph outlines. No CPU framebuffer is used for presentation.
//
// Nothing here throws. Every recoverable failure (no device, a device without
// tier-2 read/write textures, a failed shader or allocation, a size beyond the
// device limit, a mismatched device) is a returned status or a NULL handle;
// the Rust side turns it into a Result or draws on the CPU path instead.
#import "metal.h"
#include "metal_source.h"
#import <CoreText/CoreText.h>
#import <Foundation/Foundation.h>
#import <Metal/Metal.h>
#import <MetalPerformanceShaders/MetalPerformanceShaders.h>
#include <simd/simd.h>
#include <stdatomic.h>

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
@interface QState : NSObject
@property CGAffineTransform transform;
@property id<MTLTexture> clip;
@property CGRect clipRect;
@property vector_float4 shadow, shadowColor, gradientOptions;
@property int blendMode, aliased, interpolation;
@end
@implementation QState
@end
@class QMetalRenderer;
static void recycle_texture(QMetalRenderer *r, id<MTLTexture> texture);
// Sharing this owner, rather than only the Metal handle, prevents snapshots and
// nested layers from returning aliased storage to the pool prematurely.
@interface QTexture : NSObject
@property QMetalRenderer *renderer;
@property id<MTLTexture> handle, mipmap;
@end
@implementation QTexture
- (void)dealloc {
  recycle_texture(_renderer, _handle);
}
@end
@interface QLayer : NSObject
@property QTexture *parent;
@property double opacity;
@property int blend;
// No texture could be allocated: the layer's content went to its parent.
@property BOOL failed;
@end
@implementation QLayer
@end
// One renderer per Metal device. Surfaces keep the renderer they were made on.
@interface QMetalRenderer : NSObject
@property id<MTLDevice> device;
@property id<MTLCommandQueue> queue;
@property id<MTLCommandBuffer> pending;
@property NSUInteger pendingOperations, pendingBytes, poolBytes, maxTextureSize;
@property NSMutableDictionary<NSString *, NSMutableArray<id<MTLTexture>> *>
    *texturePool;
@property id<MTLLibrary> library;
@property id<MTLComputePipelineState> paint;
@property id<MTLComputePipelineState> convertFrame;
@property id<MTLComputePipelineState> layerFilter, gaussian;
@property id<MTLSharedEvent> event;
@property NSMutableDictionary<NSData *, NSData *> *edgeCache;
@property NSUInteger edgeCacheBytes;
@property CFMutableDictionaryRef glyphCache;
@property uint64_t eventValue;
@property id<MTLTexture> white, transparent;
@property NSMutableDictionary<NSNumber *, id<MTLRenderPipelineState>>
    *presentPipelines;
@end
@implementation QMetalRenderer
@end
@interface QMetalSurface : NSObject
@property QMetalRenderer *renderer;
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
  owner.renderer = _renderer;
  owner.handle = texture;
  self.storage = owner;
}
@end

// One lock serializes all renderers: their pools, batches and caches.
static NSRecursiveLock *quartz_lock(void) {
  static NSRecursiveLock *lock;
  static dispatch_once_t once;
  dispatch_once(&once, ^{
    lock = [NSRecursiveLock new];
  });
  return lock;
}
#define LOCK                                                                   \
  @autoreleasepool {                                                           \
    NSRecursiveLock *lock_ = quartz_lock();                                    \
    [lock_ lock];                                                              \
    @try {
#define UNLOCK                                                                 \
  }                                                                            \
  @finally {                                                                   \
    [lock_ unlock];                                                            \
  }                                                                            \
  }

static QMetalSurface *surface_of(void *p) {
  if (!p)
    return nil;
  id object = (__bridge id)p;
  if (![object isKindOfClass:[QMetalSurface class]])
    return nil;
  QMetalSurface *s = object;
  return s.renderer ? s : nil;
}
int qm_is_surface(void *p) { return surface_of(p) != nil; }

static NSMutableDictionary<NSNumber *, QMetalRenderer *> *renderers;
static NSMutableDictionary<NSNumber *, NSNumber *> *failures;
static QMetalRenderer *current_renderer;
static BOOL default_attempted;
static int default_failure = QM_OK;

static id<MTLComputePipelineState>
compute_pipeline(id<MTLDevice> device, id<MTLLibrary> library,
                 NSString *name) {
  id<MTLFunction> function = [library newFunctionWithName:name];
  if (!function) {
    NSLog(@"Quartz Metal: missing kernel %@", name);
    return nil;
  }
  NSError *error = nil;
  id<MTLComputePipelineState> pipeline =
      [device newComputePipelineStateWithFunction:function error:&error];
  if (!pipeline)
    NSLog(@"Quartz Metal: kernel %@ failed: %@", name, error);
  return pipeline;
}
static id<MTLTexture> constant_texture(id<MTLDevice> device, uint32_t value) {
  MTLTextureDescriptor *d = [MTLTextureDescriptor
      texture2DDescriptorWithPixelFormat:MTLPixelFormatRGBA8Unorm
                                   width:1
                                  height:1
                               mipmapped:NO];
  d.usage = MTLTextureUsageShaderRead;
  id<MTLTexture> t = [device newTextureWithDescriptor:d];
  [t replaceRegion:MTLRegionMake2D(0, 0, 1, 1)
       mipmapLevel:0
         withBytes:&value
       bytesPerRow:4];
  return t;
}
static QMetalRenderer *create_renderer(id<MTLDevice> device, int *status) {
  if (!device) {
    *status = QM_NO_DEVICE;
    return nil;
  }
  // The paint kernel reads and writes RGBA8 textures in place. A device
  // without tier-2 read/write textures gets no renderer: the CPU path draws.
  if (device.readWriteTextureSupport < MTLReadWriteTextureTier2) {
    *status = QM_UNSUPPORTED_DEVICE;
    return nil;
  }
  QMetalRenderer *r = [QMetalRenderer new];
  r.device = device;
  r.queue = [device newCommandQueueWithMaxCommandBufferCount:4096];
  r.event = [device newSharedEvent];
  r.white = constant_texture(device, 0xffffffff);
  r.transparent = constant_texture(device, 0);
  if (!r.queue || !r.event || !r.white || !r.transparent) {
    *status = QM_OUT_OF_MEMORY;
    return nil;
  }
  NSError *error = nil;
  r.library = [device newLibraryWithSource:@(q_metal_source)
                                   options:nil
                                     error:&error];
  if (!r.library) {
    NSLog(@"Quartz Metal: shader compilation failed: %@", error);
    *status = QM_SHADER_FAILED;
    return nil;
  }
  r.paint = compute_pipeline(device, r.library, @"paint");
  r.convertFrame = compute_pipeline(device, r.library, @"convert_frame");
  r.layerFilter = compute_pipeline(device, r.library, @"layer_filter");
  r.gaussian = compute_pipeline(device, r.library, @"gaussian_pass");
  if (!r.paint || !r.convertFrame || !r.layerFilter || !r.gaussian) {
    *status = QM_SHADER_FAILED;
    return nil;
  }
  // Every GPU family that runs macOS 13 (Mac2, Apple7 and later) supports
  // 16384-pixel 2D textures.
  r.maxTextureSize = 16384;
  r.texturePool = [NSMutableDictionary new];
  r.edgeCache = [NSMutableDictionary new];
  r.presentPipelines = [NSMutableDictionary new];
  r.glyphCache =
      CFDictionaryCreateMutable(NULL, 0, &kCFTypeDictionaryKeyCallBacks,
                                &kCFTypeDictionaryValueCallBacks);
  if (!r.glyphCache) {
    *status = QM_OUT_OF_MEMORY;
    return nil;
  }
  *status = QM_OK;
  return r;
}
// Callers hold the lock. A device that failed once is not retried.
static QMetalRenderer *renderer_for_device(id<MTLDevice> device, int *status) {
  if (!device) {
    *status = QM_NO_DEVICE;
    return nil;
  }
  if (!renderers)
    renderers = [NSMutableDictionary new];
  if (!failures)
    failures = [NSMutableDictionary new];
  NSNumber *key = @(device.registryID);
  QMetalRenderer *r = renderers[key];
  if (r) {
    *status = QM_OK;
    return r;
  }
  NSNumber *failed = failures[key];
  if (failed) {
    *status = failed.intValue;
    return nil;
  }
  r = create_renderer(device, status);
  if (r)
    renderers[key] = r;
  else
    failures[key] = @(*status);
  return r;
}
// The renderer new surfaces are made on: the platform's device once one is
// configured, the system default device before that.
static QMetalRenderer *default_renderer(int *status) {
  if (current_renderer) {
    *status = QM_OK;
    return current_renderer;
  }
  if (default_attempted) {
    *status = default_failure;
    return nil;
  }
  default_attempted = YES;
  QMetalRenderer *r = renderer_for_device(MTLCreateSystemDefaultDevice(), status);
  if (r)
    current_renderer = r;
  else
    default_failure = *status;
  return r;
}
int qm_status(void) {
  int status = QM_NO_DEVICE;
  LOCK default_renderer(&status);
  UNLOCK
  return status;
}
size_t qm_max_texture_size(void) {
  size_t size = 0;
  LOCK int status = QM_OK;
  QMetalRenderer *r = default_renderer(&status);
  size = r ? r.maxTextureSize : 0;
  UNLOCK
  return size;
}
static void commit(id<MTLCommandBuffer> cb) {
  if (!cb)
    return;
  [cb addCompletedHandler:^(id<MTLCommandBuffer> done) {
    if (done.status == MTLCommandBufferStatusError) {
      atomic_fetch_add(&counters[6], 1);
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
static id<MTLCommandBuffer> batch_buffer(QMetalRenderer *r) {
  if (!r.pending) {
    r.pending = [r.queue commandBuffer];
    r.pending.label = @"FerroUI Quartz GPU batch";
  }
  return r.pending;
}
static void flush_batch(QMetalRenderer *r) {
  id<MTLCommandBuffer> cb = r.pending;
  r.pending = nil;
  r.pendingOperations = 0;
  r.pendingBytes = 0;
  commit(cb);
}
static void finish_operation(QMetalRenderer *r) {
  r.pendingOperations += 1;
  if (r.pendingOperations >= 256)
    flush_batch(r);
}
static NSString *texture_key(NSUInteger w, NSUInteger h) {
  return [NSString
      stringWithFormat:@"%lux%lu", (unsigned long)w, (unsigned long)h];
}
static void recycle_texture(QMetalRenderer *r, id<MTLTexture> t) {
  if (!r || !t)
    return;
  @autoreleasepool {
    NSRecursiveLock *lock = quartz_lock();
    [lock lock];
    // The ordered queue makes reuse safe even if an earlier encoder is still
    // executing. All command buffers retain their resources until completion.
    NSUInteger bytes = t.width * t.height * 4;
    if (t.device == r.device && bytes <= 64 * 1024 * 1024) {
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
    [lock unlock];
  }
}
// A pooled or new private RGBA8 texture, or nil for an empty size, a size
// beyond the device limit, or a failed allocation.
static id<MTLTexture> texture(QMetalRenderer *r, size_t w, size_t h) {
  if (!r || !w || !h || w > r.maxTextureSize || h > r.maxTextureSize)
    return nil;
  NSUInteger bytes = w * h * 4;
  if (r.pendingBytes && r.pendingBytes + bytes > 64 * 1024 * 1024)
    flush_batch(r);
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
  id<MTLTexture> t = [r.device newTextureWithDescriptor:d];
  if (!t)
    return nil;
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
  return n;
}
static QParams params(QMetalSurface *s) {
  QParams p = {0};
  p.blend = s.state.blendMode;
  p.flipped = (s.state.aliased ? 1 : 0) |
              (s.state.interpolation == 1 ? 2 : 0) |
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
static NSData *binned_edges(QMetalRenderer *r, QParams *p, NSData *edges) {
  if (p->edges < 32 || p->mode == 6 || !p->extent.y)
    return edges;
  const uint32_t stride = 8;
  uint32_t origin = p->origin.y / stride * stride,
           rows = (p->origin.y + p->extent.y - origin + stride - 1) / stride;
  vector_uint4 header = {origin, rows, stride, 0};
  NSMutableData *key = [NSMutableData dataWithBytes:&header
                                             length:sizeof(header)];
  [key appendData:edges];
  NSData *cached = r.edgeCache[key];
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
  if (r.edgeCache.count >= 128 ||
      r.edgeCacheBytes + bytesToKeep > 16 * 1024 * 1024) {
    [r.edgeCache removeAllObjects];
    r.edgeCacheBytes = 0;
  }
  if (bytesToKeep <= 16 * 1024 * 1024) {
    r.edgeCache[key] = result;
    r.edgeCacheBytes += bytesToKeep;
  }
  p->edgeRows = rows;
  p->edgeOriginY = origin;
  p->edgeStride = stride;
  return result;
}
static void dispatch(QMetalSurface *s, id<MTLTexture> target, QParams p,
                     NSData *edges, NSData *stops, id<MTLTexture> image) {
  if (!target || !p.extent.x || !p.extent.y)
    return;
  QMetalRenderer *r = s.renderer;
  edges = binned_edges(r, &p, edges);
  vector_float4 zero = {0};
  id<MTLBuffer> e =
      [r.device newBufferWithBytes:edges.length ? edges.bytes : &zero
                            length:MAX(edges.length, sizeof(zero))
                           options:MTLResourceStorageModeShared];
  id<MTLBuffer> g =
      [r.device newBufferWithBytes:stops.length ? stops.bytes : &zero
                            length:MAX(stops.length, sizeof(zero))
                           options:MTLResourceStorageModeShared];
  if (!e || !g)
    return;
  id<MTLCommandBuffer> cb = batch_buffer(r);
  id<MTLComputeCommandEncoder> enc = [cb computeCommandEncoder];
  if (!enc)
    return;
  if (target == s.texture)
    s.storage.mipmap = nil;
  [enc setComputePipelineState:r.paint];
  [enc setTexture:target atIndex:0];
  [enc setTexture:s.state.clip ?: r.white atIndex:1];
  [enc setTexture:image ?: r.white atIndex:2];
  [enc setBuffer:e offset:0 atIndex:0];
  [enc setBytes:&p length:sizeof(p) atIndex:1];
  [enc setBuffer:g offset:0 atIndex:2];
  [enc dispatchThreads:MTLSizeMake(p.extent.x, p.extent.y, 1)
      threadsPerThreadgroup:MTLSizeMake(8, 8, 1)];
  [enc endEncoding];
  finish_operation(r);
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
    // A flattened path has no curves; anything else contributes no edge.
    break;
  }
}
static NSData *path_edges(QMetalSurface *s, CGPathRef path, CGRect *box) {
  NSMutableData *data = [NSMutableData new];
  *box = CGRectNull;
  if (!path)
    return data;
  CGAffineTransform t = s.state.transform;
  CGPathRef transformed = CGPathCreateCopyByTransformingPath(path, &t);
  if (!transformed)
    return data;
  *box = CGPathGetPathBoundingBox(transformed);
  CGPathRef flat = CGPathCreateCopyByFlattening(transformed, 0.25);
  if (flat) {
    EdgeSink sink = {data, {0, 0}, {0, 0}, false};
    CGPathApply(flat, &sink, path_edge);
    if (sink.open)
      edge(&sink, sink.first);
    CGPathRelease(flat);
  }
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
static QMetalSurface *new_surface(QMetalRenderer *r) {
  QMetalSurface *s = [QMetalSurface new];
  s.renderer = r;
  s.states = [NSMutableArray new];
  s.layers = [NSMutableArray new];
  s.state = [QState new];
  s.state.transform = CGAffineTransformIdentity;
  s.state.gradientOptions = (vector_float4){0, 1, 0, 0};
  return s;
}
static QMetalSurface *create_surface(QMetalRenderer *r, size_t w, size_t h) {
  id<MTLTexture> t = texture(r, w, h);
  if (!t)
    return nil;
  QMetalSurface *s = new_surface(r);
  s.texture = t;
  clear_texture(s);
  atomic_fetch_add(&counters[0], 1);
  return s;
}
void *qm_surface(size_t w, size_t h) {
  void *result = NULL;
  LOCK int status = QM_OK;
  QMetalSurface *s = create_surface(default_renderer(&status), w, h);
  if (s)
    result = (__bridge_retained void *)s;
  UNLOCK
  return result;
}
int qm_configure(void *device) {
  int status = QM_NO_DEVICE;
  LOCK QMetalRenderer *r =
      renderer_for_device((__bridge id<MTLDevice>)device, &status);
  // New surfaces are made on the platform's device; earlier ones keep theirs
  // and are copied across when drawn onto it.
  if (r)
    current_renderer = r;
  UNLOCK
  return status;
}
void qm_save(void *surface) {
  LOCK QMetalSurface *s = surface_of(surface);
  if (s)
    [s.states addObject:copy_state(s.state)];
  UNLOCK
}
void qm_restore(void *surface) {
  LOCK QMetalSurface *s = surface_of(surface);
  // An unbalanced restore keeps the current state.
  if (s && s.states.count) {
    s.state = s.states.lastObject;
    [s.states removeLastObject];
  }
  UNLOCK
}
void qm_set_transform(void *surface, double a, double b, double c, double d,
                      double x, double y, double sx, double sy) {
  LOCK QMetalSurface *s = surface_of(surface);
  if (s)
    s.state.transform =
        CGAffineTransformMake(a * sx, b * sy, c * sx, d * sy, x * sx, y * sy);
  UNLOCK
}
void qm_clear(void *surface, double red, double green, double blue,
              double alpha) {
  LOCK QMetalSurface *s = surface_of(surface);
  if (!s)
    return;
  QParams p = params(s);
  p.mode = 5;
  p.blend = 17;
  p.color = (vector_float4){red, green, blue, alpha};
  dispatch(s, s.texture, p, nil, nil, nil);
  UNLOCK
}
void qm_clip(void *surface, CGPathRef path, int evenodd) {
  LOCK QMetalSurface *s = surface_of(surface);
  if (!s)
    return;
  CGRect box;
  NSData *edges = path_edges(s, path, &box);
  box = crop(s, box);
  QParams p = params(s);
  p.mode = 1;
  p.edges = (uint32_t)(edges.length / 16);
  p.evenodd = evenodd;
  bounds(&p, box);
  id<MTLTexture> mask =
      texture(s.renderer, MAX(1, p.extent.x), MAX(1, p.extent.y));
  if (!mask) {
    // Without a mask nothing inside the clip can be shown faithfully: clip
    // everything rather than draw outside it.
    s.state.clip = s.renderer.transparent;
    s.state.clipRect = CGRectZero;
    return;
  }
  dispatch(s, mask, p, edges, nil, nil);
  s.state.clip = mask;
  s.state.clipRect = box;
  UNLOCK
}
void qm_fill(void *surface, CGPathRef path, int evenodd, double red,
             double green, double blue, double alpha) {
  LOCK QMetalSurface *s = surface_of(surface);
  if (!s)
    return;
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
  LOCK QMetalSurface *s = surface_of(surface);
  if (!s || !n || !colors || !locations)
    return;
  QParams p = params(s);
  p.mode = radial == 2 ? 7 : (radial ? 3 : 2);
  p.reserved = (uint32_t)s.state.gradientOptions.x;
  p.shadow = (vector_float4){s.state.gradientOptions.y,
                             s.state.gradientOptions.z, 0, 0};
  p.gradient = (vector_float4){x1, y1, x2, y2};
  p.translation.z = radius;
  p.stops = (uint32_t)n;
  bounds(&p, crop(s, CGRectMake(0, 0, s.texture.width, s.texture.height)));
  NSMutableData *stops = [NSMutableData dataWithLength:n * sizeof(QStop)];
  QStop *out = stops.mutableBytes;
  if (!out)
    return;
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
  if (!image || dw <= 0 || dh <= 0)
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
  p.edges = (uint32_t)(edges.length / 16);
  bounds(&p, crop(s, box));
  dispatch(s, s.texture, p, edges, nil, image);
}
void qm_begin_layer(void *surface, double opacity, int blend) {
  LOCK QMetalSurface *s = surface_of(surface);
  if (!s)
    return;
  qm_save(surface);
  QLayer *layer = [QLayer new];
  layer.parent = s.storage;
  layer.opacity = opacity;
  layer.blend = blend;
  [s.layers addObject:layer];
  id<MTLTexture> t = texture(s.renderer, layer.parent.handle.width,
                             layer.parent.handle.height);
  if (!t) {
    // Keep push/pop balanced; the content draws straight onto the parent.
    layer.failed = YES;
    return;
  }
  s.texture = t;
  clear_texture(s);
  UNLOCK
}
void qm_end_layer(void *surface) {
  LOCK QMetalSurface *s = surface_of(surface);
  QLayer *layer = s.layers.lastObject;
  if (!s || !layer)
    return;
  QTexture *__attribute__((objc_precise_lifetime)) imageOwner = s.storage;
  id<MTLTexture> image = imageOwner.handle;
  s.storage = layer.parent;
  [s.layers removeLastObject];
  qm_restore(surface);
  if (layer.failed)
    return;
  CGAffineTransform transform = s.state.transform;
  s.state.transform = CGAffineTransformIdentity;
  draw_texture(s, image, 0, 0, image.width, image.height, 0, 0, image.width,
               image.height, layer.opacity, layer.blend);
  s.state.transform = transform;
  (void)imageOwner; // Keep the owner alive through encoding.
  UNLOCK
}
static void copy_texture(QMetalRenderer *r, id<MTLTexture> dst,
                         id<MTLTexture> src) {
  if (!dst || !src)
    return;
  id<MTLCommandBuffer> cb = batch_buffer(r);
  id<MTLBlitCommandEncoder> e = [cb blitCommandEncoder];
  if (!e)
    return;
  [e copyFromTexture:src
            sourceSlice:0
            sourceLevel:0
           sourceOrigin:MTLOriginMake(0, 0, 0)
             sourceSize:MTLSizeMake(MIN(src.width, dst.width),
                                    MIN(src.height, dst.height), 1)
              toTexture:dst
       destinationSlice:0
       destinationLevel:0
      destinationOrigin:MTLOriginMake(0, 0, 0)];
  [e endEncoding];
  finish_operation(r);
}
// Cache a GPU-generated mip chain until the source is next written.
static id<MTLTexture> sampled_texture(QMetalSurface *source,
                                      int interpolation) {
  if (interpolation < 3)
    return source.texture;
  if (source.storage.mipmap)
    return source.storage.mipmap;
  QMetalRenderer *r = source.renderer;
  MTLTextureDescriptor *d = [MTLTextureDescriptor
      texture2DDescriptorWithPixelFormat:MTLPixelFormatRGBA8Unorm
                                   width:source.texture.width
                                  height:source.texture.height
                               mipmapped:YES];
  d.storageMode = MTLStorageModePrivate;
  d.usage = MTLTextureUsageShaderRead;
  id<MTLTexture> mip = [r.device newTextureWithDescriptor:d];
  if (!mip)
    return source.texture; // Sample without mip levels.
  copy_texture(r, mip, source.texture);
  id<MTLBlitCommandEncoder> e = [batch_buffer(r) blitCommandEncoder];
  if (!e)
    return source.texture;
  [e generateMipmapsForTexture:mip];
  [e endEncoding];
  finish_operation(r);
  source.storage.mipmap = mip;
  return mip;
}
// Moves pixels between a texture and `bytes` (tightly packed premultiplied
// BGRA). A readback waits for the GPU.
static int transfer_texture(QMetalRenderer *r, id<MTLTexture> t,
                            uint8_t *bytes, bool upload) {
  if (!r || !t || !bytes)
    return QM_INVALID_ARGUMENT;
  size_t w = t.width, h = t.height, pitch = (w * 4 + 255) & ~(size_t)255;
  id<MTLBuffer> buffer =
      [r.device newBufferWithLength:pitch * h
                            options:MTLResourceStorageModeShared];
  if (!buffer)
    return QM_OUT_OF_MEMORY;
  uint8_t *data = buffer.contents;
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
  id<MTLCommandBuffer> cb = batch_buffer(r);
  id<MTLBlitCommandEncoder> e = [cb blitCommandEncoder];
  if (!e)
    return QM_GPU_FAILED;
  if (upload)
    [e copyFromBuffer:buffer
               sourceOffset:0
          sourceBytesPerRow:pitch
        sourceBytesPerImage:pitch * h
                 sourceSize:MTLSizeMake(w, h, 1)
                  toTexture:t
           destinationSlice:0
           destinationLevel:0
          destinationOrigin:MTLOriginMake(0, 0, 0)];
  else
    [e copyFromTexture:t
                     sourceSlice:0
                     sourceLevel:0
                    sourceOrigin:MTLOriginMake(0, 0, 0)
                      sourceSize:MTLSizeMake(w, h, 1)
                        toBuffer:buffer
               destinationOffset:0
          destinationBytesPerRow:pitch
        destinationBytesPerImage:pitch * h];
  [e endEncoding];
  finish_operation(r);
  if (!upload) {
    flush_batch(r);
    [cb waitUntilCompleted];
    if (cb.status != MTLCommandBufferStatusCompleted)
      return QM_GPU_FAILED;
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
  return QM_OK;
}
static int transfer(QMetalSurface *s, uint8_t *bytes, bool upload) {
  if (upload)
    s.storage.mipmap = nil;
  return transfer_texture(s.renderer, s.texture, bytes, upload);
}
// A texture of `source` that encoders of renderer `r` may sample. A surface of
// another device is copied across through shared memory; *keep then owns the
// copy for the duration of the encoding.
static id<MTLTexture> source_texture(QMetalRenderer *r, QMetalSurface *source,
                                     int interpolation,
                                     QTexture *__strong *keep) {
  if (!r || !source || !source.texture)
    return nil;
  if (source.renderer == r)
    return sampled_texture(source, interpolation);
  size_t w = source.texture.width, h = source.texture.height;
  NSMutableData *data = [NSMutableData dataWithLength:w * h * 4];
  if (!data || transfer_texture(source.renderer, source.texture,
                                data.mutableBytes, false) != QM_OK)
    return nil;
  id<MTLTexture> t = texture(r, w, h);
  if (!t || transfer_texture(r, t, data.mutableBytes, true) != QM_OK)
    return nil;
  QTexture *owner = [QTexture new];
  owner.renderer = r;
  owner.handle = t;
  *keep = owner;
  return t;
}
void qm_copy_surface(void *surface, void *source) {
  LOCK QMetalSurface *s = surface_of(surface);
  QMetalSurface *from = surface_of(source);
  if (!s || !from)
    return;
  QTexture *__attribute__((objc_precise_lifetime)) keep = nil;
  id<MTLTexture> image = source_texture(s.renderer, from, 0, &keep);
  if (!image)
    return;
  s.storage.mipmap = nil;
  copy_texture(s.renderer, s.texture, image);
  UNLOCK
}
void qm_draw_surface(void *surface, void *source, double sx, double sy,
                     double sw, double sh, double dx, double dy, double dw,
                     double dh, double alpha) {
  LOCK QMetalSurface *s = surface_of(surface);
  QMetalSurface *from = surface_of(source);
  if (!s || !from)
    return;
  QTexture *__attribute__((objc_precise_lifetime)) keep = nil;
  id<MTLTexture> image =
      source_texture(s.renderer, from, s.state.interpolation, &keep);
  if (!image)
    return;
  if (image == s.texture) {
    id<MTLTexture> copy = texture(s.renderer, image.width, image.height);
    if (!copy)
      return;
    copy_texture(s.renderer, copy, image);
    image = copy;
  }
  draw_texture(s, image, sx, sy, sw, sh, dx, dy, dw, dh, alpha,
               s.state.blendMode);
  (void)keep;
  UNLOCK
}
int qm_read_pixels(void *surface, uint8_t *out, size_t length) {
  int status = QM_INVALID_ARGUMENT;
  LOCK QMetalSurface *s = surface_of(surface);
  if (s && s.texture && out &&
      length == s.texture.width * s.texture.height * 4)
    status = transfer(s, out, false);
  UNLOCK
  return status;
}
int qm_write_pixels(void *surface, const uint8_t *bytes, size_t length) {
  int status = QM_INVALID_ARGUMENT;
  LOCK QMetalSurface *s = surface_of(surface);
  if (s && s.texture && bytes &&
      length == s.texture.width * s.texture.height * 4)
    status = transfer(s, (uint8_t *)bytes, true);
  UNLOCK
  return status;
}
void *qm_snapshot(void *surface) {
  CGImageRef image = NULL;
  LOCK QMetalSurface *s = surface_of(surface);
  if (!s || !s.texture)
    return NULL;
  size_t w = s.texture.width, h = s.texture.height;
  NSMutableData *data = [NSMutableData dataWithLength:w * h * 4];
  if (!data || transfer(s, data.mutableBytes, false) != QM_OK)
    return NULL;
  CGDataProviderRef provider =
      CGDataProviderCreateWithCFData((__bridge CFDataRef)data);
  CGColorSpaceRef cs = CGColorSpaceCreateWithName(kCGColorSpaceSRGB);
  if (provider && cs)
    image = CGImageCreate(
        w, h, 8, 32, w * 4, cs,
        kCGImageAlphaPremultipliedFirst | kCGBitmapByteOrder32Little, provider,
        NULL, false, kCGRenderingIntentDefault);
  CGColorSpaceRelease(cs);
  CGDataProviderRelease(provider);
  UNLOCK
  return (void *)image;
}
void qm_draw_image(void *surface, CGImageRef image, double sx, double sy,
                   double sw, double sh, double dx, double dy, double dw,
                   double dh, double alpha) {
  LOCK QMetalSurface *s = surface_of(surface);
  if (!s || !image)
    return;
  size_t w = CGImageGetWidth(image), h = CGImageGetHeight(image);
  if (!w || !h)
    return;
  NSMutableData *data = [NSMutableData dataWithLength:w * h * 4];
  if (!data)
    return;
  CGColorSpaceRef cs = CGColorSpaceCreateWithName(kCGColorSpaceSRGB);
  CGContextRef c = CGBitmapContextCreate(data.mutableBytes, w, h, 8, w * 4, cs,
                                         kCGImageAlphaPremultipliedFirst |
                                             kCGBitmapByteOrder32Little);
  CGColorSpaceRelease(cs);
  if (!c)
    return;
  CGContextDrawImage(c, CGRectMake(0, 0, w, h), image);
  CGContextRelease(c);
  QMetalSurface *tmp = create_surface(s.renderer, w, h);
  if (!tmp || transfer(tmp, data.mutableBytes, true) != QM_OK)
    return;
  qm_draw_surface(surface, (__bridge void *)tmp, sx, sy, sw, sh, dx, dy, dw,
                  dh, alpha);
  UNLOCK
}
void qm_glyphs(void *surface, void *font, double size, const uint16_t *glyphs,
               const double *positions, size_t n, double red, double green,
               double blue, double alpha) {
  LOCK QMetalSurface *s = surface_of(surface);
  if (!s || !font || !n || !glyphs || !positions)
    return;
  QMetalRenderer *r = s.renderer;
  CTFontRef f = CTFontCreateCopyWithAttributes(font, size, NULL, NULL);
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
    if (value != kCFNull) {
      CGPathAddPath(combined, &transform, (CGPathRef)value);
    } else { // Color glyphs are small CPU-generated image inputs, never a CPU
             // frame.
      CGRect b = CTFontGetBoundingRectsForGlyphs(
          f, kCTFontOrientationHorizontal, &glyphs[i], NULL, 1);
      b = CGRectIntegral(b);
      if (b.size.width <= 0 || b.size.height <= 0)
        continue;
      size_t w = b.size.width, h = b.size.height;
      CGColorSpaceRef cs = CGColorSpaceCreateWithName(kCGColorSpaceSRGB);
      CGContextRef c = CGBitmapContextCreate(NULL, w, h, 8, w * 4, cs,
                                             kCGImageAlphaPremultipliedFirst |
                                                 kCGBitmapByteOrder32Little);
      CGColorSpaceRelease(cs);
      if (!c)
        continue;
      CGContextSetRGBFillColor(c, red, green, blue, alpha);
      CGPoint point = CGPointMake(-b.origin.x, -b.origin.y);
      CTFontDrawGlyphs(f, &glyphs[i], &point, 1, c);
      CGImageRef image = CGBitmapContextCreateImage(c);
      if (image) {
        qm_draw_image(surface, image, 0, 0, w, h,
                      positions[i * 2] + b.origin.x,
                      positions[i * 2 + 1] - CGRectGetMaxY(b), w, h, alpha);
        CGImageRelease(image);
      }
      CGContextRelease(c);
    }
  }
  qm_fill(surface, combined, 0, red, green, blue, alpha);
  CGPathRelease(combined);
  CFRelease(f);
  UNLOCK
}
void qm_shadow(void *surface, double x, double y, double blur, double red,
               double green, double blue, double alpha) {
  LOCK QMetalSurface *s = surface_of(surface);
  if (!s)
    return;
  CGAffineTransform t = s.state.transform;
  s.state.shadow =
      (vector_float4){t.a * x + t.c * y, t.b * x + t.d * y,
                      blur * MAX(hypot(t.a, t.b), hypot(t.c, t.d)), 0};
  s.state.shadowColor = (vector_float4){red, green, blue, alpha};
  UNLOCK
}
int qm_present(void *surface, void *rawTexture, void *platformQueue,
               int flipped) {
  int status = QM_INVALID_ARGUMENT;
  LOCK QMetalSurface *s = surface_of(surface);
  id<MTLTexture> dst = (__bridge id<MTLTexture>)rawTexture;
  id<MTLCommandQueue> queue = (__bridge id<MTLCommandQueue>)platformQueue;
  if (!s || !s.texture || !dst || !queue)
    return status;
  QMetalRenderer *r = s.renderer;
  if (dst.device.registryID != r.device.registryID ||
      queue.device.registryID != r.device.registryID) {
    status = QM_DEVICE_MISMATCH;
    return status;
  }
  NSNumber *key = @(dst.pixelFormat);
  id<MTLRenderPipelineState> pipeline = r.presentPipelines[key];
  if (!pipeline) {
    MTLRenderPipelineDescriptor *d = [MTLRenderPipelineDescriptor new];
    d.vertexFunction = [r.library newFunctionWithName:@"present_vertex"];
    d.fragmentFunction = [r.library newFunctionWithName:@"present_fragment"];
    d.colorAttachments[0].pixelFormat = dst.pixelFormat;
    NSError *error = nil;
    if (d.vertexFunction && d.fragmentFunction)
      pipeline = [r.device newRenderPipelineStateWithDescriptor:d
                                                          error:&error];
    if (!pipeline) {
      NSLog(@"Quartz Metal: presentation pipeline failed: %@", error);
      status = QM_SHADER_FAILED;
      return status;
    }
    r.presentPipelines[key] = pipeline;
  }
  MTLRenderPassDescriptor *pass =
      [MTLRenderPassDescriptor renderPassDescriptor];
  pass.colorAttachments[0].texture = dst;
  pass.colorAttachments[0].loadAction = MTLLoadActionDontCare;
  pass.colorAttachments[0].storeAction = MTLStoreActionStore;
  id<MTLCommandBuffer> cb = batch_buffer(r);
  id<MTLRenderCommandEncoder> e = [cb renderCommandEncoderWithDescriptor:pass];
  if (!e) {
    status = QM_GPU_FAILED;
    return status;
  }
  [e setRenderPipelineState:pipeline];
  [e setFragmentTexture:s.texture atIndex:0];
  uint32_t flip = flipped;
  [e setFragmentBytes:&flip length:4 atIndex:0];
  [e drawPrimitives:MTLPrimitiveTypeTriangle vertexStart:0 vertexCount:3];
  [e endEncoding];
  r.eventValue += 1;
  uint64_t value = r.eventValue;
  [cb encodeSignalEvent:r.event value:value];
  flush_batch(r);
  id<MTLCommandBuffer> wait = [queue commandBuffer];
  if (!wait) {
    status = QM_GPU_FAILED;
    return status;
  }
  [wait encodeWaitForEvent:r.event value:value];
  commit(wait);
  atomic_fetch_add(&counters[3], 1);
  status = QM_OK;
  UNLOCK
  return status;
}
void qm_statistics(uint64_t *out, size_t count) {
  // Diagnostics include completion errors from everything previously submitted.
  LOCK for (QMetalRenderer *r in renderers.allValues) {
    flush_batch(r);
    id<MTLCommandBuffer> done = [r.queue commandBuffer];
    commit(done);
    [done waitUntilCompleted];
  }
  if (out)
    for (size_t i = 0; i < MIN(count, 9); i++)
      out[i] = atomic_load(&counters[i]);
  UNLOCK
}

int qm_convert_frame(void *surface, void *rawTexture, int rec709, int flipped,
                     int opaque, void *context,
                     void (*completed)(void *, int)) {
  int result = 0;
  LOCK QMetalSurface *s = surface_of(surface);
  id<MTLTexture> source = (__bridge id<MTLTexture>)rawTexture;
  if (!s || !s.texture || !source)
    return result;
  QMetalRenderer *r = s.renderer;
  if (source.device.registryID != r.device.registryID ||
      source.textureType != MTLTextureType2D || source.sampleCount != 1 ||
      source.width != s.texture.width || source.height != s.texture.height ||
      !(source.usage & MTLTextureUsageShaderRead) ||
      (source.pixelFormat != MTLPixelFormatBGRA8Unorm &&
       source.pixelFormat != MTLPixelFormatRGBA8Unorm))
    return result;
  id<MTLCommandBuffer> cb = batch_buffer(r);
  id<MTLComputeCommandEncoder> e = [cb computeCommandEncoder];
  if (!e)
    return result;
  // The conversion rewrites the surface, so its cached mip chain is stale.
  s.storage.mipmap = nil;
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
  flush_batch(r);
  result = 1;
  UNLOCK
  return result;
}

// Wait/copy/signal all use the renderer queue, not the platform presentation
// queue. The returned bitmap owns independent pixels before signaling reuse.
int qm_snapshot_external(void *surface, void *source, int flipped,
                         void *waitEvent, uint64_t waitValue,
                         void *signalEvent, uint64_t signalValue) {
  int result = 0;
  LOCK QMetalSurface *s = surface_of(surface);
  if (!s)
    return result;
  QMetalRenderer *r = s.renderer;
  id<MTLSharedEvent> wait = (__bridge id<MTLSharedEvent>)waitEvent;
  id<MTLSharedEvent> signal = (__bridge id<MTLSharedEvent>)signalEvent;
  BOOL canWait = wait && (!wait.device ||
                          wait.device.registryID == r.device.registryID);
  BOOL canSignal = signal && (!signal.device ||
                              signal.device.registryID == r.device.registryID);
  if (canWait) {
    [batch_buffer(r) encodeWaitForEvent:wait value:waitValue];
    result = qm_convert_frame(surface, source, 0, flipped, 0, NULL, NULL);
  }
  // The producer reuses its storage after this signal, so it is encoded
  // whether or not the copy happened: a failed copy never stalls it.
  if (canSignal)
    [batch_buffer(r) encodeSignalEvent:signal value:signalValue];
  flush_batch(r);
  UNLOCK
  return result;
}
#ifdef QUARTZ_MEDIA_TEST_FIXTURES
#import <CoreVideo/CoreVideo.h>
#import <IOSurface/IOSurfaceRef.h>
// Synthetic camera input only. GPU-generated immutable CoreVideo storage, with
// the same IOSurface import path as AVFoundation. No CPU pixel mapping/upload.
void *qm_fixture_buffer(uint64_t sequence) {
  CVPixelBufferRef result = NULL;
  LOCK int status = QM_OK;
  QMetalRenderer *r = default_renderer(&status);
  if (!r)
    return NULL;
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
  id<MTLCommandBuffer> cb = batch_buffer(r);
  id<MTLRenderCommandEncoder> e = [cb renderCommandEncoderWithDescriptor:pass];
  [e endEncoding];
  flush_batch(r);
  [cb waitUntilCompleted];
  if (!cb || cb.status != MTLCommandBufferStatusCompleted) {
    CVPixelBufferRelease(buffer);
    return NULL;
  }
  CVBufferSetAttachment(buffer, kCVImageBufferColorPrimariesKey,
                        kCVImageBufferColorPrimaries_ITU_R_709_2,
                        kCVAttachmentMode_ShouldPropagate);
  CVBufferSetAttachment(buffer, kCVImageBufferTransferFunctionKey,
                        kCVImageBufferTransferFunction_sRGB,
                        kCVAttachmentMode_ShouldPropagate);
  result = buffer;
  UNLOCK
  return result;
}
#endif

static void filter_compute(QMetalSurface *s,
                           id<MTLComputePipelineState> pipeline,
                           const void *data, size_t length, uint32_t kind,
                           id<MTLTexture> secondary) {
  QMetalRenderer *r = s.renderer;
  id<MTLTexture> out = texture(r, s.texture.width, s.texture.height);
  if (!out)
    return; // The layer keeps its unfiltered content.
  vector_float4 zero = {0};
  // The ABI data are scalars padded to float4 alignment; no shader reads past
  // the documented parameters for its operation.
  size_t size = MAX(16, (length + 15) & ~(size_t)15);
  id<MTLBuffer> parameters =
      [r.device newBufferWithLength:size options:MTLResourceStorageModeShared];
  if (!parameters)
    return;
  memcpy(parameters.contents, data ?: (const void *)&zero,
         data ? length : sizeof(zero));
  id<MTLCommandBuffer> cb = batch_buffer(r);
  id<MTLComputeCommandEncoder> e = [cb computeCommandEncoder];
  if (!e)
    return;
  [e setComputePipelineState:pipeline];
  [e setTexture:out atIndex:0];
  [e setTexture:s.texture atIndex:1];
  [e setTexture:secondary ?: r.white atIndex:2];
  [e setBuffer:parameters offset:0 atIndex:0];
  [e setBytes:&kind length:4 atIndex:1];
  [e dispatchThreads:MTLSizeMake(out.width, out.height, 1)
      threadsPerThreadgroup:MTLSizeMake(8, 8, 1)];
  [e endEncoding];
  finish_operation(r);
  s.texture = out;
}
void *qm_layer_snapshot(void *surface) {
  void *result = NULL;
  LOCK QMetalSurface *s = surface_of(surface);
  if (s) {
    QMetalSurface *copy = new_surface(s.renderer);
    copy.storage = s.storage;
    result = (__bridge_retained void *)copy;
  }
  UNLOCK
  return result;
}
void qm_blur(void *surface, double sigma, double ax, double ay, double bx,
             double by) {
  LOCK QMetalSurface *s = surface_of(surface);
  if (!s || !(sigma > 0) || !isfinite(sigma))
    return;
  QMetalRenderer *r = s.renderer;
  double x = hypot(ax, ay), y = hypot(bx, by);
  if (fabs(ay) < 1e-6 && fabs(bx) < 1e-6 && fabs(x - y) < 1e-6 && x > 0 &&
      MPSSupportsMTLDevice(r.device)) {
    id<MTLTexture> out = texture(r, s.texture.width, s.texture.height);
    MPSImageGaussianBlur *blur =
        [[MPSImageGaussianBlur alloc] initWithDevice:r.device sigma:sigma * x];
    id<MTLCommandBuffer> cb = batch_buffer(r);
    if (!out || !blur || !cb)
      return;
    blur.edgeMode = MPSImageEdgeModeZero;
    [blur encodeToCommandBuffer:cb
                  sourceTexture:s.texture
             destinationTexture:out];
    finish_operation(r);
    s.texture = out;
  } else {
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
  LOCK QMetalSurface *s = surface_of(surface);
  if (!s)
    return;
  QTexture *__attribute__((objc_precise_lifetime)) keep = nil;
  QMetalSurface *other = surface_of(secondary);
  id<MTLTexture> input =
      other ? source_texture(s.renderer, other, 0, &keep) : nil;
  if (other && !input)
    return; // The secondary input is unavailable: leave the layer as it is.
  filter_compute(s, s.renderer.layerFilter, data, length * sizeof(float), kind,
                 input);
  (void)keep;
  UNLOCK
}

void qm_gradient_options(void *surface, int spread, double aspect,
                         double angle) {
  LOCK QMetalSurface *s = surface_of(surface);
  if (s)
    s.state.gradientOptions = (vector_float4){spread, aspect, angle, 0};
  UNLOCK
}

void qm_draw_tile(void *surface, void *source, double x, double y, double w,
                  double h, double alpha, int mode) {
  LOCK QMetalSurface *s = surface_of(surface);
  QMetalSurface *from = surface_of(source);
  if (!s || !from)
    return;
  QTexture *__attribute__((objc_precise_lifetime)) keep = nil;
  id<MTLTexture> image =
      source_texture(s.renderer, from, s.state.interpolation, &keep);
  if (!image)
    return;
  QParams p = params(s);
  p.mode = 8;
  p.dst = (vector_float4){x, y, w, h};
  p.translation.w = alpha;
  p.reserved = mode;
  bounds(&p, crop(s, CGRectMake(0, 0, s.texture.width, s.texture.height)));
  dispatch(s, s.texture, p, nil, nil, image);
  (void)keep;
  UNLOCK
}

void qm_render_options(void *surface, int blend, int aliased,
                       int interpolation) {
  LOCK QMetalSurface *s = surface_of(surface);
  if (s) {
    s.state.blendMode = blend;
    s.state.aliased = aliased;
    s.state.interpolation = interpolation;
  }
  UNLOCK
}

void *qm_take_layer(void *surface) {
  void *result = NULL;
  LOCK QMetalSurface *s = surface_of(surface);
  if (!s)
    return NULL;
  id<MTLTexture> next = texture(s.renderer, s.texture.width, s.texture.height);
  if (!next)
    return NULL; // The layer keeps accumulating into its current texture.
  result = qm_layer_snapshot(surface);
  s.texture = next;
  clear_texture(s);
  UNLOCK
  return result;
}

void qm_acrylic(void *surface, const double *colors, void *noise, int replace) {
  LOCK QMetalSurface *s = surface_of(surface);
  QMetalSurface *source = surface_of(noise);
  if (!s || !source || !colors)
    return;
  QTexture *__attribute__((objc_precise_lifetime)) keep = nil;
  id<MTLTexture> image = source_texture(s.renderer, source, 0, &keep);
  if (!image)
    return;
  QParams p = params(s);
  p.mode = 9;
  if (replace)
    p.blend = 17;
  p.color = (vector_float4){colors[0], colors[1], colors[2], colors[3]};
  p.gradient = (vector_float4){colors[4], colors[5], colors[6], colors[7]};
  bounds(&p, crop(s, CGRectMake(0, 0, s.texture.width, s.texture.height)));
  dispatch(s, s.texture, p, nil, nil, image);
  (void)keep;
  UNLOCK
}

void *qm_shader_compile(const char *source, const char *vertex,
                        const char *fragment, char *message, size_t length) {
  void *result = NULL;
  LOCK int status = QM_OK;
  QMetalRenderer *r = default_renderer(&status);
  NSString *text = source ? @(source) : nil;
  NSString *vertexName = vertex ? @(vertex) : nil;
  NSString *fragmentName = fragment ? @(fragment) : nil;
  if (!r || !text || !vertexName || !fragmentName) {
    if (message && length)
      snprintf(message, length, "%s",
               r ? "Invalid shader source or entry point"
                 : "No Metal device for runtime shaders");
    return NULL;
  }
  NSError *error = nil;
  id<MTLLibrary> library = [r.device newLibraryWithSource:text
                                                  options:nil
                                                    error:&error];
  MTLRenderPipelineDescriptor *d = [MTLRenderPipelineDescriptor new];
  d.vertexFunction = [library newFunctionWithName:vertexName];
  d.fragmentFunction = [library newFunctionWithName:fragmentName];
  d.colorAttachments[0].pixelFormat = MTLPixelFormatRGBA8Unorm;
  id<MTLRenderPipelineState> pipeline = nil;
  if (library && d.vertexFunction && d.fragmentFunction)
    pipeline = [r.device newRenderPipelineStateWithDescriptor:d error:&error];
  if (!pipeline && message && length)
    snprintf(message, length, "%s",
             error.localizedDescription.UTF8String
                 ?: "Missing shader entry point");
  if (pipeline)
    result = (__bridge_retained void *)pipeline;
  UNLOCK
  return result;
}
void qm_shader_apply(void *surface, void *rawPipeline, const uint8_t *uniforms,
                     size_t size, void *const *children, size_t count,
                     double x, double y, double w, double h) {
  LOCK QMetalSurface *s = surface_of(surface);
  id<MTLRenderPipelineState> pipeline =
      (__bridge id<MTLRenderPipelineState>)rawPipeline;
  if (!s || !pipeline || !uniforms || !size)
    return;
  QMetalRenderer *r = s.renderer;
  // A pipeline belongs to the device it was compiled on.
  if (pipeline.device.registryID != r.device.registryID)
    return;
  NSInteger left = MAX(0, floor(x)), top = MAX(0, floor(y));
  NSInteger right = MIN((NSInteger)s.texture.width, (NSInteger)ceil(x + w)),
            bottom = MIN((NSInteger)s.texture.height, (NSInteger)ceil(y + h));
  if (right <= left || bottom <= top)
    return;
  NSUInteger width = right - left, height = bottom - top;
  id<MTLTexture> input = texture(r, width, height),
                 output = texture(r, width, height);
  id<MTLBuffer> block =
      [r.device newBufferWithBytes:uniforms
                            length:size
                           options:MTLResourceStorageModeShared];
  if (!input || !output || !block)
    return;
  NSMutableArray *__attribute__((objc_precise_lifetime)) keeps =
      [NSMutableArray new];
  NSMutableArray<id<MTLTexture>> *inputs = [NSMutableArray new];
  for (NSUInteger i = 0; i < count; i++) {
    QMetalSurface *child = children ? surface_of(children[i]) : nil;
    QTexture *keep = nil;
    id<MTLTexture> t = child ? source_texture(r, child, 0, &keep) : nil;
    if (keep)
      [keeps addObject:keep];
    [inputs addObject:t ?: r.transparent];
  }
  id<MTLCommandBuffer> cb = batch_buffer(r);
  id<MTLBlitCommandEncoder> copy = [cb blitCommandEncoder];
  if (!copy)
    return;
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
  if (!e)
    return;
  [e setRenderPipelineState:pipeline];
  [e setFragmentBuffer:block offset:0 atIndex:0];
  vector_float4 area = {left - x, top - y, width, height};
  [e setFragmentBytes:&area length:sizeof(area) atIndex:1];
  [e setFragmentTexture:input atIndex:0];
  for (NSUInteger i = 0; i < inputs.count; i++)
    [e setFragmentTexture:inputs[i] atIndex:i + 1];
  [e drawPrimitives:MTLPrimitiveTypeTriangle vertexStart:0 vertexCount:3];
  [e endEncoding];
  copy = [cb blitCommandEncoder];
  if (!copy)
    return;
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
  finish_operation(r);
  (void)keeps;
  UNLOCK
}
