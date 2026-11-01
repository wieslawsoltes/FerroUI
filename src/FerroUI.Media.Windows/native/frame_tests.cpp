// Included only by the opt-in native-frame-tests feature. All pixels originate
// in independent D3D11 GPU clears; diagnostic photo reads are explicit.
static void require(bool value) {
  if (!value)
    throw Failure{E_FAIL};
}
static std::shared_ptr<Session> test_device(D3D_DRIVER_TYPE driver) {
  auto s = std::make_shared<Session>();
  check(D3D11CreateDevice(nullptr, driver, nullptr,
                          D3D11_CREATE_DEVICE_BGRA_SUPPORT |
                              (driver == D3D_DRIVER_TYPE_HARDWARE
                                   ? D3D11_CREATE_DEVICE_VIDEO_SUPPORT
                                   : 0),
                          nullptr, 0, D3D11_SDK_VERSION, &s->device, nullptr,
                          &s->context));
  s->width = 8;
  s->height = 8;
  s->color = 1;
  return s;
}
static Packet test_packet(Session &s,
                          UINT bind = D3D11_BIND_SHADER_RESOURCE |
                                      D3D11_BIND_RENDER_TARGET,
                          DXGI_FORMAT format = DXGI_FORMAT_B8G8R8A8_UNORM) {
  D3D11_TEXTURE2D_DESC desc{};
  desc.Width = s.width;
  desc.Height = s.height;
  desc.MipLevels = 1;
  desc.ArraySize = 2;
  desc.Format = format;
  desc.SampleDesc.Count = 1;
  desc.Usage = D3D11_USAGE_DEFAULT;
  desc.BindFlags = bind;
  ComPtr<ID3D11Texture2D> texture;
  check(s.device->CreateTexture2D(&desc, nullptr, &texture));
  if (bind & D3D11_BIND_RENDER_TARGET) {
    D3D11_RENDER_TARGET_VIEW_DESC view{};
    view.Format = desc.Format;
    view.ViewDimension = D3D11_RTV_DIMENSION_TEXTURE2DARRAY;
    view.Texture2DArray.FirstArraySlice = 1;
    view.Texture2DArray.ArraySize = 1;
    ComPtr<ID3D11RenderTargetView> target;
    check(s.device->CreateRenderTargetView(texture.Get(), &view, &target));
    ComPtr<ID3D11DeviceContext1> context;
    check(s.context.As(&context));
    const float colors[4][4] = {
        {1, 0, 0, 1}, {0, 1, 0, 1}, {0, 0, 1, 1}, {1, 1, 1, 1}};
    for (int i = 0; i < 4; i++) {
      D3D11_RECT rect{(i % 2) * 4, (i / 2) * 4, (i % 2) * 4 + 4,
                      (i / 2) * 4 + 4};
      context->ClearView(target.Get(), colors[i], &rect, 1);
    }
  }
  ComPtr<IMFMediaBuffer> buffer;
  check(MFCreateDXGISurfaceBuffer(__uuidof(ID3D11Texture2D), texture.Get(), 1,
                                  FALSE, &buffer));
  Packet p;
  p.epoch = 7;
  p.pts = 12345;
  check(MFCreateSample(&p.sample));
  check(p.sample->AddBuffer(buffer.Get()));
  return p;
}
extern "C" int ferro_windows_test_frames() {
  HRESULT initialized = CoInitializeEx(nullptr, COINIT_MULTITHREADED);
  const char *stage = "initialization";
  try {
    check(initialized);
    runtime();
    stage = "independent hardware producer";
    auto s = test_device(D3D_DRIVER_TYPE_HARDWARE);
    auto packet = test_packet(*s);
    auto f = video_frame(*s, packet);
    require(f->slice == 1 && f->info.bytes == 512 && f->info.epoch == 7 &&
            f->info.timestamp_ns == 1234500);
    packet.sample.Reset(); // The frame alone must retain the native sample.
    auto deadline = Clock::now() + std::chrono::seconds(5);
    while (!ready(*f)) {
      require(Clock::now() < deadline);
      std::this_thread::sleep_for(std::chrono::milliseconds(1));
    }
    stage = "diagnostic quadrant pixels";
    std::vector<uint8_t> rgb(8 * 8 * 3);
    require(ferro_windows_photo_rgb(f.get(), rgb.data(), rgb.size()) == 1);
    const uint8_t expected[4][3] = {
        {255, 0, 0}, {0, 255, 0}, {0, 0, 255}, {255, 255, 255}};
    for (int i = 0; i < 4; i++)
      for (int c = 0; c < 3; c++)
        require(rgb[((i / 2 * 4 + 2) * 8 + i % 2 * 4 + 2) * 3 + c] ==
                expected[i][c]);
    stage = "incompatible storage rejection";
    auto reject = [&](Session &session, const Packet &p) {
      bool rejected = false;
      try {
        video_frame(session, p);
      } catch (Failure e) {
        rejected = e.hr == MF_E_UNSUPPORTED_D3D_TYPE;
      }
      require(rejected);
    };
    auto missing_bind = test_packet(*s, D3D11_BIND_RENDER_TARGET);
    reject(*s, missing_bind);
    auto other = test_device(D3D_DRIVER_TYPE_HARDWARE);
    auto foreign = test_packet(*other);
    reject(*s, foreign);
    auto wrong_size = test_packet(*s);
    s->width = 16;
    reject(*s, wrong_size);
    s->width = 8;
    auto xrgb =
        test_packet(*s, D3D11_BIND_SHADER_RESOURCE | D3D11_BIND_RENDER_TARGET,
                    DXGI_FORMAT_B8G8R8X8_UNORM);
    reject(*s, xrgb);
    stage = "paused read-ahead publication";
    s->epoch = 7;
    for (int i = 0; i < 3; i++) {
      auto queued = test_packet(*s);
      queued.pts = i * 333333;
      auto output = video_frame(*s, queued);
      auto until = Clock::now() + std::chrono::seconds(5);
      while (!ready(*output)) {
        require(Clock::now() < until);
        std::this_thread::sleep_for(std::chrono::milliseconds(1));
      }
      s->video.push_back(std::move(output));
    }
    present_video(*s, 0);
    require(s->latest && s->latest->info.timestamp_ns == 0 &&
            s->video.size() == 2);
    present_video(*s, 0);
    require(s->latest->info.timestamp_ns == 0 && s->video.size() == 2);
    s->playing = true;
    present_video(*s, 1000000);
    require(s->video.empty() && s->latest->info.timestamp_ns == 66666600);
    stage = "software device rejection";
    auto software = test_device(D3D_DRIVER_TYPE_WARP);
    bool rejected = false;
    try {
      prepare(software);
    } catch (Failure e) {
      rejected = e.hr == MF_E_UNSUPPORTED_D3D_TYPE;
    }
    require(rejected);
    puts("NATIVE_D3D11_FRAMES_PASS gpu_quadrants=4 retained_array_slice=1 "
         "incompatible_storage=rejected software=rejected "
         "paused_read_ahead=held");
    CoUninitialize();
    return 0;
  } catch (Failure e) {
    fprintf(stderr, "NATIVE_D3D11_FRAMES_FAILED stage=%s %s\n", stage,
            hresult(e.hr).c_str());
  } catch (...) {
    fprintf(stderr, "NATIVE_D3D11_FRAMES_FAILED stage=%s\n", stage);
  }
  if (SUCCEEDED(initialized))
    CoUninitialize();
  return 1;
}

extern "C" int ferro_windows_test_alias(intptr_t display,
                                        const FerroWindowsEgl *egl,
                                        const intptr_t *gl) {
  using Attach = void(WINAPI *)(unsigned int, unsigned int, unsigned int,
                                unsigned int, int);
  using Status = unsigned int(WINAPI *)(unsigned int);
  using Read =
      void(WINAPI *)(int, int, int, int, unsigned int, unsigned int, void *);
  HRESULT initialized = CoInitializeEx(nullptr, COINIT_MULTITHREADED);
  unsigned int framebuffer = 0;
  void *alias = nullptr;
  try {
    check(initialized);
    runtime();
    auto s = std::make_shared<Session>();
    s->device.Attach((ID3D11Device *)ferro_windows_device(display, egl));
    require(s->device != nullptr);
    s->device->GetImmediateContext(&s->context);
    s->width = 8;
    s->height = 8;
    s->color = 1;
    auto packet = test_packet(*s);
    auto frame = video_frame(*s, packet);
    packet.sample.Reset();
    auto deadline = Clock::now() + std::chrono::seconds(5);
    while (!ready(*frame)) {
      require(Clock::now() < deadline);
      std::this_thread::sleep_for(std::chrono::milliseconds(1));
    }
    unsigned int texture = 0;
    alias = ferro_windows_alias(frame.get(), display, egl, &texture);
    require(alias && texture);
    reinterpret_cast<TextureArray>(gl[0])(1, &framebuffer);
    reinterpret_cast<BindTexture>(gl[1])(0x8d40, framebuffer);
    reinterpret_cast<Attach>(gl[2])(0x8d40, 0x8ce0, 0x0de1, texture, 0);
    require(reinterpret_cast<Status>(gl[3])(0x8d40) == 0x8cd5);
    uint8_t pixels[8 * 8 * 4]{};
    reinterpret_cast<Read>(gl[4])(0, 0, 8, 8, 0x1908, 0x1401, pixels);
    require(entry<GetError>(*egl, 10)() == 0);
    const uint8_t expected[4][4] = {{255, 0, 0, 255},
                                    {0, 255, 0, 255},
                                    {0, 0, 255, 255},
                                    {255, 255, 255, 255}};
    for (int i = 0; i < 4; i++)
      for (int c = 0; c < 4; c++)
        require(pixels[((i / 2 * 4 + 2) * 8 + i % 2 * 4 + 2) * 4 + c] ==
                expected[i][c]);
    reinterpret_cast<BindTexture>(gl[1])(0x8d40, 0);
    reinterpret_cast<TextureArray>(gl[5])(1, &framebuffer);
    framebuffer = 0;
    ferro_windows_alias_release(alias, 1);
    alias = nullptr;
    puts("NATIVE_ANGLE_ALIAS_PASS gpu_quadrants=4 array_slice=1 "
         "diagnostic_readback=true");
    CoUninitialize();
    return 0;
  } catch (Failure e) {
    fprintf(stderr, "NATIVE_ANGLE_ALIAS_FAILED %s\n", hresult(e.hr).c_str());
  } catch (...) {
    fputs("NATIVE_ANGLE_ALIAS_FAILED\n", stderr);
  }
  if (framebuffer) {
    reinterpret_cast<BindTexture>(gl[1])(0x8d40, 0);
    reinterpret_cast<TextureArray>(gl[5])(1, &framebuffer);
  }
  if (alias)
    ferro_windows_alias_release(alias, 1);
  if (SUCCEEDED(initialized))
    CoUninitialize();
  return 1;
}
