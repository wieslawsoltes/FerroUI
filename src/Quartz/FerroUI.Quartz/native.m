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
  @try {
    if (@available(macOS 13.0, iOS 16.0, *))
      return 1;
    return 0;

  } @catch (NSException *error) {
    NSLog(@"Quartz native operation failed: %@", error);
    return 0;
  }
}
void q_release(void *p) {
  @try {
    if (p)
      CFRelease(p);

  } @catch (NSException *error) {
    NSLog(@"Quartz native operation failed: %@", error);
    return;
  }
}
void *q_retain(void *p) {
  @try {
    return p ? (void *)CFRetain(p) : NULL;
  } @catch (NSException *error) {
    NSLog(@"Quartz native operation failed: %@", error);
    return NULL;
  }
}
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
// Public Core Text/AppKit normalized weight anchors for CSS/OpenType weights.
static const double q_weights[9] = {-0.8, -0.6, -0.4, 0,   0.23,
                                    0.3,  0.4,  0.56, 0.62};
static double q_native_weight(int weight) {
  double index = fmax(0, fmin(8, (weight - 100) / 100.0));
  int lower = floor(index), upper = MIN(8, lower + 1);
  return q_weights[lower] +
         (q_weights[upper] - q_weights[lower]) * (index - lower);
}
static int q_css_weight(double weight) {
  for (int i = 1; i < 9; i++)
    if (weight <= q_weights[i]) {
      double fraction =
          (weight - q_weights[i - 1]) / (q_weights[i] - q_weights[i - 1]);
      return MAX(100, lround(100 * i + 100 * fraction));
    }
  return 900;
}
static bool q_system_family(const char *name) {
  return !name || !*name || !strcmp(name, "Default") ||
         !strcmp(name, "$Default") || !strcmp(name, ".AppleSystemUIFont");
}
static CFDataRef empty_data(void) { return CFDataCreate(NULL, NULL, 0); }
void *q_font(const char *name, int weight, int italic, int stretch) {
  @try {
    @autoreleasepool {
      CTFontRef font;
      if (q_system_family(name)) {
        // Copy the UI descriptor rather than matching its hidden family as a
        // normal installed family, which can silently substitute Helvetica.
        CTFontRef system =
            CTFontCreateUIFontForLanguage(kCTFontUIFontSystem, 12, NULL);
        if (!system)
          return NULL;
        CTFontDescriptorRef base = CTFontCopyFontDescriptor(system);
        if (!base) {
          CFRelease(system);
          return NULL;
        }
        NSDictionary *attributes = @{
          (__bridge id)kCTFontTraitsAttribute : @{
            (__bridge id)kCTFontWeightTrait : @(q_native_weight(weight)),
            (__bridge id)kCTFontWidthTrait : @((stretch - 5) / 5.0)
          }
        };
        CTFontDescriptorRef descriptor =
            CTFontDescriptorCreateCopyWithAttributes(
                base, (__bridge CFDictionaryRef)attributes);
        font = descriptor ? CTFontCreateWithFontDescriptor(descriptor, 12, NULL)
                          : NULL;
        if (descriptor)
          CFRelease(descriptor);
        CFRelease(base);
        CFRelease(system);
        if (font && italic) {
          CTFontRef styled = CTFontCreateCopyWithSymbolicTraits(
              font, 12, NULL, kCTFontItalicTrait, kCTFontItalicTrait);
          if (styled) {
            CFRelease(font);
            font = styled;
          }
        }
      } else {
        CFStringRef family = string(name);
        if (!family)
          return NULL;
        double w = q_native_weight(weight);
        NSDictionary *traits = @{
          (__bridge id)kCTFontWeightTrait : @(w),
          (__bridge id)kCTFontWidthTrait : @((stretch - 5) / 5.0),
          (__bridge id)kCTFontSymbolicTrait : @(italic ? kCTFontItalicTrait : 0)
        };
        NSDictionary *attrs = @{
          (__bridge id)kCTFontFamilyNameAttribute : (__bridge id)family,
          (__bridge id)kCTFontTraitsAttribute : traits
        };
        CTFontDescriptorRef desc = CTFontDescriptorCreateWithAttributes(
            (__bridge CFDictionaryRef)attrs);
        font = desc ? CTFontCreateWithFontDescriptor(desc, 12, NULL) : NULL;
        if (desc)
          CFRelease(desc);
        CFRelease(family);
      }
      return (void *)font;
    }

  } @catch (NSException *error) {
    NSLog(@"Quartz native operation failed: %@", error);
    return NULL;
  }
}
void *q_font_data(const uint8_t *bytes, size_t n) {
  @try {
    if (!bytes || !n)
      return NULL;
    CFDataRef data = CFDataCreate(NULL, bytes, n);
    if (!data)
      return NULL;
    CGDataProviderRef provider = CGDataProviderCreateWithCFData(data);
    CGFontRef cg = provider ? CGFontCreateWithDataProvider(provider) : NULL;
    CTFontRef font =
        cg ? CTFontCreateWithGraphicsFont(cg, 12, NULL, NULL) : NULL;
    if (cg)
      CGFontRelease(cg);
    CGDataProviderRelease(provider);
    CFRelease(data);
    return (void *)font;

  } @catch (NSException *error) {
    NSLog(@"Quartz native operation failed: %@", error);
    return NULL;
  }
}
void q_font_name(void *font, char *out, size_t n) {
  @try {
    if (!font)
      return;
    CFStringRef s = font ? CTFontCopyFamilyName(font) : NULL;
    utf8(s, out, n);
    if (s)
      CFRelease(s);

  } @catch (NSException *error) {
    NSLog(@"Quartz native operation failed: %@", error);
    return;
  }
}
void *q_font_table(void *font, uint32_t tag) {
  @try {
    if (!font)
      return NULL;
    return font
               ? (void *)CTFontCopyTable(font, tag, kCTFontTableOptionNoOptions)
               : NULL;

  } @catch (NSException *error) {
    NSLog(@"Quartz native operation failed: %@", error);
    return NULL;
  }
}
size_t q_data_len(void *data) {
  @try {
    return data ? CFDataGetLength(data) : 0;
  } @catch (NSException *error) {
    NSLog(@"Quartz native operation failed: %@", error);
    return 0;
  }
}
const uint8_t *q_data_bytes(void *data) {
  @try {
    return data ? CFDataGetBytePtr(data) : NULL;

  } @catch (NSException *error) {
    NSLog(@"Quartz native operation failed: %@", error);
    return NULL;
  }
}
void *q_font_names(void) {
  @try {
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

  } @catch (NSException *error) {
    NSLog(@"Quartz native operation failed: %@", error);
    return NULL;
  }
}
uint32_t q_font_traits(void *font) {
  @try {
    if (!font)
      return 0;
    return CTFontGetSymbolicTraits(font);
  } @catch (NSException *error) {
    NSLog(@"Quartz native operation failed: %@", error);
    return 0;
  }
}
void q_font_info(void *font, int *info) {
  @try {
    if (!font)
      return;
    @autoreleasepool {
      NSDictionary *traits = CFBridgingRelease(CTFontCopyTraits(font));
      info[0] =
          q_css_weight([traits[(__bridge id)kCTFontWeightTrait] doubleValue]);
      info[1] = (CTFontGetSymbolicTraits(font) & kCTFontItalicTrait) != 0;
      info[2] = MAX(1, MIN(9, lround(5 + [traits[(__bridge id)kCTFontWidthTrait]
                                             doubleValue] *
                                             5)));
    }

  } @catch (NSException *error) {
    NSLog(@"Quartz native operation failed: %@", error);
    return;
  }
}
void *q_font_match(void *font, const uint16_t *text, size_t n,
                   const char *language) {
  @try {
    if (!font)
      return NULL;
    CFStringRef s = CFStringCreateWithCharacters(NULL, text, n);
    CFStringRef locale = language && *language ? string(language) : NULL;
    CTFontRef matched =
        CTFontCreateForStringWithLanguage(font, s, CFRangeMake(0, n), locale);
    if (locale)
      CFRelease(locale);
    CFRelease(s);
    return (void *)matched;

  } @catch (NSException *error) {
    NSLog(@"Quartz native operation failed: %@", error);
    return NULL;
  }
}
// Cache immutable font configuration, not text or shaping results. Each call
// still shapes its complete UTF-16 context through Core Text.
@interface QShapingFonts : NSObject
@property CFMutableDictionaryRef faces;
@property NSUInteger entries;
@end
@implementation QShapingFonts
- (instancetype)init {
  if ((self = [super init]))
    _faces = CFDictionaryCreateMutable(NULL, 0, &kCFTypeDictionaryKeyCallBacks,
                                       &kCFTypeDictionaryValueCallBacks);
  return self;
}
- (void)dealloc {
  if (_faces)
    CFRelease(_faces);
}
@end
static NSDictionary *shaping_attributes(CTFontRef font, double size, int rtl,
                                        const char *language) {
  NSMutableDictionary *thread = NSThread.currentThread.threadDictionary;
  QShapingFonts *cache = thread[@"FerroUI.Quartz.ShapingFonts"];
  if (!cache) {
    cache = [QShapingFonts new];
    thread[@"FerroUI.Quartz.ShapingFonts"] = cache;
  }
  if (cache.entries >= 128) {
    CFDictionaryRemoveAllValues(cache.faces);
    cache.entries = 0;
  }
  if (!cache.faces)
    return nil;
  NSMutableDictionary *face =
      (__bridge NSMutableDictionary *)CFDictionaryGetValue(cache.faces, font);
  NSMutableData *key = [NSMutableData dataWithBytes:&size length:sizeof(size)];
  [key appendBytes:&rtl length:sizeof(rtl)];
  if (language && *language)
    [key appendBytes:language length:strlen(language)];
  NSDictionary *attributes = face[key];
  if (!attributes) {
    CTFontRef sized = CTFontCreateCopyWithAttributes(font, size, NULL, NULL);
    if (!sized)
      return nil;
    NSMutableDictionary *configured = [@{
      (__bridge id)kCTFontAttributeName : (__bridge id)sized,
      (__bridge id)kCTWritingDirectionAttributeName :
          @[ @(kCTWritingDirectionOverride |
               (rtl ? kCTWritingDirectionRightToLeft
                    : kCTWritingDirectionLeftToRight)) ],
      (__bridge id)kCTForegroundColorFromContextAttributeName : @YES
    } mutableCopy];
    CFRelease(sized);
    if (language && *language)
      configured[(__bridge id)kCTLanguageAttributeName] = @(language);
    attributes = [configured copy];
    if (!face) {
      face = [NSMutableDictionary new];
      CFDictionarySetValue(cache.faces, font, (__bridge CFTypeRef)face);
    }
    face[key] = attributes;
    cache.entries++;
  }
  return attributes;
}
void q_shape(void *font, const uint16_t *text, size_t n, size_t start,
             size_t length, const char *language, double size, int rtl,
             const uint32_t *tags, const int *values, const uint32_t *starts,
             const uint32_t *ends, size_t features, void *info,
             GlyphCallback emit) {
  @try {
    if (!font)
      return;
    @autoreleasepool {
      if (!font || !text || !emit || start > n || length > n - start)
        return;
      NSDictionary *configured = shaping_attributes(font, size, rtl, language);
      if (!configured)
        return;
      CTFontRef sized =
          (__bridge CTFontRef)configured[(__bridge id)kCTFontAttributeName];
      CFStringRef s = CFStringCreateWithCharacters(NULL, text, n);
      if (!s)
        return;
      // The common case has uniform immutable attributes. Avoid constructing
      // a mutable string/attribute-run store when no feature ranges are edited.
      // The CFString owns its UTF-16 bytes; no borrowed Rust storage escapes.
      NSAttributedString *uniform =
          features ? nil
                   : CFBridgingRelease(CFAttributedStringCreate(
                         NULL, s, (__bridge CFDictionaryRef)configured));
      NSMutableAttributedString *a =
          features ? [[NSMutableAttributedString alloc]
                         initWithString:(__bridge NSString *)s
                             attributes:configured]
                   : nil;
      CFAttributedStringRef attributed =
          (__bridge CFAttributedStringRef)(features ? a : uniform);
      if (!attributed) {
        CFRelease(s);
        return;
      }
      // Feature ranges use UTF-16 indices in the containing text, just like
      // HarfBuzz clusters. Keep that context intact when configuring subranges.
      if (features) {
        NSMutableIndexSet *boundaries = [NSMutableIndexSet indexSetWithIndex:0];
        [boundaries addIndex:n];
        for (size_t i = 0; i < features; i++) {
          [boundaries addIndex:MIN((size_t)starts[i], n)];
          [boundaries addIndex:MIN((size_t)ends[i], n)];
        }
        for (NSUInteger lower = boundaries.firstIndex; lower < n;) {
          NSUInteger upper = [boundaries indexGreaterThanIndex:lower];
          NSMutableDictionary<NSString *, NSNumber *> *active =
              [NSMutableDictionary dictionary];
          for (size_t i = 0; i < features; i++) {
            if (starts[i] > lower || ends[i] <= lower)
              continue;
            char tag[5] = {(char)(tags[i] >> 24), (char)(tags[i] >> 16),
                           (char)(tags[i] >> 8), (char)tags[i], 0};
            // Core Text requires a four-character string, not the numeric tag.
            active[@(tag)] = @(values[i]);
          }
          if (active.count) {
            NSMutableArray *settings = [NSMutableArray array];
            for (NSString *tag in active)
              [settings addObject:@{
                (__bridge id)kCTFontOpenTypeFeatureTag : tag,
                (__bridge id)kCTFontOpenTypeFeatureValue : active[tag]
              }];
            NSDictionary *fontAttrs =
                @{(__bridge id)kCTFontFeatureSettingsAttribute : settings};
            CTFontDescriptorRef desc = CTFontDescriptorCreateWithAttributes(
                (__bridge CFDictionaryRef)fontAttrs);
            CTFontRef ranged =
                CTFontCreateCopyWithAttributes(sized, size, NULL, desc);
            if (ranged)
              [a addAttribute:(__bridge id)kCTFontAttributeName
                        value:(__bridge id)ranged
                        range:NSMakeRange(lower, upper - lower)];
            if (ranged)
              CFRelease(ranged);
            if (desc)
              CFRelease(desc);
          }
          lower = upper;
        }
      }
      CTTypesetterRef typesetter =
          (start != 0 || length != n)
              ? CTTypesetterCreateWithAttributedString(attributed)
              : NULL;
      CTLineRef line =
          (start == 0 && length == n)
              ? CTLineCreateWithAttributedString(attributed)
          : typesetter
              ? CTTypesetterCreateLine(typesetter, CFRangeMake(start, length))
              : NULL;
      if (typesetter)
        CFRelease(typesetter);
      CFArrayRef runs = line ? CTLineGetGlyphRuns(line) : NULL;
      for (CFIndex r = 0; runs && r < CFArrayGetCount(runs); r++) {
        CTRunRef run = CFArrayGetValueAtIndex(runs, r);
        CFIndex count = CTRunGetGlyphCount(run);
        const CGGlyph *gs = CTRunGetGlyphsPtr(run);
        const CGSize *adv = CTRunGetAdvancesPtr(run);
        const CGPoint *pos = CTRunGetPositionsPtr(run);
        const CFIndex *ix = CTRunGetStringIndicesPtr(run);
        CGGlyph *ownedGS = gs ? NULL : calloc(count, sizeof(*gs));
        CGSize *ownedAdv = adv ? NULL : calloc(count, sizeof(*adv));
        CGPoint *ownedPos = pos ? NULL : calloc(count, sizeof(*pos));
        CFIndex *ownedIX = ix ? NULL : calloc(count, sizeof(*ix));
        if (count && ((!gs && !ownedGS) || (!adv && !ownedAdv) ||
                      (!pos && !ownedPos) || (!ix && !ownedIX))) {
          free(ownedGS);
          free(ownedAdv);
          free(ownedPos);
          free(ownedIX);
          continue;
        }
        if (!gs) {
          CTRunGetGlyphs(run, CFRangeMake(0, 0), ownedGS);
          gs = ownedGS;
        }
        if (!adv) {
          CTRunGetAdvances(run, CFRangeMake(0, 0), ownedAdv);
          adv = ownedAdv;
        }
        if (!pos) {
          CTRunGetPositions(run, CFRangeMake(0, 0), ownedPos);
          pos = ownedPos;
        }
        if (!ix) {
          CTRunGetStringIndices(run, CFRangeMake(0, 0), ownedIX);
          ix = ownedIX;
        }
        CTFontRef actual = (CTFontRef)CFDictionaryGetValue(
            CTRunGetAttributes(run), kCTFontAttributeName);
        bool same = CFEqual(actual, sized);
        if (!same) {
          CFStringRef expectedName = CTFontCopyPostScriptName(sized),
                      actualName = CTFontCopyPostScriptName(actual);
          same =
              expectedName && actualName && CFEqual(expectedName, actualName);
          if (expectedName)
            CFRelease(expectedName);
          if (actualName)
            CFRelease(actualName);
        }
        double pen = count ? pos[0].x : 0;
        for (CFIndex i = 0; i < count; i++) {
          QGlyph g = {same ? gs[i] : 0, (int32_t)(ix[i] - start), adv[i].width,
                      pos[i].x - pen, -pos[i].y};
          emit(info, g);
          pen += adv[i].width;
        }
        free(ownedGS);
        free(ownedAdv);
        free(ownedPos);
        free(ownedIX);
      }
      if (line)
        CFRelease(line);
      CFRelease(s);
    }

  } @catch (NSException *error) {
    NSLog(@"Quartz native operation failed: %@", error);
    return;
  }
}
void *q_path(const QCommand *cmd, size_t n) {
  @try {
    CGMutablePathRef path = CGPathCreateMutable();
    if (!path || (n && !cmd))
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

  } @catch (NSException *error) {
    NSLog(@"Quartz native operation failed: %@", error);
    return NULL;
  }
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
  @try {
    if (!path || !emit)
      return;
    PathSink sink = {info, emit};
    CGPathApply(path, &sink, path_element);

  } @catch (NSException *error) {
    NSLog(@"Quartz native operation failed: %@", error);
    return;
  }
}
void *q_glyph_path(void *font, uint16_t glyph, double size) {
  @try {
    if (!font)
      return NULL;
    CTFontRef f = CTFontCreateCopyWithAttributes(font, size, NULL, NULL);
    if (!f)
      return NULL;
    CGAffineTransform flip = CGAffineTransformMakeScale(1, -1);
    CGPathRef p = CTFontCreatePathForGlyph(f, glyph, &flip);
    CFRelease(f);
    return (void *)p;

  } @catch (NSException *error) {
    NSLog(@"Quartz native operation failed: %@", error);
    return NULL;
  }
}
void *q_combine(void *a, void *b, int op, int ea, int eb) {
  @try {
    if (!a || !b)
      return NULL;
    if (@available(macOS 13.0, iOS 16.0, *)) {
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

  } @catch (NSException *error) {
    NSLog(@"Quartz native operation failed: %@", error);
    return NULL;
  }
}
void *q_stroke(void *path, double width, int cap, int join, double miter,
               const double *dash, size_t n, double phase) {
  @try {
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

  } @catch (NSException *error) {
    NSLog(@"Quartz native operation failed: %@", error);
    return NULL;
  }
}
void *q_surface(size_t w, size_t h) {
  @try {
#ifdef QUARTZ_METAL
    return qm_surface(w, h);
#else
    if (!w || !h || w > 16384 || h > 16384)
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

#endif

  } @catch (NSException *error) {
    NSLog(@"Quartz native operation failed: %@", error);
    return NULL;
  }
}
void q_read_pixels(void *ctx, uint8_t *out, size_t length) {
  @try {
    if (!ctx)
      return;
#ifdef QUARTZ_METAL
    qm_read_pixels(ctx, out, length);
#else
    memcpy(out, CGBitmapContextGetData(ctx), length);
#endif

  } @catch (NSException *error) {
    NSLog(@"Quartz native operation failed: %@", error);
    return;
  }
}
void q_write_pixels(void *ctx, const uint8_t *data, size_t length) {
  @try {
    if (!ctx)
      return;
#ifdef QUARTZ_METAL
    qm_write_pixels(ctx, data, length);
#else
    memcpy(CGBitmapContextGetData(ctx), data, length);
#endif

  } @catch (NSException *error) {
    NSLog(@"Quartz native operation failed: %@", error);
    return;
  }
}
void *q_snapshot(void *ctx) {
  @try {
    if (!ctx)
      return NULL;
#ifdef QUARTZ_METAL
    return qm_snapshot(ctx);
#else
    return CGBitmapContextCreateImage(ctx);
#endif

  } @catch (NSException *error) {
    NSLog(@"Quartz native operation failed: %@", error);
    return NULL;
  }
}
void q_save(void *c) {
  @try {
    if (!c)
      return;
#ifdef QUARTZ_METAL
    qm_save(c);
#else
    CGContextSaveGState(c);
#endif

  } @catch (NSException *error) {
    NSLog(@"Quartz native operation failed: %@", error);
    return;
  }
}
void q_restore(void *c) {
  @try {
    if (!c)
      return;
#ifdef QUARTZ_METAL
    qm_restore(c);
#else
    CGContextRestoreGState(c);
#endif

  } @catch (NSException *error) {
    NSLog(@"Quartz native operation failed: %@", error);
    return;
  }
}
void q_transform(void *c, double a, double b, double d, double e, double x,
                 double y) {
  @try {
    if (!c)
      return;
    CGContextConcatCTM(c, CGAffineTransformMake(a, b, d, e, x, y));

  } @catch (NSException *error) {
    NSLog(@"Quartz native operation failed: %@", error);
    return;
  }
}
void q_clear(void *c, double w, double h, double r, double g, double b,
             double a) {
  @try {
    if (!c)
      return;
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

  } @catch (NSException *error) {
    NSLog(@"Quartz native operation failed: %@", error);
    return;
  }
}
void q_clip(void *c, void *p, int evenodd) {
  @try {
    if (!c)
      return;
#ifdef QUARTZ_METAL
    qm_clip(c, p, evenodd);
#else

    CGContextAddPath(c, p);
    if (evenodd)
      CGContextEOClip(c);
    else
      CGContextClip(c);

#endif

  } @catch (NSException *error) {
    NSLog(@"Quartz native operation failed: %@", error);
    return;
  }
}
void q_fill(void *c, void *p, int evenodd, double r, double g, double b,
            double a) {
  @try {
    if (!c)
      return;
#ifdef QUARTZ_METAL
    qm_fill(c, p, evenodd, r, g, b, a);
#else

    CGContextSetRGBFillColor(c, r, g, b, a);
    CGContextAddPath(c, p);
    CGContextDrawPath(c, evenodd ? kCGPathEOFill : kCGPathFill);

#endif

  } @catch (NSException *error) {
    NSLog(@"Quartz native operation failed: %@", error);
    return;
  }
}
void q_gradient(void *c, const double *colors, const double *stops, size_t n,
                double x1, double y1, double x2, double y2, double radius,
                int radial) {
  @try {
    if (!c)
      return;
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

  } @catch (NSException *error) {
    NSLog(@"Quartz native operation failed: %@", error);
    return;
  }
}
void q_begin_layer(void *c, double opacity, int blend) {
  @try {
    if (!c)
      return;
#ifdef QUARTZ_METAL
    qm_begin_layer(c, opacity, blend);
#else

    CGContextSaveGState(c);
    CGContextSetAlpha(c, opacity);
    CGContextSetBlendMode(c, blend);
    CGContextBeginTransparencyLayer(c, NULL);

#endif

  } @catch (NSException *error) {
    NSLog(@"Quartz native operation failed: %@", error);
    return;
  }
}
void q_end_layer(void *c) {
  @try {
    if (!c)
      return;
#ifdef QUARTZ_METAL
    qm_end_layer(c);
#else

    CGContextEndTransparencyLayer(c);
    CGContextRestoreGState(c);

#endif

  } @catch (NSException *error) {
    NSLog(@"Quartz native operation failed: %@", error);
    return;
  }
}
void q_draw_image(void *c, void *image, double sx, double sy, double sw,
                  double sh, double dx, double dy, double dw, double dh,
                  double alpha) {
  @try {
    if (!c)
      return;
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

  } @catch (NSException *error) {
    NSLog(@"Quartz native operation failed: %@", error);
    return;
  }
}
void q_glyphs(void *c, void *font, double size, const uint16_t *glyphs,
              const double *positions, size_t n, double r, double g, double b,
              double a) {
  @try {
    if (!c || !font)
      return;
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

  } @catch (NSException *error) {
    NSLog(@"Quartz native operation failed: %@", error);
    return;
  }
}
void *q_decode(const uint8_t *bytes, size_t n) {
  @try {
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

  } @catch (NSException *error) {
    NSLog(@"Quartz native operation failed: %@", error);
    return NULL;
  }
}
size_t q_image_width(void *i) {
  @try {
    return i ? CGImageGetWidth(i) : 0;
  } @catch (NSException *error) {
    NSLog(@"Quartz native operation failed: %@", error);
    return 0;
  }
}
size_t q_image_height(void *i) {
  @try {
    return i ? CGImageGetHeight(i) : 0;
  } @catch (NSException *error) {
    NSLog(@"Quartz native operation failed: %@", error);
    return 0;
  }
}
void *q_encode(void *image, int jpeg, double quality) {
  @try {
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
      NSDictionary *props = @{
        (__bridge id)kCGImageDestinationLossyCompressionQuality : @(quality)
      };
      CGImageDestinationAddImage(d, image, (__bridge CFDictionaryRef)props);
      bool ok = CGImageDestinationFinalize(d);
      CFRelease(d);
      if (!ok) {
        CFRelease(data);
        return NULL;
      }
      return (void *)data;
    }

  } @catch (NSException *error) {
    NSLog(@"Quartz native operation failed: %@", error);
    return NULL;
  }
}
void q_set_transform(void *c, double a, double b, double d, double e, double x,
                     double y, double sx, double sy) {
  @try {
    if (!c)
      return;
#ifdef QUARTZ_METAL
    qm_set_transform(c, a, b, d, e, x, y, sx, sy);
#else

    CGAffineTransform current = CGContextGetCTM(c);
    CGContextConcatCTM(c, CGAffineTransformInvert(current));
    CGContextTranslateCTM(c, 0, CGBitmapContextGetHeight(c));
    CGContextScaleCTM(c, sx, -sy);
    CGContextConcatCTM(c, CGAffineTransformMake(a, b, d, e, x, y));

#endif

  } @catch (NSException *error) {
    NSLog(@"Quartz native operation failed: %@", error);
    return;
  }
}
void q_shadow(void *c, double x, double y, double blur, double r, double g,
              double b, double a) {
  @try {
    if (!c)
      return;
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

  } @catch (NSException *error) {
    NSLog(@"Quartz native operation failed: %@", error);
    return;
  }
}
void q_glyph_bounds(void *font, uint16_t glyph, double size, double *out) {
  @try {
    if (!font)
      return;
    CTFontRef f = CTFontCreateCopyWithAttributes(font, size, NULL, NULL);
    CGRect rect = CTFontGetBoundingRectsForGlyphs(
        f, kCTFontOrientationHorizontal, &glyph, NULL, 1);
    out[0] = rect.origin.x;
    out[1] = -rect.origin.y - rect.size.height;
    out[2] = rect.size.width;
    out[3] = rect.size.height;
    CFRelease(f);

  } @catch (NSException *error) {
    NSLog(@"Quartz native operation failed: %@", error);
    return;
  }
}

// Public Core Text axis selection, with explicit rejection of
// unknown/out-of-range values.
void *q_font_variations(void *raw, const uint32_t *tags, const double *values,
                        size_t count) {
  @try {
    if (!raw || (count && (!tags || !values)))
      return NULL;
    @autoreleasepool {
      CTFontRef font = raw;
      CFArrayRef axes = CTFontCopyVariationAxes(font);
      if (!axes)
        return count ? NULL : (void *)CFRetain(font);
      NSMutableDictionary *settings = [NSMutableDictionary new];
      bool valid = true;
      for (size_t i = 0; i < count; i++) {
        NSDictionary *axis = nil;
        for (NSDictionary *item in (__bridge NSArray *)axes)
          if ([item[(__bridge id)kCTFontVariationAxisIdentifierKey]
                  unsignedIntValue] == tags[i]) {
            axis = item;
            break;
          }
        if (!axis || !isfinite(values[i]) ||
            values[i] < [axis[(__bridge id)kCTFontVariationAxisMinimumValueKey]
                            doubleValue] ||
            values[i] > [axis[(__bridge id)kCTFontVariationAxisMaximumValueKey]
                            doubleValue]) {
          valid = false;
          break;
        }
        settings[@(tags[i])] = @(values[i]);
      }
      CFRelease(axes);
      if (!valid)
        return NULL;
      CTFontDescriptorRef d = CTFontDescriptorCreateWithAttributes(
          (__bridge CFDictionaryRef)
              @{(__bridge id)kCTFontVariationAttribute : settings});
      if (!d)
        return NULL;
      CTFontRef result =
          CTFontCreateCopyWithAttributes(font, CTFontGetSize(font), NULL, d);
      CFRelease(d);
      return (void *)result;
    }

  } @catch (NSException *error) {
    NSLog(@"Quartz native operation failed: %@", error);
    return NULL;
  }
}
void *q_font_family_faces(const char *family) {
  @try {
    @autoreleasepool {
      if (q_system_family(family)) {
        NSMutableString *styles = [NSMutableString new];
        for (int weight = 100; weight <= 900; weight += 100)
          for (int italic = 0; italic < 2; italic++)
            [styles appendFormat:@"%d,%d,5\n", weight, italic];
        NSData *data = [styles dataUsingEncoding:NSUTF8StringEncoding];
        return (void *)CFDataCreate(NULL, data.bytes, data.length);
      }
      NSDictionary *attrs =
          @{(__bridge id)kCTFontFamilyNameAttribute : @(family)};
      CTFontDescriptorRef d =
          CTFontDescriptorCreateWithAttributes((__bridge CFDictionaryRef)attrs);
      NSSet *mandatory =
          [NSSet setWithObject:(__bridge id)kCTFontFamilyNameAttribute];
      CFArrayRef faces = CTFontDescriptorCreateMatchingFontDescriptors(
          d, (__bridge CFSetRef)mandatory);
      NSMutableString *result = [NSMutableString new];
      for (id descriptor in (__bridge NSArray *)faces) {
        CTFontRef font = CTFontCreateWithFontDescriptor(
            (__bridge CTFontDescriptorRef)descriptor, 12, NULL);
        if (!font)
          continue;
        NSDictionary *traits = CFBridgingRelease(CTFontCopyTraits(font));
        double weight = [traits[(__bridge id)kCTFontWeightTrait] doubleValue];
        int w = q_css_weight(weight);
        int stretch =
            MAX(1, MIN(9, lround(5 + [traits[(__bridge id)kCTFontWidthTrait]
                                         doubleValue] *
                                         5)));
        [result appendFormat:@"%d,%d,%d\n", MAX(1, MIN(1000, w)),
                             (CTFontGetSymbolicTraits(font) &
                              kCTFontItalicTrait) != 0,
                             stretch];
        CFRelease(font);
      }
      if (faces)
        CFRelease(faces);
      CFRelease(d);
      NSData *data = [result dataUsingEncoding:NSUTF8StringEncoding];
      return (void *)CFDataCreate(NULL, data.bytes, data.length);
    }

  } @catch (NSException *error) {
    NSLog(@"Quartz native operation failed: %@", error);
    return NULL;
  }
}
void *q_path_intercepts(void *raw, double lower, double upper) {
  @try {
    if (!raw || !(upper > lower))
      return (void *)empty_data();
    @autoreleasepool {
      CGPathRef path = raw;
      CGRect box = CGPathGetPathBoundingBox(path);
      if (CGRectIsNull(box) || CGRectIsEmpty(box))
        return (void *)empty_data();
      CGPathRef strip =
          CGPathCreateWithRect(CGRectMake(box.origin.x - 1, lower,
                                          box.size.width + 2, upper - lower),
                               NULL);
      CGPathRef clipped =
          strip ? CGPathCreateCopyByIntersectingPath(path, strip, false) : NULL;
      CFArrayRef components =
          clipped ? CGPathCreateSeparateComponents(clipped, false) : NULL;
      NSMutableData *data = [NSMutableData new];
      for (id item in (__bridge NSArray *)components) {
        CGRect bounds = CGPathGetPathBoundingBox((__bridge CGPathRef)item);
        if (!CGRectIsEmpty(bounds)) {
          float pair[2] = {CGRectGetMinX(bounds), CGRectGetMaxX(bounds)};
          [data appendBytes:pair length:sizeof(pair)];
        }
      }
      if (components)
        CFRelease(components);
      if (clipped)
        CGPathRelease(clipped);
      if (strip)
        CGPathRelease(strip);
      return (void *)CFDataCreate(NULL, data.bytes, data.length);
    }

  } @catch (NSException *error) {
    NSLog(@"Quartz native operation failed: %@", error);
    return NULL;
  }
}
