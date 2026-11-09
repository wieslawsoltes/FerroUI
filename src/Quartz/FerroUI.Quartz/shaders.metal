#include <metal_stdlib>
using namespace metal;
// Keep this layout identical to QParams in metal.m (all float4 fields are
// 16-byte aligned).
struct Params {
  uint2 origin, extent;
  uint edges, evenodd, mode, blend;
  float4 color, gradient, inverse, translation, src, dst, clip, shadow, target;
  uint stops, clipped, flipped, reserved;
  uint edgeRows, edgeOriginY, edgeStride, edgePad;
  float4 rclip, rradius, sclip, rmap;
};

// The coverage of a rectangle clip (`clipped & 4`): the area of the pixel
// inside the rectangle.
static float clip_rectangle(float2 c, float4 rect) {
  float2 inside = saturate(min(c + 0.5f, rect.zw) - max(c - 0.5f, rect.xy));
  return inside.x * inside.y;
}

// The coverage of the analytic clip (`clipped & 2`) at a pixel center: a
// rectangle with elliptical corners of radii `rad.xy` (none when zero),
// antialiased over a pixel. A turned clip (`map` not zero) is in its own space:
// the center is mapped into it through the inverse transform `map` and the
// translation `rad.zw`, and its distances scaled back to pixels.
static float clip_rounded(float2 c, float4 rect, float4 rad, float4 map) {
  float scale = 1;
  if (any(map != 0)) {
    c = float2(map.x * c.x + map.z * c.y, map.y * c.x + map.w * c.y) + rad.zw;
    scale = rsqrt(abs(map.x * map.w - map.y * map.z));
  }
  float2 mid = (rect.xy + rect.zw) * 0.5f;
  float2 halfSize = (rect.zw - rect.xy) * 0.5f;
  float2 d = abs(c - mid) - halfSize;
  float coverage = saturate(0.5f - max(d.x, d.y) * scale);
  if (rad.x > 0 && rad.y > 0) {
    float2 k = d + rad.xy;
    if (k.x > 0 && k.y > 0) {
      float distance = (length(k / rad.xy) - 1.0f) * min(rad.x, rad.y);
      coverage = min(coverage, saturate(0.5f - distance * scale));
    }
  }
  return coverage;
}

// Whether the sample `q` is inside the box `box` (low and high corners, the
// low edge included, the high one excluded) with elliptical corners of radii
// `rad` (none when either is zero).
static bool inside_box(float2 q, float4 box, float2 rad) {
  if (any(q < box.xy) || any(q >= box.zw))
    return false;
  if (rad.x > 0 && rad.y > 0) {
    float2 k = abs(q - (box.xy + box.zw) * 0.5f) -
               ((box.zw - box.xy) * 0.5f - rad);
    if (k.x > 0 && k.y > 0)
      return dot(k / rad, k / rad) <= 1;
  }
  return true;
}

// The coverage of a solid box (`reserved & 8`, rounded with `reserved & 16`)
// at a pixel: the four samples the path coverage takes, tested against the box
// itself instead of its edges, or the pixel center when aliased. A box under a
// transform that turns it (`reserved & 32`) is in its own space: the samples
// are mapped into it through the inverse transform (`inverse`, `translation`).
static float box_coverage(float2 center, float4 box, float2 rad, bool aliased,
                          bool mapped, float4 inverse, float2 translation) {
  float value = 0;
  uint count = aliased ? 1 : 4;
  for (uint i = 0; i < count; i++) {
    float2 q = aliased ? center
                       : center + float2(float(i & 1) * 0.5f - 0.25f,
                                         float(i >> 1) * 0.5f - 0.25f);
    if (mapped)
      q = float2(inverse.x * q.x + inverse.z * q.y,
                 inverse.y * q.x + inverse.w * q.y) +
          translation;
    value += inside_box(q, box, rad);
  }
  return value / float(count);
}
struct Stop {
  float4 color;
  float4 location;
};
float coverage_at(float2 point, device const float4 *edges, uint count,
                  bool evenodd) {
  int winding = 0;
  for (uint i = 0; i < count; i++) {
    float2 a = edges[i].xy, b = edges[i].zw;
    if ((a.y <= point.y && b.y > point.y) ||
        (b.y <= point.y && a.y > point.y)) {
      float x = a.x + (point.y - a.y) * (b.x - a.x) / (b.y - a.y);
      if (x > point.x)
        winding += b.y > a.y ? 1 : -1;
    }
  }
  return evenodd ? ((abs(winding) & 1) != 0) : winding != 0;
}
float coverage(float2 point, device const float4 *edges, uint count,
               bool evenodd) {
  if (!count)
    return 1;
  float value = 0;
  for (uint y = 0; y < 2; y++)
    for (uint x = 0; x < 2; x++)
      value += coverage_at(
          point + float2(float(x) * 0.5f + 0.25f, float(y) * 0.5f + 0.25f),
          edges, count, evenodd);
  return value * 0.25f;
}
float lum(float3 c) { return dot(c, float3(0.3f, 0.59f, 0.11f)); }
float3 clip_color(float3 c) {
  float l = lum(c), lo = min(c.r, min(c.g, c.b)), hi = max(c.r, max(c.g, c.b));
  if (lo < 0)
    c = l + (c - l) * l / (l - lo);
  if (hi > 1)
    c = l + (c - l) * (1 - l) / (hi - l);
  return c;
}
float3 set_lum(float3 c, float l) { return clip_color(c + (l - lum(c))); }
float saturation(float3 c) {
  return max(c.r, max(c.g, c.b)) - min(c.r, min(c.g, c.b));
}
float3 set_sat(float3 c, float s) {
  float lo = min(c.r, min(c.g, c.b)), hi = max(c.r, max(c.g, c.b));
  return hi > lo ? (c - lo) * s / (hi - lo) : float3(0);
}
float3 mix_color(float3 d, float3 s, uint mode) {
  switch (mode) {
  case 1:
    return d * s;
  case 2:
    return d + s - d * s;
  case 3:
    return select(2 * d * s, 1 - 2 * (1 - d) * (1 - s), d > 0.5f);
  case 4:
    return min(d, s);
  case 5:
    return max(d, s);
  case 6:
    return min(float3(1), d / max(1 - s, float3(1e-6f)));
  case 7:
    return 1 - min(float3(1), (1 - d) / max(s, float3(1e-6f)));
  case 8: {
    float3 g = select(((16 * d - 12) * d + 4) * d, sqrt(d), d > 0.25f);
    return select(d - (1 - 2 * s) * d * (1 - d), d + (2 * s - 1) * (g - d),
                  s > 0.5f);
  }
  case 9:
    return select(2 * d * s, 1 - 2 * (1 - d) * (1 - s), s > 0.5f);
  case 10:
    return abs(d - s);
  case 11:
    return d + s - 2 * d * s;
  case 12:
    return set_lum(set_sat(s, saturation(d)), lum(d));
  case 13:
    return set_lum(set_sat(d, saturation(s)), lum(d));
  case 14:
    return set_lum(s, lum(d));
  case 15:
    return set_lum(d, lum(s));
  default:
    return s;
  }
}
float4 composite(float4 d, float4 s, uint mode) {
  if (mode == 28)
    return d;
  if (mode == 17)
    return s;
  if (mode == 18)
    return s * d.a;
  if (mode == 19)
    return s * (1 - d.a);
  if (mode == 20)
    return s * d.a + d * (1 - s.a);
  if (mode == 21)
    return d + s * (1 - d.a);
  if (mode == 22)
    return d * s.a;
  if (mode == 23)
    return d * (1 - s.a);
  if (mode == 24)
    return d * s.a + s * (1 - d.a);
  if (mode == 25)
    return s * (1 - d.a) + d * (1 - s.a);
  if (mode == 27)
    return min(float4(1), s + d);
  float3 dc = d.a > 0 ? d.rgb / d.a : float3(0),
         sc = s.a > 0 ? s.rgb / s.a : float3(0);
  return float4((1 - s.a) * d.rgb + (1 - d.a) * s.rgb +
                    s.a * d.a * mix_color(dc, sc, mode),
                s.a + d.a * (1 - s.a));
}
float gradient_spread(float t, uint mode) {
  if (mode == 2)
    return fract(t);
  if (mode == 1) {
    float cycle = t - floor(t / 2) * 2;
    return 1 - abs(cycle - 1);
  }
  return clamp(t, 0.0f, 1.0f);
}
float4 gradient_color(float t, device const Stop *stops, uint n) {
  if (!n)
    return float4(0);
  if (t <= stops[0].location.x)
    return stops[0].color;
  for (uint i = 1; i < n; i++)
    if (t <= stops[i].location.x) {
      float delta = stops[i].location.x - stops[i - 1].location.x;
      float f = delta > 0 ? (t - stops[i - 1].location.x) / delta : 1;
      return mix(stops[i - 1].color, stops[i].color, clamp(f, 0.0f, 1.0f));
    }
  return stops[n - 1].color;
}
// Mitchell-Netravali reconstruction (B=C=1/3) for high-quality magnification.
float mitchell(float x) {
  x = abs(x);
  if (x < 1)
    return ((7 * x - 12) * x * x + 16.0f / 3) / 6;
  if (x < 2)
    return (((-7.0f / 3 * x + 12) * x - 20) * x + 32.0f / 3) / 6;
  return 0;
}
float4 cubic_sample_rect(texture2d<float> image, float2 uv, float4 rect) {
  constexpr sampler nearest(coord::pixel, address::clamp_to_edge,
                            filter::nearest);
  float2 p = uv * float2(image.get_width(), image.get_height()) - 0.5f;
  float2 base = floor(p), f = p - base;
  float4 sum = 0;
  float total = 0;
  for (int y = -1; y <= 2; y++)
    for (int x = -1; x <= 2; x++) {
      float w = mitchell(float(x) - f.x) * mitchell(float(y) - f.y);
      sum += image.sample(nearest,
                          clamp(base + float2(x, y) + 0.5f, rect.xy + 0.5f,
                                rect.xy + rect.zw - 0.5f),
                          level(0)) *
             w;
      total += w;
    }
  float4 color = clamp(sum / max(total, 1e-8f), 0.0f, 1.0f);
  // Negative reconstruction lobes must not create non-premultiplied fringes.
  color.rgb = min(color.rgb, float3(color.a));
  return color;
}
float4 cubic_sample(texture2d<float> image, float2 uv) {
  return cubic_sample_rect(image, uv,
                           float4(0, 0, image.get_width(), image.get_height()));
}
#ifdef QUARTZ_SEPARATE_DESTINATION
kernel void paint(texture2d<float, access::write> target [[texture(0)]],
#else
kernel void paint(texture2d<float, access::read_write> target [[texture(0)]],
#endif
                  texture2d<float, access::sample> clip [[texture(1)]],
                  texture2d<float, access::sample> image [[texture(2)]],
#ifdef QUARTZ_SEPARATE_DESTINATION
                  texture2d<float, access::read> previous [[texture(3)]],
#endif
                  device const float4 *edges [[buffer(0)]],
                  constant Params &p [[buffer(1)]],
                  device const Stop *stops [[buffer(2)]],
                  uint2 tid [[thread_position_in_grid]]) {
  if (any(tid >= p.extent))
    return;
  uint2 position = p.origin + tid;
  float2 pixel = float2(position), center = pixel + 0.5f;
  float mask = 1;
  if (p.clipped & 1) {
    float2 relative = center - p.clip.xy;
    mask = all(relative >= 0) && all(relative < p.clip.zw)
               ? clip.read(uint2(relative)).r
               : 0;
  }
  if (p.clipped & 2)
    mask *= clip_rounded(center, p.rclip, p.rradius, p.rmap);
  if (p.clipped & 4)
    mask *= clip_rectangle(center, p.sclip);
  device const float4 *pathEdges = edges;
  uint count = p.edges;
  if (p.edgeRows) {
    uint row = min((position.y - p.edgeOriginY) / p.edgeStride, p.edgeRows - 1);
    uint4 bin = as_type<uint4>(edges[row]);
    pathEdges = edges + p.edgeRows + bin.x;
    count = bin.y;
  }
  float cov = (p.edges && !count)
                  ? 0
                  : ((p.flipped & 1)
                         ? (count ? coverage_at(center, pathEdges, count,
                                                p.evenodd != 0)
                                  : 1)
                         : coverage(pixel, pathEdges, count, p.evenodd != 0));
  if (p.mode == 0 && (p.reserved & 8))
    cov = box_coverage(center, p.gradient,
                       (p.reserved & 16) ? p.src.xy : float2(0),
                       p.flipped & 1, p.reserved & 32, p.inverse,
                       p.translation.xy);
  cov *= mask;
  if (p.mode == 1) {
    target.write(float4(cov), tid);
    return;
  }
  uint2 output = uint2(float2(position) - p.target.xy);
  if (cov <= 0 && p.mode != 6)
    return;
  float2 local = float2(p.inverse.x * center.x + p.inverse.z * center.y,
                        p.inverse.y * center.x + p.inverse.w * center.y) +
                 p.translation.xy;
  float4 color = p.color;
  if (p.mode == 2) {
    float2 v = p.gradient.zw - p.gradient.xy;
    float t = dot(local - p.gradient.xy, v) / max(dot(v, v), 1e-8f);
    color = gradient_color(gradient_spread(t, p.reserved), stops, p.stops);
  }
  if (p.mode == 3) {
    float radius = p.translation.z;
    float2 q = local - p.gradient.xy, v = p.gradient.zw - p.gradient.xy;
    q.y *= p.shadow.x;
    v.y *= p.shadow.x;
    float A = dot(v, v) - radius * radius, B = -2 * dot(q, v), C = dot(q, q),
          D = B * B - 4 * A * C;
    float t = radius <= 0 ? 1 : 0;
    if (radius > 0 && abs(A) > 1e-8f && D >= 0) {
      float root = sqrt(D);
      t = max((-B - root) / (2 * A), (-B + root) / (2 * A));
    } else if (radius > 0 && abs(B) > 1e-8f)
      t = -C / B;
    color = gradient_color(gradient_spread(t, p.reserved), stops, p.stops);
  }
  if (p.mode == 7) {
    float2 q = local - p.gradient.xy;
    float t = fract(atan2(q.x, -q.y) / (2 * M_PI_F) - p.shadow.y / 360.0f + 1);
    color = gradient_color(gradient_spread(t, p.reserved), stops, p.stops);
  }
  if (p.mode == 4 || p.mode == 8) {
    float2 uv = (local - p.dst.xy) / p.dst.zw;
    if (p.mode == 8 && p.reserved != 0) {
      uv.x = (p.reserved == 1 || p.reserved == 3) ? gradient_spread(uv.x, 1)
                                                  : fract(uv.x);
      uv.y = (p.reserved == 2 || p.reserved == 3) ? gradient_spread(uv.y, 1)
                                                  : fract(uv.y);
    } else if (p.mode == 4 && p.edges != 0) {
      // Quad coverage includes samples beyond the pixel center. Preserve that
      // fractional coverage and extend the boundary texel into the fringe.
      uv = clamp(uv, float2(0), float2(1));
    } else if (any(uv < 0) || any(uv > 1))
      return;
    float2 source = p.src.xy + uv * p.src.zw;
    if (p.mode == 4 && (p.reserved & 4))
      source = clamp(source, p.src.xy + 0.5f, p.src.xy + p.src.zw - 0.5f);
    constexpr sampler linear(coord::normalized, address::clamp_to_edge,
                             filter::linear);
    constexpr sampler nearest(coord::normalized, address::clamp_to_edge,
                              filter::nearest);
    float2 sample =
        p.mode == 8 ? uv
                    : source / float2(image.get_width(), image.get_height());
    uint interpolation = p.flipped >> 2;
    float2 ratio =
        p.mode == 8 ? float2(image.get_width(), image.get_height()) / p.dst.zw
                    : p.src.zw / p.dst.zw;
    float footprint = max(length(float2(p.inverse.x, p.inverse.y) * ratio),
                          length(float2(p.inverse.z, p.inverse.w) * ratio));
    constexpr sampler mip(coord::normalized, address::clamp_to_edge,
                          filter::linear, mip_filter::linear);
    if (interpolation >= 3 && footprint > 1)
      color = image.sample(mip, sample, level(log2(footprint)));
    else if (interpolation == 4)
      color = (p.mode == 4 && (p.reserved & 4))
                  ? cubic_sample_rect(
                        image,
                        (p.src.xy + uv * p.src.zw) /
                            float2(image.get_width(), image.get_height()),
                        p.src)
                  : cubic_sample(image, sample);
    else
      color = (p.flipped & 2) ? image.sample(nearest, sample, level(0))
                              : image.sample(linear, sample, level(0));
    if (p.mode == 4 && (p.reserved & 3) == 2)
      color = color.rrrr;
    if (p.mode == 4 && (p.reserved & 3))
      color *= float4(p.color.rgb * p.color.a, p.color.a);
    color *= p.translation.w; // sampled images are already premultiplied
  } else {
    if (p.mode == 6) {
      float2 point = center - p.shadow.xy;
      float inside = coverage_at(point, edges, p.edges, p.evenodd != 0),
            dist = 1e20f;
      for (uint i = 0; i < p.edges; i++) {
        float2 a = edges[i].xy, b = edges[i].zw, v = b - a;
        float t = clamp(dot(point - a, v) / max(dot(v, v), 1e-8f), 0.0f, 1.0f);
        dist = min(dist, length(point - (a + t * v)));
      }
      float sigma = max(p.shadow.z * 0.5f, 0.5f);
      cov =
          mask * (inside > 0 ? 1 : exp(-0.5f * dist * dist / (sigma * sigma)));
    }
    color.rgb *= color.a;
  }
  if (p.mode == 9) {
    float4 tint = float4(p.gradient.rgb * p.gradient.a, p.gradient.a);
    color = tint + color * (1 - tint.a);
    constexpr sampler repeat(coord::normalized, address::repeat,
                             filter::linear);
    float4 noise = image.sample(
        repeat, local / float2(image.get_width(), image.get_height()));
    float alpha = floor(noise.a * 255.0f * 0.0225f) / 255.0f;
    float3 rgb = noise.a > 0 ? noise.rgb / noise.a : float3(0);
    color = float4(rgb * alpha, alpha) + color * (1 - alpha);
  }
  if (p.mode == 5 && p.blend == 17 && !p.edges && !p.clipped) {
    target.write(color, output);
    return;
  }
#ifdef QUARTZ_SEPARATE_DESTINATION
  float4 destination = previous.read(output);
#else
  float4 destination = target.read(output);
#endif
  target.write(mix(destination, composite(destination, color, p.blend), cov),
               output);
}
struct Vertex {
  float4 position [[position]];
  float2 uv;
};
vertex Vertex present_vertex(uint id [[vertex_id]]) {
  float2 uv = float2((id << 1) & 2, id & 2);
  return {float4(uv.x * 2 - 1, 1 - uv.y * 2, 0, 1), uv};
}
fragment float4 present_fragment(Vertex v [[stage_in]],
                                 texture2d<float> image [[texture(0)]],
                                 constant uint &flipped [[buffer(0)]]) {
  constexpr sampler s(coord::normalized, address::clamp_to_edge,
                      filter::nearest);
  return image.sample(s, float2(v.uv.x, flipped ? 1 - v.uv.y : v.uv.y));
}

// CoreVideo frame conversion: GPU-only BGRA/RGBA sampling, orientation and
// BT.1886 reference-display Rec.709 to the canvas's sRGB encoding.
kernel void convert_frame(texture2d<float, access::write> target [[texture(0)]],
                          texture2d<float, access::read> source [[texture(1)]],
                          constant uint4 &mode [[buffer(0)]],
                          uint2 pixel [[thread_position_in_grid]]) {
  if (pixel.x >= target.get_width() || pixel.y >= target.get_height())
    return;
  uint2 sample = pixel;
  if (mode.y)
    sample.y = source.get_height() - 1 - pixel.y;
  float4 color = source.read(sample);
  if (mode.z)
    color.a = 1;
  if (mode.x) {
    float3 linear = pow(max(color.rgb, float3(0)), float3(2.4f));
    color.rgb = select(1.055f * pow(linear, float3(1.0f / 2.4f)) - 0.055f,
                       12.92f * linear, linear < 0.0031308f);
  }
  target.write(color, pixel);
}

kernel void gaussian_pass(texture2d<float, access::write> output [[texture(0)]],
                          texture2d<float, access::sample> source
                          [[texture(1)]],
                          constant float4 &axis [[buffer(0)]],
                          uint2 p [[thread_position_in_grid]]) {
  if (p.x >= output.get_width() || p.y >= output.get_height())
    return;
  constexpr sampler s(coord::pixel, address::clamp_to_zero, filter::linear);
  float4 sum = 0;
  float total = 0;
  int radius = int(ceil(axis.z * 3));
  for (int i = -radius; i <= radius; i++) {
    float weight = exp(-0.5f * float(i * i) / (axis.z * axis.z));
    sum += source.sample(s, float2(p) + 0.5f + axis.xy * float(i)) * weight;
    total += weight;
  }
  output.write(sum / max(total, 1e-8f), p);
}
float3 quartz_light_at(float2 pixel, device const float4 *v) {
  float3 point = float3(pixel, 1);
  float2 local = float2(dot(v[4].xyz, point), dot(v[5].xyz, point));
  float3 result = v[0].rgb;
  float4 material = v[1];
  for (uint i = 0; i < min(uint(v[2].z), 8u); i++) {
    device const float4 *l = v + 6 + i * 10;
    float3 q = float3(local, 1);
    float w = dot(l[9].xyz, q);
    if (abs(w) < 1e-12f)
      continue;
    float2 p = float2(dot(l[7].xyz, q), dot(l[8].xyz, q)) / w;
    float3 toLight = float3(l[3].xy - p, l[3].z);
    float distance = length(toLight);
    if (l[6].y > 0 && distance > l[6].x)
      continue;
    float d = max(max(distance, l[5].w), 0.0f);
    float denominator = l[5].x + l[5].y * d + l[5].z * d * d;
    float attenuation = denominator > 0 ? 1 / denominator : 1;
    float incidence = distance > 0 && l[3].z > 0 ? l[3].z / distance : 0;
    float cone = 1;
    float3 color = l[1].rgb;
    if (l[0].x > 1.5f) {
      float3 direction = l[4].xyz, toPoint = -toLight;
      float a = length(direction) * distance;
      if (a <= 0)
        continue;
      float c = dot(toPoint, direction) / a;
      if (c >= l[0].y)
        cone = 1;
      else if (c <= l[0].z || l[0].y <= l[0].z)
        continue;
      else {
        float t = clamp((c - l[0].z) / (l[0].y - l[0].z), 0.0f, 1.0f);
        cone = t * t * (3 - 2 * t);
      }
      color = mix(l[2].rgb, l[1].rgb, cone);
    }
    float specular = 0;
    if (distance > 0 && toLight.z > 0) {
      float3 h = toLight / distance + float3(0, 0, 1);
      float len = length(h);
      if (len > 0)
        specular = pow(max(h.z / len, 0.0f), max(material.w, 1.0f));
    }
    result += color * cone * attenuation *
              (incidence * material.y + specular * material.z);
  }
  return result;
}
float quartz_burn(float b, float s) {
  return b >= 1 ? 1 : (s <= 0 ? 0 : 1 - min((1 - b) / s, 1.0f));
}
float quartz_dodge(float b, float s) {
  return b <= 0 ? 0 : (s >= 1 ? 1 : min(b / (1 - s), 1.0f));
}
float quartz_blend_channel(uint mode, float b, float s) {
  switch (mode) {
  case 6:
    return max(b + s - 1, 0.0f);
  case 14:
    return s <= 0.5f ? quartz_burn(b, 2 * s) : quartz_dodge(b, 2 * (s - 0.5f));
  case 15:
    return clamp(b + 2 * s - 1, 0.0f, 1.0f);
  case 16:
    return s <= 0.5f ? min(b, 2 * s) : max(b, 2 * s - 1);
  case 17:
    return b + s >= 1 ? 1 : 0;
  case 24:
    return max(b - s, 0.0f);
  case 25:
    return s <= 0 ? (b <= 0 ? 0 : 1) : min(b / s, 1.0f);
  default:
    return s;
  }
}
float4 quartz_pixel_operation(float4 d, float4 s, float2 pixel,
                              device const float4 *v) {
  uint kind = uint(v[0].x);
  if (kind == 1) {
    float4 c =
        clamp(v[1].x * d * s + v[1].y * d + v[1].z * s + v[1].w, 0.0f, 1.0f);
    c.rgb = min(c.rgb, float3(c.a));
    return c;
  }
  if (kind == 2)
    return float4(s.a * (float3(d.a) - d.rgb) + (1 - s.a) * d.rgb, d.a);
  uint mode = uint(v[0].y);
  float3 cb = d.a > 0 ? clamp(d.rgb / d.a, 0.0f, 1.0f) : float3(0),
         cs = s.a > 0 ? clamp(s.rgb / s.a, 0.0f, 1.0f) : float3(0);
  if (mode == 4) {
    float3 p = float3(pixel, 1);
    float2 point = float2(dot(v[4].xyz, p), dot(v[5].xyz, p));
    float2 cell = clamp(floor((point - v[2].xy) * v[3].xy / v[2].zw), float2(0),
                        v[3].xy - 1);
    uint hash = (uint(cell.y) * uint(v[3].x) + uint(cell.x)) ^ 0x9E3779B9u;
    hash *= 0x85EBCA6Bu;
    hash ^= hash >> 13;
    hash *= 0xC2B2AE35u;
    hash ^= hash >> 16;
    return s.a > 0 && float(hash & 0xFFFFu) / 65535.0f < s.a ? float4(cs, 1)
                                                             : d;
  }
  float3 blended;
  if (mode == 7)
    blended = lum(cs) < lum(cb) ? cs : cb;
  else if (mode == 8)
    blended = lum(cs) > lum(cb) ? cs : cb;
  else if (mode == 10)
    blended = min(cb + cs, float3(1));
  else if (mode == 6 || mode == 14 || mode == 15 || mode == 16 || mode == 17 ||
           mode == 24 || mode == 25)
    blended = float3(quartz_blend_channel(mode, cb.r, cs.r),
                     quartz_blend_channel(mode, cb.g, cs.g),
                     quartz_blend_channel(mode, cb.b, cs.b));
  else {
    uint cg = 0;
    switch (mode) {
    case 0:
      cg = 1;
      break;
    case 1:
      cg = 2;
      break;
    case 2:
      cg = 4;
      break;
    case 3:
      cg = 5;
      break;
    case 5:
      cg = 7;
      break;
    case 9:
      cg = 6;
      break;
    case 11:
      cg = 3;
      break;
    case 12:
      cg = 8;
      break;
    case 13:
      cg = 9;
      break;
    case 18:
      cg = 10;
      break;
    case 19:
      cg = 11;
      break;
    case 20:
      cg = 12;
      break;
    case 21:
      cg = 13;
      break;
    case 22:
      cg = 14;
      break;
    case 23:
      cg = 15;
      break;
    }
    blended = mix_color(cb, cs, cg);
  }
  return float4((1 - s.a) * d.rgb + (1 - d.a) * s.rgb + s.a * d.a * blended,
                s.a + d.a * (1 - s.a));
}

float4 filter_fetch(texture2d<float, access::sample> source, int2 p,
                    int4 bounds, bool hard) {
  if (hard)
    p = clamp(p, bounds.xy, bounds.zw - 1);
  if (any(p < bounds.xy) || any(p >= bounds.zw) || any(p < 0) ||
      p.x >= int(source.get_width()) || p.y >= int(source.get_height()))
    return 0;
  return source.read(uint2(p));
}
kernel void layer_filter(texture2d<float, access::write> output [[texture(0)]],
                         texture2d<float, access::sample> source [[texture(1)]],
                         texture2d<float, access::sample> secondary
                         [[texture(2)]],
                         device const float4 *v [[buffer(0)]],
                         constant uint &kind [[buffer(1)]],
                         uint2 p [[thread_position_in_grid]]) {
  if (p.x >= output.get_width() || p.y >= output.get_height())
    return;
  float4 color = source.read(p);
  if (kind == 0) {
    bool straight = v[5].x > 0;
    float4 c = color;
    if (straight && c.a > 0)
      c.rgb /= c.a;
    c = c.r * v[0] + c.g * v[1] + c.b * v[2] + c.a * v[3] + v[4];
    c = clamp(c, 0.0f, 1.0f);
    if (straight)
      c.rgb *= c.a;
    else
      c.rgb = min(c.rgb, float3(c.a));
    color = c;
  } else if (kind == 1) {
    color = float4(0, 0, 0, dot(color.rgb, float3(0.2125f, 0.7154f, 0.0721f)));
  } else if (kind == 2) {
    constexpr sampler pixel(coord::pixel, address::clamp_to_zero,
                            filter::linear);
    float alpha = source.sample(pixel, float2(p) + 0.5f - v[0].xy).a;
    float4 shadow = float4(v[1].rgb * v[1].a, v[1].a) * alpha;
    float4 foreground = secondary.read(p);
    color = foreground + shadow * (1 - foreground.a);
  } else if (kind == 3) {
    float2 pos = float2(p) + 0.5f;
    float2 local = float2(v[0].x * pos.x + v[0].z * pos.y,
                          v[0].y * pos.x + v[0].w * pos.y) +
                   v[1].xy;
    float2 uv = (local - v[2].xy) / v[2].zw;
    constexpr sampler linear(coord::normalized, address::clamp_to_edge,
                             filter::linear);
    float3 factors = secondary.sample(linear, uv).rgb * v[1].z;
    color.rgb = min(color.rgb * factors, float3(color.a));
    if (any(uv < 0) || any(uv > 1))
      color = 0;
  } else if (kind == 4) {
    uint width = clamp(uint(v[0].x), 1u, 5u),
         height = clamp(uint(v[0].y), 1u, 5u);
    float4 sum = 0;
    device const float *weights = (device const float *)(v + 3);
    int4 bounds = int4(v[2]);
    for (uint y = 0; y < height; y++)
      for (uint x = 0; x < width; x++) {
        int2 point = int2(p) + int2(x, y) - int2(width / 2, height / 2);
        float4 c = filter_fetch(source, point, bounds, v[1].x > 0);
        if (v[1].y > 0 && c.a > 0)
          c.rgb /= c.a;
        sum += c * weights[y * width + x];
      }
    sum = sum * v[0].z + v[0].w;
    if (v[1].y > 0) {
      sum.a = color.a;
      sum.rgb = clamp(sum.rgb, 0.0f, 1.0f) * color.a;
    } else {
      sum = clamp(sum, 0.0f, 1.0f);
      sum.rgb = min(sum.rgb, float3(sum.a));
    }
    color = sum;
  }
  if (kind == 6) {
    float3 light = quartz_light_at(float2(p) + 0.5f, v);
    if (v[2].x > 0)
      color.rgb = min(color.rgb * clamp(light, float3(0), float3(v[2].y)),
                      float3(color.a));
    else
      color = float4(clamp(light, 0.0f, 1.0f), 1);
  }
  if (kind == 7)
    color =
        quartz_pixel_operation(secondary.read(p), color, float2(p) + 0.5f, v);
  if (kind == 5) {
    constexpr sampler pixel(coord::pixel, address::clamp_to_zero,
                            filter::linear);
    float alpha = source.sample(pixel, float2(p) + 0.5f - v[0].xy).a;
    float mask = secondary.read(p).a;
    alpha = (v[0].z > 0 ? (1 - alpha) * mask : alpha * (1 - mask)) * v[1].a;
    color = float4(v[1].rgb * alpha, alpha);
  }
  output.write(color, p);
}

// Source-over draws use the tile renderer's fixed-function premultiplied blend.
// Instances retain the compute path's exact device-space coverage and samples.
struct DrawVertex {
  float4 position [[position]];
  uint instance [[flat]];
};
vertex DrawVertex draw_vertex(uint id [[vertex_id]],
                              device const Params *draws [[buffer(0)]],
                              constant float2 &size [[buffer(1)]]) {
  uint instance = id / 6, vid = id % 6;
  Params p = draws[instance];
  float2 corner = float2(vid == 1 || vid == 4 || vid == 5,
                         vid == 2 || vid == 3 || vid == 5);
  float2 pixel = float2(p.origin) + corner * float2(p.extent);
  return {float4((pixel.x - p.target.x) / size.x * 2 - 1,
                 1 - (pixel.y - p.target.y) / size.y * 2, 0, 1),
          instance};
}
fragment float4 draw_fragment(DrawVertex v [[stage_in]],
                              device const Params *draws [[buffer(0)]],
                              device const float4 *allEdges [[buffer(1)]],
                              texture2d<float> clip [[texture(0)]],
                              texture2d<float> image [[texture(1)]]) {
  Params p = draws[v.instance];
  device const float4 *edges = allEdges + p.edgePad;
  uint2 position = uint2(v.position.xy + p.target.xy);
  float2 center = float2(position) + 0.5f;
  float mask = 1;
  if (p.clipped & 1) {
    float2 relative = center - p.clip.xy;
    mask = all(relative >= 0) && all(relative < p.clip.zw)
               ? clip.read(uint2(relative)).r
               : 0;
  }
  if (p.clipped & 2)
    mask *= clip_rounded(center, p.rclip, p.rradius, p.rmap);
  if (p.clipped & 4)
    mask *= clip_rectangle(center, p.sclip);
  device const float4 *pathEdges = edges;
  uint count = p.edges;
  if (p.edgeRows) {
    uint row = min((position.y - p.edgeOriginY) / p.edgeStride, p.edgeRows - 1);
    uint4 bin = as_type<uint4>(edges[row]);
    pathEdges = edges + p.edgeRows + bin.x;
    count = bin.y;
  }
  float cov =
      (p.edges && !count) ? 0
      : (p.flipped & 1)
          ? (count ? coverage_at(center, pathEdges, count, p.evenodd != 0) : 1)
          : coverage(float2(position), pathEdges, count, p.evenodd != 0);
  if (p.mode == 0 && (p.reserved & 8))
    cov = box_coverage(center, p.gradient,
                       (p.reserved & 16) ? p.src.xy : float2(0),
                       p.flipped & 1, p.reserved & 32, p.inverse,
                       p.translation.xy);
  cov *= mask;
  if (cov <= 0)
    return float4(0);
  float4 color = p.color;
  if (p.mode == 4) {
    float2 local = float2(p.inverse.x * center.x + p.inverse.z * center.y,
                          p.inverse.y * center.x + p.inverse.w * center.y) +
                   p.translation.xy;
    float2 uv = (local - p.dst.xy) / p.dst.zw;
    if (p.edges)
      uv = clamp(uv, float2(0), float2(1));
    else if (any(uv < 0) || any(uv > 1))
      return float4(0);
    float2 source = p.src.xy + uv * p.src.zw;
    if (p.reserved & 4)
      source = clamp(source, p.src.xy + 0.5f, p.src.xy + p.src.zw - 0.5f);
    float2 sample = source / float2(image.get_width(), image.get_height());
    constexpr sampler linear(coord::normalized, address::clamp_to_edge,
                             filter::linear);
    constexpr sampler nearest(coord::normalized, address::clamp_to_edge,
                              filter::nearest);
    constexpr sampler mip(coord::normalized, address::clamp_to_edge,
                          filter::linear, mip_filter::linear);
    uint interpolation = p.flipped >> 2;
    float2 ratio = p.src.zw / p.dst.zw;
    float footprint = max(length(float2(p.inverse.x, p.inverse.y) * ratio),
                          length(float2(p.inverse.z, p.inverse.w) * ratio));
    if (interpolation >= 3 && footprint > 1)
      color = image.sample(mip, sample, level(log2(footprint)));
    else if (interpolation == 4)
      color = (p.reserved & 4)
                  ? cubic_sample_rect(
                        image,
                        (p.src.xy + uv * p.src.zw) /
                            float2(image.get_width(), image.get_height()),
                        p.src)
                  : cubic_sample(image, sample);
    else
      color = (p.flipped & 2) ? image.sample(nearest, sample, level(0))
                              : image.sample(linear, sample, level(0));
    if ((p.reserved & 3) == 2)
      color = color.rrrr;
    if (p.reserved & 3)
      color *= float4(p.color.rgb * p.color.a, p.color.a);
    color *= p.translation.w;
  } else
    color.rgb *= color.a;
  return color * cov;
}

// Keep common pipelines small: glyph quads do not need the arbitrary-path,
// gradient or high-quality reconstruction machinery in every fragment.
fragment float4 path_fragment(DrawVertex v [[stage_in]],
                              device const Params *draws [[buffer(0)]],
                              device const float4 *allEdges [[buffer(1)]],
                              texture2d<float> clip [[texture(0)]]) {
  Params p = draws[v.instance];
  device const float4 *edges = allEdges + p.edgePad;
  uint2 position = uint2(v.position.xy + p.target.xy);
  float2 center = float2(position) + 0.5f;
  float mask = 1;
  if (p.clipped & 1) {
    float2 relative = center - p.clip.xy;
    mask = all(relative >= 0) && all(relative < p.clip.zw)
               ? clip.read(uint2(relative)).r
               : 0;
  }
  if (p.clipped & 2)
    mask *= clip_rounded(center, p.rclip, p.rradius, p.rmap);
  if (p.clipped & 4)
    mask *= clip_rectangle(center, p.sclip);
  device const float4 *pathEdges = edges;
  uint count = p.edges;
  if (p.edgeRows) {
    uint row = min((position.y - p.edgeOriginY) / p.edgeStride, p.edgeRows - 1);
    uint4 bin = as_type<uint4>(edges[row]);
    pathEdges = edges + p.edgeRows + bin.x;
    count = bin.y;
  }
  float cov =
      (p.edges && !count) ? 0
      : (p.flipped & 1)
          ? (count ? coverage_at(center, pathEdges, count, p.evenodd != 0) : 1)
          : coverage(float2(position), pathEdges, count, p.evenodd != 0);
  if (p.mode == 0 && (p.reserved & 8))
    cov = box_coverage(center, p.gradient,
                       (p.reserved & 16) ? p.src.xy : float2(0),
                       p.flipped & 1, p.reserved & 32, p.inverse,
                       p.translation.xy);
  cov *= mask;
  if (cov <= 0)
    return float4(0);
  return float4(p.color.rgb * p.color.a, p.color.a) * cov;
}
struct ImageParams {
  float4 bounds, uv, limits, tint;
  uint4 flags;
};
struct ImageVertex {
  float4 position [[position]];
  float2 uv;
  float4 limits [[flat]];
  float4 tint [[flat]];
  uint4 flags [[flat]];
};
vertex ImageVertex image_vertex(uint id [[vertex_id]],
                                device const ImageParams *draws [[buffer(0)]],
                                constant float2 &size [[buffer(1)]]) {
  uint instance = id / 6, vid = id % 6;
  ImageParams p = draws[instance];
  float2 corner = float2(vid == 1 || vid == 4 || vid == 5,
                         vid == 2 || vid == 3 || vid == 5);
  float2 pixel = p.bounds.xy + corner * p.bounds.zw;
  return {float4(pixel.x / size.x * 2 - 1, 1 - pixel.y / size.y * 2, 0, 1),
          p.uv.xy + corner * p.uv.zw, p.limits, p.tint, p.flags};
}
fragment float4 image_fragment(ImageVertex v [[stage_in]],
                               texture2d<float> image [[texture(1)]]) {
  if (any(v.uv < v.limits.xy) || any(v.uv > v.limits.zw))
    return float4(0);
  float2 uv = v.uv;
  if (v.flags.z) {
    float2 halfTexel = 0.5f / float2(image.get_width(), image.get_height());
    uv = clamp(uv, v.limits.xy + halfTexel, v.limits.zw - halfTexel);
  }
  constexpr sampler linear(coord::normalized, address::clamp_to_edge,
                           filter::linear);
  constexpr sampler nearest(coord::normalized, address::clamp_to_edge,
                            filter::nearest);
  float4 color = v.flags.x ? image.sample(nearest, uv, level(0))
                           : image.sample(linear, uv, level(0));
  return (v.flags.y ? color.rrrr : color) * v.tint;
}
// Integral solid rectangles need neither sampled textures nor per-fragment
// path parameters. Fractional/transformed paths keep their exact coverage.
struct SolidParams {
  float4 bounds, color, radii, shape;
};
struct SolidVertex {
  float4 position [[position]];
  float4 color [[flat]];
  float4 bounds [[flat]];
  float2 radii [[flat]];
  float4 shape [[flat]];
};
vertex SolidVertex solid_vertex(uint id [[vertex_id]],
                                device const SolidParams *draws [[buffer(0)]],
                                constant float2 &size [[buffer(1)]]) {
  uint vid = id % 6;
  SolidParams p = draws[id / 6];
  float2 corner = float2(vid == 1 || vid == 4 || vid == 5,
                         vid == 2 || vid == 3 || vid == 5);
  float2 low = p.bounds.xy, high = low + p.bounds.zw;
  float2 pixel = mix(floor(low), ceil(high), corner);
  return {float4(pixel.x / size.x * 2 - 1, 1 - pixel.y / size.y * 2, 0, 1),
          p.color, float4(low, high), p.radii.xy, p.shape};
}
fragment float4 solid_fragment(SolidVertex v [[stage_in]]) {
  float2 center = floor(v.position.xy) + 0.5f;
  if (v.radii.x > 0 && v.radii.y > 0)
    return v.color * box_coverage(center, v.shape, v.radii, false, false,
                                  float4(1, 0, 0, 1), float2(0));
  float2 a = center - 0.25f, b = center + 0.25f;
  float2 covered = float2((a >= v.bounds.xy) & (a < v.bounds.zw)) +
                   float2((b >= v.bounds.xy) & (b < v.bounds.zw));
  return v.color * (covered.x * covered.y * 0.25f);
}
struct RectImageParams {
  ImageParams image;
  float4 clip;
};
struct RectImageVertex {
  float4 position [[position]];
  float2 uv;
  float4 limits [[flat]];
  float4 tint [[flat]];
  uint4 flags [[flat]];
  float4 coverageBounds [[flat]];
};
vertex RectImageVertex rect_image_vertex(uint id [[vertex_id]],
                                         device const RectImageParams *draws
                                         [[buffer(0)]],
                                         constant float2 &size [[buffer(1)]]) {
  uint instance = id / 6, vid = id % 6;
  RectImageParams q = draws[instance];
  ImageParams p = q.image;
  float2 corner = float2(vid == 1 || vid == 4 || vid == 5,
                         vid == 2 || vid == 3 || vid == 5);
  float2 low = p.bounds.xy, high = low + p.bounds.zw;
  float2 pixel = mix(max(floor(low), q.clip.xy),
                     min(ceil(high), q.clip.xy + q.clip.zw), corner);
  float2 fraction = p.flags.w ? (pixel - low) / p.bounds.zw : corner;
  return {float4(pixel.x / size.x * 2 - 1, 1 - pixel.y / size.y * 2, 0, 1),
          p.uv.xy + fraction * p.uv.zw,
          p.limits,
          p.tint,
          p.flags,
          float4(low, high)};
}
fragment float4 rect_image_fragment(RectImageVertex v [[stage_in]],
                                    texture2d<float> image [[texture(1)]]) {
  float coverage = 1;
  float2 uv = v.uv;
  if (v.flags.w) {
    float2 center = floor(v.position.xy) + 0.5f;
    if (v.flags.w & 2) {
      coverage = all(center >= v.coverageBounds.xy) &&
                 all(center < v.coverageBounds.zw);
    } else {
      float2 a = center - 0.25f, b = center + 0.25f;
      float2 covered =
          float2((a >= v.coverageBounds.xy) & (a < v.coverageBounds.zw)) +
          float2((b >= v.coverageBounds.xy) & (b < v.coverageBounds.zw));
      coverage = covered.x * covered.y * 0.25f;
    }
    if (coverage <= 0)
      return float4(0);
    uv = clamp(uv, v.limits.xy, v.limits.zw);
  } else if (any(uv < v.limits.xy) || any(uv > v.limits.zw))
    return float4(0);
  if (v.flags.z) {
    float2 halfTexel = 0.5f / float2(image.get_width(), image.get_height());
    uv = clamp(uv, v.limits.xy + halfTexel, v.limits.zw - halfTexel);
  }
  constexpr sampler linear(coord::normalized, address::clamp_to_edge,
                           filter::linear);
  constexpr sampler nearest(coord::normalized, address::clamp_to_edge,
                            filter::nearest);
  float4 color = v.flags.x ? image.sample(nearest, uv, level(0))
                           : image.sample(linear, uv, level(0));
  return (v.flags.y ? color.rrrr : color) * v.tint * coverage;
}
fragment float4 clear_fragment() { return float4(0); }
