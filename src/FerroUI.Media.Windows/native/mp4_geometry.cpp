// Bounded local ISO-BMFF metadata inspection. No decoded pixels are read.
// MF's RGB output can apply clap but omit the track's reflection/rotation.
namespace mp4 {
struct Geometry {
  bool known = false;
  uint32_t width = 0, height = 0, x = 0, y = 0, w = 0, h = 0, rotation = 0,
           mirror = 0;
};
using Bytes = std::vector<uint8_t>;
static void require(bool ok) {
  if (!ok)
    throw Failure{E_NOTIMPL};
}
static uint32_t u32(const Bytes &b, size_t p) {
  require(p <= b.size() && b.size() - p >= 4);
  return uint32_t(b[p]) << 24 | uint32_t(b[p + 1]) << 16 |
         uint32_t(b[p + 2]) << 8 | b[p + 3];
}
static uint64_t u64(const Bytes &b, size_t p) {
  return uint64_t(u32(b, p)) << 32 | u32(b, p + 4);
}
static constexpr uint32_t tag(char a, char b, char c, char d) {
  return uint32_t(a) << 24 | uint32_t(b) << 16 | uint32_t(c) << 8 | uint32_t(d);
}
struct Box {
  size_t payload, end;
  uint32_t kind;
};
static std::vector<Box> boxes(const Bytes &b, size_t start, size_t end,
                              int &budget) {
  require(end <= b.size() && start <= end);
  std::vector<Box> out;
  while (start < end) {
    require(--budget >= 0 && end - start >= 8);
    uint64_t n = u32(b, start);
    size_t header = 8;
    if (n == 1) {
      require(end - start >= 16);
      n = u64(b, start + 8);
      header = 16;
    }
    if (n == 0)
      n = end - start;
    require(n >= header && n <= end - start);
    out.push_back({start + header, start + size_t(n), u32(b, start + 4)});
    start += size_t(n);
  }
  return out;
}
static Box child(const Bytes &b, Box parent, uint32_t kind, int &budget) {
  Box found{};
  int count = 0;
  for (auto v : boxes(b, parent.payload, parent.end, budget))
    if (v.kind == kind) {
      found = v;
      ++count;
    }
  require(count == 1);
  return found;
}
static double ratio(const Bytes &b, size_t p, bool sign = false) {
  auto d = u32(b, p + 4);
  require(d != 0);
  return (sign ? double(int32_t(u32(b, p))) : double(u32(b, p))) / d;
}
static uint32_t integral(double v) {
  require(std::isfinite(v) && v >= 0 && v <= 8192 &&
          std::abs(v - std::round(v)) < 1e-6);
  return uint32_t(std::round(v));
}
static Geometry parse(const Bytes &b) {
  int budget = 4096;
  auto roots = boxes(b, 0, b.size(), budget);
  require(roots.size() == 1 && roots[0].kind == tag('m', 'o', 'o', 'v'));
  Geometry g;
  for (auto track : boxes(b, roots[0].payload, roots[0].end, budget)) {
    if (track.kind != tag('t', 'r', 'a', 'k'))
      continue;
    auto mdia = child(b, track, tag('m', 'd', 'i', 'a'), budget),
         handler = child(b, mdia, tag('h', 'd', 'l', 'r'), budget);
    require(handler.end - handler.payload >= 12);
    if (u32(b, handler.payload + 8) != tag('v', 'i', 'd', 'e'))
      continue;
    require(!g.known); // The Source Reader's selected video track must be
                       // unambiguous.
    auto tkhd = child(b, track, tag('t', 'k', 'h', 'd'), budget);
    require(tkhd.end - tkhd.payload >= 4);
    auto version = b[tkhd.payload];
    require(version <= 1);
    size_t matrix = tkhd.payload + (version == 0 ? 40 : 52);
    require(matrix <= tkhd.end && tkhd.end - matrix >= 44);
    require(u32(b, matrix + 8) == 0 && u32(b, matrix + 20) == 0 &&
            u32(b, matrix + 32) == 0x40000000);
    const int32_t matrices[8][4] = {
        {65536, 0, 0, 65536},  {0, 65536, -65536, 0}, {-65536, 0, 0, -65536},
        {0, -65536, 65536, 0}, {-65536, 0, 0, 65536}, {0, 65536, 65536, 0},
        {65536, 0, 0, -65536}, {0, -65536, -65536, 0}};
    int orientation = -1;
    for (int i = 0; i < 8; i++)
      if (int32_t(u32(b, matrix)) == matrices[i][0] &&
          int32_t(u32(b, matrix + 4)) == matrices[i][1] &&
          int32_t(u32(b, matrix + 12)) == matrices[i][2] &&
          int32_t(u32(b, matrix + 16)) == matrices[i][3])
        orientation = i;
    require(orientation >= 0);
    g.rotation = (orientation % 4) * 90;
    g.mirror = orientation >= 4;
    auto minf = child(b, mdia, tag('m', 'i', 'n', 'f'), budget),
         stbl = child(b, minf, tag('s', 't', 'b', 'l'), budget),
         stsd = child(b, stbl, tag('s', 't', 's', 'd'), budget);
    require(stsd.end - stsd.payload >= 8 && u32(b, stsd.payload + 4) == 1);
    auto entries = boxes(b, stsd.payload + 8, stsd.end, budget);
    require(entries.size() == 1);
    auto entry = entries[0];
    require(entry.end - entry.payload >= 78);
    g.width = u32(b, entry.payload + 24) >> 16;
    g.height = u32(b, entry.payload + 24) & 65535;
    require(g.width > 0 && g.height > 0 && g.width <= 8192 && g.height <= 8192);
    g.w = g.width;
    g.h = g.height;
    bool clean = false, aspect = false;
    for (auto field : boxes(b, entry.payload + 78, entry.end, budget)) {
      if (field.kind == tag('c', 'l', 'a', 'p')) {
        require(!clean && field.end - field.payload == 32);
        clean = true;
        double w = ratio(b, field.payload), h = ratio(b, field.payload + 8);
        g.x = integral((g.width - w) / 2 + ratio(b, field.payload + 16, true));
        g.y = integral((g.height - h) / 2 + ratio(b, field.payload + 24, true));
        g.w = integral(w);
        g.h = integral(h);
        require(g.w > 0 && g.h > 0 && g.x + g.w <= g.width &&
                g.y + g.h <= g.height);
      } else if (field.kind == tag('p', 'a', 's', 'p')) {
        require(!aspect && field.end - field.payload == 8);
        aspect = true;
        require(u32(b, field.payload) > 0 &&
                u32(b, field.payload) == u32(b, field.payload + 4));
      }
    }
    g.known = true;
  }
  require(g.known);
  return g;
}
struct File {
  HANDLE handle = INVALID_HANDLE_VALUE;
  ~File() {
    if (handle != INVALID_HANDLE_VALUE)
      CloseHandle(handle);
  }
};
static void read(File &file, uint64_t at, uint8_t *out, DWORD size) {
  LARGE_INTEGER p;
  p.QuadPart = at;
  check(SetFilePointerEx(file.handle, p, nullptr, FILE_BEGIN)
            ? S_OK
            : HRESULT_FROM_WIN32(GetLastError()));
  DWORD n = 0;
  check(ReadFile(file.handle, out, size, &n, nullptr)
            ? S_OK
            : HRESULT_FROM_WIN32(GetLastError()));
  require(n == size);
}
static Geometry inspect(const std::wstring &path,
                        const std::atomic<bool> &cancelled) {
  // Network containers remain Source Reader-owned; this parser does not
  // download or buffer remote media, nor make reflection claims for that
  // profile.
  if (path.find(L"://") != std::wstring::npos)
    return {};
  File file;
  file.handle =
      CreateFileW(path.c_str(), GENERIC_READ,
                  FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                  nullptr, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, nullptr);
  check(file.handle != INVALID_HANDLE_VALUE
            ? S_OK
            : HRESULT_FROM_WIN32(GetLastError()));
  LARGE_INTEGER length;
  check(GetFileSizeEx(file.handle, &length)
            ? S_OK
            : HRESULT_FROM_WIN32(GetLastError()));
  require(length.QuadPart >= 0);
  uint64_t end = length.QuadPart, at = 0;
  bool iso = false;
  int budget = 4096;
  while (at < end) {
    if (cancelled.load())
      throw Failure{E_ABORT};
    require(--budget >= 0);
    if (end - at < 8) {
      require(!iso);
      return {};
    }
    Bytes header(16);
    read(file, at, header.data(), 8);
    uint64_t n = u32(header, 0);
    uint32_t kind = u32(header, 4);
    uint64_t h = 8;
    if (at == 0 && kind != tag('f', 't', 'y', 'p') &&
        kind != tag('m', 'o', 'o', 'v') && kind != tag('f', 'r', 'e', 'e') &&
        kind != tag('w', 'i', 'd', 'e') && kind != tag('m', 'd', 'a', 't') &&
        kind != tag('s', 'k', 'i', 'p'))
      return {};
    if (kind == tag('f', 't', 'y', 'p') || kind == tag('m', 'o', 'o', 'v'))
      iso = true;
    if (n == 1) {
      require(end - at >= 16);
      read(file, at + 8, header.data() + 8, 8);
      n = u64(header, 8);
      h = 16;
    }
    if (n == 0)
      n = end - at;
    require(n >= h && n <= end - at);
    if (kind == tag('m', 'o', 'o', 'v')) {
      require(n <= 8 * 1024 * 1024);
      Bytes metadata(size_t(n), 0);
      read(file, at, metadata.data(), DWORD(n));
      return parse(metadata);
    }
    at += n;
  }
  require(!iso);
  return {};
}
} // namespace mp4
