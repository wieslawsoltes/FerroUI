// All retained objects crossing this boundary are released by the Rust owner.
#import <CoreGraphics/CoreGraphics.h>
#import <CoreText/CoreText.h>
#import <Foundation/Foundation.h>
#import <ImageIO/ImageIO.h>
#ifdef QUARTZ_METAL
#import "metal.h"
#endif
#include <stdlib.h>
#include <string.h>

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
static CFStringRef string(const char *s) {
  return CFStringCreateWithCString(NULL, s, kCFStringEncodingUTF8);
}
static void utf8(CFStringRef s, char *out, size_t n) {
  if (n) {
    out[0] = 0;
    if (s)
      CFStringGetCString(s, out, n, kCFStringEncodingUTF8);
  }
}
void *q_font(const char *name, int weight, int italic, int stretch) {
  @autoreleasepool {
    CTFontRef font;
    if (!name || !*name || !strcmp(name, "Default") ||
        !strcmp(name, "$Default") || !strcmp(name, ".AppleSystemUIFont")) {
      // The hidden UI family cannot be matched as an ordinary family with
      // weight traits: Core Text silently substitutes Helvetica for bold.
      font = CTFontCreateUIFontForLanguage(
          weight >= 600 ? kCTFontUIFontEmphasizedSystem : kCTFontUIFontSystem,
          12, NULL);
      if (italic) {
        CTFontRef styled = CTFontCreateCopyWithSymbolicTraits(
            font, 12, NULL, kCTFontItalicTrait, kCTFontItalicTrait);
        if (styled) {
          CFRelease(font);
          font = styled;
        }
      }
    } else {
      CFStringRef family = string(name);
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
      font = CTFontCreateWithFontDescriptor(desc, 12, NULL);
      CFRelease(desc);
      CFRelease(family);
    }
    return (void *)font;
  }
}
void *q_font_data(const uint8_t *bytes, size_t n) {
  CFDataRef data = CFDataCreate(NULL, bytes, n);
  CGDataProviderRef provider = CGDataProviderCreateWithCFData(data);
  CGFontRef cg = CGFontCreateWithDataProvider(provider);
  CTFontRef font = cg ? CTFontCreateWithGraphicsFont(cg, 12, NULL, NULL) : NULL;
  if (cg)
    CGFontRelease(cg);
  CGDataProviderRelease(provider);
  CFRelease(data);
  return (void *)font;
}
void q_font_name(void *font, char *out, size_t n) {
  CFStringRef s = CTFontCopyFamilyName(font);
  utf8(s, out, n);
  CFRelease(s);
}
void *q_font_table(void *font, uint32_t tag) {
  return (void *)CTFontCopyTable(font, tag, kCTFontTableOptionNoOptions);
}
size_t q_data_len(void *data) { return CFDataGetLength(data); }
const uint8_t *q_data_bytes(void *data) { return CFDataGetBytePtr(data); }
void *q_font_names(void) {
  @autoreleasepool {
    CFArrayRef names = CTFontManagerCopyAvailableFontFamilyNames();
    NSString *joined =
        [(__bridge NSArray *)names componentsJoinedByString:@"\n"];
    NSData *data = [joined dataUsingEncoding:NSUTF8StringEncoding];
    CFDataRef result = CFDataCreate(NULL, data.bytes, data.length);
    CFRelease(names);
    return (void *)result;
  }
}
void *q_font_match(void *font, const uint16_t *text, size_t n) {
  CFStringRef s = CFStringCreateWithCharacters(NULL, text, n);
  CTFontRef matched = CTFontCreateForString(font, s, CFRangeMake(0, n));
  CFRelease(s);
  return (void *)matched;
}
void q_shape(void *font, const uint16_t *text, size_t n, size_t start, size_t length, const char *language, double size, int rtl,
             const uint32_t *tags, const int *values, size_t features,
             void *info, GlyphCallback emit) {
  @autoreleasepool {
    CTFontRef sized = CTFontCreateCopyWithAttributes(font, size, NULL, NULL);
    NSMutableArray *settings = [NSMutableArray array];
    for (size_t i = 0; i < features; i++)
      [settings addObject:@{
        (__bridge id)kCTFontOpenTypeFeatureTag : @(tags[i]),
        (__bridge id)kCTFontOpenTypeFeatureValue : @(values[i])
      }];
    if (features) {
      NSDictionary *attrs =
          @{(__bridge id)kCTFontFeatureSettingsAttribute : settings};
      CTFontDescriptorRef desc =
          CTFontDescriptorCreateWithAttributes((__bridge CFDictionaryRef)attrs);
      CTFontRef configured =
          CTFontCreateCopyWithAttributes(sized, size, NULL, desc);
      CFRelease(desc);
      CFRelease(sized);
      sized = configured;
    }
    CFStringRef s = CFStringCreateWithCharacters(NULL, text, n);
    // The formatter already resolves a single font and bidi level for this run.
    NSDictionary *attrs = @{
      (__bridge id)kCTFontAttributeName : (__bridge id)sized,
      (__bridge id)kCTWritingDirectionAttributeName :
          @[ @(kCTWritingDirectionOverride |
               (rtl ? kCTWritingDirectionRightToLeft
                    : kCTWritingDirectionLeftToRight)) ],
      (__bridge id)kCTForegroundColorFromContextAttributeName : @YES
    };
    NSMutableDictionary *configured=[attrs mutableCopy];
    if(language && *language) configured[(__bridge id)kCTLanguageAttributeName]=@(language);
    CFAttributedStringRef a = CFAttributedStringCreate(NULL, s, (__bridge CFDictionaryRef)configured);
    CTTypesetterRef typesetter=CTTypesetterCreateWithAttributedString(a);
    CTLineRef line=CTTypesetterCreateLine(typesetter,CFRangeMake(start,length));
    CFRelease(typesetter);
    CFArrayRef runs = CTLineGetGlyphRuns(line);
    for (CFIndex r = 0; r < CFArrayGetCount(runs); r++) {
      CTRunRef run = CFArrayGetValueAtIndex(runs, r);
      CFIndex count = CTRunGetGlyphCount(run);
      CGGlyph *gs = calloc(count, sizeof(CGGlyph));
      CGSize *adv = calloc(count, sizeof(CGSize));
      CGPoint *pos = calloc(count, sizeof(CGPoint));
      CFIndex *ix = calloc(count, sizeof(CFIndex));
      CTRunGetGlyphs(run, CFRangeMake(0, 0), gs);
      CTRunGetAdvances(run, CFRangeMake(0, 0), adv);
      CTRunGetPositions(run, CFRangeMake(0, 0), pos);
      CTRunGetStringIndices(run, CFRangeMake(0, 0), ix);
      CTFontRef actual = (CTFontRef)CFDictionaryGetValue(
          CTRunGetAttributes(run), kCTFontAttributeName);
      CFStringRef expectedName = CTFontCopyPostScriptName(sized),
                  actualName = CTFontCopyPostScriptName(actual);
      bool same = CFEqual(expectedName, actualName);
      CFRelease(expectedName);
      CFRelease(actualName);
      double pen = count ? pos[0].x : 0;
      for (CFIndex i = 0; i < count; i++) {
        QGlyph g = {same ? gs[i] : 0, (int32_t)(ix[i]-start), adv[i].width,
                    pos[i].x - pen, -pos[i].y};
        emit(info, g);
        pen += adv[i].width;
      }
      free(gs);
      free(adv);
      free(pos);
      free(ix);
    }
    CFRelease(line);
    CFRelease(a);
    CFRelease(s);
    CFRelease(sized);
  }
}
void *q_path(const QCommand *cmd, size_t n) {
  CGMutablePathRef path = CGPathCreateMutable();
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
  PathSink sink = {info, emit};
  CGPathApply(path, &sink, path_element);
}
void *q_glyph_path(void *font, uint16_t glyph, double size) {
  CTFontRef f = CTFontCreateCopyWithAttributes(font, size, NULL, NULL);
  CGAffineTransform flip = CGAffineTransformMakeScale(1, -1);
  CGPathRef p = CTFontCreatePathForGlyph(f, glyph, &flip);
  CFRelease(f);
  return (void *)p;
}
void *q_combine(void *a, void *b, int op, int ea, int eb) {
  if (@available(macOS 13.0, *)) {
    CGPathRef na = CGPathCreateCopyByNormalizing(a, ea),
              nb = CGPathCreateCopyByNormalizing(b, eb), r = NULL;
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
void *q_stroke(void *path, double width, int cap, int join, double miter,
               const double *dash, size_t n, double phase) {
  CGPathRef dashed =
      n ? CGPathCreateCopyByDashingPath(path, NULL, phase, dash, n)
        : CGPathRetain(path);
  CGPathRef result =
      CGPathCreateCopyByStrokingPath(dashed, NULL, width, cap, join, miter);
  CGPathRelease(dashed);
  return (void *)result;
}
void *q_surface(size_t w, size_t h) {
#ifdef QUARTZ_METAL
  return qm_surface(w, h);
#else

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

#endif
}
void q_read_pixels(void *ctx, uint8_t *out, size_t length) {
#ifdef QUARTZ_METAL
  qm_read_pixels(ctx, out, length);
#else
  memcpy(out, CGBitmapContextGetData(ctx), length);
#endif
}
void q_write_pixels(void *ctx, const uint8_t *data, size_t length) {
#ifdef QUARTZ_METAL
  qm_write_pixels(ctx, data, length);
#else
  memcpy(CGBitmapContextGetData(ctx), data, length);
#endif
}
void *q_snapshot(void *ctx) {
#ifdef QUARTZ_METAL
  return qm_snapshot(ctx);
#else
  return CGBitmapContextCreateImage(ctx);
#endif
}
void q_save(void *c) {
#ifdef QUARTZ_METAL
  qm_save(c);
#else
  CGContextSaveGState(c);
#endif
}
void q_restore(void *c) {
#ifdef QUARTZ_METAL
  qm_restore(c);
#else
  CGContextRestoreGState(c);
#endif
}
void q_transform(void *c, double a, double b, double d, double e, double x,
                 double y) {
  CGContextConcatCTM(c, CGAffineTransformMake(a, b, d, e, x, y));
}
void q_clear(void *c, double w, double h, double r, double g, double b,
             double a) {
#ifdef QUARTZ_METAL
  (void)w;
  (void)h;
  qm_clear(c, r, g, b, a);
#else

  CGContextSaveGState(c);
  CGContextSetBlendMode(c, kCGBlendModeCopy);
  CGContextSetRGBFillColor(c, r, g, b, a);
  CGContextFillRect(c, CGRectMake(0, 0, w, h));
  CGContextRestoreGState(c);

#endif
}
void q_clip(void *c, void *p, int evenodd) {
#ifdef QUARTZ_METAL
  qm_clip(c, p, evenodd);
#else

  CGContextAddPath(c, p);
  if (evenodd)
    CGContextEOClip(c);
  else
    CGContextClip(c);

#endif
}
void q_fill(void *c, void *p, int evenodd, double r, double g, double b,
            double a) {
#ifdef QUARTZ_METAL
  qm_fill(c, p, evenodd, r, g, b, a);
#else

  CGContextSetRGBFillColor(c, r, g, b, a);
  CGContextAddPath(c, p);
  CGContextDrawPath(c, evenodd ? kCGPathEOFill : kCGPathFill);

#endif
}
void q_gradient(void *c, const double *colors, const double *stops, size_t n,
                double x1, double y1, double x2, double y2, double radius,
                int radial) {
#ifdef QUARTZ_METAL
  qm_gradient(c, colors, stops, n, x1, y1, x2, y2, radius, radial);
#else

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

#endif
}
void q_begin_layer(void *c, double opacity, int blend) {
#ifdef QUARTZ_METAL
  qm_begin_layer(c, opacity, blend);
#else

  CGContextSaveGState(c);
  CGContextSetAlpha(c, opacity);
  CGContextSetBlendMode(c, blend);
  CGContextBeginTransparencyLayer(c, NULL);

#endif
}
void q_end_layer(void *c) {
#ifdef QUARTZ_METAL
  qm_end_layer(c);
#else

  CGContextEndTransparencyLayer(c);
  CGContextRestoreGState(c);

#endif
}
void q_draw_image(void *c, void *image, double sx, double sy, double sw,
                  double sh, double dx, double dy, double dw, double dh,
                  double alpha) {
#ifdef QUARTZ_METAL
  qm_draw_image(c, image, sx, sy, sw, sh, dx, dy, dw, dh, alpha);
#else

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

#endif
}
void q_glyphs(void *c, void *font, double size, const uint16_t *glyphs,
              const double *positions, size_t n, double r, double g, double b,
              double a) {
#ifdef QUARTZ_METAL
  qm_glyphs(c, font, size, glyphs, positions, n, r, g, b, a);
#else

  CTFontRef f = CTFontCreateCopyWithAttributes(font, size, NULL, NULL);
  CGPoint *ps = calloc(n, sizeof(CGPoint));
  for (size_t i = 0; i < n; i++)
    ps[i] = CGPointMake(positions[2 * i], -positions[2 * i + 1]);
  CGContextSaveGState(c);
  CGContextSetRGBFillColor(c, r, g, b, a);
  CGContextScaleCTM(c, 1, -1);
  CTFontDrawGlyphs(f, glyphs, ps, n, c);
  CGContextRestoreGState(c);
  free(ps);
  CFRelease(f);

#endif
}
void *q_decode(const uint8_t *bytes, size_t n) {
  CFDataRef data = CFDataCreate(NULL, bytes, n);
  CGImageSourceRef s = CGImageSourceCreateWithData(data, NULL);
  CGImageRef image = s ? CGImageSourceCreateImageAtIndex(s, 0, NULL) : NULL;
  if (s)
    CFRelease(s);
  CFRelease(data);
  return (void *)image;
}
size_t q_image_width(void *i) { return CGImageGetWidth(i); }
size_t q_image_height(void *i) { return CGImageGetHeight(i); }
void *q_encode(void *image, int jpeg, double quality) {
  @autoreleasepool {
    CFMutableDataRef data = CFDataCreateMutable(NULL, 0);
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
#ifdef QUARTZ_METAL
  qm_set_transform(c, a, b, d, e, x, y, sx, sy);
#else

  CGAffineTransform current = CGContextGetCTM(c);
  CGContextConcatCTM(c, CGAffineTransformInvert(current));
  CGContextTranslateCTM(c, 0, CGBitmapContextGetHeight(c));
  CGContextScaleCTM(c, sx, -sy);
  CGContextConcatCTM(c, CGAffineTransformMake(a, b, d, e, x, y));

#endif
}
void q_shadow(void *c, double x, double y, double blur, double r, double g,
              double b, double a) {
#ifdef QUARTZ_METAL
  qm_shadow(c, x, y, blur, r, g, b, a);
#else

  CGColorSpaceRef cs = CGColorSpaceCreateWithName(kCGColorSpaceSRGB);
  double components[] = {r, g, b, a};
  CGColorRef color = CGColorCreate(cs, components);
  CGContextSetShadowWithColor(c, CGSizeMake(x, -y), blur, color);
  CGColorRelease(color);
  CGColorSpaceRelease(cs);

#endif
}
void q_glyph_bounds(void *font, uint16_t glyph, double size, double *out) {
  CTFontRef f = CTFontCreateCopyWithAttributes(font, size, NULL, NULL);
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
    CTFontRef result=CTFontCreateCopyWithAttributes(font,CTFontGetSize(font),NULL,d);
    CFRelease(d);
    return (void *)result;
  }
}
void *q_font_family_faces(const char *family) {
  @autoreleasepool {
    NSDictionary *attrs=@{(__bridge id)kCTFontFamilyNameAttribute:@(family)};
    CTFontDescriptorRef d=CTFontDescriptorCreateWithAttributes((__bridge CFDictionaryRef)attrs);
    NSSet *mandatory=[NSSet setWithObject:(__bridge id)kCTFontFamilyNameAttribute];
    CFArrayRef faces=CTFontDescriptorCreateMatchingFontDescriptors(d,(__bridge CFSetRef)mandatory);
    NSMutableString *result=[NSMutableString new];
    for(id descriptor in (__bridge NSArray *)faces) {
      CTFontRef font=CTFontCreateWithFontDescriptor((__bridge CTFontDescriptorRef)descriptor,12,NULL);
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
void *q_path_intercepts(void *raw, double lower, double upper) {
  @autoreleasepool {
    CGPathRef path=raw;
    CGRect box=CGPathGetPathBoundingBox(path);
    CGPathRef strip=CGPathCreateWithRect(CGRectMake(box.origin.x-1,lower,box.size.width+2,upper-lower),NULL);
    CGPathRef clipped=CGPathCreateCopyByIntersectingPath(path,strip,false);
    CFArrayRef components=CGPathCreateSeparateComponents(clipped,false);
    NSMutableData *data=[NSMutableData new];
    for(id item in (__bridge NSArray *)components) {
      CGRect bounds=CGPathGetPathBoundingBox((__bridge CGPathRef)item);
      if(!CGRectIsEmpty(bounds)) { float pair[2]={CGRectGetMinX(bounds),CGRectGetMaxX(bounds)};[data appendBytes:pair length:sizeof(pair)]; }
    }
    if(components) CFRelease(components);
    if(clipped) CGPathRelease(clipped);
    CGPathRelease(strip);
    return (void *)CFDataCreate(NULL,data.bytes,data.length);
  }
}
