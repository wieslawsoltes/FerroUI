// Opt-in, in-process MF camera fixture. No device registration, privacy
// changes, microphone, CPU pixel generation or production fallback. Each sample
// owns a fresh GPU-cleared texture; the ordinary Source Reader/camera path
// consumes it.
#include <wrl/implements.h>
namespace {
using Microsoft::WRL::ChainInterfaces;
using Microsoft::WRL::ClassicCom;
using Microsoft::WRL::FtmBase;
using Microsoft::WRL::Make;
using Microsoft::WRL::RuntimeClass;
using Microsoft::WRL::RuntimeClassFlags;
static std::atomic<uint32_t> synthetic_sources{0}, synthetic_textures{0};
static std::atomic<uint64_t> synthetic_pattern_epoch{0};
static const GUID texture_credit = {
    0x638caf5a,
    0xb670,
    0x4ef5,
    {0x84, 0xdf, 0xa0, 0x4a, 0x96, 0xb1, 0x70, 0x29}};
class TextureCredit final
    : public RuntimeClass<RuntimeClassFlags<ClassicCom>, IUnknown> {
public:
  ~TextureCredit() { --synthetic_textures; }
};
// Blocking GetEvent must not hold the source/stream mutex: Shutdown wakes it.
#define SYNTHETIC_EVENTS                                                       \
  STDMETHODIMP GetEvent(DWORD flags, IMFMediaEvent **event) override {         \
    ComPtr<IMFMediaEventQueue> q;                                              \
    {                                                                          \
      std::lock_guard<std::mutex> g(lock);                                     \
      q = events;                                                              \
    }                                                                          \
    return q ? q->GetEvent(flags, event) : MF_E_SHUTDOWN;                      \
  }                                                                            \
  STDMETHODIMP BeginGetEvent(IMFAsyncCallback *cb, IUnknown *state) override { \
    ComPtr<IMFMediaEventQueue> q;                                              \
    {                                                                          \
      std::lock_guard<std::mutex> g(lock);                                     \
      q = events;                                                              \
    }                                                                          \
    return q ? q->BeginGetEvent(cb, state) : MF_E_SHUTDOWN;                    \
  }                                                                            \
  STDMETHODIMP EndGetEvent(IMFAsyncResult *result, IMFMediaEvent **event)      \
      override {                                                               \
    ComPtr<IMFMediaEventQueue> q;                                              \
    {                                                                          \
      std::lock_guard<std::mutex> g(lock);                                     \
      q = events;                                                              \
    }                                                                          \
    return q ? q->EndGetEvent(result, event) : MF_E_SHUTDOWN;                  \
  }                                                                            \
  STDMETHODIMP QueueEvent(MediaEventType type, REFGUID extended,               \
                          HRESULT status, const PROPVARIANT *value) override { \
    ComPtr<IMFMediaEventQueue> q;                                              \
    {                                                                          \
      std::lock_guard<std::mutex> g(lock);                                     \
      q = events;                                                              \
    }                                                                          \
    return q ? q->QueueEventParamVar(type, extended, status, value)            \
             : MF_E_SHUTDOWN;                                                  \
  }
class SyntheticStream final
    : public RuntimeClass<
          RuntimeClassFlags<ClassicCom>,
          ChainInterfaces<IMFMediaStream, IMFMediaEventGenerator>, FtmBase> {
  std::mutex lock;
  ComPtr<IMFMediaEventQueue> events;
  ComPtr<IMFMediaSource> owner;
  ComPtr<IMFStreamDescriptor> descriptor;
  ComPtr<ID3D11Device> device;
  ComPtr<ID3D11DeviceContext1> context;
  bool running = false;
  uint64_t sequence = 0;
  uint64_t disconnect_after = 0;
  uint64_t pattern = 0;
  Clock::time_point next;

public:
  HRESULT initialize(IMFMediaSource *source, ID3D11Device *d,
                     uint64_t disconnect, uint64_t palette) {
    try {
      disconnect_after = disconnect;
      pattern = palette;
      device = d;
      ComPtr<ID3D11DeviceContext> immediate;
      d->GetImmediateContext(&immediate);
      check(immediate.As(&context));
      check(MFCreateEventQueue(&events));
      ComPtr<IMFMediaType> type;
      check(MFCreateMediaType(&type));
      check(type->SetGUID(MF_MT_MAJOR_TYPE, MFMediaType_Video));
      check(type->SetGUID(MF_MT_SUBTYPE, MFVideoFormat_ARGB32));
      check(MFSetAttributeSize(type.Get(), MF_MT_FRAME_SIZE, 320, 240));
      check(MFSetAttributeRatio(type.Get(), MF_MT_FRAME_RATE, 30000, 1001));
      check(MFSetAttributeRatio(type.Get(), MF_MT_PIXEL_ASPECT_RATIO, 1, 1));
      check(
          type->SetUINT32(MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive));
      check(type->SetUINT32(MF_MT_ALL_SAMPLES_INDEPENDENT, TRUE));
      check(type->SetUINT32(MF_MT_FIXED_SIZE_SAMPLES, TRUE));
      check(type->SetUINT32(MF_MT_SAMPLE_SIZE, 320 * 240 * 4));
      check(type->SetUINT32(MF_MT_DEFAULT_STRIDE, 320 * 4));
      check(type->SetUINT32(MF_MT_VIDEO_PRIMARIES, MFVideoPrimaries_BT709));
      check(type->SetUINT32(MF_MT_TRANSFER_FUNCTION, MFVideoTransFunc_sRGB));
      check(type->SetUINT32(MF_MT_VIDEO_NOMINAL_RANGE, MFNominalRange_0_255));
      IMFMediaType *types[] = {type.Get()};
      check(MFCreateStreamDescriptor(0, 1, types, &descriptor));
      ComPtr<IMFMediaTypeHandler> handler;
      check(descriptor->GetMediaTypeHandler(&handler));
      check(handler->SetCurrentMediaType(type.Get()));
      owner = source;
      return S_OK;
    } catch (Failure e) {
      return e.hr;
    } catch (...) {
      return E_OUTOFMEMORY;
    }
  }
  SYNTHETIC_EVENTS
  STDMETHODIMP GetMediaSource(IMFMediaSource **out) override {
    if (!out)
      return E_POINTER;
    std::lock_guard<std::mutex> g(lock);
    *out = nullptr;
    return owner ? owner.CopyTo(out) : MF_E_SHUTDOWN;
  }
  STDMETHODIMP GetStreamDescriptor(IMFStreamDescriptor **out) override {
    if (!out)
      return E_POINTER;
    std::lock_guard<std::mutex> g(lock);
    *out = nullptr;
    return descriptor ? descriptor.CopyTo(out) : MF_E_SHUTDOWN;
  }
  HRESULT start(const PROPVARIANT *position) {
    std::lock_guard<std::mutex> g(lock);
    if (!events)
      return MF_E_SHUTDOWN;
    running = true;
    next = Clock::now();
    return events->QueueEventParamVar(MEStreamStarted, GUID_NULL, S_OK,
                                      position);
  }
  HRESULT stop() {
    std::lock_guard<std::mutex> g(lock);
    if (!events)
      return MF_E_SHUTDOWN;
    running = false;
    return events->QueueEventParamVar(MEStreamStopped, GUID_NULL, S_OK,
                                      nullptr);
  }
  void shutdown() {
    ComPtr<IMFMediaEventQueue> q;
    {
      std::lock_guard<std::mutex> g(lock);
      running = false;
      q = std::move(events);
      owner.Reset();
      descriptor.Reset();
    }
    if (q)
      q->Shutdown();
  }
  STDMETHODIMP RequestSample(IUnknown *token) override {
    try {
      Clock::time_point due;
      {
        std::lock_guard<std::mutex> g(lock);
        if (!events)
          return MF_E_SHUTDOWN;
        if (!running)
          return MF_E_INVALIDREQUEST;
        due = next;
        next =
            std::max(next, Clock::now()) + std::chrono::nanoseconds(33366667);
      }
      // Pacing stays on MF workers. Stop and Shutdown can invalidate requests.
      std::this_thread::sleep_until(due);
      std::unique_lock<std::mutex> g(lock);
      if (!events)
        return MF_E_SHUTDOWN;
      if (!running)
        return MF_E_INVALIDREQUEST;
      if (disconnect_after && sequence >= disconnect_after) {
        running = false;
        auto source = owner;
        g.unlock();
        fputs("SYNTHETIC_MF_DISCONNECT_INJECTED\n", stderr);
        return source->QueueEvent(MEError, GUID_NULL,
                                  MF_E_VIDEO_RECORDING_DEVICE_INVALIDATED,
                                  nullptr);
      }
      uint32_t live = synthetic_textures.load();
      do {
        if (live >= 16)
          return MF_E_NOTACCEPTING;
      } while (!synthetic_textures.compare_exchange_weak(live, live + 1));
      auto credit = Make<TextureCredit>();
      if (!credit) {
        --synthetic_textures;
        return E_OUTOFMEMORY;
      }
      D3D11_TEXTURE2D_DESC desc{};
      desc.Width = 320;
      desc.Height = 240;
      desc.MipLevels = desc.ArraySize = 1;
      desc.Format = DXGI_FORMAT_B8G8R8A8_UNORM;
      desc.SampleDesc.Count = 1;
      desc.BindFlags = D3D11_BIND_RENDER_TARGET | D3D11_BIND_SHADER_RESOURCE;
      ComPtr<ID3D11Texture2D> texture;
      check(device->CreateTexture2D(&desc, nullptr, &texture));
      check(texture->SetPrivateDataInterface(texture_credit, credit.Get()));
      ComPtr<ID3D11RenderTargetView> target;
      check(device->CreateRenderTargetView(texture.Get(), nullptr, &target));
      const float colors[4][4] = {
          {1, 0, 0, 1}, {0, 1, 0, 1}, {0, 0, 1, 1}, {1, 1, 1, 1}};
      for (int i = 0; i < 4; i++) {
        D3D11_RECT rect{(i % 2) * 160, (i / 2) * 120, (i % 2 + 1) * 160,
                        (i / 2 + 1) * 120};
        context->ClearView(target.Get(), colors[(i + pattern) % 4], &rect, 1);
      }
      if (pattern) {
        // Asymmetric landmarks distinguish stretch, crop and orientation.
        const float cyan[] = {0, 1, 1, 1}, yellow[] = {1, 1, 0, 1},
                    magenta[] = {1, 0, 1, 1};
        D3D11_RECT vertical{40, 0, 56, 240}, horizontal{0, 90, 320, 102};
        D3D11_RECT border[] = {{0, 0, 320, 8},
                               {0, 232, 320, 240},
                               {0, 0, 8, 240},
                               {312, 0, 320, 240}};
        context->ClearView(target.Get(), cyan, &vertical, 1);
        context->ClearView(target.Get(), yellow, &horizontal, 1);
        context->ClearView(target.Get(), magenta, border, 4);
      }
      ComPtr<IMFMediaBuffer> buffer;
      check(MFCreateDXGISurfaceBuffer(__uuidof(ID3D11Texture2D), texture.Get(),
                                      0, FALSE, &buffer));
      ComPtr<IMFSample> sample;
      check(MFCreateSample(&sample));
      check(sample->AddBuffer(buffer.Get()));
      check(sample->SetSampleTime(sequence * 10000000ull * 1001 / 30000));
      check(sample->SetSampleDuration(10000000ull * 1001 / 30000));
      ++sequence;
      if (token)
        check(sample->SetUnknown(MFSampleExtension_Token, token));
      return events->QueueEventParamUnk(MEMediaSample, GUID_NULL, S_OK,
                                        sample.Get());
    } catch (Failure e) {
      return e.hr;
    } catch (...) {
      return E_OUTOFMEMORY;
    }
  }
};
class SyntheticSource final
    : public RuntimeClass<
          RuntimeClassFlags<ClassicCom>,
          ChainInterfaces<IMFMediaSource, IMFMediaEventGenerator>, FtmBase> {
  std::mutex lock;
  ComPtr<IMFMediaEventQueue> events;
  ComPtr<SyntheticStream> stream;
  ComPtr<IMFPresentationDescriptor> descriptor;
  bool announced = false;

public:
  SyntheticSource() { ++synthetic_sources; }
  ~SyntheticSource() {
    Shutdown();
    --synthetic_sources;
  }
  HRESULT initialize(ID3D11Device *device, uint64_t disconnect,
                     uint64_t palette) {
    try {
      check(MFCreateEventQueue(&events));
      stream = Make<SyntheticStream>();
      if (!stream)
        return E_OUTOFMEMORY;
      check(stream->initialize(this, device, disconnect, palette));
      ComPtr<IMFStreamDescriptor> d;
      check(stream->GetStreamDescriptor(&d));
      IMFStreamDescriptor *streams[] = {d.Get()};
      check(MFCreatePresentationDescriptor(1, streams, &descriptor));
      check(descriptor->SelectStream(0));
      return S_OK;
    } catch (Failure e) {
      Shutdown();
      return e.hr;
    } catch (...) {
      Shutdown();
      return E_OUTOFMEMORY;
    }
  }
  SYNTHETIC_EVENTS
  STDMETHODIMP GetCharacteristics(DWORD *out) override {
    if (!out)
      return E_POINTER;
    std::lock_guard<std::mutex> g(lock);
    if (!events)
      return MF_E_SHUTDOWN;
    *out = MFMEDIASOURCE_IS_LIVE;
    return S_OK;
  }
  STDMETHODIMP
  CreatePresentationDescriptor(IMFPresentationDescriptor **out) override {
    if (!out)
      return E_POINTER;
    std::lock_guard<std::mutex> g(lock);
    *out = nullptr;
    return descriptor ? descriptor->Clone(out) : MF_E_SHUTDOWN;
  }
  STDMETHODIMP Start(IMFPresentationDescriptor *pd, const GUID *format,
                     const PROPVARIANT *position) override {
    if (!pd || !position)
      return E_POINTER;
    if (format && *format != GUID_NULL)
      return MF_E_UNSUPPORTED_TIME_FORMAT;
    std::lock_guard<std::mutex> g(lock);
    if (!events)
      return MF_E_SHUTDOWN;
    HRESULT hr = events->QueueEventParamUnk(
        announced ? MEUpdatedStream : MENewStream, GUID_NULL, S_OK,
        static_cast<IMFMediaStream *>(stream.Get()));
    if (FAILED(hr))
      return hr;
    announced = true;
    hr = stream->start(position);
    if (FAILED(hr))
      return hr;
    return events->QueueEventParamVar(MESourceStarted, GUID_NULL, S_OK,
                                      position);
  }
  STDMETHODIMP Stop() override {
    std::lock_guard<std::mutex> g(lock);
    if (!events)
      return MF_E_SHUTDOWN;
    HRESULT hr = stream->stop();
    if (FAILED(hr))
      return hr;
    return events->QueueEventParamVar(MESourceStopped, GUID_NULL, S_OK,
                                      nullptr);
  }
  STDMETHODIMP Pause() override { return MF_E_INVALID_STATE_TRANSITION; }
  STDMETHODIMP Shutdown() override {
    ComPtr<IMFMediaEventQueue> q;
    ComPtr<SyntheticStream> s;
    {
      std::lock_guard<std::mutex> g(lock);
      q = std::move(events);
      s = std::move(stream);
      descriptor.Reset();
    }
    if (s)
      s->shutdown();
    if (q)
      q->Shutdown();
    return S_OK;
  }
};
#undef SYNTHETIC_EVENTS
static HRESULT make_synthetic_camera(ID3D11Device *device, IMFMediaSource **out,
                                     uint64_t disconnect = 0,
                                     bool vary_pattern = false) {
  auto source = Make<SyntheticSource>();
  if (!source)
    return E_OUTOFMEMORY;
  HRESULT hr = source->initialize(device, disconnect,
                                  vary_pattern ? ++synthetic_pattern_epoch : 0);
  if (FAILED(hr))
    return hr;
  return source.CopyTo(out);
}
} // namespace
extern "C" uint64_t ferro_windows_synthetic_pattern_epoch() {
  return synthetic_pattern_epoch.load();
}
extern "C" void ferro_windows_synthetic_usage(uint32_t *sources,
                                              uint32_t *textures,
                                              uint32_t *workers) {
  *sources = synthetic_sources.load();
  *textures = synthetic_textures.load();
  *workers = active_sessions.load();
}
