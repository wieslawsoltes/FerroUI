// Independent encoded metadata fixtures, including malformed-input boundaries.
namespace mp4_test {
using mp4::Bytes;
static void put(Bytes &b, size_t at, uint32_t n) {
  for (int i = 0; i < 4; i++)
    b.at(at + i) = uint8_t(n >> (24 - i * 8));
}
static Bytes box(const char *kind, Bytes payload, bool wide = false) {
  Bytes b(wide ? 16 : 8);
  put(b, 0, wide ? 1 : uint32_t(payload.size() + 8));
  for (int i = 0; i < 4; i++)
    b[4 + i] = kind[i];
  if (wide)
    put(b, 12, uint32_t(payload.size() + 16));
  b.insert(b.end(), payload.begin(), payload.end());
  return b;
}
static Bytes join(std::initializer_list<Bytes> parts) {
  Bytes b;
  for (auto &p : parts)
    b.insert(b.end(), p.begin(), p.end());
  return b;
}
static Bytes fixture(int orientation = 0, int version = 0, bool wide = false,
                     Bytes fields = {}, bool duplicate = false) {
  const int32_t matrices[8][4] = {{1, 0, 0, 1},  {0, 1, -1, 0}, {-1, 0, 0, -1},
                                  {0, -1, 1, 0}, {-1, 0, 0, 1}, {0, 1, 1, 0},
                                  {1, 0, 0, -1}, {0, -1, -1, 0}};
  Bytes tkhd(version ? 96 : 84);
  tkhd[0] = uint8_t(version);
  size_t m = version ? 52 : 40;
  for (int i = 0; i < 4; i++)
    put(tkhd, m + (i / 2) * 12 + (i % 2) * 4,
        uint32_t(matrices[orientation][i] * 65536));
  put(tkhd, m + 32, 0x40000000);
  Bytes handler(12);
  put(handler, 8, mp4::tag('v', 'i', 'd', 'e'));
  Bytes visual(78);
  put(visual, 24, 160 << 16 | 90);
  visual.insert(visual.end(), fields.begin(), fields.end());
  Bytes stsd(8);
  put(stsd, 4, 1);
  auto entry = box("avc1", visual, wide);
  stsd.insert(stsd.end(), entry.begin(), entry.end());
  auto mdia = box("mdia", join({box("hdlr", handler),
                                box("minf", box("stbl", box("stsd", stsd)))}));
  auto track = box("trak", join({box("tkhd", tkhd), mdia}));
  return box("moov", duplicate ? join({track, track}) : track, wide);
}
static Bytes clean(uint32_t w = 144, uint32_t wn = 1, uint32_t h = 80,
                   int32_t x = 2, int32_t y = 1) {
  Bytes b(32);
  put(b, 0, w);
  put(b, 4, wn);
  put(b, 8, h);
  put(b, 12, 1);
  put(b, 16, uint32_t(x));
  put(b, 20, 1);
  put(b, 24, uint32_t(y));
  put(b, 28, 1);
  return box("clap", b);
}
static void reject(const Bytes &b) {
  bool failed = false;
  try {
    mp4::parse(b);
  } catch (Failure) {
    failed = true;
  }
  require(failed);
}
} // namespace mp4_test
extern "C" int ferro_windows_test_mp4_geometry() {
  using namespace mp4_test;
  try {
    for (int v = 0; v < 2; v++)
      for (int wide = 0; wide < 2; wide++)
        for (int i = 0; i < 8; i++) {
          auto g = mp4::parse(fixture(i, v, wide, clean()));
          require(g.known && g.rotation == uint32_t(i % 4) * 90 &&
                  g.mirror == uint32_t(i >= 4) && g.width == 160 &&
                  g.height == 90 && g.x == 10 && g.y == 6 && g.w == 144 &&
                  g.h == 80);
        }
    auto valid = fixture();
    for (size_t n = 0; n < valid.size(); n++)
      reject(Bytes(valid.begin(), valid.begin() + n));
    auto corrupt = valid;
    put(corrupt, 0, 0xffffffff);
    reject(corrupt);
    reject(fixture(0, 0, false, {}, true));
    reject(fixture(0, 0, false, join({clean(), clean()})));
    reject(
        fixture(0, 0, false, clean(143, 2))); // fractional clean width / offset
    reject(fixture(0, 0, false, clean(144, 0)));          // zero denominator
    reject(fixture(0, 0, false, clean(144, 1, 80, -99))); // crop outside image
    Bytes aspect(8);
    put(aspect, 0, 2);
    put(aspect, 4, 1);
    reject(fixture(0, 0, false, box("pasp", aspect)));
    // Locate the explicit tkhd bytes, then introduce scale and perspective.
    int budget = 4096;
    auto root = mp4::boxes(valid, 0, valid.size(), budget)[0];
    auto track = mp4::child(valid, root, mp4::tag('t', 'r', 'a', 'k'), budget);
    auto tkhd = mp4::child(valid, track, mp4::tag('t', 'k', 'h', 'd'), budget);
    corrupt = valid;
    put(corrupt, tkhd.payload + 40, 2 * 65536);
    reject(corrupt);
    corrupt = valid;
    put(corrupt, tkhd.payload + 48, 1);
    reject(corrupt);
    puts("NATIVE_MP4_GEOMETRY_PASS orientations=8 versions=2 "
         "extended_boxes=true truncation=all malformed=rejected");
    return 0;
  } catch (Failure e) {
    fprintf(stderr, "NATIVE_MP4_GEOMETRY_FAILED %s\n", hresult(e.hr).c_str());
  } catch (...) {
    fputs("NATIVE_MP4_GEOMETRY_FAILED\n", stderr);
  }
  return 1;
}
