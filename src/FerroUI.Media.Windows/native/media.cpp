// Windows SDK and system codecs. Preview never maps native video buffers.
#include "media.h"
#include <windows.h>
#include <mfapi.h>
#include <mfidl.h>
#include <mfreadwrite.h>
#include <mferror.h>
#include <d3d11.h>
#include <d3d11_4.h>
#include <cstdio>
#include <dxgi1_2.h>
#include <dxgi1_4.h>
#include <psapi.h>
#include <mmdeviceapi.h>
#include <audioclient.h>
#include <propvarutil.h>
#include <wrl/client.h>
#include <wrl/implements.h>
#include <atomic>
#include <mutex>
#include <condition_variable>
#include <thread>
#include <deque>
#include <vector>
#include <string>
#include <memory>
#include <chrono>
#include <algorithm>
#include <cstring>
#include <cmath>
#include <set>
using Microsoft::WRL::ComPtr;
using Clock=std::chrono::steady_clock;
struct Failure {HRESULT hr;};
static void check(HRESULT hr){if(FAILED(hr))throw Failure{hr};}
static std::atomic<int> active_sessions{0};
#ifdef FERROUI_SYNTHETIC_CAMERA
static std::atomic<uint32_t> fixture_frames{0},fixture_aliases{0},fixture_owners{0},fixture_callbacks{0};
struct FixtureFrame {
    FixtureFrame(){++fixture_frames;}FixtureFrame(const FixtureFrame&){++fixture_frames;}
    ~FixtureFrame(){--fixture_frames;}
};
struct FixtureAlias {FixtureAlias(){++fixture_aliases;}~FixtureAlias(){--fixture_aliases;}};
#endif
static std::once_flag mf_once;
static void runtime(){std::call_once(mf_once,[]{check(MFStartup(MF_VERSION,MFSTARTUP_FULL));});} // process lifetime; leases may outlive sessions
static std::string utf8(const std::wstring &s){if(s.empty())return {};int n=WideCharToMultiByte(CP_UTF8,0,s.data(),(int)s.size(),nullptr,0,nullptr,nullptr);std::string v(n,'\0');WideCharToMultiByte(CP_UTF8,0,s.data(),(int)s.size(),v.data(),n,nullptr,nullptr);return v;}
static std::string hresult(HRESULT hr){wchar_t *text=nullptr;FormatMessageW(FORMAT_MESSAGE_ALLOCATE_BUFFER|FORMAT_MESSAGE_FROM_SYSTEM|FORMAT_MESSAGE_IGNORE_INSERTS,nullptr,hr,0,(wchar_t*)&text,0,nullptr);std::string result=text?utf8(text):"Native media error";if(text)LocalFree(text);char code[32];snprintf(code,sizeof(code)," (0x%08lx)",(unsigned long)hr);return result+code;}
#include "mp4_geometry.cpp"
#ifdef FERROUI_SYNTHETIC_CAMERA
#include "synthetic_camera.cpp"
extern "C" int ferro_windows_fixture_process_stats(void* device,uint64_t* out){
    PROCESS_MEMORY_COUNTERS_EX memory{};memory.cb=sizeof(memory);DWORD handles=0;
    if(!GetProcessMemoryInfo(GetCurrentProcess(),(PROCESS_MEMORY_COUNTERS*)&memory,sizeof(memory))||!GetProcessHandleCount(GetCurrentProcess(),&handles))return 0;
    out[0]=memory.PrivateUsage;out[1]=memory.WorkingSetSize;out[2]=UINT64_MAX;out[3]=handles;
    ComPtr<IDXGIDevice> dxgi;ComPtr<IDXGIAdapter> adapter;ComPtr<IDXGIAdapter3> adapter3;
    if(SUCCEEDED(((ID3D11Device*)device)->QueryInterface(IID_PPV_ARGS(&dxgi)))&&SUCCEEDED(dxgi->GetAdapter(&adapter))&&SUCCEEDED(adapter.As(&adapter3))){
        DXGI_QUERY_VIDEO_MEMORY_INFO gpu{};if(SUCCEEDED(adapter3->QueryVideoMemoryInfo(0,DXGI_MEMORY_SEGMENT_GROUP_LOCAL,&gpu)))out[2]=gpu.CurrentUsage;
    }
    return 1;
}
// Read-only attribution, opt-in fixture only. Do not enumerate/dereference
// arbitrary process heaps: a native worker could destroy one concurrently.
extern "C" void ferro_windows_fixture_memory_stats(uint64_t* out){
    struct Summary {DWORD cb;SIZE_T allocated,committed,reserved,max_reserve;};
    using HeapSummaryFn=BOOL(WINAPI*)(HANDLE,DWORD,Summary*);
    auto summarize=(HeapSummaryFn)GetProcAddress(GetModuleHandleW(L"kernel32.dll"),"HeapSummary");
    Summary heap{};heap.cb=sizeof(heap);out[0]=out[1]=UINT64_MAX;
    if(summarize&&summarize(GetProcessHeap(),0,&heap)){out[0]=heap.allocated;out[1]=heap.committed;}
    out[2]=out[3]=out[4]=0;
    MEMORY_BASIC_INFORMATION region{};uintptr_t at=0;
    while(VirtualQuery((void*)at,&region,sizeof(region))==sizeof(region)){
        if(region.State==MEM_COMMIT){
            if(region.Type==MEM_PRIVATE)out[2]+=region.RegionSize;
            if(region.Type==MEM_MAPPED)out[3]+=region.RegionSize;
            if(region.Type==MEM_IMAGE)out[4]+=region.RegionSize;
        }
        auto next=(uintptr_t)region.BaseAddress+region.RegionSize;if(next<=at)break;at=next;
    }
}
extern "C" void ferro_windows_fixture_retirement_stats(void* ptr,uint32_t* out){
    out[0]=fixture_frames.load();out[1]=fixture_aliases.load();
    ComPtr<ID3D11DeviceContext> context;((ID3D11Device*)ptr)->GetImmediateContext(&context);
    std::set<ID3D11Resource*> sources;
    auto note=[&](ID3D11View* view){if(!view)return;ComPtr<ID3D11Resource> resource;view->GetResource(&resource);
        IUnknown* credit=nullptr;UINT size=sizeof(credit);
        if(SUCCEEDED(resource->GetPrivateData(texture_credit,&size,&credit))&&credit){sources.insert(resource.Get());credit->Release();}
    };
    ID3D11ShaderResourceView* views[D3D11_COMMONSHADER_INPUT_RESOURCE_SLOT_COUNT]{};
    for(int stage=0;stage<3;stage++){
        if(stage==0)context->VSGetShaderResources(0,D3D11_COMMONSHADER_INPUT_RESOURCE_SLOT_COUNT,views);
        if(stage==1)context->PSGetShaderResources(0,D3D11_COMMONSHADER_INPUT_RESOURCE_SLOT_COUNT,views);
        if(stage==2)context->CSGetShaderResources(0,D3D11_COMMONSHADER_INPUT_RESOURCE_SLOT_COUNT,views);
        for(auto& view:views){if(view){note(view);view->Release();view=nullptr;}}
    }
    ID3D11RenderTargetView* targets[D3D11_SIMULTANEOUS_RENDER_TARGET_COUNT]{};
    context->OMGetRenderTargets(D3D11_SIMULTANEOUS_RENDER_TARGET_COUNT,targets,nullptr);
    for(auto target:targets)if(target){note(target);target->Release();}
    out[2]=(uint32_t)sources.size();
}
#endif
struct Session;
struct FerroWindowsFrame {
#ifdef FERROUI_SYNTHETIC_CAMERA
    FixtureFrame tracked;
#endif
    ComPtr<IMFSample> sample;
    ComPtr<ID3D11Texture2D> texture;
    ComPtr<ID3D11DeviceContext> context;
    ComPtr<ID3D11Query> ready;
    UINT slice=0;
    FerroWindowsFrameInfo info{};
    std::weak_ptr<Session> owner;
};
struct Packet {HRESULT status=S_OK;DWORD stream=0,flags=0;LONGLONG pts=0;uint64_t epoch=0;ComPtr<IMFSample> sample;};
#include "recording.inc"
struct Session;
class ReaderCallback final:public IMFSourceReaderCallback {
    std::atomic<ULONG> refs{1};std::weak_ptr<Session> session;
public:
    explicit ReaderCallback(std::weak_ptr<Session> s):session(std::move(s)){
#ifdef FERROUI_SYNTHETIC_CAMERA
        ++fixture_callbacks;
#endif
    }
    ~ReaderCallback(){
#ifdef FERROUI_SYNTHETIC_CAMERA
        --fixture_callbacks;
#endif
    }
    STDMETHODIMP QueryInterface(REFIID id,void **out)override {if(!out)return E_POINTER;*out=nullptr;if(id==__uuidof(IUnknown)||id==__uuidof(IMFSourceReaderCallback)){*out=(IMFSourceReaderCallback*)this;AddRef();return S_OK;}return E_NOINTERFACE;}
    STDMETHODIMP_(ULONG) AddRef()override{return ++refs;}
    STDMETHODIMP_(ULONG) Release()override{ULONG n=--refs;if(!n)delete this;return n;}
    STDMETHODIMP OnReadSample(HRESULT status,DWORD stream,DWORD flags,LONGLONG pts,IMFSample *sample)override;
    STDMETHODIMP OnFlush(DWORD stream)override;
    STDMETHODIMP OnEvent(DWORD stream,IMFMediaEvent *event)override;
};
struct AudioChunk {std::vector<BYTE> bytes;UINT offset=0;LONGLONG pts=0;};
static int native_error_kind(HRESULT hr){
    if(hr==E_ACCESSDENIED)return 2;
    if(hr==MF_E_UNSUPPORTED_D3D_TYPE||hr==E_NOTIMPL)return 1;
    if(hr==MF_E_VIDEO_RECORDING_DEVICE_INVALIDATED||hr==DXGI_ERROR_DEVICE_REMOVED||
       hr==DXGI_ERROR_DEVICE_RESET||hr==DXGI_ERROR_DEVICE_HUNG)return 3;
    return 0;
}
struct Session : std::enable_shared_from_this<Session> {
#ifdef FERROUI_SYNTHETIC_CAMERA
    Session(){++fixture_owners;}~Session(){--fixture_owners;}
#endif
    std::mutex lock;std::condition_variable changed;std::atomic<bool> closed{false};
    FerroWindowsState state{};
    std::wstring source;bool camera=false,resource_file=false;
    const char *operation="native initialization";
    UINT requested_w=0,requested_h=0,requested_n=0,requested_d=1;
    int intent=0;double volume=1.;bool muted=false;
    double requested_rate=1.,rate=1.;
    bool rate_supported=false;
    uint64_t epoch=1,pending_seek=0,request_epoch=1,sequence=0;LONGLONG seek_target=0;
    bool in_flight=false,flushed=false,packet_ready=false;
    HRESULT event_error=S_OK;
    Packet packet;
    bool recording_encoder = false;
    bool recording_format = false;
    std::shared_ptr<Recording> recording;
    ComPtr<ID3D11Device> device;ComPtr<ID3D11DeviceContext> context;
    ComPtr<IMFDXGIDeviceManager> manager;ComPtr<IMFSourceReader> reader;ComPtr<IMFMediaSource> camera_source;
    ComPtr<ReaderCallback> callback;
    ComPtr<IAudioClient> audio;ComPtr<IAudioRenderClient> render;ComPtr<IAudioClock> audio_clock;ComPtr<ISimpleAudioVolume> audio_volume;
    ComPtr<IAudioClockAdjustment> audio_rate;
    uint64_t audio_frames_written=0;
    bool use_audio_frame_clock=false;
    UINT audio_capacity=0;UINT64 audio_frequency=0;bool has_video=true,has_audio=false,playing=false,audio_started=false;
    bool video_eos=false,audio_eos=false,prepared_frame=false,ever_played=false;LONGLONG clock_base=0,position=0;bool clock_based=false;
    Clock::time_point started;
    std::deque<AudioChunk> pcm;size_t pcm_bytes=0;
    std::deque<std::unique_ptr<FerroWindowsFrame>> video;
    std::unique_ptr<FerroWindowsFrame> latest;
    UINT color=0,rotation=0,width=0,height=0;
    UINT mirrored=0,visible_x=0,visible_y=0,visible_w=0,visible_h=0;
    mp4::Geometry geometry;
    void fail(HRESULT hr,const char *detail=nullptr){
        std::lock_guard<std::mutex> guard(lock);
        if(FAILED(event_error))hr=event_error;
        state.state=5;state.error_kind=native_error_kind(hr);
        std::string error=detail?detail:std::string(operation)+": "+hresult(hr);
        strncpy_s(state.error,error.c_str(),_TRUNCATE);
    }
    LONGLONG clock_position(){
        if(!playing)return position;
        if(has_audio){
            if(!audio_started||!clock_based)return position;
            if(use_audio_frame_clock){
                UINT padding=0;check(audio->GetCurrentPadding(&padding));
                // Padding is counted in input PCM frames. At any endpoint rate,
                // consuming one input frame advances media time by 1/48000 s.
                return clock_base+(LONGLONG)((long double)(audio_frames_written-std::min<uint64_t>(padding,audio_frames_written))*10000000/48000);
            }
            UINT64 p=0;check(audio_clock->GetPosition(&p,nullptr));return clock_base+(LONGLONG)((long double)p*10000000/audio_frequency);
        }
        return clock_base+(LONGLONG)(std::chrono::duration_cast<std::chrono::nanoseconds>(Clock::now()-started).count()/100.*rate);
    }
};
struct FerroWindowsSession {std::shared_ptr<Session> value;};
#ifdef FERROUI_SYNTHETIC_CAMERA
extern "C" void ferro_windows_fixture_owners(uint32_t* out){
    out[0]=fixture_owners.load();out[1]=fixture_callbacks.load();out[2]=synthetic_streams.load();out[3]=fixture_frames.load();out[4]=fixture_aliases.load();
}
#endif
STDMETHODIMP ReaderCallback::OnReadSample(HRESULT status,DWORD stream,DWORD flags,LONGLONG pts,IMFSample *sample){
    if(auto s=session.lock()){
        std::lock_guard<std::mutex> guard(s->lock);if(s->closed.load())return S_OK;
        // Source failures can arrive independently of the outstanding read.
        // Latch them separately from the replaceable normal-sample slot.
        if(FAILED(status)||(flags&MF_SOURCE_READERF_ERROR)){
            if(SUCCEEDED(s->event_error))s->event_error=FAILED(status)?status:E_FAIL;
        }else if(SUCCEEDED(s->event_error)){
            s->packet={status,stream,flags,pts,s->request_epoch,sample};s->packet_ready=true;
        }
        s->in_flight=false;s->changed.notify_all();
    }
    return S_OK;
}
STDMETHODIMP ReaderCallback::OnFlush(DWORD stream){(void)stream;if(auto s=session.lock()){std::lock_guard<std::mutex> guard(s->lock);s->flushed=true;s->in_flight=false;s->changed.notify_all();}return S_OK;}
STDMETHODIMP ReaderCallback::OnEvent(DWORD stream,IMFMediaEvent *event){
    (void)stream;HRESULT status=S_OK;MediaEventType type=MEUnknown;
    HRESULT result=event?event->GetStatus(&status):E_POINTER;if(FAILED(result))status=result;
    if(event)event->GetType(&type);
    if(FAILED(status)||type==MEError){
        if(auto s=session.lock()){
            std::lock_guard<std::mutex> guard(s->lock);
            // Publish the first error on the worker, so concurrent normal
            // callbacks cannot overwrite failure with Playing/Paused state.
            if(!s->closed.load()&&SUCCEEDED(s->event_error)){
                s->event_error=FAILED(status)?status:E_FAIL;
                s->changed.notify_all();
            }
        }
    }
    return S_OK;
}
static void setup_audio(Session &s){
    ComPtr<IMMDeviceEnumerator> enumerator;check(CoCreateInstance(__uuidof(MMDeviceEnumerator),nullptr,CLSCTX_ALL,IID_PPV_ARGS(&enumerator)));
    ComPtr<IMMDevice> endpoint;check(enumerator->GetDefaultAudioEndpoint(eRender,eConsole,&endpoint));
    check(endpoint->Activate(__uuidof(IAudioClient),CLSCTX_ALL,nullptr,(void**)s.audio.GetAddressOf()));
    WAVEFORMATEX format{};format.wFormatTag=WAVE_FORMAT_PCM;format.nChannels=2;format.nSamplesPerSec=48000;format.wBitsPerSample=16;format.nBlockAlign=4;format.nAvgBytesPerSec=192000;
    constexpr DWORD flags=AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM|AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY;
    HRESULT adjustable=s.audio->Initialize(AUDCLNT_SHAREMODE_SHARED,flags|AUDCLNT_STREAMFLAGS_RATEADJUST,2000000,0,&format,nullptr);
    if(FAILED(adjustable)){
        s.audio.Reset();
        check(endpoint->Activate(__uuidof(IAudioClient),CLSCTX_ALL,nullptr,(void**)s.audio.GetAddressOf()));
        check(s.audio->Initialize(AUDCLNT_SHAREMODE_SHARED,flags,2000000,0,&format,nullptr));
    }else if(SUCCEEDED(s.audio->GetService(IID_PPV_ARGS(&s.audio_rate)))){
        bool slow=SUCCEEDED(s.audio_rate->SetSampleRate(24000.f));
        bool fast=SUCCEEDED(s.audio_rate->SetSampleRate(96000.f));
        check(s.audio_rate->SetSampleRate(48000.f));
        if(!slow||!fast)s.audio_rate.Reset();
    }
    check(s.audio->GetBufferSize(&s.audio_capacity));check(s.audio->GetService(IID_PPV_ARGS(&s.render)));check(s.audio->GetService(IID_PPV_ARGS(&s.audio_clock)));check(s.audio_clock->GetFrequency(&s.audio_frequency));check(s.audio->GetService(IID_PPV_ARGS(&s.audio_volume)));
    if(!s.audio_frequency||s.audio_capacity>48000)throw Failure{E_NOTIMPL};
}
static void video_type(Session &s){
    ComPtr<IMFMediaType> type;check(s.reader->GetCurrentMediaType(MF_SOURCE_READER_FIRST_VIDEO_STREAM,&type));
    GUID subtype;check(type->GetGUID(MF_MT_SUBTYPE,&subtype));if(subtype!=MFVideoFormat_RGB32&&subtype!=MFVideoFormat_ARGB32)throw Failure{MF_E_UNSUPPORTED_D3D_TYPE};
    check(MFGetAttributeSize(type.Get(),MF_MT_FRAME_SIZE,&s.width,&s.height));
    UINT32 primaries=0,transfer=0,range=0;
    check(type->GetUINT32(MF_MT_VIDEO_PRIMARIES,&primaries));check(type->GetUINT32(MF_MT_TRANSFER_FUNCTION,&transfer));check(type->GetUINT32(MF_MT_VIDEO_NOMINAL_RANGE,&range));
    if(primaries!=MFVideoPrimaries_BT709||range!=MFNominalRange_0_255)throw Failure{MF_E_UNSUPPORTED_D3D_TYPE};
    s.color=transfer==MFVideoTransFunc_sRGB?1:transfer==MFVideoTransFunc_709?2:0;
    if(!s.color||!s.width||!s.height||s.width>8192||s.height>8192)throw Failure{MF_E_UNSUPPORTED_D3D_TYPE};
    UINT32 n=1,d=1;MFGetAttributeRatio(type.Get(),MF_MT_PIXEL_ASPECT_RATIO,&n,&d);if(n!=d)throw Failure{E_NOTIMPL};
    s.rotation=0;type->GetUINT32(MF_MT_VIDEO_ROTATION,&s.rotation);
    if(s.rotation!=0&&s.rotation!=90&&s.rotation!=180&&s.rotation!=270)throw Failure{E_NOTIMPL};
    MFVideoArea aperture{};UINT32 copied=0;
    wchar_t geometry_diagnostic[2];
    if(s.geometry.known&&GetEnvironmentVariableW(L"FERROUI_MEDIA_GEOMETRY_DIAGNOSTICS",geometry_diagnostic,2)){
        fprintf(stderr,"NATIVE_GEOMETRY_OUTPUT texture=%ux%u rotation=%u encoded=%ux%u clean=%u,%u,%u,%u track=%u mirror=%u\n",s.width,s.height,s.rotation,s.geometry.width,s.geometry.height,s.geometry.x,s.geometry.y,s.geometry.w,s.geometry.h,s.geometry.rotation,s.geometry.mirror);
    }
    if(SUCCEEDED(type->GetBlob(MF_MT_MINIMUM_DISPLAY_APERTURE,(UINT8*)&aperture,sizeof(aperture),&copied)) && (aperture.OffsetX.value||aperture.OffsetX.fract||aperture.OffsetY.value||aperture.OffsetY.fract||aperture.Area.cx!=(LONG)s.width||aperture.Area.cy!=(LONG)s.height))throw Failure{E_NOTIMPL};
    s.visible_x=s.visible_y=s.mirrored=0;s.visible_w=s.width;s.visible_h=s.height;
    if(s.geometry.known){
        auto& g=s.geometry;
        if(s.width!=g.width||s.height!=g.height)throw Failure{E_NOTIMPL};
        // Native RGB output stays in coded coordinates. Exact clean aperture
        // and all eight track transforms are sampled by the shared GPU path.
        s.visible_x=g.x;s.visible_y=g.y;s.visible_w=g.w;s.visible_h=g.h;
        s.rotation=g.rotation;s.mirrored=g.mirror;
    }
    UINT32 fps_n=0,fps_d=1;MFGetAttributeRatio(type.Get(),MF_MT_FRAME_RATE,&fps_n,&fps_d);
    UINT32 scan = MFVideoInterlace_Unknown;
    type->GetUINT32(MF_MT_INTERLACE_MODE, &scan);
    std::lock_guard<std::mutex> guard(s.lock);s.state.width=s.rotation%180?s.visible_h:s.visible_w;s.state.height=s.rotation%180?s.visible_w:s.visible_h;s.state.fps_n=fps_n;s.state.fps_d=fps_d;
    s.recording_format = scan == MFVideoInterlace_Progressive && s.rotation == 0 &&
        recording_profile(s.width, s.height, fps_n, fps_d);
}
static void prepare(const std::shared_ptr<Session> &owner){
    Session &s=*owner;runtime();
    ComPtr<ID3D11VideoDevice> video_device;if(FAILED(s.device.As(&video_device)))throw Failure{MF_E_UNSUPPORTED_D3D_TYPE};
    if(!(s.device->GetCreationFlags()&D3D11_CREATE_DEVICE_VIDEO_SUPPORT))throw Failure{MF_E_UNSUPPORTED_D3D_TYPE};
    ComPtr<IDXGIDevice> dxgi;check(s.device.As(&dxgi));ComPtr<IDXGIAdapter> adapter;check(dxgi->GetAdapter(&adapter));ComPtr<IDXGIAdapter1> adapter1;check(adapter.As(&adapter1));DXGI_ADAPTER_DESC1 desc{};check(adapter1->GetDesc1(&desc));if(desc.Flags&DXGI_ADAPTER_FLAG_SOFTWARE)throw Failure{MF_E_UNSUPPORTED_D3D_TYPE};
    s.device->GetImmediateContext(&s.context);ComPtr<ID3D11Multithread> multithread;if(FAILED(s.context.As(&multithread)))throw Failure{MF_E_UNSUPPORTED_D3D_TYPE};multithread->SetMultithreadProtected(TRUE);
    // Probe the Windows 8+ entry point instead of making it a mandatory
    // application import on the framework's older Windows UI profiles.
    using CreateManager=HRESULT(WINAPI*)(UINT*,IMFDXGIDeviceManager**);
    auto create_manager=(CreateManager)GetProcAddress(GetModuleHandleW(L"mfplat.dll"),"MFCreateDXGIDeviceManager");
    if(!create_manager)throw Failure{E_NOTIMPL};
    UINT token;check(create_manager(&token,s.manager.GetAddressOf()));check(s.manager->ResetDevice(s.device.Get(),token));
    if(!s.camera){s.operation="local MP4 geometry inspection";s.geometry=mp4::inspect(s.source,s.closed);}
    ComPtr<IMFAttributes> attributes;check(MFCreateAttributes(&attributes,8));
    s.callback.Attach(new ReaderCallback(owner));check(attributes->SetUnknown(MF_SOURCE_READER_ASYNC_CALLBACK,s.callback.Get()));
    check(attributes->SetUnknown(MF_SOURCE_READER_D3D_MANAGER,s.manager.Get()));check(attributes->SetUINT32(MF_READWRITE_ENABLE_HARDWARE_TRANSFORMS,TRUE));
    check(attributes->SetUINT32(MF_SOURCE_READER_ENABLE_ADVANCED_VIDEO_PROCESSING,TRUE));
    check(attributes->SetUINT32(MF_SOURCE_READER_DISABLE_DXVA,FALSE));
    if(s.camera){
        s.recording_encoder = has_h264_encoder();
        s.operation="camera source activation";
#ifdef FERROUI_SYNTHETIC_CAMERA
        if(s.source==L"ferroui:synthetic-camera"||s.source==L"ferroui:synthetic-camera/disconnect"||s.source==L"ferroui:synthetic-camera/recovery"){
            check(make_synthetic_camera(s.device.Get(),&s.camera_source,s.source==L"ferroui:synthetic-camera/disconnect"?30:0,s.source==L"ferroui:synthetic-camera/recovery"));
        }else
#endif
        {
        ComPtr<IMFAttributes> selection;check(MFCreateAttributes(&selection,2));check(selection->SetGUID(MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE,MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_GUID));
        check(selection->SetString(MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_SYMBOLIC_LINK,s.source.c_str()));
        check(MFCreateDeviceSource(selection.Get(),&s.camera_source));
        }
        s.operation="camera Source Reader creation";check(MFCreateSourceReaderFromMediaSource(s.camera_source.Get(),attributes.Get(),&s.reader));
    }else check(MFCreateSourceReaderFromURL(s.source.c_str(),attributes.Get(),&s.reader));
    check(s.reader->SetStreamSelection(MF_SOURCE_READER_ALL_STREAMS,FALSE));
    HRESULT video=s.reader->SetStreamSelection(MF_SOURCE_READER_FIRST_VIDEO_STREAM,TRUE);
    // A PCM cue has no video stream. Keep its existing WASAPI path without
    // allocating a video processor or weakening camera/video admission.
    s.has_video=s.camera||video!=MF_E_INVALIDSTREAMNUMBER;
    if(s.has_video){
        check(video);
        // ANGLE's D3D11 image import accepts BGRA, but not B8G8R8X8 storage.
        // Request ARGB32 so the native processor allocates an importable format.
        ComPtr<IMFMediaType> type;check(MFCreateMediaType(&type));check(type->SetGUID(MF_MT_MAJOR_TYPE,MFMediaType_Video));check(type->SetGUID(MF_MT_SUBTYPE,MFVideoFormat_ARGB32));
        if(s.geometry.known){
            check(type->SetUINT32(MF_MT_VIDEO_ROTATION,0));
            check(MFSetAttributeSize(type.Get(),MF_MT_FRAME_SIZE,s.geometry.width,s.geometry.height));
            MFVideoArea full{};full.Area.cx=s.geometry.width;full.Area.cy=s.geometry.height;
            check(type->SetBlob(MF_MT_MINIMUM_DISPLAY_APERTURE,(UINT8*)&full,sizeof(full)));
            check(type->SetBlob(MF_MT_GEOMETRIC_APERTURE,(UINT8*)&full,sizeof(full)));
        }
        // Explicit GPU video-processor output request. Validate the negotiated tags
        // and IMFDXGIBuffer on every frame; no system-memory output is admitted.
        check(type->SetUINT32(MF_MT_VIDEO_PRIMARIES,MFVideoPrimaries_BT709));check(type->SetUINT32(MF_MT_TRANSFER_FUNCTION,MFVideoTransFunc_sRGB));check(type->SetUINT32(MF_MT_VIDEO_NOMINAL_RANGE,MFNominalRange_0_255));
        if(s.requested_w&&s.requested_h)check(MFSetAttributeSize(type.Get(),MF_MT_FRAME_SIZE,s.requested_w,s.requested_h));
        if(s.requested_n&&s.requested_d)check(MFSetAttributeRatio(type.Get(),MF_MT_FRAME_RATE,s.requested_n,s.requested_d));
        s.operation="video output negotiation";check(s.reader->SetCurrentMediaType(MF_SOURCE_READER_FIRST_VIDEO_STREAM,nullptr,type.Get()));
        if(s.geometry.known){
            s.operation="native untransformed video configuration";
            ComPtr<IMFSourceReaderEx> reader;check(s.reader.As(&reader));bool configured=false;
            for(DWORD index=0;index<16;index++){
                GUID category;ComPtr<IMFTransform> transform;HRESULT hr=reader->GetTransformForStream(MF_SOURCE_READER_FIRST_VIDEO_STREAM,index,&category,&transform);
                if(hr==MF_E_INVALIDINDEX)break;check(hr);
                ComPtr<IMFVideoProcessorControl> control;if(FAILED(transform.As(&control)))continue;
                RECT full{0,0,(LONG)s.geometry.width,(LONG)s.geometry.height};
                check(control->SetRotation(ROTATION_NONE));check(control->SetMirror(MIRROR_NONE));
                check(control->SetSourceRectangle(&full));check(control->SetDestinationRectangle(&full));configured=true;
            }
            if(!configured)throw Failure{E_NOTIMPL};
        }
        s.operation="video metadata validation";video_type(s);
    }
    if(!s.camera){
        ComPtr<IMFMediaType> native_audio;HRESULT audio=s.reader->GetNativeMediaType(MF_SOURCE_READER_FIRST_AUDIO_STREAM,0,&native_audio);
        if(SUCCEEDED(audio)){
            check(s.reader->SetStreamSelection(MF_SOURCE_READER_FIRST_AUDIO_STREAM,TRUE));
            ComPtr<IMFMediaType> pcm;check(MFCreateMediaType(&pcm));check(pcm->SetGUID(MF_MT_MAJOR_TYPE,MFMediaType_Audio));check(pcm->SetGUID(MF_MT_SUBTYPE,MFAudioFormat_PCM));
            check(pcm->SetUINT32(MF_MT_AUDIO_NUM_CHANNELS,2));check(pcm->SetUINT32(MF_MT_AUDIO_SAMPLES_PER_SECOND,48000));check(pcm->SetUINT32(MF_MT_AUDIO_BITS_PER_SAMPLE,16));check(pcm->SetUINT32(MF_MT_AUDIO_BLOCK_ALIGNMENT,4));check(pcm->SetUINT32(MF_MT_AUDIO_AVG_BYTES_PER_SECOND,192000));check(pcm->SetUINT32(MF_MT_ALL_SAMPLES_INDEPENDENT,TRUE));
            check(s.reader->SetCurrentMediaType(MF_SOURCE_READER_FIRST_AUDIO_STREAM,nullptr,pcm.Get()));setup_audio(s);s.has_audio=true;
        }else if(audio!=MF_E_INVALIDSTREAMNUMBER)check(audio);
        PROPVARIANT duration;PropVariantInit(&duration);if(SUCCEEDED(s.reader->GetPresentationAttribute(MF_SOURCE_READER_MEDIASOURCE,MF_PD_DURATION,&duration))&&duration.vt==VT_UI8){std::lock_guard<std::mutex> guard(s.lock);s.state.duration_ns=duration.uhVal.QuadPart*100;}PropVariantClear(&duration);
        PROPVARIANT characteristics;PropVariantInit(&characteristics);if(SUCCEEDED(s.reader->GetPresentationAttribute(MF_SOURCE_READER_MEDIASOURCE,MF_SOURCE_READER_MEDIASOURCE_CHARACTERISTICS,&characteristics))&&characteristics.vt==VT_UI4){std::lock_guard<std::mutex> guard(s.lock);s.state.seekable=(characteristics.ulVal&MFMEDIASOURCE_CAN_SEEK)!=0;}PropVariantClear(&characteristics);
    }
    if(!s.has_video){
        if(!s.has_audio)throw Failure{MF_E_INVALIDMEDIATYPE};
        s.video_eos=true;
        s.prepared_frame=true;
        std::lock_guard<std::mutex> guard(s.lock);
        s.state.state=1;
    }
}
static std::unique_ptr<FerroWindowsFrame> video_frame(Session &s,const Packet &packet){
    if(packet.flags&MF_SOURCE_READERF_CURRENTMEDIATYPECHANGED)video_type(s);
    ComPtr<IMFMediaBuffer> buffer;check(packet.sample->GetBufferByIndex(0,&buffer));ComPtr<IMFDXGIBuffer> dxgi;if(FAILED(buffer.As(&dxgi)))throw Failure{MF_E_UNSUPPORTED_D3D_TYPE};
    auto f=std::make_unique<FerroWindowsFrame>();check(dxgi->GetResource(IID_PPV_ARGS(&f->texture)));check(dxgi->GetSubresourceIndex(&f->slice));
    ComPtr<ID3D11Device> actual;f->texture->GetDevice(&actual);if(actual.Get()!=s.device.Get())throw Failure{MF_E_UNSUPPORTED_D3D_TYPE};
    D3D11_TEXTURE2D_DESC desc{};f->texture->GetDesc(&desc);
    if(desc.Width!=s.width||desc.Height!=s.height||desc.MipLevels!=1||desc.SampleDesc.Count!=1||!(desc.BindFlags&D3D11_BIND_SHADER_RESOURCE)||desc.Format!=DXGI_FORMAT_B8G8R8A8_UNORM||f->slice>=desc.ArraySize)throw Failure{MF_E_UNSUPPORTED_D3D_TYPE};
    uint64_t bytes=(uint64_t)desc.Width*desc.Height*4*desc.ArraySize;if(bytes>128ull*1024*1024)throw Failure{E_OUTOFMEMORY};
    f->sample=packet.sample;f->context=s.context;f->owner=s.weak_from_this();
    D3D11_QUERY_DESC query{D3D11_QUERY_EVENT,0};check(s.device->CreateQuery(&query,&f->ready));s.context->End(f->ready.Get());s.context->Flush();
    f->info={s.width,s.height,s.color,s.rotation,bytes,(uint64_t)std::max<LONGLONG>(0,packet.pts)*100,++s.sequence,packet.epoch,s.mirrored,s.visible_x,s.visible_y,s.visible_w,s.visible_h};return f;
}
static bool ready(FerroWindowsFrame &f){BOOL done=FALSE;HRESULT hr=f.context->GetData(f.ready.Get(),&done,sizeof(done),D3D11_ASYNC_GETDATA_DONOTFLUSH);check(hr);return hr==S_OK&&done;}
static void reset(Session &s,uint64_t epoch,LONGLONG target){
    {std::lock_guard<std::mutex> guard(s.lock);s.flushed=false;}
    check(s.reader->Flush(MF_SOURCE_READER_ALL_STREAMS));
    {std::unique_lock<std::mutex> guard(s.lock);if(!s.changed.wait_for(guard,std::chrono::seconds(5),[&]{return s.flushed||s.closed.load();}))throw Failure{HRESULT_FROM_WIN32(WAIT_TIMEOUT)};s.packet_ready=false;s.packet={};}
    if(s.closed)return;
    PROPVARIANT value;PropVariantInit(&value);value.vt=VT_I8;value.hVal.QuadPart=target;check(s.reader->SetCurrentPosition(GUID_NULL,value));
    s.video.clear();s.pcm.clear();s.pcm_bytes=0;s.video_eos=!s.has_video;s.audio_eos=false;s.prepared_frame=false;s.position=s.clock_base=target;s.clock_based=false;
    s.started=Clock::now();
    if(s.audio){check(s.audio->Stop());check(s.audio->Reset());}s.audio_started=false;s.audio_frames_written=0;
    std::lock_guard<std::mutex> guard(s.lock);s.request_epoch=epoch;s.latest.reset();s.state.position_ns=target*100;s.state.state=0;
}
static void audio_packet(Session &s,const Packet &p){
    ComPtr<IMFMediaBuffer> buffer;check(p.sample->ConvertToContiguousBuffer(&buffer));BYTE *data=nullptr;DWORD maximum,length;check(buffer->Lock(&data,&maximum,&length));
    if(length>192000||length%4||s.pcm_bytes+length>384000){buffer->Unlock();throw Failure{E_OUTOFMEMORY};}
    AudioChunk chunk;chunk.pts=p.pts;chunk.bytes.assign(data,data+length);buffer->Unlock();
    if(chunk.pts<s.position){uint64_t skip=(uint64_t)(s.position-chunk.pts)*48000/10000000*4;chunk.offset=(UINT)std::min<uint64_t>(skip,length);chunk.pts+=(LONGLONG)chunk.offset/4*10000000/48000;}
    if(chunk.offset<length){s.pcm_bytes+=length-chunk.offset;s.pcm.push_back(std::move(chunk));}
}
static void write_audio(Session &s){
    UINT padding;check(s.audio->GetCurrentPadding(&padding));if(padding>s.audio_capacity)throw Failure{E_FAIL};UINT available=s.audio_capacity-padding;
    while(available&&!s.pcm.empty()){
        auto &chunk=s.pcm.front();UINT frames=std::min<UINT>(available,(UINT)(chunk.bytes.size()-chunk.offset)/4);
        if(!s.clock_based){s.clock_base=chunk.pts;s.clock_based=true;}
        BYTE *out;check(s.render->GetBuffer(frames,&out));memcpy(out,chunk.bytes.data()+chunk.offset,frames*4);check(s.render->ReleaseBuffer(frames,0));
        s.audio_frames_written+=frames;
        chunk.offset+=frames*4;s.pcm_bytes-=frames*4;available-=frames;
        if(chunk.offset==chunk.bytes.size())s.pcm.pop_front();
    }
    if(!s.audio_started&&s.clock_based){check(s.audio->Start());s.audio_started=true;}
}
static void present_video(Session &s,LONGLONG clock){
    // A paused seek publishes its first ready frame once. Read-ahead packets
    // can already be queued while its GPU query settles; keep those for resume
    // instead of replacing the achieved seek frame.
    while((s.camera||s.playing||!s.prepared_frame)&&!s.video.empty()&&ready(*s.video.front())&&(s.camera||!s.playing||(LONGLONG)(s.video.front()->info.timestamp_ns/100)<=clock+100000)){
        auto frame=std::move(s.video.front());s.video.pop_front();
        std::lock_guard<std::mutex> guard(s.lock);
        if(frame->info.epoch==s.epoch){s.position=s.playing?clock:(LONGLONG)(frame->info.timestamp_ns/100);s.state.seek_done=frame->info.epoch;s.latest=std::move(frame);s.prepared_frame=true;s.state.state=s.playing?2:1;}
        if(!s.playing)break;
    }
}
static void run(const std::shared_ptr<Session> &owner){
    auto &s=*owner;HRESULT initialized=CoInitializeEx(nullptr,COINIT_MULTITHREADED);
    try{
        check(initialized);prepare(owner);s.operation="native streaming";s.started=Clock::now();
        {std::lock_guard<std::mutex> guard(s.lock);s.rate_supported=!s.camera&&(!s.has_audio||s.audio_rate.Get()!=nullptr);}
        while(!s.closed.load()){
            int intent;uint64_t current_epoch=0,seek=0;LONGLONG target=0;double volume,rate;bool muted;
            {std::lock_guard<std::mutex> guard(s.lock);check(s.event_error);intent=s.intent;current_epoch=s.epoch;seek=s.pending_seek;target=s.seek_target;s.pending_seek=0;volume=s.volume;muted=s.muted;rate=s.requested_rate;}
            if(seek&&!s.camera)reset(s,seek,target);
            if(s.closed)break;
            if(rate!=s.rate){
                if(s.audio_rate){
                    check(s.audio_rate->SetSampleRate((float)(48000.*rate)));
                    s.use_audio_frame_clock=true;
                }else if(!s.has_audio){
                    s.clock_base=s.clock_position();s.started=Clock::now();
                }
                s.rate=rate;
            }
            if((intent==1)!=s.playing){
                if(intent==1){if(!s.ever_played&&!s.camera&&s.state.seekable)reset(s,current_epoch,s.position);s.ever_played=true;s.started=Clock::now();if(!s.has_audio)s.clock_base=s.position;if(s.audio&&s.clock_based){check(s.audio->Start());s.audio_started=true;}s.playing=true;}
                else{s.position=s.clock_position();if(s.audio)check(s.audio->Stop());s.audio_started=false;s.playing=false;std::lock_guard<std::mutex> guard(s.lock);if(s.prepared_frame&&s.state.state!=4)s.state.state=1;}
            }
            if(s.audio_volume){check(s.audio_volume->SetMasterVolume((float)volume,nullptr));check(s.audio_volume->SetMute(muted,nullptr));}
            bool received=false;Packet packet;
            {std::lock_guard<std::mutex> guard(s.lock);if(s.packet_ready){packet=std::move(s.packet);s.packet_ready=false;received=true;}}
            if(received){
                check(packet.status);
                uint64_t epoch;{std::lock_guard<std::mutex> guard(s.lock);epoch=s.epoch;}
                if(packet.epoch==epoch){
                    ComPtr<IMFMediaType> type;check(s.reader->GetCurrentMediaType(packet.stream,&type));GUID major;check(type->GetGUID(MF_MT_MAJOR_TYPE,&major));bool video=major==MFMediaType_Video;
                    if (packet.flags & MF_SOURCE_READERF_ENDOFSTREAM) {
                        if (s.camera) throw Failure{MF_E_VIDEO_RECORDING_DEVICE_INVALIDATED};
                        if (video) s.video_eos = true;
                        else s.audio_eos = true;
                    }
                    if(!s.has_video&&!s.prepared_frame&&(packet.sample||s.audio_eos)){
                        std::lock_guard<std::mutex> guard(s.lock);
                        if(packet.epoch==s.epoch){
                            // Audio can return a packet starting before the requested position.
                            // audio_packet trims the preceding PCM, so keep the requested floor.
                            s.position=std::max(s.position,packet.pts);
                            s.state.seek_done=packet.epoch;
                            s.prepared_frame=true;
                            s.state.state=s.playing?2:1;
                        }
                    }
                    if (packet.sample) {
                        if (video) {
                            auto frame = video_frame(s, packet);
                            if (s.camera) {
                                std::shared_ptr<Recording> recording;
                                { std::lock_guard<std::mutex> guard(s.lock); recording = s.recording; }
                                if (recording) {
                                    if (packet.flags & MF_SOURCE_READERF_CURRENTMEDIATYPECHANGED)
                                        recording->cancel();
                                    else if (!recording->worker_done)
                                        recording->enqueue(packet);
                                }
                            }
                            if (s.camera || packet.pts >= s.position) s.video.push_back(std::move(frame));
                        } else if (s.playing && s.has_audio) audio_packet(s, packet);
                    }
                }
            }
            if(s.playing&&s.has_audio)write_audio(s);
            LONGLONG clock=s.clock_position();
            present_video(s,clock);
            bool request=false;
            {std::lock_guard<std::mutex> guard(s.lock);
                if(s.playing&&s.state.state!=5)s.state.state=2;
                if(s.has_audio&&s.playing){UINT padding=0;check(s.audio->GetCurrentPadding(&padding));s.state.buffering=(s.audio_started&&padding==0&&!s.audio_eos)?1:0;}else s.state.buffering=0;
                s.state.position_ns=(uint64_t)std::max<LONGLONG>(0,s.playing?clock:s.position)*100;
                if(!s.pending_seek&&s.request_epoch==s.epoch&&s.video_eos&&(!s.has_audio||s.audio_eos)&&s.video.empty()&&s.pcm.empty()){
                    UINT padding=0;if(s.audio)check(s.audio->GetCurrentPadding(&padding));
                    if(!padding){s.state.state=4;s.state.seek_done=s.epoch;s.intent=0;}
                }
                check(s.event_error);
                bool need=s.camera||s.playing||!s.prepared_frame;
                if(need&&!s.pending_seek&&!s.in_flight&&!s.packet_ready&&s.video.size()<3&&s.pcm_bytes<96000&&!(s.video_eos&&(!s.has_audio||s.audio_eos))){s.in_flight=true;s.request_epoch=s.epoch;request=true;}
            }
            if(request)check(s.reader->ReadSample(MF_SOURCE_READER_ANY_STREAM,0,nullptr,nullptr,nullptr,nullptr));
            // ReadSample can complete before the worker enters this wait.
            // Handle an already delivered callback without waiting on its
            // spent notification. Keep the timeout for the clock/GPU checks.
            std::unique_lock<std::mutex> guard(s.lock);
            s.changed.wait_for(guard, std::chrono::milliseconds(2), [&] {
                return s.closed || s.packet_ready || FAILED(s.event_error) ||
                    s.pending_seek || s.intent != intent || s.requested_rate != rate;
            });
        }
    }catch(Failure f){s.fail(f.hr);}catch(...){s.fail(E_FAIL,"Native media worker failed");}
    s.closed=true;
    {
        std::lock_guard<std::mutex> guard(s.lock);
        if (s.recording) s.recording->cancel();
        s.recording.reset();
    }
    if(s.audio)s.audio->Stop();
    if(s.reader){
        {std::lock_guard<std::mutex> guard(s.lock);s.flushed=false;}
        // Flush is asynchronous even during teardown. Keep the callback and
        // source owner until the acknowledgement (bounded wait on this worker).
        HRESULT flush=s.reader->Flush(MF_SOURCE_READER_ALL_STREAMS);
        if(SUCCEEDED(flush)){
            std::unique_lock<std::mutex> guard(s.lock);
            bool acknowledged=s.changed.wait_for(guard,std::chrono::seconds(5),[&]{return s.flushed;});
#ifdef FERROUI_SYNTHETIC_CAMERA
            if(!acknowledged&&s.source.rfind(L"ferroui:synthetic-camera",0)==0)fputs("SYNTHETIC_NATIVE_FLUSH_TIMEOUT\n",stderr);
#else
            (void)acknowledged;
#endif
        }
    }
    s.reader.Reset();s.callback.Reset();
    if(s.camera_source)s.camera_source->Shutdown();s.camera_source.Reset();
    s.video.clear();s.pcm.clear();{std::lock_guard<std::mutex> guard(s.lock);s.latest.reset();s.packet={};s.packet_ready=false;}s.render.Reset();s.audio_clock.Reset();s.audio_rate.Reset();s.audio_volume.Reset();s.audio.Reset();s.manager.Reset();
    // D3D11 destruction is deferred. Flush after releasing source resources,
    // when no future capture/presentation commands may arrive to do it for us.
    if(s.context)s.context->Flush();s.context.Reset();
    if(s.resource_file)DeleteFileW(s.source.c_str());
    if(SUCCEEDED(initialized))CoUninitialize();--active_sessions;
}
FerroWindowsSession *ferro_windows_open(void *device,const uint16_t *source,int camera,int resource_file,uint32_t w,uint32_t h,uint32_t n,uint32_t d){
    if(!device||!source)return nullptr;int count=active_sessions.load();do{if(count>=4)return nullptr;}while(!active_sessions.compare_exchange_weak(count,count+1));
    try{auto s=std::make_shared<Session>();s->device=(ID3D11Device*)device;s->source=(const wchar_t*)source;s->camera=camera!=0;s->resource_file=resource_file!=0;s->intent=s->camera?1:0;s->requested_w=w;s->requested_h=h;s->requested_n=n;s->requested_d=d;s->state.duration_ns=UINT64_MAX;
        auto handle=std::make_unique<FerroWindowsSession>();handle->value=s;std::thread([s]{run(s);}).detach();return handle.release();
    }catch(...){--active_sessions;return nullptr;}
}
void ferro_windows_close(FerroWindowsSession *s){
    {
        std::lock_guard<std::mutex> guard(s->value->lock);
        s->value->closed = true;
        if (s->value->recording) s->value->recording->cancel();
    }
    s->value->changed.notify_all();
    delete s;
}
static bool can_record(const Session &s) {
    return !s.closed && s.state.state == 2 && s.recording_encoder && s.recording_format;
}
int ferro_windows_can_record(FerroWindowsSession *handle) {
    auto &s = *handle->value;
    std::lock_guard<std::mutex> guard(s.lock);
    return can_record(s);
}
FerroWindowsRecording *ferro_windows_record(FerroWindowsSession *handle) {
    auto &s = *handle->value;
    std::lock_guard<std::mutex> guard(s.lock);
    if (!can_record(s) || (s.recording && !s.recording->worker_done)) return nullptr;
    try {
        auto recording = std::make_shared<Recording>(s.state.width, s.state.height,
                                                     s.state.fps_n, s.state.fps_d);
        auto result = std::make_unique<FerroWindowsRecording>();
        result->value = recording;
        std::thread(encode_recording, recording).detach();
        s.recording = std::move(recording);
        return result.release();
    } catch (...) { return nullptr; }
}
void ferro_windows_record_finish(FerroWindowsRecording *handle) {
    auto &r = *handle->value;
    std::lock_guard<std::mutex> guard(r.lock);
    r.finish = true;
    r.changed.notify_one();
}
void ferro_windows_record_cancel(FerroWindowsRecording *handle) { handle->value->cancel(); }
void ferro_windows_record_release(FerroWindowsRecording *handle) {
    handle->value->cancel();
    delete handle;
}
int ferro_windows_record_poll(FerroWindowsRecording *handle, FerroWindowsRecordingInfo *out) {
    auto &r = *handle->value;
    std::lock_guard<std::mutex> guard(r.lock);
    *out = r.info;
    return r.status;
}
int ferro_windows_record_copy(FerroWindowsRecording *handle, uint8_t *bytes, uint64_t length) {
    auto &r = *handle->value;
    std::lock_guard<std::mutex> guard(r.lock);
    if (r.status != 2 || length != r.encoded.size()) return 0;
    memcpy(bytes, r.encoded.data(), r.encoded.size());
    return 1;
}
void ferro_windows_state(FerroWindowsSession *s,FerroWindowsState *out){std::lock_guard<std::mutex> guard(s->value->lock);*out=s->value->state;}
void ferro_windows_intent(FerroWindowsSession *s,int intent){std::lock_guard<std::mutex> guard(s->value->lock);s->value->intent=intent;s->value->changed.notify_all();}
uint64_t ferro_windows_seek(FerroWindowsSession *s,uint64_t ns){std::lock_guard<std::mutex> guard(s->value->lock);uint64_t epoch=++s->value->epoch;s->value->pending_seek=epoch;s->value->seek_target=(LONGLONG)std::min<uint64_t>(ns/100,INT64_MAX);s->value->latest.reset();s->value->changed.notify_all();return epoch;}
void ferro_windows_audio(FerroWindowsSession *s,double volume,int muted){std::lock_guard<std::mutex> guard(s->value->lock);s->value->volume=volume;s->value->muted=muted!=0;}
int ferro_windows_can_set_rate(FerroWindowsSession *s){std::lock_guard<std::mutex> guard(s->value->lock);return s->value->rate_supported;}
int ferro_windows_set_rate(FerroWindowsSession *s,double rate){
    std::lock_guard<std::mutex> guard(s->value->lock);
    if(!s->value->rate_supported||!(rate>=0.5&&rate<=2.))return 0;
    s->value->requested_rate=rate;s->value->changed.notify_all();return 1;
}
FerroWindowsFrame *ferro_windows_take(FerroWindowsSession *s){std::lock_guard<std::mutex> guard(s->value->lock);if(s->value->latest&&s->value->latest->info.epoch==s->value->epoch)return s->value->latest.release();return nullptr;}
void ferro_windows_frame_info(FerroWindowsFrame *f,FerroWindowsFrameInfo *out){*out=f->info;}
void ferro_windows_frame_release(FerroWindowsFrame *f){
    auto owner=f->owner.lock();auto context=f->context;
    delete f;
    // A consumer/photo can retire after the source worker's final Flush. This
    // submits deferred destruction only; it does not prove GPU completion or
    // release quarantined storage. Avoid extra per-frame flushes while streaming.
    // Check closed after deletion: otherwise Stop's Flush could race between
    // the check and this last release, leaving destruction deferred again.
    if(context&&(!owner||owner->closed.load()))context->Flush();
}

// Function addresses come from the active context's loader; never load a second
// ANGLE DLL or assume the process's default graphics adapter is compatible.
template<class T>static T entry(const FerroWindowsEgl &egl,int index){return reinterpret_cast<T>(egl.functions[index]);}
using QueryAttrib=unsigned int (WINAPI*)(void*,int,intptr_t*);
using QueryString=const char* (WINAPI*)(void*,int);
using CreateImage=void* (WINAPI*)(void*,void*,unsigned int,void*,const int*);
using DestroyImage=unsigned int (WINAPI*)(void*,void*);
using ImageTarget=void (WINAPI*)(unsigned int,void*);
using GetInt=void (WINAPI*)(unsigned int,int*);
using OneEnum=void (WINAPI*)(unsigned int);
using TextureArray=void (WINAPI*)(int,unsigned int*);
using BindTexture=void (WINAPI*)(unsigned int,unsigned int);
using TextureParameter=void (WINAPI*)(unsigned int,unsigned int,int);
using GetError=unsigned int (WINAPI*)();
static bool contains_extension(const char *extensions,const char *name){if(!extensions)return false;std::string all=" ";all+=extensions;all+=' ';return all.find(std::string(" ")+name+" ")!=std::string::npos;}
void *ferro_windows_device(intptr_t display,const FerroWindowsEgl *egl){
    if(!egl)return nullptr;for(int i=0;i<13;i++)if(!egl->functions[i])return nullptr;
    const char *extensions=entry<QueryString>(*egl,12)((void*)display,0x3055);
    if(!contains_extension(extensions,"EGL_ANGLE_image_d3d11_texture"))return nullptr;
    intptr_t device=0,d3d=0;
    if(!entry<QueryAttrib>(*egl,0)((void*)display,0x322c,&device)||!entry<QueryAttrib>(*egl,1)((void*)device,0x33a1,&d3d)||!d3d)return nullptr;
    ((ID3D11Device*)d3d)->AddRef();return (void*)d3d;
}
void ferro_windows_device_release(void *device){if(device)((ID3D11Device*)device)->Release();}
struct Alias {
#ifdef FERROUI_SYNTHETIC_CAMERA
    FixtureAlias tracked;
#endif
    FerroWindowsEgl egl;void *display=nullptr,*image=nullptr;unsigned int texture=0;};
void *ferro_windows_alias(FerroWindowsFrame *f,intptr_t display,const FerroWindowsEgl *egl,uint32_t *texture){
    if(!egl)return nullptr;
    auto a=std::make_unique<Alias>();a->egl=*egl;a->display=(void*)display;
    int attributes[]={0x3493,(int)f->slice,0x30d2,1,0x3038}; // array slice, IMAGE_PRESERVED_KHR, NONE
    a->image=entry<CreateImage>(*egl,2)(a->display,nullptr,0x3484,f->texture.Get(),attributes);
    if(!a->image)return nullptr;
    int active=0,previous=0;entry<GetInt>(*egl,5)(0x84e0,&active);entry<OneEnum>(*egl,6)(0x84c0);entry<GetInt>(*egl,5)(0x8069,&previous);
    entry<TextureArray>(*egl,7)(1,&a->texture);entry<BindTexture>(*egl,8)(0x0de1,a->texture);
    entry<TextureParameter>(*egl,9)(0x0de1,0x2801,0x2601);entry<TextureParameter>(*egl,9)(0x0de1,0x2800,0x2601);
    entry<TextureParameter>(*egl,9)(0x0de1,0x2802,0x812f);entry<TextureParameter>(*egl,9)(0x0de1,0x2803,0x812f);
    entry<ImageTarget>(*egl,4)(0x0de1,a->image);unsigned int error=entry<GetError>(*egl,10)();
    entry<BindTexture>(*egl,8)(0x0de1,previous);entry<OneEnum>(*egl,6)(active);
    if(error||!a->texture){ferro_windows_alias_release(a.release(),1);return nullptr;}
    *texture=a->texture;return a.release();
}
void ferro_windows_alias_release(void *ptr,int alive){std::unique_ptr<Alias>a((Alias*)ptr);if(alive){entry<TextureArray>(a->egl,11)(1,&a->texture);entry<DestroyImage>(a->egl,3)(a->display,a->image);}}
struct PhotoResources {ComPtr<ID3D11Texture2D> staging;ComPtr<ID3D11Query> query;std::unique_ptr<FerroWindowsFrame> input;};
static std::atomic<bool> photo_active{false};
#ifdef FERROUI_SYNTHETIC_CAMERA
static std::atomic<bool> photo_quarantined{false};
extern "C" uint32_t ferro_windows_fixture_photo_status(){return (photo_active.load()?1:0)|(photo_quarantined.load()?2:0);}
#endif
int ferro_windows_photo_rgb(FerroWindowsFrame *f,uint8_t *rgb,uint64_t length){
    bool idle=false;if(!photo_active.compare_exchange_strong(idle,true))return 0;
    std::unique_ptr<PhotoResources> work;bool submitted=false,completed=false;
    auto failed=[&](HRESULT hr){
#ifdef FERROUI_SYNTHETIC_CAMERA
        fprintf(stderr,"SYNTHETIC_NATIVE_PHOTO_FAILURE submitted=%d completed=%d %s\n",submitted,completed,hresult(hr).c_str());
#else
        (void)hr;
#endif
        if(submitted&&!completed){
#ifdef FERROUI_SYNTHETIC_CAMERA
            photo_quarantined=true;
#endif
            // Unknown completion retains source, staging and the global slot;
            // cancellation must never convert this into premature destruction.
            work.release();return 2;
        }
        photo_active=false;return 0;
    };
    try{
        if(f->info.width>4096||f->info.height>4096||length!=(uint64_t)f->info.width*f->info.height*3)throw Failure{E_INVALIDARG};
        work=std::make_unique<PhotoResources>();work->input=std::make_unique<FerroWindowsFrame>(*f);
        ComPtr<ID3D11Device> device;f->texture->GetDevice(&device);
        D3D11_TEXTURE2D_DESC desc{};f->texture->GetDesc(&desc);desc.ArraySize=1;desc.BindFlags=0;desc.MiscFlags=0;desc.Usage=D3D11_USAGE_STAGING;desc.CPUAccessFlags=D3D11_CPU_ACCESS_READ;
        check(device->CreateTexture2D(&desc,nullptr,&work->staging));
        D3D11_QUERY_DESC q{D3D11_QUERY_EVENT,0};check(device->CreateQuery(&q,&work->query));
        submitted=true;f->context->CopySubresourceRegion(work->staging.Get(),0,0,0,0,f->texture.Get(),f->slice,nullptr);
        f->context->End(work->query.Get());f->context->Flush();
        auto deadline=Clock::now()+std::chrono::seconds(5);BOOL done=FALSE;
        while(true){HRESULT result=f->context->GetData(work->query.Get(),&done,sizeof(done),D3D11_ASYNC_GETDATA_DONOTFLUSH);check(result);if(result==S_OK&&done)break;if(Clock::now()>=deadline)throw Failure{HRESULT_FROM_WIN32(WAIT_TIMEOUT)};std::this_thread::sleep_for(std::chrono::milliseconds(1));}
        completed=true;
        D3D11_MAPPED_SUBRESOURCE map{};check(f->context->Map(work->staging.Get(),0,D3D11_MAP_READ,0,&map));
        for(UINT y=0;y<f->info.height;y++)for(UINT x=0;x<f->info.width;x++){
            BYTE *in=(BYTE*)map.pData+(size_t)y*map.RowPitch+x*4;BYTE *out=rgb+((size_t)y*f->info.width+x)*3;
            for(int c=0;c<3;c++){float v=in[2-c]/255.f;if(f->info.color==2){v=std::pow(v,2.4f);v=v<=.0031308f?12.92f*v:1.055f*std::pow(v,1.f/2.4f)-.055f;}out[c]=(BYTE)std::lround(std::clamp(v,0.f,1.f)*255.f);}
        }
        f->context->Unmap(work->staging.Get(),0);photo_active=false;return 1;
    }catch(Failure e){return failed(e.hr);}catch(...){return failed(E_FAIL);}
}
static std::string json_string(const std::string &text){std::string out="\"";for(unsigned char c:text){if(c=='"'||c=='\\'){out+='\\';out+=(char)c;}else if(c<32){char buf[7];snprintf(buf,sizeof(buf),"\\u%04x",c);out+=buf;}else out+=(char)c;}return out+'"';}
char *ferro_windows_devices(void){
    HRESULT initialized=CoInitializeEx(nullptr,COINIT_MULTITHREADED);char *result=nullptr;IMFActivate **devices=nullptr;UINT32 count=0;
    try{
        check(initialized);runtime();ComPtr<IMFAttributes> attributes;check(MFCreateAttributes(&attributes,1));check(attributes->SetGUID(MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE,MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_GUID));check(MFEnumDeviceSources(attributes.Get(),&devices,&count));
        std::string json="[";
        for(UINT32 i=0;i<count;i++){
            wchar_t *id=nullptr,*label=nullptr;UINT32 length;
            HRESULT hr=devices[i]->GetAllocatedString(MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_SYMBOLIC_LINK,&id,&length);
            devices[i]->GetAllocatedString(MF_DEVSOURCE_ATTRIBUTE_FRIENDLY_NAME,&label,&length);
            if(SUCCEEDED(hr)&&id){if(json.size()>1)json+=',';json+="{\"id\":"+json_string(utf8(id))+",\"label\":"+(label?json_string(utf8(label)):"null")+"}";}
            CoTaskMemFree(id);CoTaskMemFree(label);
        }
        json+=']';if(json.size()<=1024*1024){result=(char*)CoTaskMemAlloc(json.size()+1);if(result)memcpy(result,json.c_str(),json.size()+1);}
    }catch(...){}
    for(UINT32 i=0;i<count;i++)devices[i]->Release();CoTaskMemFree(devices);if(SUCCEEDED(initialized))CoUninitialize();return result;
}
void ferro_windows_string_release(char *text){CoTaskMemFree(text);}

#ifdef FERROUI_NATIVE_FRAME_TESTS
#include "frame_tests.cpp"
#include "mp4_geometry_tests.cpp"
#endif
