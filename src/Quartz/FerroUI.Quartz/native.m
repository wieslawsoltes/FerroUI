// All retained objects crossing this boundary are released by the Rust owner.
// Every function accepts NULL handles and returns NULL or an empty result for
// them; nothing here throws.
//
// With QUARTZ_METAL a drawing context is either a GPU surface (metal.m) or,
// when no usable Metal device exists or a GPU allocation fails, a Core
// Graphics bitmap context. Each entry point dispatches on the kind it gets.
#import <CoreGraphics/CoreGraphics.h>
#import <CoreText/CoreText.h>
#import <Foundation/Foundation.h>
#import <ImageIO/ImageIO.h>
#ifdef QUARTZ_METAL
#import "metal.h"
#endif
#include <math.h>
#include <stdlib.h>
#include <string.h>

enum { Q_OK = 0, Q_INVALID_ARGUMENT = 8 };

#ifdef QUARTZ_METAL
#define GPU(c) qm_is_surface(c)
#else
#define GPU(c) 0
#endif

typedef struct {
  int kind;
  double p[6];
} QCommand;
typedef struct {
  uint16_t glyph;
  int32_t cluster;
  double advance, x, y;
} QGlyph;
typedef void (*PathCallback)(void *, QCommand);
typedef void (*GlyphCallback)(void *, QGlyph);

int q_supported(void) {
  if (@available(macOS 13.0, *))
    return 1;
  return 0;
}
void q_release(void *p) {
  if (p)
    CFRelease(p);
}
void *q_retain(void *p) { return p ? (void *)CFRetain(p) : NULL; }
int q_is_gpu(void *c) { return c ? GPU(c) : 0; }
static CFStringRef string(const char *s) {
  return s ? CFStringCreateWithCString(NULL, s, kCFStringEncodingUTF8) : NULL;
}
static void utf8(CFStringRef s, char *out, size_t n) {
  if (n) {
    out[0] = 0;
    if (s)
      CFStringGetCString(s, out, n, kCFStringEncodingUTF8);
  }
}
static CFDataRef empty_data(void) { return CFDataCreate(NULL, NULL, 0); }
void *q_font(const char *name, int weight, int italic, int stretch) {
  @autoreleasepool {
    CTFontRef font = NULL;
    CFStringRef family = NULL;
    bool system = !name || !*name || !strcmp(name, "Default") ||
                  !strcmp(name, "$Default") ||
                  !strcmp(name, ".AppleSystemUIFont");
    if (!system) {
      family = string(name);
      system = family == NULL;
    }
    if (system) {
      // The hidden UI family cannot be matched as an ordinary family with
      // weight traits: Core Text silently substitutes Helvetica for bold.
      font = CTFontCreateUIFontForLanguage(
          weight >= 600 ? kCTFontUIFontEmphasizedSystem : kCTFontUIFontSystem,
          12, NULL);
      if (font && italic) {
        CTFontRef styled = CTFontCreateCopyWithSymbolicTraits(
            font, 12, NULL, kCTFontItalicTrait, kCTFontItalicTrait);
        if (styled) {
          CFRelease(font);
          font = styled;
        }
      }
    } else {
      double w = weight < 400 ? (weight - 400) / 500.0 : (weight - 400) / 650.0;
      NSDictionary *traits = @{
        (__bridge id)kCTFontWeightTrait : @(w),
        (__bridge id)kCTFontWidthTrait : @((stretch - 5) / 5.0),
        (__bridge id)kCTFontSymbolicTrait : @(italic ? kCTFontItalicTrait : 0)
      };
      NSDictionary *attrs = @{
        (__bridge id)kCTFontFamilyNameAttribute : (__bridge id)family,
        (__bridge id)kCTFontTraitsAttribute : traits
      };
      CTFontDescriptorRef desc =
          CTFontDescriptorCreateWithAttributes((__bridge CFDictionaryRef)attrs);
      if (desc) {
        font = CTFontCreateWithFontDescriptor(desc, 12, NULL);
        CFRelease(desc);
      }
      CFRelease(family);
    }
    return (void *)font;
  }
}
void *q_font_data(const uint8_t *bytes, size_t n) {
  if (!bytes || !n)
    return NULL;
  CFDataRef data = CFDataCreate(NULL, bytes, n);
  if (!data)
    return NULL;
  CGDataProviderRef provider = CGDataProviderCreateWithCFData(data);
  CGFontRef cg = provider ? CGFontCreateWithDataProvider(provider) : NULL;
  CTFontRef font = cg ? CTFontCreateWithGraphicsFont(cg, 12, NULL, NULL) : NULL;
  if (cg)
    CGFontRelease(cg);
  CGDataProviderRelease(provider);
  CFRelease(data);
  return (void *)font;
}
void q_font_name(void *font, char *out, size_t n) {
  CFStringRef s = font ? CTFontCopyFamilyName(font) : NULL;
  utf8(s, out, n);
  if (s)
    CFRelease(s);
}
void *q_font_table(void *font, uint32_t tag) {
  return font ? (void *)CTFontCopyTable(font, tag, kCTFontTableOptionNoOptions)
              : NULL;
}
size_t q_data_len(void *data) { return data ? CFDataGetLength(data) : 0; }
const uint8_t *q_data_bytes(void *data) {
  return data ? CFDataGetBytePtr(data) : NULL;
}
void *q_font_names(void) {
  @autoreleasepool {
    CFArrayRef names = CTFontManagerCopyAvailableFontFamilyNames();
    if (!names)
      return (void *)empty_data();
    NSString *joined =
        [(__bridge NSArray *)names componentsJoinedByString:@"\n"];
    NSData *data = [joined dataUsingEncoding:NSUTF8StringEncoding];
    CFDataRef result = CFDataCreate(NULL, data.bytes, data.length);
    CFRelease(names);
    return (void *)result;
  }
}
void *q_font_match(void *font, const uint16_t *text, size_t n) {
  if (!font || !text || !n)
    return NULL;
  CFStringRef s = CFStringCreateWithCharacters(NULL, text, n);
  if (!s)
    return NULL;
  CTFontRef matched = CTFontCreateForString(font, s, CFRangeMake(0, n));
  CFRelease(s);
  return (void *)matched;
}
void q_shape(void *font, const uint16_t *text, size_t n, size_t start,
             size_t length, const char *language, double size, int rtl,
             const uint32_t *tags, const int *values, size_t features,
             void *info, GlyphCallback emit) {
  if (!font || !emit || (n && !text) || start > n || length > n - start ||
      !length)
    return;
  @autoreleasepool {
    CTFontRef sized = CTFontCreateCopyWithAttributes(font, size, NULL, NULL);
    if (!sized)
      return;
    if (features && tags && values) {
      NSMutableArray *settings = [NSMutableArray array];
      for (size_t i = 0; i < features; i++)
        [settings addObject:@{
          (__bridge id)kCTFontOpenTypeFeatureTag : @(tags[i]),
          (__bridge id)kCTFontOpenTypeFeatureValue : @(values[i])
        }];
      NSDictionary *attrs =
          @{(__bridge id)kCTFontFeatureSettingsAttribute : settings};
      CTFontDescriptorRef desc =
          CTFontDescriptorCreateWithAttributes((__bridge CFDictionaryRef)attrs);
      CTFontRef configured =
          desc ? CTFontCreateCopyWithAttributes(sized, size, NULL, desc) : NULL;
      if (desc)
        CFRelease(desc);
      if (configured) {
        CFRelease(sized);
        sized = configured;
      }
    }
    CFStringRef s = CFStringCreateWithCharacters(NULL, text, n);
    if (!s) {
      CFRelease(sized);
      return;
    }
    // The formatter already resolves a single font and bidi level for this run.
    NSMutableDictionary *configured = [@{
      (__bridge id)kCTFontAttributeName : (__bridge id)sized,
      (__bridge id)kCTWritingDirectionAttributeName :
          @[ @(kCTWritingDirectionOverride |
               (rtl ? kCTWritingDirectionRightToLeft
                    : kCTWritingDirectionLeftToRight)) ],
      (__bridge id)kCTForegroundColorFromContextAttributeName : @YES
    } mutableCopy];
    NSString *locale = language && *language ? @(language) : nil;
    if (locale)
      configured[(__bridge id)kCTLanguageAttributeName] = locale;
    CFAttributedStringRef a = CFAttributedStringCreate(
        NULL, s, (__bridge CFDictionaryRef)configured);
    CTTypesetterRef typesetter =
        a ? CTTypesetterCreateWithAttributedString(a) : NULL;
    CTLineRef line =
        typesetter ? CTTypesetterCreateLine(typesetter, CFRangeMake(start, length))
                   : NULL;
    if (typesetter)
      CFRelease(typesetter);
    CFArrayRef runs = line ? CTLineGetGlyphRuns(line) : NULL;
    CFStringRef expectedName = CTFontCopyPostScriptName(sized);
    for (CFIndex r = 0; runs && r < CFArrayGetCount(runs); r++) {
      CTRunRef run = CFArrayGetValueAtIndex(runs, r);
      CFIndex count = CTRunGetGlyphCount(run);
      if (count <= 0)
        continue;
      CGGlyph *gs = calloc(count, sizeof(CGGlyph));
      CGSize *adv = calloc(count, sizeof(CGSize));
      CGPoint *pos = calloc(count, sizeof(CGPoint));
      CFIndex *ix = calloc(count, sizeof(CFIndex));
      if (gs && adv && pos && ix) {
        CTRunGetGlyphs(run, CFRangeMake(0, 0), gs);
        CTRunGetAdvances(run, CFRangeMake(0, 0), adv);
        CTRunGetPositions(run, CFRangeMake(0, 0), pos);
        CTRunGetStringIndices(run, CFRangeMake(0, 0), ix);
        CTFontRef actual = (CTFontRef)CFDictionaryGetValue(
            CTRunGetAttributes(run), kCTFontAttributeName);
        CFStringRef actualName =
            actual ? CTFontCopyPostScriptName(actual) : NULL;
        bool same = expectedName && actualName &&
                    CFEqual(expectedName, actualName);
        if (actualName)
          CFRelease(actualName);
        double pen = pos[0].x;
        for (CFIndex i = 0; i < count; i++) {
          QGlyph g = {same ? gs[i] : 0, (int32_t)(ix[i] - (CFIndex)start),
                      adv[i].width, pos[i].x - pen, -pos[i].y};
          emit(info, g);
          pen += adv[i].width;
        }
      }
      free(gs);
      free(adv);
      free(pos);
      free(ix);
    }
    if (expectedName)
      CFRelease(expectedName);
    if (line)
      CFRelease(line);
    if (a)
      CFRelease(a);
    CFRelease(s);
    CFRelease(sized);
  }
}
void *q_path(const QCommand *cmd, size_t n) {
  CGMutablePathRef path = CGPathCreateMutable();
  if (!path || !cmd)
    return path;
  for (size_t i = 0; i < n; i++) {
    const double *p = cmd[i].p;
    switch (cmd[i].kind) {
    case 0:
      CGPathMoveToPoint(path, NULL, p[0], p[1]);
      break;
    case 1:
      CGPathAddLineToPoint(path, NULL, p[0], p[1]);
      break;
    case 2:
      CGPathAddQuadCurveToPoint(path, NULL, p[0], p[1], p[2], p[3]);
      break;
    case 3:
      CGPathAddCurveToPoint(path, NULL, p[0], p[1], p[2], p[3], p[4], p[5]);
      break;
    case 4:
      CGPathCloseSubpath(path);
      break;
    }
  }
  return path;
}
typedef struct {
  void *info;
  PathCallback emit;
} PathSink;
static void path_element(void *info, const CGPathElement *e) {
  PathSink *s = info;
  QCommand c = {0, {0}};
  c.kind = e->type;
  int n = e->type == kCGPathElementAddCurveToPoint       ? 3
          : e->type == kCGPathElementAddQuadCurveToPoint ? 2
          : e->type == kCGPathElementCloseSubpath        ? 0
                                                         : 1;
  for (int i = 0; i < n; i++) {
    c.p[i * 2] = e->points[i].x;
    c.p[i * 2 + 1] = e->points[i].y;
  }
  s->emit(s->info, c);
}
void q_path_elements(void *path, void *info, PathCallback emit) {
  if (!path || !emit)
    return;
  PathSink sink = {info, emit};
  CGPathApply(path, &sink, path_element);
}
void *q_glyph_path(void *font, uint16_t glyph, double size) {
  CTFontRef f = font ? CTFontCreateCopyWithAttributes(font, size, NULL, NULL)
                     : NULL;
  if (!f)
    return NULL;
  CGAffineTransform flip = CGAffineTransformMakeScale(1, -1);
  CGPathRef p = CTFontCreatePathForGlyph(f, glyph, &flip);
  CFRelease(f);
  return (void *)p;
}
void *q_combine(void *a, void *b, int op, int ea, int eb) {
  if (!a || !b)
    return NULL;
  if (@available(macOS 13.0, *)) {
    CGPathRef na = CGPathCreateCopyByNormalizing(a, ea),
              nb = CGPathCreateCopyByNormalizing(b, eb), r = NULL;
    if (na && nb)
      switch (op) {
      case 0:
        r = CGPathCreateCopyByUnioningPath(na, nb, false);
        break;
      case 1:
        r = CGPathCreateCopyByIntersectingPath(na, nb, false);
        break;
      case 2:
        r = CGPathCreateCopyBySymmetricDifferenceOfPath(na, nb, false);
        break;
      case 3:
        r = CGPathCreateCopyBySubtractingPath(na, nb, false);
        break;
      }
    CGPathRelease(na);
    CGPathRelease(nb);
    return (void *)r;
  }
  return NULL;
}
// Never NULL: a degenerate pen or a failed dashing/stroking yields an empty
// path. A dash pattern with a non-finite or negative length, or no positive
// length at all, strokes solid.
void *q_stroke(void *path, double width, int cap, int join, double miter,
               const double *dash, size_t n, double phase) {
  if (!path || !(width > 0) || !isfinite(width))
    return CGPathCreateMutable();
  bool dashed_pattern = n && dash && isfinite(phase);
  double total = 0;
  for (size_t i = 0; dashed_pattern && i < n; i++) {
    if (!isfinite(dash[i]) || dash[i] < 0)
      dashed_pattern = false;
    else
      total += dash[i];
  }
  if (!(total > 0))
    dashed_pattern = false;
  CGPathRef dashed =
      dashed_pattern
          ? CGPathCreateCopyByDashingPath(path, NULL, phase, dash, n)
          : CGPathRetain(path);
  CGPathRef result =
      dashed ? CGPathCreateCopyByStrokingPath(dashed, NULL, width, cap, join,
                                              isfinite(miter) ? miter : 10)
             : NULL;
  CGPathRelease(dashed);
  return result ? (void *)result : (void *)CGPathCreateMutable();
}
static CGContextRef cpu_surface(size_t w, size_t h) {
  if (!w || !h)
    return NULL;
  CGColorSpaceRef cs = CGColorSpaceCreateWithName(kCGColorSpaceSRGB);
  CGContextRef ctx = CGBitmapContextCreate(NULL, w, h, 8, w * 4, cs,
                                           kCGImageAlphaPremultipliedFirst |
                                               kCGBitmapByteOrder32Little);
  CGColorSpaceRelease(cs);
  if (ctx) {
    CGContextTranslateCTM(ctx, 0, h);
    CGContextScaleCTM(ctx, 1, -1);
  }
  return ctx;
}
void *q_surface(size_t w, size_t h) {
#ifdef QUARTZ_METAL
  void *gpu = qm_surface(w, h);
  if (gpu)
    return gpu;
#endif
  return cpu_surface(w, h);
}
// Rows are tightly packed premultiplied BGRA on the Rust side.
int q_read_pixels(void *ctx, uint8_t *out, size_t length) {
  if (!ctx || !out)
    return Q_INVALID_ARGUMENT;
#ifdef QUARTZ_METAL
  if (GPU(ctx))
    return qm_read_pixels(ctx, out, length);
#endif
  const uint8_t *data = CGBitmapContextGetData(ctx);
  size_t w = CGBitmapContextGetWidth(ctx), h = CGBitmapContextGetHeight(ctx),
         stride = CGBitmapContextGetBytesPerRow(ctx);
  if (!data || length != w * h * 4 || stride < w * 4)
    return Q_INVALID_ARGUMENT;
  for (size_t y = 0; y < h; y++)
    memcpy(out + y * w * 4, data + y * stride, w * 4);
  return Q_OK;
}
int q_write_pixels(void *ctx, const uint8_t *bytes, size_t length) {
  if (!ctx || !bytes)
    return Q_INVALID_ARGUMENT;
#ifdef QUARTZ_METAL
  if (GPU(ctx))
    return qm_write_pixels(ctx, bytes, length);
#endif
  uint8_t *data = CGBitmapContextGetData(ctx);
  size_t w = CGBitmapContextGetWidth(ctx), h = CGBitmapContextGetHeight(ctx),
         stride = CGBitmapContextGetBytesPerRow(ctx);
  if (!data || length != w * h * 4 || stride < w * 4)
    return Q_INVALID_ARGUMENT;
  for (size_t y = 0; y < h; y++)
    memcpy(data + y * stride, bytes + y * w * 4, w * 4);
  return Q_OK;
}
void *q_snapshot(void *ctx) {
  if (!ctx)
    return NULL;
#ifdef QUARTZ_METAL
  if (GPU(ctx))
    return qm_snapshot(ctx);
#endif
  return CGBitmapContextCreateImage(ctx);
}
void q_save(void *c) {
  if (!c)
    return;
#ifdef QUARTZ_METAL
  if (GPU(c)) {
    qm_save(c);
    return;
  }
#endif
  CGContextSaveGState(c);
}
void q_restore(void *c) {
  if (!c)
    return;
#ifdef QUARTZ_METAL
  if (GPU(c)) {
    qm_restore(c);
    return;
  }
#endif
  CGContextRestoreGState(c);
}
void q_clear(void *c, double w, double h, double r, double g, double b,
             double a) {
  if (!c)
    return;
#ifdef QUARTZ_METAL
  if (GPU(c)) {
    qm_clear(c, r, g, b, a);
    return;
  }
#endif
  CGContextSaveGState(c);
  CGContextSetBlendMode(c, kCGBlendModeCopy);
  CGContextSetRGBFillColor(c, r, g, b, a);
  CGContextFillRect(c, CGRectMake(0, 0, w, h));
  CGContextRestoreGState(c);
}
void q_clip(void *c, void *p, int evenodd) {
  if (!c)
    return;
#ifdef QUARTZ_METAL
  if (GPU(c)) {
    qm_clip(c, p, evenodd);
    return;
  }
#endif
  if (!p || CGPathIsEmpty(p)) {
    CGContextClipToRect(c, CGRectZero);
    return;
  }
  CGContextAddPath(c, p);
  if (evenodd)
    CGContextEOClip(c);
  else
    CGContextClip(c);
}
void q_fill(void *c, void *p, int evenodd, double r, double g, double b,
            double a) {
  if (!c || !p)
    return;
#ifdef QUARTZ_METAL
  if (GPU(c)) {
    qm_fill(c, p, evenodd, r, g, b, a);
    return;
  }
#endif
  CGContextSetRGBFillColor(c, r, g, b, a);
  CGContextAddPath(c, p);
  CGContextDrawPath(c, evenodd ? kCGPathEOFill : kCGPathFill);
}
void q_gradient(void *c, const double *colors, const double *stops, size_t n,
                double x1, double y1, double x2, double y2, double radius,
                int radial) {
  if (!c || !colors || !stops || !n)
    return;
#ifdef QUARTZ_METAL
  if (GPU(c)) {
    qm_gradient(c, colors, stops, n, x1, y1, x2, y2, radius, radial);
    return;
  }
#endif
  if (radial == 2)
    return; // Conic gradients are drawn by the GPU path only.
  CGColorSpaceRef cs = CGColorSpaceCreateWithName(kCGColorSpaceSRGB);
  CGGradientRef gradient =
      CGGradientCreateWithColorComponents(cs, colors, stops, n);
  if (gradient) {
    if (radial)
      CGContextDrawRadialGradient(c, gradient, CGPointMake(x1, y1), 0,
                                  CGPointMake(x2, y2), radius, 3);
    else
      CGContextDrawLinearGradient(c, gradient, CGPointMake(x1, y1),
                                  CGPointMake(x2, y2), 3);
    CGGradientRelease(gradient);
  }
  CGColorSpaceRelease(cs);
}
void q_begin_layer(void *c, double opacity, int blend) {
  if (!c)
    return;
#ifdef QUARTZ_METAL
  if (GPU(c)) {
    qm_begin_layer(c, opacity, blend);
    return;
  }
#endif
  CGContextSaveGState(c);
  CGContextSetAlpha(c, opacity);
  // 28 is the GPU path's "keep the destination" mode; Core Graphics has no
  // such blend mode, so the layer composites normally there.
  CGContextSetBlendMode(c, blend >= 0 && blend <= 27 ? blend : 0);
  CGContextBeginTransparencyLayer(c, NULL);
}
void q_end_layer(void *c) {
  if (!c)
    return;
#ifdef QUARTZ_METAL
  if (GPU(c)) {
    qm_end_layer(c);
    return;
  }
#endif
  CGContextEndTransparencyLayer(c);
  CGContextRestoreGState(c);
}
static void cg_draw_image(CGContextRef c, CGImageRef image, double sx,
                          double sy, double sw, double sh, double dx,
                          double dy, double dw, double dh, double alpha) {
  if (!c || !image)
    return;
  CGImageRef crop =
      CGImageCreateWithImageInRect(image, CGRectMake(sx, sy, sw, sh));
  if (!crop)
    return;
  CGContextSaveGState(c);
  CGContextSetAlpha(c, alpha);
  CGContextTranslateCTM(c, dx, dy + dh);
  CGContextScaleCTM(c, 1, -1);
  CGContextDrawImage(c, CGRectMake(0, 0, dw, dh), crop);
  CGContextRestoreGState(c);
  CGImageRelease(crop);
}
void q_draw_image(void *c, void *image, double sx, double sy, double sw,
                  double sh, double dx, double dy, double dw, double dh,
                  double alpha) {
  if (!c || !image)
    return;
#ifdef QUARTZ_METAL
  if (GPU(c)) {
    qm_draw_image(c, image, sx, sy, sw, sh, dx, dy, dw, dh, alpha);
    return;
  }
#endif
  cg_draw_image(c, image, sx, sy, sw, sh, dx, dy, dw, dh, alpha);
}
// A +1 drawable source for `ctx`: the GPU surface itself, or a copy-on-write
// image of a Core Graphics context.
void *q_source(void *ctx) {
  if (!ctx)
    return NULL;
#ifdef QUARTZ_METAL
  if (GPU(ctx))
    return q_retain(ctx);
#endif
  return CGBitmapContextCreateImage(ctx);
}
// Draws a source of q_source onto either kind of context.
void q_draw_source(void *c, void *source, double sx, double sy, double sw,
                   double sh, double dx, double dy, double dw, double dh,
                   double alpha) {
  if (!c || !source)
    return;
#ifdef QUARTZ_METAL
  if (GPU(c)) {
    if (GPU(source))
      qm_draw_surface(c, source, sx, sy, sw, sh, dx, dy, dw, dh, alpha);
    else
      qm_draw_image(c, source, sx, sy, sw, sh, dx, dy, dw, dh, alpha);
    return;
  }
  if (GPU(source)) {
    CGImageRef image = qm_snapshot(source);
    cg_draw_image(c, image, sx, sy, sw, sh, dx, dy, dw, dh, alpha);
    if (image)
      CGImageRelease(image);
    return;
  }
#endif
  cg_draw_image(c, source, sx, sy, sw, sh, dx, dy, dw, dh, alpha);
}
void q_glyphs(void *c, void *font, double size, const uint16_t *glyphs,
              const double *positions, size_t n, double r, double g, double b,
              double a) {
  if (!c || !font || !n || !glyphs || !positions)
    return;
#ifdef QUARTZ_METAL
  if (GPU(c)) {
    qm_glyphs(c, font, size, glyphs, positions, n, r, g, b, a);
    return;
  }
#endif
  CTFontRef f = CTFontCreateCopyWithAttributes(font, size, NULL, NULL);
  CGPoint *ps = calloc(n, sizeof(CGPoint));
  if (f && ps) {
    for (size_t i = 0; i < n; i++)
      ps[i] = CGPointMake(positions[2 * i], -positions[2 * i + 1]);
    CGContextSaveGState(c);
    CGContextSetRGBFillColor(c, r, g, b, a);
    CGContextScaleCTM(c, 1, -1);
    CTFontDrawGlyphs(f, glyphs, ps, n, c);
    CGContextRestoreGState(c);
  }
  free(ps);
  if (f)
    CFRelease(f);
}
void *q_decode(const uint8_t *bytes, size_t n) {
  if (!bytes || !n)
    return NULL;
  CFDataRef data = CFDataCreate(NULL, bytes, n);
  if (!data)
    return NULL;
  CGImageSourceRef s = CGImageSourceCreateWithData(data, NULL);
  CGImageRef image = s ? CGImageSourceCreateImageAtIndex(s, 0, NULL) : NULL;
  if (s)
    CFRelease(s);
  CFRelease(data);
  return (void *)image;
}
size_t q_image_width(void *i) { return i ? CGImageGetWidth(i) : 0; }
size_t q_image_height(void *i) { return i ? CGImageGetHeight(i) : 0; }
void *q_encode(void *image, int jpeg, double quality) {
  if (!image)
    return NULL;
  @autoreleasepool {
    CFMutableDataRef data = CFDataCreateMutable(NULL, 0);
    if (!data)
      return NULL;
    CGImageDestinationRef d = CGImageDestinationCreateWithData(
        data, jpeg ? CFSTR("public.jpeg") : CFSTR("public.png"), 1, NULL);
    if (!d) {
      CFRelease(data);
      return NULL;
    }
    NSDictionary *props =
        @{(__bridge id)kCGImageDestinationLossyCompressionQuality : @(quality)};
    CGImageDestinationAddImage(d, image, (__bridge CFDictionaryRef)props);
    bool ok = CGImageDestinationFinalize(d);
    CFRelease(d);
    if (!ok) {
      CFRelease(data);
      return NULL;
    }
    return (void *)data;
  }
}
void q_set_transform(void *c, double a, double b, double d, double e, double x,
                     double y, double sx, double sy) {
  if (!c)
    return;
#ifdef QUARTZ_METAL
  if (GPU(c)) {
    qm_set_transform(c, a, b, d, e, x, y, sx, sy);
    return;
  }
#endif
  CGAffineTransform current = CGContextGetCTM(c);
  CGContextConcatCTM(c, CGAffineTransformInvert(current));
  CGContextTranslateCTM(c, 0, CGBitmapContextGetHeight(c));
  CGContextScaleCTM(c, sx, -sy);
  CGContextConcatCTM(c, CGAffineTransformMake(a, b, d, e, x, y));
}
void q_shadow(void *c, double x, double y, double blur, double r, double g,
              double b, double a) {
  if (!c)
    return;
#ifdef QUARTZ_METAL
  if (GPU(c)) {
    qm_shadow(c, x, y, blur, r, g, b, a);
    return;
  }
#endif
  CGColorSpaceRef cs = CGColorSpaceCreateWithName(kCGColorSpaceSRGB);
  double components[] = {r, g, b, a};
  CGColorRef color = cs ? CGColorCreate(cs, components) : NULL;
  CGContextSetShadowWithColor(c, CGSizeMake(x, -y), blur, color);
  if (color)
    CGColorRelease(color);
  CGColorSpaceRelease(cs);
}
void q_glyph_bounds(void *font, uint16_t glyph, double size, double *out) {
  if (!out)
    return;
  out[0] = out[1] = out[2] = out[3] = 0;
  CTFontRef f = font ? CTFontCreateCopyWithAttributes(font, size, NULL, NULL)
                     : NULL;
  if (!f)
    return;
  CGRect rect = CTFontGetBoundingRectsForGlyphs(f, kCTFontOrientationHorizontal,
                                                &glyph, NULL, 1);
  out[0] = rect.origin.x;
  out[1] = -rect.origin.y - rect.size.height;
  out[2] = rect.size.width;
  out[3] = rect.size.height;
  CFRelease(f);
}

// Public Core Text axis selection, with explicit rejection of unknown/out-of-range values.
void *q_font_variations(void *raw, const uint32_t *tags, const double *values, size_t count) {
  if (!raw || (count && (!tags || !values)))
    return NULL;
  @autoreleasepool {
    CTFontRef font=raw;
    CFArrayRef axes=CTFontCopyVariationAxes(font);
    if(!axes) return count ? NULL : (void *)CFRetain(font);
    NSMutableDictionary *settings=[NSMutableDictionary new];
    bool valid=true;
    for(size_t i=0;i<count;i++) {
      NSDictionary *axis=nil;
      for(NSDictionary *item in (__bridge NSArray *)axes)
        if([item[(__bridge id)kCTFontVariationAxisIdentifierKey] unsignedIntValue]==tags[i]) {axis=item;break;}
      if(!axis || !isfinite(values[i]) || values[i]<[axis[(__bridge id)kCTFontVariationAxisMinimumValueKey] doubleValue] ||
          values[i]>[axis[(__bridge id)kCTFontVariationAxisMaximumValueKey] doubleValue]) {valid=false;break;}
      settings[@(tags[i])]=@(values[i]);
    }
    CFRelease(axes);
    if(!valid) return NULL;
    CTFontDescriptorRef d=CTFontDescriptorCreateWithAttributes((__bridge CFDictionaryRef)@{(__bridge id)kCTFontVariationAttribute:settings});
    if(!d) return NULL;
    CTFontRef result=CTFontCreateCopyWithAttributes(font,CTFontGetSize(font),NULL,d);
    CFRelease(d);
    return (void *)result;
  }
}
void *q_font_family_faces(const char *family) {
  NSString *name = family ? @(family) : nil;
  if (!name)
    return (void *)empty_data();
  @autoreleasepool {
    NSDictionary *attrs=@{(__bridge id)kCTFontFamilyNameAttribute:name};
    CTFontDescriptorRef d=CTFontDescriptorCreateWithAttributes((__bridge CFDictionaryRef)attrs);
    if(!d) return (void *)empty_data();
    NSSet *mandatory=[NSSet setWithObject:(__bridge id)kCTFontFamilyNameAttribute];
    CFArrayRef faces=CTFontDescriptorCreateMatchingFontDescriptors(d,(__bridge CFSetRef)mandatory);
    NSMutableString *result=[NSMutableString new];
    for(id descriptor in (__bridge NSArray *)faces) {
      CTFontRef font=CTFontCreateWithFontDescriptor((__bridge CTFontDescriptorRef)descriptor,12,NULL);
      if(!font) continue;
      NSDictionary *traits=CFBridgingRelease(CTFontCopyTraits(font));
      double weight=[traits[(__bridge id)kCTFontWeightTrait] doubleValue];
      int w=lround(400+weight*(weight<0?500:650));
      int stretch=MAX(1,MIN(9,lround(5+[traits[(__bridge id)kCTFontWidthTrait] doubleValue]*5)));
      [result appendFormat:@"%d,%d,%d\n",MAX(1,MIN(1000,w)),(CTFontGetSymbolicTraits(font)&kCTFontItalicTrait)!=0,stretch];
      CFRelease(font);
    }
    if(faces) CFRelease(faces);
    CFRelease(d);
    NSData *data=[result dataUsingEncoding:NSUTF8StringEncoding];
    return (void *)CFDataCreate(NULL,data.bytes,data.length);
  }
}
// Never NULL: an empty path, or one whose strip intersection fails, has no
// intercepts.
void *q_path_intercepts(void *raw, double lower, double upper) {
  if (!raw || !(upper > lower))
    return (void *)empty_data();
  @autoreleasepool {
    CGPathRef path=raw;
    CGRect box=CGPathGetPathBoundingBox(path);
    if(CGRectIsNull(box) || CGRectIsEmpty(box)) return (void *)empty_data();
    CGPathRef strip=CGPathCreateWithRect(CGRectMake(box.origin.x-1,lower,box.size.width+2,upper-lower),NULL);
    CGPathRef clipped=strip ? CGPathCreateCopyByIntersectingPath(path,strip,false) : NULL;
    CFArrayRef components=clipped ? CGPathCreateSeparateComponents(clipped,false) : NULL;
    NSMutableData *data=[NSMutableData new];
    for(id item in (__bridge NSArray *)components) {
      CGRect bounds=CGPathGetPathBoundingBox((__bridge CGPathRef)item);
      if(!CGRectIsEmpty(bounds)) { float pair[2]={CGRectGetMinX(bounds),CGRectGetMaxX(bounds)};[data appendBytes:pair length:sizeof(pair)]; }
    }
    if(components) CFRelease(components);
    if(clipped) CGPathRelease(clipped);
    if(strip) CGPathRelease(strip);
    return (void *)CFDataCreate(NULL,data.bytes,data.length);
  }
}
