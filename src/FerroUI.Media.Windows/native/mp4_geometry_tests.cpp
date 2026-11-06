// Independent encoded metadata fixtures, including malformed-input boundaries.
#include <winioctl.h>
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
namespace mp4_file_test {
using namespace mp4_test;
struct Temp {
  wchar_t path[MAX_PATH]{};
  Temp() {
    wchar_t directory[MAX_PATH]{};
    auto length = GetTempPathW(MAX_PATH, directory);
    require(length > 0 && length < MAX_PATH);
    require(GetTempFileNameW(directory, L"fmg", 0, path) != 0);
  }
  ~Temp() { DeleteFileW(path); }
  void write(const Bytes &bytes, uint64_t offset = 0) {
    mp4::File file;
    file.handle = CreateFileW(path, GENERIC_WRITE, 0, nullptr, CREATE_ALWAYS,
                              FILE_ATTRIBUTE_NORMAL, nullptr);
    require(file.handle != INVALID_HANDLE_VALUE);
    if (offset) {
      DWORD returned = 0;
      require(DeviceIoControl(file.handle, FSCTL_SET_SPARSE, nullptr, 0,
                              nullptr, 0, &returned, nullptr) != 0);
      // A real extended-size mdat crossing 32-bit offsets, with no payload
      // allocated/read. Sparse is mandatory, never create a dense 4 GiB file.
      Bytes header(16);
      put(header, 0, 1);
      put(header, 4, mp4::tag('m', 'd', 'a', 't'));
      put(header, 8, uint32_t(offset >> 32));
      put(header, 12, uint32_t(offset));
      DWORD written = 0;
      require(WriteFile(file.handle, header.data(), DWORD(header.size()),
                        &written, nullptr) &&
              written == header.size());
      LARGE_INTEGER position;
      position.QuadPart = offset;
      require(SetFilePointerEx(file.handle, position, nullptr, FILE_BEGIN) !=
              0);
    }
    DWORD written = 0;
    require(WriteFile(file.handle, bytes.data(), DWORD(bytes.size()), &written,
                      nullptr) &&
            written == bytes.size());
  }
};
static void reject_file(Temp &file, const Bytes &bytes) {
  file.write(bytes);
  std::atomic<bool> cancelled{false};
  bool rejected = false;
  try {
    rejected = !mp4::inspect(file.path, cancelled).known;
  } catch (Failure e) {
    require(e.hr == E_NOTIMPL);
    rejected = true;
  }
  require(rejected);
  // inspect's RAII must close the handle on success and every exception.
  mp4::File exclusive;
  exclusive.handle = CreateFileW(file.path, GENERIC_WRITE, 0, nullptr,
                                 OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, nullptr);
  require(exclusive.handle != INVALID_HANDLE_VALUE);
}
static void run() {
  Temp file;
  std::atomic<bool> cancelled{false};
  auto valid = join({box("ftyp", {}), fixture(5, 1, true, clean())});
  file.write(valid);
  auto g = mp4::inspect(file.path, cancelled);
  require(g.known && g.rotation == 90 && g.mirror == 1 && g.x == 10 &&
          g.y == 6);
  for (size_t n = 0; n < valid.size(); n++)
    reject_file(file, Bytes(valid.begin(), valid.begin() + n));
  auto nested = fixture();
  for (int i = 0; i < 4096; i++) {
    auto free = box("free", {});
    nested.insert(nested.end(), free.begin(), free.end());
  }
  put(nested, 0, uint32_t(nested.size()));
  reject_file(file, nested);
  Bytes top;
  for (int i = 0; i < 4096; i++) {
    auto free = box("free", {});
    top.insert(top.end(), free.begin(), free.end());
  }
  auto movie = fixture();
  // Exactly 4,096 top-level boxes is admitted, the next box is not.
  Bytes boundary(top.begin(), top.end() - 8);
  boundary.insert(boundary.end(), movie.begin(), movie.end());
  file.write(boundary);
  require(mp4::inspect(file.path, cancelled).known);
  top.insert(top.end(), movie.begin(), movie.end());
  reject_file(file, top);
  auto metadata_limit = fixture();
  auto padding =
      box("free", Bytes(8 * 1024 * 1024 - metadata_limit.size() - 8));
  metadata_limit.insert(metadata_limit.end(), padding.begin(), padding.end());
  put(metadata_limit, 0, uint32_t(metadata_limit.size()));
  file.write(metadata_limit);
  require(mp4::inspect(file.path, cancelled).known);
  Bytes oversized(8 * 1024 * 1024 + 1);
  put(oversized, 0, uint32_t(oversized.size()));
  put(oversized, 4, mp4::tag('m', 'o', 'o', 'v'));
  reject_file(file, oversized);
  auto malformed = valid;
  put(malformed, 0, 1);
  reject_file(file, malformed);
  file.write(valid);
  cancelled = true;
  bool aborted = false;
  try {
    mp4::inspect(file.path, cancelled);
  } catch (Failure e) {
    aborted = e.hr == E_ABORT;
  }
  require(aborted);
  cancelled = false;
  // Deterministic cancellation after real metadata I/O, before parsing it.
  // Uses the same file inspection path with a cancellation predicate instead
  // of racing the test scheduler against an 8 MiB ReadFile.
  file.write(fixture());
  size_t checkpoints = 0;
  aborted = false;
  try {
    mp4::inspect_file(file.path, [&] { return ++checkpoints == 2; });
  } catch (Failure e) {
    aborted = e.hr == E_ABORT;
  }
  require(aborted && checkpoints == 2);
  file.write(fixture(7, 0, true, clean()), (1ull << 32) + 4096);
  g = mp4::inspect(file.path, cancelled);
  require(g.known && g.rotation == 270 && g.mirror == 1 && g.x == 10);
  puts("NATIVE_MP4_FILE_PASS truncation=all top_box_limit=true "
       "nested_box_limit=true exact_limits=admitted oversized_moov=rejected "
       "cancellation=before_scan_and_after_read "
       "sparse_mdat_over_4GiB=skipped handles=closed");
}
} // namespace mp4_file_test
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
    reject(fixture(0, 0, false, clean(144, 0))); // zero denominator
    reject(fixture(
        0, 0, false,
        clean(144u * 16777216 + 1,
              16777216))); // tiny fraction must not round into admission
    auto fractional_offset = clean();
    put(fractional_offset, 24, 1073741825);
    put(fractional_offset, 28, 536870912);
    reject(fixture(0, 0, false, fractional_offset));
    auto integral_offset = clean(143);
    put(integral_offset, 24, 3);
    put(integral_offset, 28, 2);
    auto odd = mp4::parse(fixture(0, 0, false, integral_offset));
    require(odd.x == 10 && odd.w == 143);
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
    mp4_file_test::run();
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
