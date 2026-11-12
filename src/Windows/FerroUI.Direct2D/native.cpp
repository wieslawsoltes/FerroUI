// System-only Windows backend. No portable renderer or font rasterizer is linked.
#include <windows.h>
#include <d3d11_4.h>
#include <dxgi1_3.h>
#include <dcomp.h>
#include <d2d1_3.h>
#include <d2d1effects_2.h>
#include <dwrite_3.h>
#include <wincodec.h>
#include <wrl/client.h>
#include <algorithm>
#include <atomic>
#include <cmath>
#include <cstdio>
#include <cstring>
#include <memory>
#include <mutex>
#include <stdexcept>
#include <string>
#include <vector>
#include <unordered_map>
#include <list>
#include <array>
#include <limits>
#include <thread>
#include <chrono>
#include <condition_variable>
#include "effects_shader.h"
#ifdef FERRO_RUNTIME_SHADERS
#include <d3dcompiler.h>
#endif
using Microsoft::WRL::ComPtr;
using namespace D2D1;
using Ptr = void*;
static std::atomic<uint64_t> statistics[12]{};
static const bool nativeProfiling=GetEnvironmentVariableW(L"FERROUI_D3D_PROFILE",nullptr,0)!=0;
static std::atomic<uint64_t> nativeTimings[20]{},nativeTimingCounts[20]{};
static uint64_t glyphAtlasHits=0,glyphAtlasBakes=0;
struct NativeTiming {
    unsigned slot;std::chrono::steady_clock::time_point start;
    explicit NativeTiming(unsigned value):slot(value){if(nativeProfiling)start=std::chrono::steady_clock::now();}
    ~NativeTiming(){if(nativeProfiling){nativeTimings[slot]+=(uint64_t)std::chrono::duration_cast<std::chrono::nanoseconds>(std::chrono::steady_clock::now()-start).count();++nativeTimingCounts[slot];}}
};
// Rust TLS destructors can run after C++ static destructors on Windows.
// The process-wide synchronization primitive must remain usable by them.
static std::recursive_mutex& nativeMutex=*new std::recursive_mutex;
static bool nativeShuttingDown=false;
static bool nativeProcessExiting=false;
struct Command { int kind; double p[6]; };
struct Glyph { uint16_t glyph; int cluster; double advance, x, y; };
static float pixelFloor(float x) {if(x>=-16777216.f&&x<=16777216.f){int v=(int)x;return (float)(v-(x<v));}return floorf(x);}
static float pixelCeil(float x) {if(x>=-16777216.f&&x<=16777216.f){int v=(int)x;return (float)(v+(x>v));}return ceilf(x);}
struct DrawCommand {
    int kind,blend,aliased,interpolation;
    double matrix[6],params[4];
    Ptr resource;double shift;
    int textAliased,fillEvenOdd;
};
static_assert(sizeof(DrawCommand)==120,"Rust/native draw command layout mismatch");
static void check(HRESULT hr) {
    if (FAILED(hr)) {
        char message[64]; sprintf_s(message, "Windows graphics HRESULT 0x%08lx", hr);
        throw std::runtime_error(message);
    }
}
#define API extern "C"
#define CATCH_PTR catch(const std::exception& e) { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex); ++statistics[8]; fprintf(stderr,"Direct2D: %s\n",e.what()); return nullptr; }
#define CATCH_VOID catch(const std::exception& e) { ++statistics[8]; fprintf(stderr,"Direct2D: %s\n",e.what()); }
// The apartment of a thread this file starts. A thread of the application is never initialised
// here: the apartment of the UI thread belongs to the windowing backend (OLE drag and drop, the
// clipboard and the pickers need a single-threaded apartment there), and the MTA usage the
// engine holds (EngineLifetime) gives every thread that has no apartment the implicit MTA.
struct Apartment {
    HRESULT result = CoInitializeEx(nullptr, COINIT_MULTITHREADED);
    ~Apartment() { if (SUCCEEDED(result)) CoUninitialize(); }
};
static void apartment() { thread_local Apartment value; (void)value; }
static std::wstring wide(const char* text) {
    if (!text || !*text) return {};
    int n = MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, text, -1, nullptr, 0);
    if (!n) return {};
    std::wstring result(n, 0);
    MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, text, -1, result.data(), n);
    result.pop_back(); return result;
}
static std::string utf8(const std::wstring& text) {
    int n = WideCharToMultiByte(CP_UTF8,0,text.data(),(int)text.size(),nullptr,0,nullptr,nullptr);
    std::string result(n,0);
    WideCharToMultiByte(CP_UTF8,0,text.data(),(int)text.size(),result.data(),n,nullptr,nullptr);
    return result;
}
static std::wstring localeName(const char* language) {
    auto result=wide(language);
    if(result.empty()) {
        WCHAR current[LOCALE_NAME_MAX_LENGTH]{};
        result=GetUserDefaultLocaleName(current,LOCALE_NAME_MAX_LENGTH)?current:L"en-us";
    }
    return result;
}
static std::wstring name(IDWriteLocalizedStrings* strings) {
    UINT32 index = 0, length = 0; BOOL exists = FALSE;
    strings->FindLocaleName(L"en-us", &index, &exists);
    if (!exists) index = 0;
    check(strings->GetStringLength(index, &length));
    std::wstring result(length + 1, 0);
    check(strings->GetString(index, result.data(), length + 1));
    result.resize(length); return result;
}
struct EngineLifetime;
static std::shared_ptr<EngineLifetime> retainEngine();
struct Object {
    // Released after every derived COM resource. In particular a bitmap held
    // by Rust TLS keeps both the device factories and their MTA usage alive.
    std::shared_ptr<EngineLifetime> engineOwner=retainEngine();
    std::atomic<unsigned> references{1};
    virtual ~Object() = default;
};
struct Data : Object {
    std::vector<uint8_t> bytes;
    Data() = default;
    explicit Data(const std::string& text) : bytes(text.begin(),text.end()) {}
};
struct Frame;
struct GlyphAtlasPage {
    ComPtr<ID2D1Bitmap1> bitmap;
    UINT x=0,y=0,rowHeight=0;
    uint64_t used=0;
    bool ready=false;
};
class GlyphAtlas {
    static constexpr UINT side=256;
    static constexpr size_t pageLimit=64;
    ComPtr<ID2D1DeviceContext> context;
    ComPtr<ID2D1SolidColorBrush> brush;
    D2D1_COLOR_F color{};
    std::shared_ptr<GlyphAtlasPage> building;
    std::vector<std::shared_ptr<GlyphAtlasPage>> pages;
    uint64_t clock=0;
    uint64_t frame=0;
    bool active=false,disabled=false;
public:
    void begin() { active=!disabled;++frame; }
    bool recording() const { return active; }
    uint64_t frameIndex() const { return frame; }
    void touch(GlyphAtlasPage& page) { page.used=++clock; }
    bool record(ID2D1Device* device,const DWRITE_GLYPH_RUN& run,D2D1_POINT_2F origin,
        D2D1_MATRIX_3X2_F matrix,D2D1_RECT_F destination,D2D1_COLOR_F paint,
        std::weak_ptr<GlyphAtlasPage>& page,D2D1_RECT_F& source) {
        const auto width=(UINT)(destination.right-destination.left);
        const auto height=(UINT)(destination.bottom-destination.top);
        if(!width||!height||width>side||height>side)return false;
        UINT x=building?building->x:0,y=building?building->y:0,row=building?building->rowHeight:0;
        if(x+width>side) {x=0;y+=row;row=0;}
        if(y+height>side)return false;
        // An integer shift must preserve the exact fractional raster phase.
        double dx=(double)x-destination.left,dy=(double)y-destination.top;
        float tx=(float)(matrix._31+dx),ty=(float)(matrix._32+dy);
        if((double)tx-dx!=matrix._31||(double)ty-dy!=matrix._32)return false;
        if(!context) {
            check(device->CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE,&context));
            context->SetUnitMode(D2D1_UNIT_MODE_PIXELS);
            context->SetTextAntialiasMode(D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE);
        }
        if(!building) {
            auto next=std::make_shared<GlyphAtlasPage>();
            auto properties=BitmapProperties1(D2D1_BITMAP_OPTIONS_TARGET,
                PixelFormat(DXGI_FORMAT_B8G8R8A8_UNORM,D2D1_ALPHA_MODE_PREMULTIPLIED));
            check(context->CreateBitmap(SizeU(side,side),nullptr,0,&properties,&next->bitmap));
            context->SetTarget(next->bitmap.Get());
            context->BeginDraw();
            context->Clear(ColorF(0,0,0,0));
            building=std::move(next);
        }
        if(!brush)check(context->CreateSolidColorBrush(paint,&brush));
        else if(memcmp(&color,&paint,sizeof(paint)))brush->SetColor(paint);
        color=paint;
        source=RectF((float)x,(float)y,(float)(x+width),(float)(y+height));
        context->SetTransform(Matrix3x2F::Identity());
        context->PushAxisAlignedClip(source,D2D1_ANTIALIAS_MODE_ALIASED);
        matrix._31=tx;matrix._32=ty;
        context->SetTransform(matrix);
        context->DrawGlyphRun(origin,&run,brush.Get(),DWRITE_MEASURING_MODE_NATURAL);
        context->PopAxisAlignedClip();
        building->x=x+width;building->y=y;building->rowHeight=std::max(row,height);
        page=building;
        return true;
    }
    void finish() {
        active=false;
        if(!building)return;
        auto page=std::move(building);
        HRESULT hr=context->EndDraw();
        context->SetTarget(nullptr);
        if(FAILED(hr)) { disable();return; }
        page->ready=true;
        touch(*page);
        if(pages.size()==pageLimit) {
            auto oldest=std::min_element(pages.begin(),pages.end(),[](const auto& a,const auto& b){return a->used<b->used;});
            *oldest=std::move(page);
        } else pages.push_back(std::move(page));
    }
    void reset() {
        active=disabled=false;
        if(building) {
            context->EndDraw();
            context->SetTarget(nullptr);
        }
        building.reset();pages.clear();brush.Reset();context.Reset();clock=0;
    }
    void disable() {
        reset();
        disabled=true;
    }
};
struct ShapeScratch {
    std::vector<uint16_t> clusters,ids;
    std::vector<DWRITE_SHAPING_TEXT_PROPERTIES> tp;
    std::vector<DWRITE_SHAPING_GLYPH_PROPERTIES> gp;
    std::vector<int> source;
    std::vector<float> advances;
    std::vector<DWRITE_GLYPH_OFFSET> offsets;
    std::vector<Glyph> result;
    bool busy=false;
};
using BoxShadowKey = std::array<uint64_t,24>;
struct BoxShadowKeyHash {
    size_t operator()(const BoxShadowKey& key) const noexcept {
        size_t hash=0;
        for(auto field:key)hash^=std::hash<uint64_t>{}(field)+0x9e3779b97f4a7c15ULL+(hash<<6)+(hash>>2);
        return hash;
    }
};
class BoxShadowCache {
    static constexpr size_t maxBytes=32*1024*1024,maxEntries=512;
    struct Entry {
        ComPtr<ID2D1Bitmap1> image;
        size_t bytes;
        std::list<BoxShadowKey>::iterator recency;
    };
    std::list<BoxShadowKey> recent;
    std::unordered_map<BoxShadowKey,Entry,BoxShadowKeyHash> entries;
    size_t bytes=0;
public:
    ID2D1Bitmap1* find(const BoxShadowKey& key) {
        auto entry=entries.find(key);
        if(entry==entries.end())return nullptr;
        recent.splice(recent.begin(),recent,entry->second.recency);
        return entry->second.image.Get();
    }
    void insert(const BoxShadowKey& key,const ComPtr<ID2D1Bitmap1>& image) {
        auto size=image->GetPixelSize();
        size_t cost=(size_t)size.width*size.height*4;
        if(cost>maxBytes)return;
        while(bytes+cost>maxBytes||entries.size()>=maxEntries) {
            auto oldest=entries.find(recent.back());
            bytes-=oldest->second.bytes;
            entries.erase(oldest);
            recent.pop_back();
        }
        recent.push_front(key);
        try { entries.emplace(key,Entry{image,cost,recent.begin()}); }
        catch(...) { recent.pop_front();throw; }
        bytes+=cost;
    }
    void clear() {
        entries.clear();
        recent.clear();
        bytes=0;
    }
};
struct Engine {
    struct SolidStrokeStyle {
        D2D1_STROKE_STYLE_PROPERTIES properties{};
        ComPtr<ID2D1StrokeStyle> style;
    };
    ShapeScratch shortShapes;
    std::array<SolidStrokeStyle,8> solidStrokeStyles;
    size_t nextSolidStrokeStyle=0;
    GlyphAtlas glyphAtlas;
    std::vector<std::shared_ptr<Frame>> framePool;
    size_t pooledBytes=0;
    BoxShadowCache boxShadows;
    uint64_t generation=1;
    ComPtr<ID3D11ComputeShader> filterShader;
    ComPtr<ID3D11ShaderResourceView> neutralShaderInput;
    ComPtr<IDWriteFactory5> fonts;
    ComPtr<IDWriteTextAnalyzer> analyzer;
    ComPtr<IDWriteTextAnalyzer1> analyzer1;
    ComPtr<IDWriteInMemoryFontFileLoader> loader;
    ComPtr<ID2D1Factory1> factory;
    ComPtr<ID3D11Device> d3d;
    ComPtr<ID3D11DeviceContext> immediate;
    ComPtr<ID2D1Device> device;
    ComPtr<IWICImagingFactory> wic;
    ComPtr<IDXGIFactory2> dxgiFactory;
    DXGI_ADAPTER_DESC adapter{};
    D3D_FEATURE_LEVEL level{};
    Engine() {
        check(DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED,__uuidof(IDWriteFactory5),
              reinterpret_cast<IUnknown**>(fonts.GetAddressOf())));
        check(fonts->CreateTextAnalyzer(&analyzer));
        check(analyzer.As(&analyzer1));
        check(fonts->CreateInMemoryFontFileLoader(&loader));
        check(fonts->RegisterFontFileLoader(loader.Get()));
        D2D1_FACTORY_OPTIONS options{};
        if (GetEnvironmentVariableW(L"FERROUI_D3D_DEBUG",nullptr,0)) options.debugLevel=D2D1_DEBUG_LEVEL_INFORMATION;
        check(D2D1CreateFactory(D2D1_FACTORY_TYPE_MULTI_THREADED,options,factory.GetAddressOf()));
        check(CoCreateInstance(CLSID_WICImagingFactory2,nullptr,CLSCTX_INPROC_SERVER,IID_PPV_ARGS(&wic)));
        createDevice();
    }
    struct Device {
        ComPtr<ID3D11Device> d3d;
        ComPtr<ID3D11DeviceContext> immediate;
        ComPtr<ID2D1Device> device;
        ComPtr<IDXGIFactory2> dxgiFactory;
        DXGI_ADAPTER_DESC adapter{};
        D3D_FEATURE_LEVEL level{};
    };
    Device makeDevice() {
        Device made;
        UINT flags = D3D11_CREATE_DEVICE_BGRA_SUPPORT | D3D11_CREATE_DEVICE_VIDEO_SUPPORT;
        if (GetEnvironmentVariableW(L"FERROUI_D3D_DEBUG",nullptr,0)) flags |= D3D11_CREATE_DEVICE_DEBUG;
        // The built-in effect kernels require Shader Model 5 compute support.
        D3D_FEATURE_LEVEL levels[]={D3D_FEATURE_LEVEL_11_1,D3D_FEATURE_LEVEL_11_0};
        // Hardware is required. WARP never silently satisfies native GPU startup.
        check(D3D11CreateDevice(nullptr,D3D_DRIVER_TYPE_HARDWARE,nullptr,flags,levels,
              ARRAYSIZE(levels),D3D11_SDK_VERSION,&made.d3d,&made.level,&made.immediate));
        ComPtr<ID3D11Multithread> multithread;
        check(made.immediate.As(&multithread)); multithread->SetMultithreadProtected(TRUE);
        ComPtr<IDXGIDevice> dxgi; check(made.d3d.As(&dxgi));
        ComPtr<IDXGIAdapter> gpu; check(dxgi->GetAdapter(&gpu)); check(gpu->GetDesc(&made.adapter));
        check(gpu->GetParent(IID_PPV_ARGS(&made.dxgiFactory)));
        check(factory->CreateDevice(dxgi.Get(), &made.device));
        return made;
    }
    void adopt(Device&& made) {
        d3d=std::move(made.d3d);immediate=std::move(made.immediate);device=std::move(made.device);
        dxgiFactory=std::move(made.dxgiFactory);adapter=made.adapter;level=made.level;
        // Which adapter draws is a diagnostic: said only when diagnostics are asked for.
        if(nativeProfiling||GetEnvironmentVariableW(L"FERROUI_D3D_STATS",nullptr,0)||
           GetEnvironmentVariableW(L"FERROUI_D3D_DEBUG",nullptr,0))
            fprintf(stderr,"Direct2D: hardware D3D11 %s, feature level 0x%x\n",utf8(adapter.Description).c_str(),level);
    }
    void createDevice() {adopt(makeDevice());}
    void recover() {
        if(SUCCEEDED(d3d->GetDeviceRemovedReason()))return;
        replaceDevice();
    }
    // The new device is made before the removed one is let go: while the adapter is away this
    // throws and leaves the engine as it was (its device removed, its generation the old one,
    // every surface of it lost), and the next recover() tries again.
    void replaceDevice() {
        auto made=makeDevice();
        glyphAtlas.reset();
        boxShadows.clear();
        framePool.clear();pooledBytes=0;
        filterShader.Reset();neutralShaderInput.Reset();
        adopt(std::move(made));++generation;
    }
    ~Engine() {
        // Cached device children must release while their device and immediate
        // context are still alive, rather than in reverse member order.
        glyphAtlas.reset();boxShadows.clear();framePool.clear();pooledBytes=0;filterShader.Reset();neutralShaderInput.Reset();
        if(immediate){immediate->ClearState();immediate->Flush();}
        if(fonts&&loader)fonts->UnregisterFontFileLoader(loader.Get());
    }
};
// Keep the factories and implicit MTA alive until their last native user, not
// merely until the C++ static root is destroyed before Rust TLS cleanup.
struct EngineLifetime {
    CO_MTA_USAGE_COOKIE cookie{};
    std::unique_ptr<Engine> value;
    EngineLifetime() {
        check(CoIncrementMTAUsage(&cookie));
        try {value=std::make_unique<Engine>();}catch(...){CoDecrementMTAUsage(cookie);throw;}
    }
    ~EngineLifetime() {
        std::lock_guard<std::recursive_mutex> lock(nativeMutex);
        value.reset();
        CoDecrementMTAUsage(cookie);
    }
};
struct EngineRoot {
    std::shared_ptr<EngineLifetime> value=std::make_shared<EngineLifetime>();
    void release() {std::lock_guard<std::recursive_mutex> lock(nativeMutex);nativeShuttingDown=true;value.reset();}
    ~EngineRoot(){nativeProcessExiting=true;release();}
};
static EngineRoot& engineRoot(){static EngineRoot root;return root;}
static std::shared_ptr<EngineLifetime> retainEngine() {
    auto value=engineRoot().value;
    if(!value)throw std::runtime_error("Native renderer is shutting down");
    return value;
}
// Native entry points hold nativeMutex; borrowing the root avoids an atomic
// shared_ptr increment/decrement on every state operation. Objects still take
// an owning reference through retainEngine() for their full lifetime.
static Engine& engine() {auto& root=engineRoot().value;if(!root)throw std::runtime_error("Native renderer is shutting down");return *root->value;}
struct ShapeScratchLease {
    ShapeScratch local;ShapeScratch* value;bool shared;
    explicit ShapeScratchLease(bool shortInput) {
        auto& scratch=engine().shortShapes;shared=shortInput&&!scratch.busy;
        value=shared?&scratch:&local;if(shared)scratch.busy=true;
    }
    ~ShapeScratchLease() {
        if(shared){
            if(value->ids.capacity()>4096||value->result.capacity()>4096){
                value->clusters.clear();value->clusters.shrink_to_fit();value->ids.clear();value->ids.shrink_to_fit();
                value->tp.clear();value->tp.shrink_to_fit();value->gp.clear();value->gp.shrink_to_fit();
                value->source.clear();value->source.shrink_to_fit();value->advances.clear();value->advances.shrink_to_fit();
                value->offsets.clear();value->offsets.shrink_to_fit();value->result.clear();value->result.shrink_to_fit();
            }
            value->busy=false;
        }
    }
};
struct Font : Object {
    ComPtr<IDWriteFontFace> face;
    ComPtr<IDWriteFontFile> file;
    std::shared_ptr<std::vector<uint8_t>> bytes;
    std::wstring family;
    // Font faces (including each variation instance) are immutable. Keep a
    // bounded design-unit cache, shared by runs at every requested em size.
    std::unordered_map<uint16_t,DWRITE_GLYPH_METRICS> glyphMetrics;
    DWRITE_FONT_METRICS metrics{};
    bool hasMetrics=false;
    // Recreated layout runs can share immutable glyph arrays and the bounded
    // device-generation-aware ink cache. Each entry owns one native reference;
    // outstanding Rust runs remain valid after eviction or font destruction.
    struct PositionedEntry {std::string key;Object* run;size_t bytes;};
    std::list<PositionedEntry> positionedRuns;
    std::unordered_map<std::string,std::list<PositionedEntry>::iterator> positionedIndex;
    size_t positionedBytes=0;
    // Admit on repeated use. A fingerprint collision can only admit an extra
    // entry: actual sharing always checks the complete key above. This fixed
    // probation table keeps one-off scrolling text out of the resource LRU.
    std::array<std::array<size_t,4>,1024> positionedProbation{};
    static void releasePositioned(Object* run){if(--run->references==0)delete run;}
    ~Font(){for(auto& entry:positionedRuns)releasePositioned(entry.run);}
    int simpleCapable=-1;
    bool supportsSimplePlacement() {
        if(simpleCapable<0) {
            if(!hasMetrics){face->GetMetrics(&metrics);hasMetrics=true;}
            ComPtr<IDWriteFontFace5> f5;
            auto units=metrics.designUnitsPerEm;
            // Variable and simulated faces can have non-integral design
            // advances. A power-of-two em keeps float scaling identical to
            // DirectWrite's natural placement for integral design advances.
            simpleCapable=units&&!(units&(units-1))&&face->GetSimulations()==DWRITE_FONT_SIMULATIONS_NONE
                &&SUCCEEDED(face.As(&f5))&&!f5->HasVariations();
        }
        return simpleCapable!=0;
    }
    struct ShapeEntry {std::string key;std::vector<Glyph> glyphs;size_t bytes;};
    std::list<ShapeEntry> shapes;
    std::unordered_map<std::string,std::list<ShapeEntry>::iterator> shapeIndex;
    size_t shapeBytes=0;
    void cacheShape(std::string key,const std::vector<Glyph>& glyphs) {
        size_t bytes=key.capacity()+key.size()+glyphs.size()*sizeof(Glyph)+sizeof(ShapeEntry)+64;
        constexpr size_t budget=1024*1024;
        if(bytes>budget)return;
        while(!shapes.empty()&&(shapeBytes+bytes>budget||shapes.size()>=2048)) {
            shapeBytes-=shapes.front().bytes;shapeIndex.erase(shapes.front().key);shapes.pop_front();
        }
        shapes.push_back({std::move(key),glyphs,bytes});auto entry=std::prev(shapes.end());
        shapeIndex.emplace(entry->key,entry);shapeBytes+=bytes;
    }
    int colorCapable=-1;
    bool hasColorGlyphs() {
        if(colorCapable<0) {
            ComPtr<IDWriteFontFace4> f4;
            // Include SVG and bitmap-only fonts, not just COLR/palette fonts.
            colorCapable=FAILED(face.As(&f4))||
                (f4->GetGlyphImageFormats()&~(DWRITE_GLYPH_IMAGE_FORMATS_TRUETYPE|DWRITE_GLYPH_IMAGE_FORMATS_CFF))!=0;
        }
        return colorCapable!=0;
    }
};
struct PositionedGlyphRun : Object {
    ComPtr<IDWriteFontFace> face;
    double x=0,y=0;float size=0;bool color=false,positionedAdvances=false;
    std::vector<uint16_t> ids;
    std::vector<float> advances;
    std::vector<DWRITE_GLYPH_OFFSET> offsets;
    struct BoundsEntry {D2D1_MATRIX_3X2_F matrix;float y;int alias;uint64_t generation;D2D1_RECT_F bounds;bool valid=false;};
    mutable std::vector<BoundsEntry> inkBounds;
    mutable unsigned nextBounds=0;
    struct RasterEntry {
        D2D1_MATRIX_3X2_F matrix;
        float baseline;
        D2D1_COLOR_F paint;
        std::weak_ptr<GlyphAtlasPage> page;
        D2D1_RECT_F source{}, destination{};
        uint64_t lastFrame=0;
    };
    static constexpr unsigned rasterLimit=4;
    mutable std::vector<RasterEntry> rasters;
    mutable unsigned nextRaster=0;
    PositionedGlyphRun(Font& font,double em,const uint16_t* glyphs,const double* positions,size_t n,const double* natural=nullptr):face(font.face),size((float)em),color(font.hasColorGlyphs()) {
        if(n>UINT32_MAX)throw std::runtime_error("Too many positioned glyphs");
        if(!n)return;
        x=positions[0];y=positions[1];ids.assign(glyphs,glyphs+n);advances.resize(n,0);offsets.resize(n);
        bool simple=natural!=nullptr;
        for(size_t i=0;i<n&&simple;++i)if(!std::isfinite(natural[i])||natural[i]<0||positions[i*2+1]!=y||(i&&positions[i*2]<positions[(i-1)*2]))simple=false;
        double exactPen=0;
        for(size_t i=0;i<n&&simple;++i){
            float target=(float)(positions[i*2]-x),offset=(float)((double)target-exactPen),advance=(float)natural[i];
            // Keep the explicit-position path unless float and double pen
            // accumulation both reproduce every original float position.
            if(exactPen+offset!=(double)target||(double)((float)exactPen+advance)!=exactPen+advance||(double)((float)x+(float)exactPen)!=(double)(float)x+exactPen)simple=false;
            exactPen+=advance;
        }
        double pen=0;
        positionedAdvances=simple;
        for(size_t i=0;i<n;++i){
            offsets[i]={simple?(float)((double)(float)(positions[i*2]-x)-pen):(float)(positions[i*2]-x),(float)(y-positions[i*2+1])};
            if(simple){advances[i]=(float)natural[i];pen+=advances[i];}
        }

    }
    DWRITE_GLYPH_RUN run() const {return {face.Get(),size,(UINT32)ids.size(),ids.data(),advances.data(),offsets.data(),FALSE,0};}
    bool bounds(Frame& frame,const D2D1_MATRIX_3X2_F& matrix,const D2D1_MATRIX_3X2_F& current,double shift,int alias,D2D1_RECT_F& result) const;
};
class FontBytes final : public IUnknown {
    std::atomic<ULONG> refs{1};
public:
    std::shared_ptr<std::vector<uint8_t>> bytes;
    explicit FontBytes(std::shared_ptr<std::vector<uint8_t>> b):bytes(std::move(b)) {}
    IFACEMETHOD(QueryInterface)(REFIID iid,void** out) override { *out=nullptr;if(iid!=__uuidof(IUnknown))return E_NOINTERFACE;*out=this;AddRef();return S_OK; }
    IFACEMETHOD_(ULONG,AddRef)() override { return ++refs; }
    IFACEMETHOD_(ULONG,Release)() override { auto count=--refs;if(!count)delete this;return count; }
};
struct Path : Object {
    ComPtr<ID2D1PathGeometry> geometry;
    D2D1_RECT_F localBounds{};
    bool hasLocalBounds=false;
};
static std::unique_ptr<Path> path() {
    auto result=std::make_unique<Path>();
    check(engine().factory->CreatePathGeometry(&result->geometry)); return result;
}
static ComPtr<ID2D1Geometry> rule(Path* p, int evenodd) {
    NativeTiming timing(5);
    // Native paths are winding-filled. A geometry group selects alternate
    // filling without changing a sink's mode after its figures were emitted.
    if(!evenodd)return p->geometry;
    ID2D1Geometry* input[]={p->geometry.Get()};ComPtr<ID2D1GeometryGroup> group;
    check(engine().factory->CreateGeometryGroup(D2D1_FILL_MODE_ALTERNATE,input,1,&group));
    return group;
}
static ComPtr<ID2D1StrokeStyle> strokeStyle(double width,int cap,int join,double miter,
        const double* dash,size_t count,double phase) {
    D2D1_STROKE_STYLE_PROPERTIES p=StrokeStyleProperties();
    p.startCap=p.endCap=p.dashCap=cap==1?D2D1_CAP_STYLE_ROUND:cap==2?D2D1_CAP_STYLE_SQUARE:D2D1_CAP_STYLE_FLAT;
    p.lineJoin=join==1?D2D1_LINE_JOIN_ROUND:join==2?D2D1_LINE_JOIN_BEVEL:D2D1_LINE_JOIN_MITER;
    p.miterLimit=(float)miter; p.dashStyle=count?D2D1_DASH_STYLE_CUSTOM:D2D1_DASH_STYLE_SOLID;
    p.dashOffset=(float)(phase/width);
    auto& e=engine();
    if(!count) {
        for(const auto& entry:e.solidStrokeStyles)
            if(entry.style&&!memcmp(&entry.properties,&p,sizeof(p)))return entry.style;
        ComPtr<ID2D1StrokeStyle> style;
        check(e.factory->CreateStrokeStyle(p,nullptr,0,&style));
        e.solidStrokeStyles[e.nextSolidStrokeStyle++%e.solidStrokeStyles.size()]={p,style};
        return style;
    }
    std::vector<float> values; for(size_t i=0;i<count;++i) values.push_back((float)(dash[i]/width));
    ComPtr<ID2D1StrokeStyle> style;
    check(e.factory->CreateStrokeStyle(p,values.data(),(UINT32)values.size(),&style)); return style;
}
API int d2_supported() try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex); engine(); return 1; } catch(...) { return 0; }
API void d2_release(Ptr p) {
    std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    // Rust's EXE TLS callback can run after d3d11/d2d1 DLL detach. COM Release
    // may flush/map resources and is unsafe then, even with a device reference.
    // Normal window/resource disposal remains deterministic; only process-exit
    // TLS remnants are left to the OS along with their retained factory owner.
    if(nativeProcessExiting)return;
    if(p && --static_cast<Object*>(p)->references==0)delete static_cast<Object*>(p);
}
API Ptr d2_retain(Ptr p) { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex); if(p) ++static_cast<Object*>(p)->references; return p; }
API size_t d2_data_len(Ptr p) { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex); return p?static_cast<Data*>(p)->bytes.size():0; }
API const uint8_t* d2_data_bytes(Ptr p) { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex); return p?static_cast<Data*>(p)->bytes.data():nullptr; }
API Ptr d2_font(const char* family,int weight,int style,int stretch) try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    auto requested=wide(family);
    if(requested.empty()||requested==L"Default"||requested==L"$Default") requested=L"Segoe UI";
    ComPtr<IDWriteFontCollection> collection;
    check(engine().fonts->GetSystemFontCollection(&collection,FALSE));
    UINT32 index=0; BOOL exists=FALSE;
    check(collection->FindFamilyName(requested.c_str(),&index,&exists));
    if(!exists) return nullptr;
    ComPtr<IDWriteFontFamily> ff; check(collection->GetFontFamily(index,&ff));
    ComPtr<IDWriteFont> f;
    check(ff->GetFirstMatchingFont((DWRITE_FONT_WEIGHT)weight,(DWRITE_FONT_STRETCH)stretch,
          style==1?DWRITE_FONT_STYLE_ITALIC:style==2?DWRITE_FONT_STYLE_OBLIQUE:DWRITE_FONT_STYLE_NORMAL,&f));
    auto result=std::make_unique<Font>(); check(f->CreateFontFace(&result->face));
    ComPtr<IDWriteLocalizedStrings> names; check(ff->GetFamilyNames(&names)); result->family=name(names.Get());
    return result.release();
} CATCH_PTR
API void d2_font_name(Ptr p,uint8_t* out,size_t n) { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    if(!n) return;
    out[0]=0; if(!p) return;
    try { auto value=utf8(static_cast<Font*>(p)->family); size_t length=std::min(n-1,value.size()); memcpy(out,value.data(),length); out[length]=0; }
    catch(const std::exception&) {}
}
API Ptr d2_font_names() try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    ComPtr<IDWriteFontCollection> collection; check(engine().fonts->GetSystemFontCollection(&collection,FALSE));
    std::string result;
    for(UINT32 i=0;i<collection->GetFontFamilyCount();++i) {
        ComPtr<IDWriteFontFamily> f; check(collection->GetFontFamily(i,&f));
        ComPtr<IDWriteLocalizedStrings> names; check(f->GetFamilyNames(&names)); result+=utf8(name(names.Get()))+"\n";
    }
    return new Data(result);
} CATCH_PTR
API Ptr d2_font_family_faces(const char* family) try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    ComPtr<IDWriteFontCollection> c; check(engine().fonts->GetSystemFontCollection(&c,FALSE));
    UINT32 index=0; BOOL exists=FALSE; check(c->FindFamilyName(wide(family).c_str(),&index,&exists));
    if(!exists) return new Data();
    ComPtr<IDWriteFontFamily> ff; check(c->GetFontFamily(index,&ff)); std::string result;
    for(UINT32 i=0;i<ff->GetFontCount();++i) {
        ComPtr<IDWriteFont> f; check(ff->GetFont(i,&f));
        int style=f->GetStyle()==DWRITE_FONT_STYLE_ITALIC?1:f->GetStyle()==DWRITE_FONT_STYLE_OBLIQUE?2:0;
        result+=std::to_string(f->GetWeight())+","+std::to_string(style)+","+std::to_string(f->GetStretch())+"\n";
    } return new Data(result);
} CATCH_PTR
API Ptr d2_font_data(const uint8_t* bytes,size_t n) try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    if(n>UINT32_MAX) return nullptr;
    auto result=std::make_unique<Font>();
    result->bytes=std::make_shared<std::vector<uint8_t>>(bytes,bytes+n);
    ComPtr<IUnknown> owner;owner.Attach(new FontBytes(result->bytes));
    check(engine().loader->CreateInMemoryFontFileReference(engine().fonts.Get(),result->bytes->data(),(UINT32)n,owner.Get(),&result->file));
    BOOL supported=FALSE; DWRITE_FONT_FILE_TYPE type; DWRITE_FONT_FACE_TYPE faceType; UINT32 faces=0;
    check(result->file->Analyze(&supported,&type,&faceType,&faces)); if(!supported||!faces) return nullptr;
    auto file=result->file.Get();
    check(engine().fonts->CreateFontFace(faceType,1,&file,0,DWRITE_FONT_SIMULATIONS_NONE,&result->face));
    ComPtr<IDWriteFontFace3> face3; check(result->face.As(&face3));
    ComPtr<IDWriteLocalizedStrings> names; check(face3->GetFamilyNames(&names)); result->family=name(names.Get());
    return result.release();
} CATCH_PTR
API Ptr d2_font_variations(Ptr p,const uint32_t* tags,const double* values,size_t n) try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    auto original=static_cast<Font*>(p); auto result=std::make_unique<Font>();
    result->face=original->face; result->bytes=original->bytes; result->file=original->file; result->family=original->family;
    if(n) {
        ComPtr<IDWriteFontFace5> face5; check(original->face.As(&face5));
        ComPtr<IDWriteFontResource> resource; check(face5->GetFontResource(&resource));
        std::vector<DWRITE_FONT_AXIS_VALUE> axes(face5->GetFontAxisValueCount());
        check(face5->GetFontAxisValues(axes.data(),(UINT32)axes.size()));
        std::vector<DWRITE_FONT_AXIS_RANGE> ranges(resource->GetFontAxisCount());
        check(resource->GetFontAxisRanges(ranges.data(),(UINT32)ranges.size()));
        for(size_t i=0;i<n;++i) {
            UINT32 tag=_byteswap_ulong(tags[i]);
            auto found=std::find_if(ranges.begin(),ranges.end(),[&](auto r){return r.axisTag==tag;});
            if(found==ranges.end() || !std::isfinite(values[i]) || values[i]<found->minValue || values[i]>found->maxValue) return nullptr;
            for(auto& axis:axes) if(axis.axisTag==tag) axis.value=(float)values[i];
        }
        ComPtr<IDWriteFontFace5> varied;
        check(resource->CreateFontFace(original->face->GetSimulations(),axes.data(),(UINT32)axes.size(),&varied)); result->face=varied;
    }
    return result.release();
} CATCH_PTR
API Ptr d2_font_table(Ptr p,uint32_t tag) try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    auto f=static_cast<Font*>(p); const void* data=nullptr; UINT32 n=0; void* context=nullptr; BOOL exists=FALSE;
    check(f->face->TryGetFontTable(_byteswap_ulong(tag),&data,&n,&context,&exists));
    if(!exists) return nullptr;
    auto result=std::make_unique<Data>(); result->bytes.assign((const uint8_t*)data,(const uint8_t*)data+n);
    f->face->ReleaseFontTable(context); return result.release();
} CATCH_PTR
// A borrowed analysis source/sink. AnalyzeScript is synchronous; the owner and locale
// remain alive throughout analysis, GetGlyphs and placement. No second paragraph layout.
class Analysis : public IDWriteTextAnalysisSource, public IDWriteTextAnalysisSink {
public:
    const WCHAR* text; UINT32 length; const WCHAR* locale; bool rtl;
    struct Run { UINT32 start,length; DWRITE_SCRIPT_ANALYSIS script; };
    std::vector<Run> runs;
    Analysis(const WCHAR* t,UINT32 n,const WCHAR* l,bool r):text(t),length(n),locale(l),rtl(r) {}
    IFACEMETHOD(QueryInterface)(REFIID iid,void** out) override {
        *out=nullptr;
        if(iid==__uuidof(IUnknown)||iid==__uuidof(IDWriteTextAnalysisSource)) *out=static_cast<IDWriteTextAnalysisSource*>(this);
        else if(iid==__uuidof(IDWriteTextAnalysisSink)) *out=static_cast<IDWriteTextAnalysisSink*>(this);
        return *out?S_OK:E_NOINTERFACE;
    }
    IFACEMETHOD_(ULONG,AddRef)() override { return 1; }
    IFACEMETHOD_(ULONG,Release)() override { return 1; }
    IFACEMETHOD(GetTextAtPosition)(UINT32 p,const WCHAR** t,UINT32* n) override { *t=p<length?text+p:nullptr; *n=p<length?length-p:0; return S_OK; }
    IFACEMETHOD(GetTextBeforePosition)(UINT32 p,const WCHAR** t,UINT32* n) override { *t=p?text:nullptr; *n=std::min(p,length); return S_OK; }
    IFACEMETHOD_(DWRITE_READING_DIRECTION,GetParagraphReadingDirection)() override { return rtl?DWRITE_READING_DIRECTION_RIGHT_TO_LEFT:DWRITE_READING_DIRECTION_LEFT_TO_RIGHT; }
    IFACEMETHOD(GetLocaleName)(UINT32 p,UINT32* n,const WCHAR** l) override { *n=length-p; *l=locale; return S_OK; }
    IFACEMETHOD(GetNumberSubstitution)(UINT32 p,UINT32* n,IDWriteNumberSubstitution** s) override { *n=length-p; *s=nullptr; return S_OK; }
    IFACEMETHOD(SetScriptAnalysis)(UINT32 p,UINT32 n,const DWRITE_SCRIPT_ANALYSIS* s) override { runs.push_back({p,n,*s}); return S_OK; }
    IFACEMETHOD(SetLineBreakpoints)(UINT32,UINT32,const DWRITE_LINE_BREAKPOINT*) override { return S_OK; }
    IFACEMETHOD(SetBidiLevel)(UINT32,UINT32,UINT8,UINT8) override { return S_OK; }
    IFACEMETHOD(SetNumberSubstitution)(UINT32,UINT32,IDWriteNumberSubstitution*) override { return S_OK; }
};
API Ptr d2_font_match(Ptr p,const uint16_t* text,size_t n,const char* language) try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    if(!n || n>UINT32_MAX) return nullptr;
    auto locale=localeName(language);
    Analysis analysis((const WCHAR*)text,(UINT32)n,locale.c_str(),false);
    ComPtr<IDWriteFontFallback> fallback; check(engine().fonts->GetSystemFontFallback(&fallback));
    ComPtr<IDWriteFont> mapped; UINT32 length=0; float scale=1;
    auto f=static_cast<Font*>(p);
    ComPtr<IDWriteFontFace3> requested;check(f->face.As(&requested));
    check(fallback->MapCharacters(&analysis,0,(UINT32)n,nullptr,f->family.c_str(),requested->GetWeight(),
          requested->GetStyle(),requested->GetStretch(),&length,&mapped,&scale));
    if(!mapped) return nullptr;
    auto result=std::make_unique<Font>(); check(mapped->CreateFontFace(&result->face));
    ComPtr<IDWriteFontFamily> family; check(mapped->GetFontFamily(&family));
    ComPtr<IDWriteLocalizedStrings> names; check(family->GetFamilyNames(&names)); result->family=name(names.Get());
    return result.release();
} CATCH_PTR
static int shapeCore(bool optimize,Ptr p,const uint16_t* text,size_t n,size_t start,size_t length,const char* language,
        double size,int rtl,const uint32_t* tags,const int* values,const uint32_t* featureStarts,const uint32_t* featureEnds,
        size_t features,Ptr info,void(*emit)(Ptr,Glyph)) try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    NativeTiming timing(10);
    if(n>UINT32_MAX || start>n || length>n-start) return 0;
    if(!length) return 1;
    auto locale=localeName(language);
    auto f=static_cast<Font*>(p);
    // Cache only bounded inputs. The complete UTF-16 owner/slice, resolved
    // locale and every shaping parameter are part of the key: substring-only
    // keys would break joining context, features and cluster offsets.
    std::string cacheKey;
    if(optimize&&n<=256&&features<=16&&locale.size()<=256) {
        cacheKey.reserve(52+n*sizeof(uint16_t)+locale.size()*sizeof(wchar_t)+features*16);
        auto add=[&](const auto& v){cacheKey.append(reinterpret_cast<const char*>(&v),sizeof(v));};
        add(n);add(start);add(length);add(size);add(rtl);add(features);size_t localeLength=locale.size();add(localeLength);
        cacheKey.append(reinterpret_cast<const char*>(text),n*sizeof(uint16_t));
        cacheKey.append(reinterpret_cast<const char*>(locale.data()),locale.size()*sizeof(wchar_t));
        for(size_t i=0;i<features;++i){add(tags[i]);add(values[i]);add(featureStarts[i]);add(featureEnds[i]);}
        auto found=f->shapeIndex.find(cacheKey);
        if(found!=f->shapeIndex.end()) {
            NativeTiming hit(14);
            auto entry=found->second;f->shapes.splice(f->shapes.end(),f->shapes,entry);
            for(auto glyph:entry->glyphs)emit(info,glyph);
            return 1;
        }
    }
    NativeTiming miss(15);
    ShapeScratchLease lease(n<=256);auto& scratch=*lease.value;
    // Ask DirectWrite about the complete owner, so substring boundaries cannot
    // hide kerning/ligatures. Only its whole-input simple result is sufficient.
    // Other scripts, locales, features and font instances retain full analysis.
    bool english=locale==L"en"||(locale.size()>3&&locale[0]==L'e'&&locale[1]==L'n'&&locale[2]==L'-');
    if(optimize&&n<=256&&!rtl&&!features&&english&&f->supportsSimplePlacement()
        &&std::all_of(text,text+n,[](uint16_t c){return c>=0x20&&c<=0x7e;})) {
        auto& ids=scratch.ids;ids.resize(n);
        BOOL simple=FALSE;UINT32 read=0;
        if(SUCCEEDED(engine().analyzer1->GetTextComplexity((const WCHAR*)text,(UINT32)n,f->face.Get(),&simple,&read,ids.data()))&&simple&&read==n) {
            auto& result=scratch.result;result.clear();result.reserve(length);
            float scale=(float)size/f->metrics.designUnitsPerEm;
            for(size_t i=start;i<start+length;++i) {
                auto found=f->glyphMetrics.find(ids[i]);DWRITE_GLYPH_METRICS gm;
                if(found!=f->glyphMetrics.end())gm=found->second;
                else {
                    check(f->face->GetDesignGlyphMetrics(&ids[i],1,&gm,FALSE));
                    if(f->glyphMetrics.size()<4096)f->glyphMetrics.emplace(ids[i],gm);
                }
                result.push_back({ids[i],(int)(i-start),(float)(gm.advanceWidth*scale),0,0});
            }
            if(!cacheKey.empty())f->cacheShape(std::move(cacheKey),result);
            for(auto glyph:result)emit(info,glyph);
            return 1;
        }
    }
    auto paragraphBreak=[](uint16_t ch){return ch==10||ch==13||ch==0x2028||ch==0x2029;};
    size_t contextStart=start,contextEnd=start+length;
    while(contextStart&&!paragraphBreak(text[contextStart-1]))--contextStart;
    while(contextEnd<n&&!paragraphBreak(text[contextEnd]))++contextEnd;
    Analysis analysis((const WCHAR*)text+contextStart,(UINT32)(contextEnd-contextStart),locale.c_str(),rtl!=0);
    auto analyzer=engine().analyzer.Get();
    check(analyzer->AnalyzeScript(&analysis,0,analysis.length,&analysis));
    struct FeatureRanges {
        std::vector<std::vector<DWRITE_FONT_FEATURE>> data;
        std::vector<DWRITE_TYPOGRAPHIC_FEATURES> descriptions;
        std::vector<const DWRITE_TYPOGRAPHIC_FEATURES*> sets;
        std::vector<UINT32> lengths;
    };
    auto makeFeatures=[&](UINT32 begin,UINT32 window) {
        FeatureRanges ranges;UINT32 end=begin+window;
        if(!features&&begin>=start&&end<=start+length)return ranges;
        std::vector<UINT32> boundaries={begin,end};
        auto boundary=[&](size_t value){if(value>begin&&value<end)boundaries.push_back((UINT32)value);};
        boundary(start);boundary(start+length);
        for(size_t i=0;i<features;++i) {
            if(featureStarts[i]<featureEnds[i]){boundary(featureStarts[i]);boundary(featureEnds[i]);}
        }
        std::sort(boundaries.begin(),boundaries.end());boundaries.erase(std::unique(boundaries.begin(),boundaries.end()),boundaries.end());
        for(size_t i=0;i+1<boundaries.size();++i) {
            UINT32 at=boundaries[i];std::vector<DWRITE_FONT_FEATURE> active;
            if(at>=start&&at<start+length) {
                // WinUI typography defaults. An explicit DirectWrite feature set
                // replaces its implicit defaults, so an override must retain the
                // other standard features rather than disabling them accidentally.
                active={{DWRITE_FONT_FEATURE_TAG_STANDARD_LIGATURES,1},{DWRITE_FONT_FEATURE_TAG_CONTEXTUAL_LIGATURES,1},
                    {DWRITE_FONT_FEATURE_TAG_CONTEXTUAL_ALTERNATES,1},{DWRITE_FONT_FEATURE_TAG_KERNING,1},
                    {DWRITE_FONT_FEATURE_TAG_REQUIRED_LIGATURES,1}};
                for(size_t j=0;j<features;++j)if(at>=featureStarts[j]&&at<featureEnds[j]) {
                    auto tag=(DWRITE_FONT_FEATURE_TAG)_byteswap_ulong(tags[j]);
                    auto existing=std::find_if(active.begin(),active.end(),[&](const auto& feature){return feature.nameTag==tag;});
                    if(existing!=active.end())existing->parameter=(UINT32)values[j];
                    else active.push_back({tag,(UINT32)values[j]});
                }
            } else {
                // Retain surrounding characters for joining analysis, but do not
                // let a ligature consume characters outside this formatter item.
                active={{DWRITE_FONT_FEATURE_TAG_REQUIRED_LIGATURES,0}};
            }
            ranges.data.push_back(std::move(active));ranges.lengths.push_back(boundaries[i+1]-at);
        }
        ranges.descriptions.resize(ranges.data.size());ranges.sets.resize(ranges.data.size());
        for(size_t i=0;i<ranges.data.size();++i) {
            ranges.descriptions[i]={ranges.data[i].data(),(UINT32)ranges.data[i].size()};ranges.sets[i]=&ranges.descriptions[i];
        }
        return ranges;
    };
    auto& result=scratch.result;result.clear();result.reserve(std::min(length,(size_t)256));
    std::vector<Analysis::Run> runs;
    for(auto run:analysis.runs) {
        run.start+=(UINT32)contextStart;
        // Script analysis can group identical scripts across paragraph delimiters.
        // Joining does not cross those delimiters, and each paragraph owns its clusters.
        UINT32 begin=run.start,end=run.start+run.length;
        for(UINT32 i=begin;i<end;++i)if(paragraphBreak(text[i])) {
            if(i>begin)runs.push_back({begin,i-begin,run.script});
            runs.push_back({i,1,run.script});begin=i+1;
        }
        if(begin<end)runs.push_back({begin,end-begin,run.script});
    }
    for(auto run:runs) {
        if(run.start>=start+length || run.start+run.length<=start) continue;
        // DirectWrite's cluster map is 16 bit and some implementations reject
        // smaller runs too. Keep the full context where it fits; for long runs,
        // commit only a prefix DirectWrite says can be split without reflow.
        // Never guess boundaries from whitespace or split a surrogate pair.
        UINT32 end=run.start+run.length;
        for(UINT32 begin=run.start;begin<end&&begin<start+length;) {
            auto scalarLength=[&](UINT32 value) {
                if(begin+value<end&&value&&text[begin+value-1]>=0xd800&&text[begin+value-1]<=0xdbff
                    &&text[begin+value]>=0xdc00&&text[begin+value]<=0xdfff)--value;
                if(!value&&end-begin>=2&&text[begin]>=0xd800&&text[begin]<=0xdbff
                    &&text[begin+1]>=0xdc00&&text[begin+1]<=0xdfff)value=2;
                return value;
            };
            UINT32 window=scalarLength(std::min<UINT32>(end-begin,UINT16_MAX)),count=0;
            UINT32 capacity=std::min<UINT32>(UINT16_MAX,window*3/2+16);
            auto& clusters=scratch.clusters;auto& ids=scratch.ids;
            auto& tp=scratch.tp;auto& gp=scratch.gp;
            clusters.clear();ids.clear();tp.clear();gp.clear();
            FeatureRanges ranges;
            for(;;) {
                clusters.resize(window);tp.resize(window);ids.resize(capacity);gp.resize(capacity);
                ranges=makeFeatures(begin,window);
                HRESULT hr=analyzer->GetGlyphs((const WCHAR*)text+begin,window,f->face.Get(),FALSE,rtl,
                    &run.script,locale.c_str(),nullptr,ranges.sets.empty()?nullptr:ranges.sets.data(),
                    ranges.lengths.empty()?nullptr:ranges.lengths.data(),(UINT32)ranges.sets.size(),
                    capacity,clusters.data(),tp.data(),ids.data(),gp.data(),&count);
                if(hr==HRESULT_FROM_WIN32(ERROR_INSUFFICIENT_BUFFER)&&capacity<UINT16_MAX) {
                    capacity=std::min<UINT32>(UINT16_MAX,capacity*2);continue;
                }
                if(hr==E_INVALIDARG||hr==HRESULT_FROM_WIN32(ERROR_INSUFFICIENT_BUFFER)) {
                    UINT32 smaller=scalarLength(window/2);
                    if(smaller&&smaller<window) {
                        window=smaller;capacity=std::min<UINT32>(UINT16_MAX,window*3/2+16);continue;
                    }
                }
                check(hr);break;
            }
            UINT32 committed=window;
            if(begin+window<end) {
                // The artificial end of the window is not a proven boundary.
                // Leave lookahead in this shaping call and carry it into the next.
                committed=window-1;
                while(committed&&(!tp[committed-1].canBreakShapingAfter
                    ||(count&&clusters[committed-1]==clusters[committed])))--committed;
                if(!committed)throw std::runtime_error("DirectWrite shaping span exceeds its native limit without a safe split");
            }
            if(count) {
                auto& advances=scratch.advances;advances.assign(count,0);
                auto& offsets=scratch.offsets;offsets.assign(count,{});
                check(analyzer->GetGlyphPlacements((const WCHAR*)text+begin,clusters.data(),tp.data(),window,
                    ids.data(),gp.data(),count,f->face.Get(),(float)size,FALSE,rtl,&run.script,locale.c_str(),
                    ranges.sets.empty()?nullptr:ranges.sets.data(),ranges.lengths.empty()?nullptr:ranges.lengths.data(),
                    (UINT32)ranges.sets.size(),advances.data(),offsets.data()));
                auto& source=scratch.source;source.assign(count,-1);
                for(UINT32 i=0;i<window;++i) {
                    if(clusters[i]>=count)throw std::runtime_error("Invalid DirectWrite cluster map");
                    if(source[clusters[i]]<0)source[clusters[i]]=(int)(begin+i);
                }
                int cluster=(int)begin;
                for(UINT32 i=0;i<count;++i) {
                    if(source[i]>=0)cluster=source[i];
                    if(cluster>=(int)start&&cluster<(int)(start+length)&&cluster<(int)(begin+committed))
                        result.push_back({ids[i],cluster-(int)start,advances[i],offsets[i].advanceOffset,-offsets[i].ascenderOffset});
                }
            }
            begin+=committed;
        }
    }
    if(rtl) std::reverse(result.begin(),result.end());
    if(!cacheKey.empty())f->cacheShape(std::move(cacheKey),result);
    for(auto glyph:result) emit(info,glyph);
    return 1;
} catch(const std::exception& e) { fprintf(stderr,"DirectWrite: %s\n",e.what()); return 0; }
API int d2_shape(Ptr p,const uint16_t* text,size_t n,size_t start,size_t length,const char* language,
        double size,int rtl,const uint32_t* tags,const int* values,const uint32_t* featureStarts,const uint32_t* featureEnds,
        size_t features,Ptr info,void(*emit)(Ptr,Glyph)) {
    return shapeCore(true,p,text,n,start,length,language,size,rtl,tags,values,featureStarts,featureEnds,features,info,emit);
}
// Independent full-pipeline reference for exact shaping regression tests. It
// deliberately bypasses both optimization and the shape-result cache.
API int d2_shape_reference(Ptr p,const uint16_t* text,size_t n,size_t start,size_t length,const char* language,
        double size,int rtl,const uint32_t* tags,const int* values,const uint32_t* featureStarts,const uint32_t* featureEnds,
        size_t features,Ptr info,void(*emit)(Ptr,Glyph)) {
    return shapeCore(false,p,text,n,start,length,language,size,rtl,tags,values,featureStarts,featureEnds,features,info,emit);
}
API Ptr d2_path(const Command* commands,size_t n) try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    NativeTiming timing(12);
    auto result=path(); ComPtr<ID2D1GeometrySink> sink; check(result->geometry->Open(&sink));
    sink->SetFillMode(D2D1_FILL_MODE_WINDING); bool figure=false;
    for(size_t i=0;i<n;++i) {
        auto& c=commands[i]; auto p=[&](int j){return Point2F((float)c.p[j],(float)c.p[j+1]);};
        switch(c.kind) {
        case 0: if(figure) sink->EndFigure(D2D1_FIGURE_END_OPEN); sink->BeginFigure(p(0),D2D1_FIGURE_BEGIN_FILLED); figure=true; break;
        case 1: sink->AddLine(p(0)); break;
        case 2: sink->AddQuadraticBezier(QuadraticBezierSegment(p(0),p(2))); break;
        case 3: sink->AddBezier(BezierSegment(p(0),p(2),p(4))); break;
        case 4: if(figure) sink->EndFigure(D2D1_FIGURE_END_CLOSED); figure=false; break;
        }
    }
    if(figure) sink->EndFigure(D2D1_FIGURE_END_OPEN); check(sink->Close()); return result.release();
} CATCH_PTR
class EmitSink : public ID2D1SimplifiedGeometrySink {
public:
    Ptr info; void(*emit)(Ptr,Command);
    bool filled=true;
    EmitSink(Ptr p,void(*e)(Ptr,Command)):info(p),emit(e) {}
    IFACEMETHOD(QueryInterface)(REFIID iid,void** out) override { *out=nullptr; if(iid==__uuidof(IUnknown)||iid==__uuidof(ID2D1SimplifiedGeometrySink)) *out=this; return *out?S_OK:E_NOINTERFACE; }
    IFACEMETHOD_(ULONG,AddRef)() override { return 1; }
    IFACEMETHOD_(ULONG,Release)() override { return 1; }
    IFACEMETHOD_(void,SetFillMode)(D2D1_FILL_MODE) override {}
    IFACEMETHOD_(void,SetSegmentFlags)(D2D1_PATH_SEGMENT) override {}
    // Boolean intersections may return hollow boundary figures. They do not
    // contribute to the filled path exported by this sink.
    IFACEMETHOD_(void,BeginFigure)(D2D1_POINT_2F p,D2D1_FIGURE_BEGIN begin) override { filled=begin==D2D1_FIGURE_BEGIN_FILLED;if(filled)emit(info,{0,{p.x,p.y}}); }
    IFACEMETHOD_(void,AddLines)(const D2D1_POINT_2F* p,UINT32 n) override { if(filled)for(UINT32 i=0;i<n;++i) emit(info,{1,{p[i].x,p[i].y}}); }
    IFACEMETHOD_(void,AddBeziers)(const D2D1_BEZIER_SEGMENT* p,UINT32 n) override { if(filled)for(UINT32 i=0;i<n;++i) emit(info,{3,{p[i].point1.x,p[i].point1.y,p[i].point2.x,p[i].point2.y,p[i].point3.x,p[i].point3.y}}); }
    IFACEMETHOD_(void,EndFigure)(D2D1_FIGURE_END end) override { if(filled&&end==D2D1_FIGURE_END_CLOSED) emit(info,{4,{}}); }
    IFACEMETHOD(Close)() override { return S_OK; }
};
API void d2_path_elements(Ptr p,Ptr info,void(*emit)(Ptr,Command)) try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    NativeTiming timing(13);
    EmitSink sink(info,emit); check(static_cast<Path*>(p)->geometry->Simplify(D2D1_GEOMETRY_SIMPLIFICATION_OPTION_CUBICS_AND_LINES,nullptr,&sink));
} CATCH_VOID
API Ptr d2_combine(Ptr a,Ptr b,int op,int ea,int eb) try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    auto result=path(); ComPtr<ID2D1GeometrySink> sink; check(result->geometry->Open(&sink));
    auto left=rule(static_cast<Path*>(a),ea),right=rule(static_cast<Path*>(b),eb);
    const D2D1_COMBINE_MODE modes[]={D2D1_COMBINE_MODE_UNION,D2D1_COMBINE_MODE_INTERSECT,D2D1_COMBINE_MODE_XOR,D2D1_COMBINE_MODE_EXCLUDE};
    check(left->CombineWithGeometry(right.Get(),modes[std::clamp(op,0,3)],nullptr,sink.Get())); check(sink->Close()); return result.release();
} CATCH_PTR
API Ptr d2_stroke(Ptr p,double width,int cap,int join,double miter,const double* dash,size_t n,double phase) try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    auto result=path(); ComPtr<ID2D1GeometrySink> sink; check(result->geometry->Open(&sink));
    if(width>0) { auto style=strokeStyle(width,cap,join,miter,dash,n,phase); check(static_cast<Path*>(p)->geometry->Widen((float)width,style.Get(),nullptr,sink.Get())); }
    check(sink->Close()); return result.release();
} CATCH_PTR
API Ptr d2_glyph_path(Ptr p,uint16_t glyph,double size) try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    NativeTiming timing(8);
    auto result=path(); ComPtr<ID2D1GeometrySink> sink; check(result->geometry->Open(&sink));
    check(static_cast<Font*>(p)->face->GetGlyphRunOutline((float)size,&glyph,nullptr,nullptr,1,FALSE,FALSE,sink.Get()));
    check(sink->Close()); return result.release();
} CATCH_PTR
static void glyphBounds(Font* f,uint16_t glyph,double size,double* out) {
    DWRITE_GLYPH_METRICS gm{};
    if(!f->hasMetrics){f->face->GetMetrics(&f->metrics);f->hasMetrics=true;}
    auto found=f->glyphMetrics.find(glyph);
    if(found!=f->glyphMetrics.end())gm=found->second;
    else {
        check(f->face->GetDesignGlyphMetrics(&glyph,1,&gm,FALSE));
        if(f->glyphMetrics.size()<4096)f->glyphMetrics.emplace(glyph,gm);
    }
    double scale=size/f->metrics.designUnitsPerEm;
    out[0]=gm.leftSideBearing*scale; out[1]=(gm.topSideBearing-gm.verticalOriginY)*scale;
    out[2]=(gm.advanceWidth-gm.leftSideBearing-gm.rightSideBearing)*scale;
    out[3]=(gm.advanceHeight-gm.topSideBearing-gm.bottomSideBearing)*scale;
}
API void d2_glyph_bounds(Ptr p,uint16_t glyph,double size,double* out) try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    NativeTiming timing(9);glyphBounds(static_cast<Font*>(p),glyph,size,out);
} CATCH_VOID
API void d2_glyph_run_bounds(Ptr p,double size,const uint16_t* ids,const double* positions,size_t count,double* out) try {std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    NativeTiming timing(9);auto font=static_cast<Font*>(p);bool ink=false;double left=0,top=0,right=0,bottom=0;
    for(size_t i=0;i<count;++i){
        double b[4];glyphBounds(font,ids[i],size,b);
        if(std::all_of(b,b+4,[](double x){return std::isfinite(x);})&&b[2]>0&&b[3]>0){
            double x=positions[i*2]+b[0],y=positions[i*2+1]+b[1];
            if(!ink){left=x;top=y;right=x+b[2];bottom=y+b[3];ink=true;}
            else {left=std::min(left,x);top=std::min(top,y);right=std::max(right,x+b[2]);bottom=std::max(bottom,y+b[3]);}
        }
    }
    out[0]=left;out[1]=top;out[2]=right-left;out[3]=bottom-top;
} CATCH_VOID

struct Clip {
    ComPtr<ID2D1Geometry> geometry;
    D2D1_MATRIX_3X2_F transform;
    bool rectangle=false;
    D2D1_RECT_F bounds{};
    D2D1_RECT_F deviceBounds{};
    D2D1_RECT_F interior{};
    bool applied=true;
    bool integral=false;
    bool redundant=false;
};
struct GlyphBatch {
    ComPtr<IDWriteFontFace> face;
    float size=0;
    D2D1_COLOR_F color{};
    D2D1_TEXT_ANTIALIAS_MODE mode{};
    std::vector<uint16_t> ids;
    std::vector<float> advances;
    std::vector<DWRITE_GLYPH_OFFSET> offsets;
    std::vector<D2D1_RECT_F> ink;
    D2D1_RECT_F rowInk{};
    float inkTop=std::numeric_limits<float>::infinity(),inkBottom=-std::numeric_limits<float>::infinity();
    bool disjointRow=false;
    void clear(){ids.clear();advances.clear();offsets.clear();ink.clear();face.Reset();disjointRow=false;inkTop=std::numeric_limits<float>::infinity();inkBottom=-std::numeric_limits<float>::infinity();}
};
struct Frame {
    ComPtr<ID3D11Device> deviceOwner;
    ComPtr<ID2D1DeviceContext> dc;
    ComPtr<ID2D1DeviceContext4> dc4;
    ComPtr<ID2D1Bitmap1> bitmap;
    ComPtr<ID2D1SolidColorBrush> solidBrush;
    D2D1_COLOR_F solidColor{};
    std::vector<Clip> clips;
    GlyphBatch glyphBatch;
    bool drawing=false;
    float opacity=1;
    int blend=0;
    bool hasColorMatrix=false;
    D2D1_MATRIX_5X4_F colorMatrix{};
    D2D1_COLORMATRIX_ALPHA_MODE colorMatrixAlphaMode=D2D1_COLORMATRIX_ALPHA_MODE_STRAIGHT;
    void begin() { if(!drawing) { dc->BeginDraw(); drawing=true; } }
    ID2D1SolidColorBrush* solid(D2D1_COLOR_F value) {
        if(!solidBrush)check(dc->CreateSolidColorBrush(value,&solidBrush));
        else if(value.r!=solidColor.r||value.g!=solidColor.g||value.b!=solidColor.b||value.a!=solidColor.a)solidBrush->SetColor(value);
        solidColor=value;return solidBrush.Get();
    }
    void end() { NativeTiming timing(0);if(drawing) { drawing=false; check(dc->EndDraw()); } }
};
bool PositionedGlyphRun::bounds(Frame& f,const D2D1_MATRIX_3X2_F& matrix,const D2D1_MATRIX_3X2_F& current,double shift,int alias,D2D1_RECT_F& result) const {
    if(color||ids.empty()||!std::isfinite(matrix._31)||!std::isfinite(matrix._32)||fabs(matrix._31)>1000000||fabs(matrix._32)>1000000)return false;
    float tx=(float)(int)matrix._31,ty=(float)(int)matrix._32;
    auto normalized=matrix;normalized._31-=tx;normalized._32-=ty;
    float baseline=(float)(y+shift);auto generation=engine().generation;
    auto found=std::find_if(inkBounds.begin(),inkBounds.end(),[&](const BoundsEntry& e){return e.valid&&e.generation==generation&&e.alias==alias&&e.y==baseline&&!memcmp(&e.matrix,&normalized,sizeof(normalized));});
    if(found==inkBounds.end()) {
        NativeTiming miss(17);
        auto glyphs=run();D2D1_RECT_F bounds{};
        f.dc->SetTransform(normalized);
        HRESULT hr=f.dc->GetGlyphRunWorldBounds(Point2F((float)x,baseline),&glyphs,DWRITE_MEASURING_MODE_NATURAL,&bounds);
        f.dc->SetTransform(current);
        if(FAILED(hr)||!std::isfinite(bounds.left)||!std::isfinite(bounds.top)||!std::isfinite(bounds.right)||!std::isfinite(bounds.bottom))return false;
        BoundsEntry entry{normalized,baseline,alias,generation,RectF(pixelFloor(bounds.left),pixelFloor(bounds.top),pixelCeil(bounds.right),pixelCeil(bounds.bottom)),true};
        size_t index=inkBounds.size();
        if(index<32)inkBounds.push_back(entry);else{index=nextBounds++%inkBounds.size();inkBounds[index]=entry;}
        found=inkBounds.begin()+index;
    } else if(nativeProfiling)++nativeTimingCounts[16];
    auto& b=found->bounds;result=RectF(b.left+tx,b.top+ty,b.right+tx,b.bottom+ty);return true;
}
static void popClip(Frame& frame,const Clip& clip) {
    if(!clip.applied)return;
    if(clip.rectangle)frame.dc->PopAxisAlignedClip();else frame.dc->PopLayer();
}
static void suspendClips(Frame& frame) {
    for(auto i=frame.clips.rbegin();i!=frame.clips.rend();++i)popClip(frame,*i);
}
static void recycleFrame(std::shared_ptr<Frame> frame,uint64_t generation,bool lost) {
    frame->dc->SetTarget(nullptr);frame->clips.clear();auto size=frame->bitmap->GetPixelSize();
    if(nativeShuttingDown)return;
    auto& e=engine();size_t bytes=(size_t)size.width*size.height*4;
    const size_t limit=64*1024*1024;
    if(lost||generation!=e.generation||bytes>limit)return;
    // The pool holds the frames used last: a frame of a size no surface asks for any more (a
    // window that was resized) leaves when a newer one needs its room.
    while(!e.framePool.empty()&&(e.framePool.size()>=16||e.pooledBytes+bytes>limit)) {
        auto oldest=e.framePool.front()->bitmap->GetPixelSize();
        e.pooledBytes-=(size_t)oldest.width*oldest.height*4;e.framePool.erase(e.framePool.begin());
    }
    e.pooledBytes+=bytes;e.framePool.push_back(std::move(frame));
}
struct Save { D2D1_MATRIX_3X2_F transform; size_t clips; int spread; float aspect,angle; int interpolation; };
struct Surface : Object {
    uint64_t generation;
    UINT32 width,height;
    std::vector<std::shared_ptr<Frame>> frames;
    // Internal layer images may be referenced by a parent's queued commands.
    // Keep their targets immutable until the root EndDraw submits those reads.
    std::vector<std::shared_ptr<Frame>> retired;
    ComPtr<ID2D1Bitmap1> cachedSnapshot;
    std::vector<Save> saves;
    int spread=0,interpolation=1;
    float aspect=1,angle=0;
    bool lost=false;
    Surface(UINT32 w,UINT32 h):generation(engine().generation),width(w),height(h) { ++statistics[0]; frames.push_back(makeFrame()); }
    ~Surface() {
        for(auto& f:frames) {
            if(f->drawing) {
                suspendClips(*f);
                f->dc->EndDraw();f->drawing=false;
            }
            recycleFrame(std::move(f),generation,lost);
        }
        releaseRetired();
    }
    void releaseRetired() {for(auto& f:retired)recycleFrame(std::move(f),generation,lost);retired.clear();}
    // The retired layer targets of a frame with many layers are bounded: once they hold more
    // than the frame pool does, the commands of the open frames that read them are submitted
    // (a flush, not a wait for the GPU) and the targets return to the pool.
    void boundRetired() {
        if(retired.size()*((size_t)width*height*4)<64*1024*1024)return;
        for(auto& f:frames)if(f->drawing&&FAILED(f->dc->Flush()))return;
        releaseRetired();
    }
    std::shared_ptr<Frame> makeFrame() {
        NativeTiming timing(1);
        auto& e=engine();std::shared_ptr<Frame> result;
        auto found=std::find_if(e.framePool.begin(),e.framePool.end(),[&](const auto& frame){auto size=frame->bitmap->GetPixelSize();return size.width==width&&size.height==height;});
        if(found!=e.framePool.end()){result=std::move(*found);e.framePool.erase(found);e.pooledBytes-=(size_t)width*height*4;}
        else {
        result=std::make_shared<Frame>();
        result->deviceOwner=e.d3d;
        check(e.device->CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE,&result->dc));
        result->dc.As(&result->dc4);
        D2D1_BITMAP_PROPERTIES1 properties=BitmapProperties1(D2D1_BITMAP_OPTIONS_TARGET,
                PixelFormat(DXGI_FORMAT_B8G8R8A8_UNORM,D2D1_ALPHA_MODE_PREMULTIPLIED));
        check(result->dc->CreateBitmap(SizeU(width,height),nullptr,0,&properties,&result->bitmap));
        }
        result->dc->SetTarget(result->bitmap.Get()); result->dc->SetUnitMode(D2D1_UNIT_MODE_PIXELS);
        result->dc->SetTransform(Matrix3x2F::Identity());result->dc->SetPrimitiveBlend(D2D1_PRIMITIVE_BLEND_SOURCE_OVER);
        result->dc->SetAntialiasMode(D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);result->opacity=1;result->blend=0;result->hasColorMatrix=false;
        // Offscreen/translucent targets always use grayscale. Color fringes must
        // never enter an opacity/effect layer. Opaque window policy is set separately.
        result->dc->SetTextAntialiasMode(D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE);
        result->begin(); result->dc->Clear(ColorF(0,0,0,0)); return result;
    }
    Frame& frame() { if(generation!=engine().generation)throw std::runtime_error("Stale Direct2D device generation");cachedSnapshot.Reset();auto& f=*frames.back(); f.begin(); return f; }
    D2D1_RECT_U clippedBounds() {
        auto bounds=RectF(0,0,(float)width,(float)height);
        for(auto& clip:frames.back()->clips) {
            D2D1_RECT_F clipped{};
            if(clip.rectangle) {
                clipped=clip.deviceBounds;
            } else check(clip.geometry->GetBounds(&clip.transform,&clipped));
            bounds.left=std::max(bounds.left,clipped.left);bounds.top=std::max(bounds.top,clipped.top);
            bounds.right=std::min(bounds.right,clipped.right);bounds.bottom=std::min(bounds.bottom,clipped.bottom);
        }
        if(bounds.right<=bounds.left||bounds.bottom<=bounds.top)return RectU(0,0,0,0);
        // Keep the antialias fringe at fractional or transformed clip edges.
        return RectU((UINT)std::max<double>(0.,floor(bounds.left)-1),(UINT)std::max<double>(0.,floor(bounds.top)-1),
            (UINT)std::min<double>(width,ceil(bounds.right)+1),(UINT)std::min<double>(height,ceil(bounds.bottom)+1));
    }
    ComPtr<ID2D1Bitmap1> snapshot(const D2D1_RECT_U& region) {
        NativeTiming timing(2);
        ++statistics[7];
        auto& f=*frames.back(); bool active=f.drawing;
        // Layers used for clips must remain balanced even around copies.
        suspendClips(f);
        f.end();
        ComPtr<ID2D1Bitmap1> copy;
        auto properties=BitmapProperties1(D2D1_BITMAP_OPTIONS_NONE,
                PixelFormat(DXGI_FORMAT_B8G8R8A8_UNORM,D2D1_ALPHA_MODE_PREMULTIPLIED));
        check(f.dc->CreateBitmap(SizeU(region.right-region.left,region.bottom-region.top),nullptr,0,&properties,&copy));
        check(copy->CopyFromBitmap(nullptr,f.bitmap.Get(),&region));
        if(active) { f.begin(); for(auto& clip:f.clips) applyClip(f,clip); }
        return copy;
    }
    ComPtr<ID2D1Bitmap1> snapshot() {
        bool immutable=frames.size()==1&&!frames.back()->drawing;
        if(immutable&&cachedSnapshot)return cachedSnapshot;
        auto image=snapshot(RectU(0,0,width,height));
        if(immutable)cachedSnapshot=image;
        return image;
    }
    static void applyClip(Frame& f,const Clip& clip) {
        if(!clip.applied)return;
        NativeTiming timing(6);
        D2D1_MATRIX_3X2_F previous;f.dc->GetTransform(&previous);
        if(clip.rectangle) {
            f.dc->SetTransform(clip.transform);
            f.dc->PushAxisAlignedClip(clip.bounds,D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);
        } else {
            f.dc->SetTransform(Matrix3x2F::Identity());
            auto properties=LayerParameters1(InfiniteRect(),clip.geometry.Get(),D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,clip.transform);
            f.dc->PushLayer(properties,nullptr);
        }
        f.dc->SetTransform(previous);
    }
};
static void materializeClips(Frame& frame) {
    for(auto& clip:frame.clips)if(!clip.applied&&!clip.redundant){clip.applied=true;Surface::applyClip(frame,clip);}
}
// A surface that could not be created reaches the bridge as a null handle (Bitmap keeps one
// while the device is unavailable); every export fails as a unit instead of reading through it.
static Surface& surface(Ptr p) { if(!p)throw std::runtime_error("Direct2D surface is null"); return *static_cast<Surface*>(p); }
static D2D1_COLOR_F color(double r,double g,double b,double a) { return ColorF((float)r,(float)g,(float)b,(float)a); }
static ComPtr<ID2D1Effect> effect(Frame& f,REFCLSID id,ID2D1Image* image) {
    ComPtr<ID2D1Effect> result; check(f.dc->CreateEffect(id,&result)); if(image) result->SetInput(0,image); return result;
}
static void replaceImage(Surface& s,ID2D1Image* image) {
    auto& f=s.frame(); D2D1_MATRIX_3X2_F m; f.dc->GetTransform(&m);
    // All filter inputs are independent snapshots, so clearing cannot create a feedback loop.
    suspendClips(f);
    f.dc->SetTransform(Matrix3x2F::Identity()); f.dc->Clear(ColorF(0,0,0,0));
    f.dc->DrawImage(image,nullptr,nullptr,D2D1_INTERPOLATION_MODE_LINEAR,D2D1_COMPOSITE_MODE_SOURCE_COPY);
    f.dc->SetTransform(m); for(auto& clip:f.clips) Surface::applyClip(f,clip);
}
static ComPtr<ID2D1Bitmap1> runKernel(Surface& s,int kind,const float* data,size_t n,Surface* secondary,UINT width=0,UINT height=0) {
    NativeTiming timing(3);
    if(!width)width=s.width;if(!height)height=s.height;
    ++statistics[10];statistics[11]+=(uint64_t)width*height;
    auto& e=engine();bool procedural=kind==8||(kind==6&&n>=12&&data[8]==0);
    ComPtr<ID3D11Texture2D> inputTexture,otherTexture;
    if(!procedural) {
        auto input=s.snapshot();auto other=secondary?secondary->snapshot():input;
        ComPtr<IDXGISurface> inputSurface,otherSurface;check(input->GetSurface(&inputSurface));check(other->GetSurface(&otherSurface));
        check(inputSurface.As(&inputTexture));check(otherSurface.As(&otherTexture));
    }
    auto makeView=[&](ID3D11Texture2D* source) {
        D3D11_TEXTURE2D_DESC desc{};source->GetDesc(&desc);
        ComPtr<ID3D11ShaderResourceView> view;
        if(desc.BindFlags&D3D11_BIND_SHADER_RESOURCE) {
            check(e.d3d->CreateShaderResourceView(source,nullptr,&view));return view;
        }
        desc.BindFlags=D3D11_BIND_SHADER_RESOURCE;desc.MiscFlags=0;desc.CPUAccessFlags=0;desc.Usage=D3D11_USAGE_DEFAULT;
        ComPtr<ID3D11Texture2D> texture;check(e.d3d->CreateTexture2D(&desc,nullptr,&texture));e.immediate->CopyResource(texture.Get(),source);++statistics[7];
        check(e.d3d->CreateShaderResourceView(texture.Get(),nullptr,&view));return view;
    };
    ComPtr<ID3D11Multithread> multithread;check(e.immediate.As(&multithread));multithread->Enter();
    struct Unlock {ID3D11Multithread* lock;~Unlock(){lock->Leave();}} unlock{multithread.Get()};
    if(procedural&&!e.neutralShaderInput) {
        D3D11_TEXTURE2D_DESC desc{};desc.Width=desc.Height=desc.MipLevels=desc.ArraySize=1;
        desc.Format=DXGI_FORMAT_R8G8B8A8_UNORM;desc.SampleDesc.Count=1;desc.BindFlags=D3D11_BIND_SHADER_RESOURCE|D3D11_BIND_RENDER_TARGET;
        ComPtr<ID3D11Texture2D> texture;check(e.d3d->CreateTexture2D(&desc,nullptr,&texture));
        ComPtr<ID3D11RenderTargetView> clear;check(e.d3d->CreateRenderTargetView(texture.Get(),nullptr,&clear));
        float zero[4]{};e.immediate->ClearRenderTargetView(clear.Get(),zero);
        check(e.d3d->CreateShaderResourceView(texture.Get(),nullptr,&e.neutralShaderInput));
    }
    auto source=procedural?e.neutralShaderInput:makeView(inputTexture.Get());
    auto second=secondary&&!procedural?makeView(otherTexture.Get()):source;
    std::vector<float> values(std::max((size_t)4,((n+3)/4)*4),0);if(n)memcpy(values.data(),data,n*sizeof(float));
    D3D11_BUFFER_DESC description{};description.ByteWidth=(UINT)values.size()*4;description.Usage=D3D11_USAGE_IMMUTABLE;
    description.BindFlags=D3D11_BIND_SHADER_RESOURCE;description.MiscFlags=D3D11_RESOURCE_MISC_BUFFER_STRUCTURED;description.StructureByteStride=16;
    D3D11_SUBRESOURCE_DATA initial{values.data(),0,0};ComPtr<ID3D11Buffer> buffer;check(e.d3d->CreateBuffer(&description,&initial,&buffer));
    ComPtr<ID3D11ShaderResourceView> uniforms;check(e.d3d->CreateShaderResourceView(buffer.Get(),nullptr,&uniforms));
    UINT parameters[4]={(UINT)kind,width,height,(UINT)n};description={};description.ByteWidth=16;description.Usage=D3D11_USAGE_IMMUTABLE;description.BindFlags=D3D11_BIND_CONSTANT_BUFFER;
    initial.pSysMem=parameters;ComPtr<ID3D11Buffer> constants;check(e.d3d->CreateBuffer(&description,&initial,&constants));
    D3D11_TEXTURE2D_DESC out{};out.Width=width;out.Height=height;out.MipLevels=out.ArraySize=1;out.Format=DXGI_FORMAT_R8G8B8A8_UNORM;
    out.SampleDesc.Count=1;out.BindFlags=D3D11_BIND_UNORDERED_ACCESS|D3D11_BIND_SHADER_RESOURCE;
    ComPtr<ID3D11Texture2D> target;check(e.d3d->CreateTexture2D(&out,nullptr,&target));ComPtr<ID3D11UnorderedAccessView> view;
    check(e.d3d->CreateUnorderedAccessView(target.Get(),nullptr,&view));
    if(!e.filterShader)check(e.d3d->CreateComputeShader(ferro_effect_shader,sizeof(ferro_effect_shader),nullptr,&e.filterShader));
    D3D11_SAMPLER_DESC sampler{};sampler.Filter=D3D11_FILTER_MIN_MAG_MIP_LINEAR;sampler.MaxLOD=D3D11_FLOAT32_MAX;
    sampler.AddressU=sampler.AddressV=sampler.AddressW=D3D11_TEXTURE_ADDRESS_BORDER;ComPtr<ID3D11SamplerState> border,clamp;
    check(e.d3d->CreateSamplerState(&sampler,&border));sampler.AddressU=sampler.AddressV=sampler.AddressW=D3D11_TEXTURE_ADDRESS_CLAMP;check(e.d3d->CreateSamplerState(&sampler,&clamp));
    // Preserve the immediate context's compute state for other native users.
    ID3D11ShaderResourceView* oldViews[3]{};ID3D11UnorderedAccessView* oldOutput=nullptr;ID3D11SamplerState* oldSamplers[2]{};
    ID3D11Buffer* oldConstants=nullptr;ID3D11ComputeShader* oldShader=nullptr;ID3D11ClassInstance* oldInstances[256]{};UINT instanceCount=256;
    e.immediate->CSGetShaderResources(0,3,oldViews);e.immediate->CSGetUnorderedAccessViews(0,1,&oldOutput);
    e.immediate->CSGetSamplers(0,2,oldSamplers);e.immediate->CSGetConstantBuffers(0,1,&oldConstants);e.immediate->CSGetShader(&oldShader,oldInstances,&instanceCount);
    ID3D11ShaderResourceView* views[]={source.Get(),second.Get(),uniforms.Get()};ID3D11UnorderedAccessView* outputs[]={view.Get()};
    ID3D11SamplerState* samplers[]={border.Get(),clamp.Get()};ID3D11Buffer* constantBuffers[]={constants.Get()};
    e.immediate->CSSetShader(e.filterShader.Get(),nullptr,0);e.immediate->CSSetShaderResources(0,3,views);e.immediate->CSSetUnorderedAccessViews(0,1,outputs,nullptr);
    e.immediate->CSSetSamplers(0,2,samplers);e.immediate->CSSetConstantBuffers(0,1,constantBuffers);e.immediate->Dispatch((width+7)/8,(height+7)/8,1);
    ID3D11ShaderResourceView* emptyViews[3]{};ID3D11UnorderedAccessView* emptyOutput=nullptr;
    e.immediate->CSSetShaderResources(0,3,emptyViews);e.immediate->CSSetUnorderedAccessViews(0,1,&emptyOutput,nullptr);
    e.immediate->CSSetShader(oldShader,oldInstances,instanceCount);e.immediate->CSSetShaderResources(0,3,oldViews);e.immediate->CSSetUnorderedAccessViews(0,1,&oldOutput,nullptr);
    e.immediate->CSSetSamplers(0,2,oldSamplers);e.immediate->CSSetConstantBuffers(0,1,&oldConstants);
    for(auto p:oldViews)if(p)p->Release();for(auto p:oldSamplers)if(p)p->Release();for(UINT i=0;i<instanceCount;++i)if(oldInstances[i])oldInstances[i]->Release();
    if(oldOutput)oldOutput->Release();if(oldConstants)oldConstants->Release();if(oldShader)oldShader->Release();
    ComPtr<IDXGISurface> dxgi;check(target.As(&dxgi));ComPtr<ID2D1Bitmap1> result;
    auto properties=BitmapProperties1(D2D1_BITMAP_OPTIONS_NONE,PixelFormat(DXGI_FORMAT_R8G8B8A8_UNORM,D2D1_ALPHA_MODE_PREMULTIPLIED));
    check(s.frame().dc->CreateBitmapFromDxgiSurface(dxgi.Get(),&properties,&result));return result;
}
API void d2_gpu_draw_lighting(Ptr p,UINT width,UINT height,double x,double y,const float* data,size_t n) try {
    std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    if(!width||!height||width>16384||height>16384||n<12||data[8]!=0)throw std::runtime_error("Invalid procedural lighting bounds");
    auto& s=surface(p);auto image=runKernel(s,6,data,n,nullptr,width,height);auto& f=s.frame();
    D2D1_MATRIX_3X2_F previous;f.dc->GetTransform(&previous);f.dc->SetTransform(Matrix3x2F::Identity());
    f.dc->DrawImage(image.Get(),Point2F((float)x,(float)y));f.dc->SetTransform(previous);
} CATCH_VOID
API Ptr d2_surface(size_t w,size_t h) try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    if(!w||!h||w>16384||h>16384) return nullptr; return new Surface((UINT32)w,(UINT32)h);
} CATCH_PTR
API void d2_finish(Ptr p) try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    auto& s=surface(p); auto& f=*s.frames.back();
    if(s.frames.size()!=1 || !s.saves.empty() || !f.clips.empty()) throw std::runtime_error("Unbalanced Direct2D drawing state");
    f.end();s.releaseRetired();
} catch(const std::exception& e) { if(p)static_cast<Surface*>(p)->lost=true; fprintf(stderr,"Direct2D finish: %s\n",e.what()); }
API Ptr d2_snapshot(Ptr p) try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    auto& s=surface(p); auto result=std::make_unique<Surface>(s.width,s.height);
    auto image=s.snapshot(); replaceImage(*result,image.Get()); return result.release();
} CATCH_PTR
API void d2_gpu_copy_surface(Ptr target,Ptr source) try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    auto image=surface(source).snapshot(); replaceImage(surface(target),image.Get());
} CATCH_VOID
API void d2_save(Ptr p) try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    auto& s=surface(p); auto& f=s.frame(); D2D1_MATRIX_3X2_F m; f.dc->GetTransform(&m);
    s.saves.push_back({m,f.clips.size(),s.spread,s.aspect,s.angle,s.interpolation});
} CATCH_VOID
API void d2_restore(Ptr p) try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    auto& s=surface(p); auto& f=s.frame(); if(s.saves.empty()) throw std::runtime_error("Unbalanced Direct2D restore");
    auto saved=s.saves.back(); s.saves.pop_back();
    while(f.clips.size()>saved.clips) { popClip(f,f.clips.back()); f.clips.pop_back(); }
    f.dc->SetTransform(saved.transform); s.spread=saved.spread; s.aspect=saved.aspect; s.angle=saved.angle; s.interpolation=saved.interpolation;
} CATCH_VOID
API void d2_set_transform(Ptr p,double a,double b,double c,double d,double x,double y,double sx,double sy) try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    surface(p).frame().dc->SetTransform(Matrix3x2F((float)(a*sx),(float)(b*sy),(float)(c*sx),(float)(d*sy),(float)(x*sx),(float)(y*sy)));
} CATCH_VOID
API void d2_clear(Ptr p,double,double,double r,double g,double b,double a) try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex); surface(p).frame().dc->Clear(color(r,g,b,a)); } CATCH_VOID
API void d2_clip(Ptr p,Ptr geometry,int evenodd) try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    auto& f=surface(p).frame(); Clip clip{rule(static_cast<Path*>(geometry),evenodd),{}}; f.dc->GetTransform(&clip.transform);
    f.clips.push_back(clip); Surface::applyClip(f,clip);
} CATCH_VOID
static void pushRectClip(Frame& f,const D2D1_MATRIX_3X2_F& transform,double x,double y,double width,double height,bool deferred=false) {
    Clip clip{};clip.transform=transform;
    clip.bounds=RectF((float)x,(float)y,(float)(x+std::max(0.,width)),(float)(y+std::max(0.,height)));
    // An axis-aligned clip transforms to an AABB. Keep geometry clipping for
    // rotated/skewed rectangles so their corners do not leak into that AABB.
    clip.rectangle=clip.transform._12==0&&clip.transform._21==0;
    if(clip.rectangle) {
        auto m=Matrix3x2F::ReinterpretBaseType(&clip.transform);
        auto a=m->TransformPoint(Point2F(clip.bounds.left,clip.bounds.top));
        auto b=m->TransformPoint(Point2F(clip.bounds.right,clip.bounds.bottom));
        clip.deviceBounds=RectF(std::min(a.x,b.x),std::min(a.y,b.y),std::max(a.x,b.x),std::max(a.y,b.y));
        auto pixel=[](float x){return x>=-16777216.f&&x<=16777216.f&&x==static_cast<int>(x);};
        auto& r=clip.deviceBounds;clip.integral=pixel(r.left)&&pixel(r.top)&&pixel(r.right)&&pixel(r.bottom);
        // A covering integer-pixel rectangle cannot further restrict an
        // existing integer-pixel rectangle. Keep fractional scopes: removing
        // those can change antialias coverage at the shared boundary.
        if(clip.integral)for(auto i=f.clips.rbegin();i!=f.clips.rend();++i) {
            if(i->rectangle&&!i->redundant&&i->integral) {
                auto& outer=clip.deviceBounds;auto& inner=i->deviceBounds;
                if(outer.left<=inner.left&&outer.top<=inner.top&&outer.right>=inner.right&&outer.bottom>=inner.bottom){clip.redundant=true;break;}
            }
        }
        clip.interior=clip.integral?clip.deviceBounds:RectF(pixelCeil(r.left),pixelCeil(r.top),pixelFloor(r.right),pixelFloor(r.bottom));
        clip.applied=!clip.redundant&&!deferred;
    } else {
        materializeClips(f);
        ComPtr<ID2D1RectangleGeometry> geometry;check(engine().factory->CreateRectangleGeometry(clip.bounds,&geometry));clip.geometry=geometry;
    }
    f.clips.push_back(clip);if(clip.rectangle&&clip.applied){NativeTiming timing(6);f.dc->PushAxisAlignedClip(clip.bounds,D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);}else Surface::applyClip(f,clip);
}
API void d2_clip_rect(Ptr p,double x,double y,double width,double height) try {std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    auto& f=surface(p).frame();D2D1_MATRIX_3X2_F transform;f.dc->GetTransform(&transform);pushRectClip(f,transform,x,y,width,height);
} CATCH_VOID
API void d2_fill(Ptr p,Ptr geometry,int evenodd,double r,double g,double b,double a) try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    ++statistics[1];
    auto& f=surface(p).frame();auto brush=f.solid(color(r,g,b,a));
    auto path=rule(static_cast<Path*>(geometry),evenodd); f.dc->FillGeometry(path.Get(),brush);
} CATCH_VOID
API void d2_gpu_gradient_options(Ptr p,int spread,double aspect,double angle) try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    auto& s=surface(p); s.spread=spread; s.aspect=(float)aspect; s.angle=(float)angle;
} CATCH_VOID
API void d2_gradient(Ptr p,const double* colors,const double* stops,size_t n,double x1,double y1,double x2,double y2,double radius,int radial) try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    if(!n) return; auto& s=surface(p); auto& f=s.frame();
    std::vector<D2D1_GRADIENT_STOP> entries;
    for(size_t i=0;i<n;++i) entries.push_back({(float)stops[i],color(colors[i*4],colors[i*4+1],colors[i*4+2],colors[i*4+3])});
    ComPtr<ID2D1GradientStopCollection> collection;
    auto spread=s.spread==1?D2D1_EXTEND_MODE_MIRROR:s.spread==2?D2D1_EXTEND_MODE_WRAP:D2D1_EXTEND_MODE_CLAMP;
    check(f.dc->CreateGradientStopCollection(entries.data(),(UINT32)n,D2D1_GAMMA_2_2,spread,&collection));
    ComPtr<ID2D1Brush> brush;
    if(radial==0) {
        ComPtr<ID2D1LinearGradientBrush> linear;
        check(f.dc->CreateLinearGradientBrush(LinearGradientBrushProperties(Point2F((float)x1,(float)y1),Point2F((float)x2,(float)y2)),collection.Get(),&linear)); brush=linear;
    } else if(radial==1) {
        ComPtr<ID2D1RadialGradientBrush> radialBrush;
        check(f.dc->CreateRadialGradientBrush(RadialGradientBrushProperties(Point2F((float)x2,(float)y2),Point2F((float)(x1-x2),(float)(y1-y2)),(float)radius,(float)radius/std::max(s.aspect,1e-6f)),collection.Get(),&radialBrush)); brush=radialBrush;
    } else {
        D2D1_MATRIX_3X2_F m;f.dc->GetTransform(&m);auto inv=*Matrix3x2F::ReinterpretBaseType(&m);if(!inv.Invert())return;
        std::vector<float> values={inv._11,inv._21,inv._31,0,inv._12,inv._22,inv._32,0,(float)x1,(float)y1,s.angle,(float)n,0,0,0,0};
        for(size_t i=0;i<n;++i){for(int c=0;c<4;++c)values.push_back((float)colors[i*4+c]);values.insert(values.end(),{(float)stops[i],0,0,0});}
        auto image=runKernel(s,8,values.data(),values.size(),nullptr);f.dc->SetTransform(Matrix3x2F::Identity());f.dc->DrawImage(image.Get());f.dc->SetTransform(m);return;
    }
    // Inverse transform of the device rectangle covers reflected and rotated brushes.
    D2D1_MATRIX_3X2_F m; f.dc->GetTransform(&m); Matrix3x2F inverse=*Matrix3x2F::ReinterpretBaseType(&m);
    if(!inverse.Invert()) return;
    D2D1_POINT_2F corners[]={inverse.TransformPoint(Point2F()),inverse.TransformPoint(Point2F((float)s.width,0)),inverse.TransformPoint(Point2F(0,(float)s.height)),inverse.TransformPoint(Point2F((float)s.width,(float)s.height))};
    auto bounds=RectF(corners[0].x,corners[0].y,corners[0].x,corners[0].y);
    for(auto corner:corners) { bounds.left=std::min(bounds.left,corner.x);bounds.top=std::min(bounds.top,corner.y);bounds.right=std::max(bounds.right,corner.x);bounds.bottom=std::max(bounds.bottom,corner.y); }
    f.dc->FillRectangle(bounds,brush.Get());
} CATCH_VOID
API void d2_begin_layer(Ptr p,double opacity,int blend) try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    auto& s=surface(p); auto& parent=s.frame(); s.boundRetired(); auto next=s.makeFrame();
    D2D1_MATRIX_3X2_F m; parent.dc->GetTransform(&m); next->dc->SetTransform(m);
    next->opacity=(float)opacity; next->blend=blend;
    for(auto& clip:parent.clips) { next->clips.push_back(clip); Surface::applyClip(*next,clip); }
    s.frames.push_back(std::move(next));
} CATCH_VOID
// The input of a blur or a shadow reaches past the clips the parent had when the layer began:
// the pixels outside them are part of what the kernel reads. The layer drops the clips it
// inherited; d2_end_layer composes it into the parent under the parent's own clips.
API void d2_gpu_reset_effect_input_clip(Ptr p) try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    auto& s=surface(p); if(s.frames.size()<2) return;
    auto& f=s.frame(); suspendClips(f); f.clips.clear();
} CATCH_VOID
static D2D1_COMPOSITE_MODE composite(int blend) {
    switch(blend) {
    case 17:return D2D1_COMPOSITE_MODE_SOURCE_COPY;case 18:return D2D1_COMPOSITE_MODE_SOURCE_IN;
    case 19:return D2D1_COMPOSITE_MODE_SOURCE_OUT;case 20:return D2D1_COMPOSITE_MODE_SOURCE_ATOP;
    case 21:return D2D1_COMPOSITE_MODE_DESTINATION_OVER;case 22:return D2D1_COMPOSITE_MODE_DESTINATION_IN;
    case 23:return D2D1_COMPOSITE_MODE_DESTINATION_OUT;case 24:return D2D1_COMPOSITE_MODE_DESTINATION_ATOP;
    case 25:return D2D1_COMPOSITE_MODE_XOR;case 27:return D2D1_COMPOSITE_MODE_PLUS;
    default:return D2D1_COMPOSITE_MODE_SOURCE_OVER;
    }
}
API void d2_end_layer(Ptr p) try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    NativeTiming timing(4);
    auto& s=surface(p); if(s.frames.size()<2) throw std::runtime_error("Unbalanced Direct2D layer");
    auto child=std::move(s.frames.back()); s.frames.pop_back();
    suspendClips(*child); child->clips.clear(); child->end();
    child->dc->SetTarget(nullptr);
    struct Retire {Surface& surface;std::shared_ptr<Frame> frame;~Retire(){surface.retired.push_back(std::move(frame));}} retire{s,child};
    auto& f=s.frame(); if(child->blend==28) return;
    if(child->blend==0&&!child->hasColorMatrix&&child->opacity>=0.f&&child->opacity<=1.f
        &&f.dc->GetPrimitiveBlend()==D2D1_PRIMITIVE_BLEND_SOURCE_OVER) {
        // Apply opacity to the already composited premultiplied layer in
        // the bitmap draw. There is no separate color-matrix image to build.
        D2D1_MATRIX_3X2_F previous;
        f.dc->GetTransform(&previous);
        f.dc->SetTransform(Matrix3x2F::Identity());
        auto size=child->bitmap->GetPixelSize();
        f.dc->DrawBitmap(child->bitmap.Get(),RectF(0,0,(float)size.width,(float)size.height),
            child->opacity,D2D1_INTERPOLATION_MODE_LINEAR);
        f.dc->SetTransform(previous);
        return;
    }
    ComPtr<ID2D1Image> output=child->bitmap;
    if(child->hasColorMatrix) {
        auto matrix=effect(f,CLSID_D2D1ColorMatrix,output.Get());
        check(matrix->SetValue(D2D1_COLORMATRIX_PROP_COLOR_MATRIX,child->colorMatrix));
        check(matrix->SetValue(D2D1_COLORMATRIX_PROP_ALPHA_MODE,child->colorMatrixAlphaMode));
        check(matrix->SetValue(D2D1_COLORMATRIX_PROP_CLAMP_OUTPUT,TRUE));matrix->GetOutput(&output);
    }
    if(child->opacity!=1) {
        auto alpha=effect(f,CLSID_D2D1ColorMatrix,output.Get());
        auto matrix=Matrix5x4F(child->opacity,0,0,0, 0,child->opacity,0,0, 0,0,child->opacity,0, 0,0,0,child->opacity, 0,0,0,0);
        check(alpha->SetValue(D2D1_COLORMATRIX_PROP_COLOR_MATRIX,matrix));
        check(alpha->SetValue(D2D1_COLORMATRIX_PROP_ALPHA_MODE,D2D1_COLORMATRIX_ALPHA_MODE_STRAIGHT)); alpha->GetOutput(&output);
    }
    D2D1_MATRIX_3X2_F m; f.dc->GetTransform(&m);
    if(child->blend==17) {
        auto background=s.snapshot();
        Surface mask(s.width,s.height);auto& mf=mask.frame();
        for(auto& clip:f.clips){mf.clips.push_back(clip);Surface::applyClip(mf,clip);}
        ComPtr<ID2D1SolidColorBrush> white;check(mf.dc->CreateSolidColorBrush(ColorF(1,1,1,1),&white));
        mf.dc->SetTransform(Matrix3x2F::Identity());mf.dc->FillRectangle(RectF(0,0,(float)s.width,(float)s.height),white.Get());
        auto coverage=mask.snapshot();auto outside=effect(f,CLSID_D2D1Composite,background.Get());outside->SetInput(1,coverage.Get());
        check(outside->SetValue(D2D1_COMPOSITE_PROP_MODE,D2D1_COMPOSITE_MODE_DESTINATION_OUT));
        ComPtr<ID2D1Image> untouched;outside->GetOutput(&untouched);
        auto compose=effect(f,CLSID_D2D1Composite,untouched.Get());compose->SetInput(1,output.Get());
        check(compose->SetValue(D2D1_COMPOSITE_PROP_MODE,D2D1_COMPOSITE_MODE_PLUS));compose->GetOutput(&output);replaceImage(s,output.Get());
    } else if(child->blend>0 && child->blend<=15) {
        auto background=s.snapshot(); auto blend=effect(f,CLSID_D2D1Blend,background.Get()); blend->SetInput(1,output.Get());
        const D2D1_BLEND_MODE modes[]={D2D1_BLEND_MODE_MULTIPLY,D2D1_BLEND_MODE_MULTIPLY,D2D1_BLEND_MODE_SCREEN,D2D1_BLEND_MODE_OVERLAY,D2D1_BLEND_MODE_DARKEN,D2D1_BLEND_MODE_LIGHTEN,D2D1_BLEND_MODE_COLOR_DODGE,D2D1_BLEND_MODE_COLOR_BURN,D2D1_BLEND_MODE_SOFT_LIGHT,D2D1_BLEND_MODE_HARD_LIGHT,D2D1_BLEND_MODE_DIFFERENCE,D2D1_BLEND_MODE_EXCLUSION,D2D1_BLEND_MODE_HUE,D2D1_BLEND_MODE_SATURATION,D2D1_BLEND_MODE_COLOR,D2D1_BLEND_MODE_LUMINOSITY};
        check(blend->SetValue(D2D1_BLEND_PROP_MODE,modes[child->blend])); blend->GetOutput(&output);
        replaceImage(s,output.Get());
    } else {
        f.dc->SetTransform(Matrix3x2F::Identity()); f.dc->DrawImage(output.Get(),nullptr,nullptr,D2D1_INTERPOLATION_MODE_LINEAR,composite(child->blend)); f.dc->SetTransform(m);
    }
} CATCH_VOID
API void d2_gpu_render_options(Ptr p,int blend,int aliased,int interpolation) try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    auto& s=surface(p); s.interpolation=interpolation;
    s.frame().dc->SetPrimitiveBlend(blend==17?D2D1_PRIMITIVE_BLEND_COPY:blend==27?D2D1_PRIMITIVE_BLEND_ADD:D2D1_PRIMITIVE_BLEND_SOURCE_OVER);
    s.frame().dc->SetAntialiasMode(aliased?D2D1_ANTIALIAS_MODE_ALIASED:D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);
    s.frame().dc->SetTextAntialiasMode(aliased?D2D1_TEXT_ANTIALIAS_MODE_ALIASED:D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE);
} CATCH_VOID
API void d2_gpu_draw_surface(Ptr target,Ptr source,double sx,double sy,double sw,double sh,double dx,double dy,double dw,double dh,double alpha) try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    ++statistics[2];
    auto image=surface(source).snapshot(); auto& s=surface(target); auto& f=s.frame();
    auto src=RectF((float)sx,(float)sy,(float)(sx+sw),(float)(sy+sh));auto dst=RectF((float)dx,(float)dy,(float)(dx+dw),(float)(dy+dh));
    auto sampling=s.interpolation==1?D2D1_INTERPOLATION_MODE_NEAREST_NEIGHBOR:s.interpolation==3?D2D1_INTERPOLATION_MODE_MULTI_SAMPLE_LINEAR:s.interpolation>=4?D2D1_INTERPOLATION_MODE_HIGH_QUALITY_CUBIC:D2D1_INTERPOLATION_MODE_LINEAR;
    f.dc->DrawBitmap(image.Get(),dst,(float)alpha,sampling,&src);
} CATCH_VOID
API void d2_draw_image(Ptr target,Ptr source,double sx,double sy,double sw,double sh,double dx,double dy,double dw,double dh,double alpha) { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    d2_gpu_draw_surface(target,source,sx,sy,sw,sh,dx,dy,dw,dh,alpha);
}
static void drawGlyphs(Frame& f,const PositionedGlyphRun& positioned,double shift,double r,double g,double b,double a) {
    NativeTiming timing(18);if(nativeProfiling&&positioned.positionedAdvances)++nativeTimingCounts[19];
    if(positioned.ids.empty())return;
    auto brush=f.solid(color(r,g,b,a));auto run=positioned.run();
    auto origin=Point2F((float)positioned.x,(float)(positioned.y+shift));
    if(!positioned.color) {f.dc->DrawGlyphRun(origin,&run,brush,DWRITE_MEASURING_MODE_NATURAL);return;}
    ComPtr<IDWriteColorGlyphRunEnumerator1> colors;
    auto formats=(DWRITE_GLYPH_IMAGE_FORMATS)(DWRITE_GLYPH_IMAGE_FORMATS_TRUETYPE|DWRITE_GLYPH_IMAGE_FORMATS_CFF|DWRITE_GLYPH_IMAGE_FORMATS_COLR|DWRITE_GLYPH_IMAGE_FORMATS_SVG|DWRITE_GLYPH_IMAGE_FORMATS_PNG|DWRITE_GLYPH_IMAGE_FORMATS_JPEG|DWRITE_GLYPH_IMAGE_FORMATS_TIFF|DWRITE_GLYPH_IMAGE_FORMATS_PREMULTIPLIED_B8G8R8A8);
    HRESULT hr=f.dc4?engine().fonts->TranslateColorGlyphRun(origin,&run,nullptr,formats,DWRITE_MEASURING_MODE_NATURAL,nullptr,0,&colors):DWRITE_E_NOCOLOR;
    if(hr==DWRITE_E_NOCOLOR) {f.dc->DrawGlyphRun(origin,&run,brush,DWRITE_MEASURING_MODE_NATURAL);return;}
    check(hr);BOOL more=FALSE;
    while(SUCCEEDED(colors->MoveNext(&more))&&more) {
        const DWRITE_COLOR_GLYPH_RUN1* layer=nullptr;check(colors->GetCurrentRun(&layer));
        auto at=Point2F(layer->baselineOriginX,layer->baselineOriginY);
        if(layer->glyphImageFormat==DWRITE_GLYPH_IMAGE_FORMATS_SVG)f.dc4->DrawSvgGlyphRun(at,&layer->glyphRun,brush,nullptr,0,DWRITE_MEASURING_MODE_NATURAL);
        else if(layer->glyphImageFormat&(DWRITE_GLYPH_IMAGE_FORMATS_PNG|DWRITE_GLYPH_IMAGE_FORMATS_JPEG|DWRITE_GLYPH_IMAGE_FORMATS_TIFF|DWRITE_GLYPH_IMAGE_FORMATS_PREMULTIPLIED_B8G8R8A8))f.dc4->DrawColorBitmapGlyphRun(layer->glyphImageFormat,at,&layer->glyphRun,DWRITE_MEASURING_MODE_NATURAL);
        else {
            ComPtr<ID2D1SolidColorBrush> paint=brush;
            if(layer->paletteIndex!=0xffff)check(f.dc->CreateSolidColorBrush(color(layer->runColor.r,layer->runColor.g,layer->runColor.b,layer->runColor.a*a),&paint));
            f.dc->DrawGlyphRun(at,&layer->glyphRun,paint.Get(),DWRITE_MEASURING_MODE_NATURAL);
        }
    }
}
static std::shared_ptr<GlyphAtlasPage> rasterGlyph(Engine& e,Frame& frame,const PositionedGlyphRun& glyphs,
    const D2D1_MATRIX_3X2_F& matrix,const D2D1_MATRIX_3X2_F& current,double shift,
    D2D1_COLOR_F paint,D2D1_RECT_F& source,D2D1_RECT_F& destination) {
    auto& atlas=e.glyphAtlas;
    if(!atlas.recording()||glyphs.color||glyphs.ids.empty()||glyphs.ids.size()>128||paint.a!=1.f||
        matrix._11<=1.f||matrix._11>4.f||matrix._11!=matrix._22||matrix._12!=0||matrix._21!=0||
        !std::isfinite(matrix._31)||!std::isfinite(matrix._32)||fabs(matrix._31)>16384||fabs(matrix._32)>16384)return {};
    float x=(float)(int)matrix._31,y=(float)(int)matrix._32;
    auto normalized=matrix;normalized._31-=x;normalized._32-=y;
    float baseline=(float)(glyphs.y+shift);
    auto& entries=glyphs.rasters;
    auto found=std::find_if(entries.begin(),entries.end(),[&](const auto& entry){
        return baseline==entry.baseline&&!memcmp(&normalized,&entry.matrix,sizeof(normalized))&&
            !memcmp(&paint,&entry.paint,sizeof(paint));
    });
    if(found==entries.end()) {
        PositionedGlyphRun::RasterEntry entry{normalized,baseline,paint};
        entry.lastFrame=atlas.frameIndex();
        if(entries.size()<PositionedGlyphRun::rasterLimit)entries.push_back(entry);
        else entries[glyphs.nextRaster++%entries.size()]=entry;
        return {};
    }
    if(auto page=found->page.lock()) {
        if(!page->ready)return {};
        atlas.touch(*page);
        source=found->source;
        auto& bounds=found->destination;
        destination=RectF(bounds.left+x,bounds.top+y,bounds.right+x,bounds.bottom+y);
        if(nativeProfiling)++glyphAtlasHits;
        return page;
    }
    // Build only repeatedly observed raster variants. The pending page is
    // sampled only after the separate atlas context has submitted its draws.
    if(found->lastFrame==atlas.frameIndex())return {};
    found->lastFrame=atlas.frameIndex();
    D2D1_RECT_F ink{};
    if(!glyphs.bounds(frame,matrix,current,shift,0,ink))return {};
    if(ink.right<=ink.left||ink.bottom<=ink.top||ink.right-ink.left>256||ink.bottom-ink.top>64)return {};
    destination=RectF(ink.left-1,ink.top-1,ink.right+1,ink.bottom+1);
    try {
        if(atlas.record(e.device.Get(),glyphs.run(),Point2F((float)glyphs.x,baseline),matrix,destination,
            paint,found->page,found->source)) {
            found->destination=RectF(destination.left-x,destination.top-y,destination.right-x,destination.bottom-y);
            if(nativeProfiling)++glyphAtlasBakes;
        }
    } catch(const std::exception&) {
        // An optional texture allocation must leave the direct glyph draw usable.
        atlas.disable();
    }
    return {};
}
API Ptr d2_positioned_glyph_run(Ptr font,double size,const uint16_t* ids,const double* positions,size_t n,const double* advances) try {std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    auto& f=*static_cast<Font*>(font);
    if(!n||n>256)return new PositionedGlyphRun(f,size,ids,positions,n,advances);
    // Every input is part of the key, including the distinction between natural
    // and explicit advances. Never infer identity from text or glyph IDs alone.
    std::string key;key.reserve(sizeof(size)+sizeof(n)+1+n*(sizeof(uint16_t)+3*sizeof(double)));
    auto add=[&](const auto& value){key.append(reinterpret_cast<const char*>(&value),sizeof(value));};
    add(size);add(n);bool natural=advances!=nullptr;add(natural);
    key.append(reinterpret_cast<const char*>(ids),n*sizeof(uint16_t));
    key.append(reinterpret_cast<const char*>(positions),n*2*sizeof(double));
    if(natural)key.append(reinterpret_cast<const char*>(advances),n*sizeof(double));
    auto found=f.positionedIndex.find(key);
    if(found!=f.positionedIndex.end()) {
        auto entry=found->second;f.positionedRuns.splice(f.positionedRuns.end(),f.positionedRuns,entry);
        ++entry->run->references;return entry->run;
    }
    size_t fingerprint=std::hash<std::string>{}(key),slot=fingerprint%f.positionedProbation.size();
    fingerprint|=1;
    auto& probes=f.positionedProbation[slot];
    if(std::find(probes.begin(),probes.end(),fingerprint)==probes.end()) {
        std::move_backward(probes.begin(),probes.end()-1,probes.end());probes.front()=fingerprint;
        return new PositionedGlyphRun(f,size,ids,positions,n,advances);
    }
    auto result=std::make_unique<PositionedGlyphRun>(f,size,ids,positions,n,advances);
    // Charge the full possible 32-entry ink cache up front, plus both key copies,
    // glyph storage and conservative list/map overhead. Long runs bypass this LRU.
    size_t bytes=sizeof(PositionedGlyphRun)+sizeof(Font::PositionedEntry)+128+key.capacity()+key.size()
        +result->ids.capacity()*sizeof(uint16_t)+result->advances.capacity()*sizeof(float)
        +result->offsets.capacity()*sizeof(DWRITE_GLYPH_OFFSET)+32*sizeof(PositionedGlyphRun::BoundsEntry)
        +PositionedGlyphRun::rasterLimit*sizeof(PositionedGlyphRun::RasterEntry);
    constexpr size_t budget=2*1024*1024-sizeof(f.positionedProbation);
    if(bytes>budget)return result.release();
    while(!f.positionedRuns.empty()&&(f.positionedBytes+bytes>budget||f.positionedRuns.size()>=1024)) {
        auto& entry=f.positionedRuns.front();f.positionedBytes-=entry.bytes;
        f.positionedIndex.erase(entry.key);Font::releasePositioned(entry.run);f.positionedRuns.pop_front();
    }
    f.positionedRuns.push_back({std::move(key),result.get(),bytes});auto entry=std::prev(f.positionedRuns.end());
    try {f.positionedIndex.emplace(entry->key,entry);}catch(...){f.positionedRuns.pop_back();throw;}
    ++result->references;f.positionedBytes+=bytes;return result.release();
} CATCH_PTR
API void d2_draw_positioned_glyph_run(Ptr p,Ptr glyphs,double shift,int aliased,double r,double g,double b,double a) try {std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    NativeTiming timing(7);auto& f=surface(p).frame();auto old=f.dc->GetTextAntialiasMode();
    auto mode=aliased?D2D1_TEXT_ANTIALIAS_MODE_ALIASED:D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE;
    if(old!=mode)f.dc->SetTextAntialiasMode(mode);
    drawGlyphs(f,*static_cast<PositionedGlyphRun*>(glyphs),shift,r,g,b,a);
    if(old!=mode)f.dc->SetTextAntialiasMode(old);
} CATCH_VOID
API void d2_glyphs(Ptr p,Ptr font,double size,const uint16_t* ids,const double* positions,size_t n,double r,double g,double b,double a) try {std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    NativeTiming timing(7);PositionedGlyphRun run(*static_cast<Font*>(font),size,ids,positions,n);drawGlyphs(surface(p).frame(),run,0,r,g,b,a);
} CATCH_VOID
static bool fillWithinClip(Path& path,const D2D1_MATRIX_3X2_F& matrix,const D2D1_RECT_F& clip) {
    if(matrix._12!=0||matrix._21!=0||matrix._11<=0||matrix._22<=0)return false;
    if(!path.hasLocalBounds) {
        if(FAILED(path.geometry->GetBounds(nullptr,&path.localBounds)))return false;
        path.hasLocalBounds=true;
    }
    const auto& local=path.localBounds;
    const double left=(double)local.left*matrix._11,top=(double)local.top*matrix._22;
    const double right=(double)local.right*matrix._11,bottom=(double)local.bottom*matrix._22;
    constexpr double limit=1<<20;
    if(fabs(matrix._31)>limit||fabs(matrix._32)>limit||fabs(left)>limit||fabs(top)>limit||
        fabs(right)>limit||fabs(bottom)>limit)return false;
    // Keep a full device pixel for coverage and float transform rounding.
    return left+matrix._31>=clip.left+1.&&top+matrix._32>=clip.top+1.&&
        right+matrix._31<=clip.right-1.&&bottom+matrix._32<=clip.bottom-1.;
}
API void d2_draw_batch(Ptr p,const DrawCommand* commands,size_t count) try {std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    auto& e=engine();auto& s=surface(p);auto& f=s.frame();auto dc=f.dc.Get();
    D2D1_MATRIX_3X2_F matrix;dc->GetTransform(&matrix);
    auto actualMatrix=matrix;
    // Deferred clips and combined glyphs consume logical transforms directly.
    // Publish a transform only for an actual draw or when leaving this batch.
    auto setActual=[&](const D2D1_MATRIX_3X2_F& value){if(memcmp(&actualMatrix,&value,sizeof(value))){dc->SetTransform(value);actualMatrix=value;}};
    auto primitive=dc->GetPrimitiveBlend();auto aa=dc->GetAntialiasMode();auto textAA=dc->GetTextAntialiasMode();
    auto& group=f.glyphBatch;group.clear();
    struct ClearBatch {GlyphBatch& group;~ClearBatch(){group.clear();}} clear{group};
    auto flush=[&](){
        if(group.ids.empty())return;
        NativeTiming timing(18);
        auto old=dc->GetTextAntialiasMode();
        setActual(Matrix3x2F::Identity());dc->SetTextAntialiasMode(group.mode);
        DWRITE_GLYPH_RUN run{group.face.Get(),group.size,(UINT32)group.ids.size(),group.ids.data(),group.advances.data(),group.offsets.data(),FALSE,0};
        dc->DrawGlyphRun(Point2F(0,0),&run,f.solid(group.color),DWRITE_MEASURING_MODE_NATURAL);
        dc->SetTextAntialiasMode(old);group.clear();
    };
    auto setMatrix=[&](const D2D1_MATRIX_3X2_F& value){matrix=value;};
    bool clipBoundsDirty=true,hasPendingClip=false;
    auto clipInterior=InfiniteRect();
    auto pendingClips=[&]() {
        if(clipBoundsDirty) {
            hasPendingClip=false;
            clipInterior=InfiniteRect();
            for(auto& clip:f.clips)if(!clip.applied&&!clip.redundant) {
                hasPendingClip=true;
                const auto& b=clip.interior;
                clipInterior=RectF(std::max(clipInterior.left,b.left),std::max(clipInterior.top,b.top),
                    std::min(clipInterior.right,b.right),std::min(clipInterior.bottom,b.bottom));
            }
            clipBoundsDirty=false;
        }
        return hasPendingClip;
    };
    auto applyPendingClips=[&]() {
        materializeClips(f);
        hasPendingClip=clipBoundsDirty=false;
    };
    for(size_t i=0;i<count;++i) {
        auto& c=commands[i];
        if(c.kind==1){
            if(s.saves.empty())throw std::runtime_error("Unbalanced native batch restore");
            auto saved=s.saves.back();s.saves.pop_back();
            while(f.clips.size()>saved.clips){if(f.clips.back().applied)flush();popClip(f,f.clips.back());f.clips.pop_back();}
            clipBoundsDirty=true;
            setMatrix(saved.transform);s.spread=saved.spread;s.aspect=saved.aspect;s.angle=saved.angle;s.interpolation=saved.interpolation;continue;
        }
        setMatrix(Matrix3x2F((float)c.matrix[0],(float)c.matrix[1],(float)c.matrix[2],(float)c.matrix[3],(float)c.matrix[4],(float)c.matrix[5]));
        auto blend=c.blend==17?D2D1_PRIMITIVE_BLEND_COPY:c.blend==27?D2D1_PRIMITIVE_BLEND_ADD:D2D1_PRIMITIVE_BLEND_SOURCE_OVER;
        if(blend!=primitive){flush();dc->SetPrimitiveBlend(blend);primitive=blend;}
        auto edge=c.aliased?D2D1_ANTIALIAS_MODE_ALIASED:D2D1_ANTIALIAS_MODE_PER_PRIMITIVE;
        if(edge!=aa){dc->SetAntialiasMode(edge);aa=edge;}
        auto text=c.aliased?D2D1_TEXT_ANTIALIAS_MODE_ALIASED:D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE;
        if(text!=textAA){dc->SetTextAntialiasMode(text);textAA=text;}
        s.interpolation=c.interpolation;
        if(c.kind==0){if(matrix._12!=0||matrix._21!=0){flush();setActual(matrix);}s.saves.push_back({matrix,f.clips.size(),s.spread,s.aspect,s.angle,s.interpolation});pushRectClip(f,matrix,c.params[0],c.params[1],c.params[2],c.params[3],true);clipBoundsDirty=true;}
        else if(c.kind==2){
            NativeTiming timing(7);auto mode=c.textAliased?D2D1_TEXT_ANTIALIAS_MODE_ALIASED:D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE;
            if(mode!=textAA)dc->SetTextAntialiasMode(mode);
            auto& glyphs=*static_cast<PositionedGlyphRun*>(c.resource);
            auto paint=color(c.params[0],c.params[1],c.params[2],c.params[3]);
            D2D1_RECT_F source{},destination{};
            auto raster=primitive==D2D1_PRIMITIVE_BLEND_SOURCE_OVER&&mode==D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE?
                rasterGlyph(e,f,glyphs,matrix,actualMatrix,c.shift,paint,source,destination):nullptr;
            D2D1_RECT_F ink{};bool hasInk=false;
            if(pendingClips()){
                if(raster) {
                    ink=RectF(destination.left+1,destination.top+1,destination.right-1,destination.bottom-1);
                    hasInk=true;
                } else hasInk=glyphs.bounds(f,matrix,actualMatrix,c.shift,c.textAliased,ink);
                bool inside=hasInk&&ink.left>=clipInterior.left&&ink.top>=clipInterior.top&&ink.right<=clipInterior.right&&ink.bottom<=clipInterior.bottom;
                if(!inside){flush();applyPendingClips();}
            }
            // Combine only monochrome, non-overlapping runs whose positions
            // remain exact when integer translations are folded into offsets.
            // Fractional DPI, affine transforms, overlapping ink and color text
            // retain the original draw path. Applied clip changes flush first.
            bool combine=primitive==D2D1_PRIMITIVE_BLEND_SOURCE_OVER&&!glyphs.color&&glyphs.positionedAdvances&&glyphs.ids.size()<=8192&&matrix._11==1&&matrix._22==1&&matrix._12==0&&matrix._21==0
                &&fabs(matrix._31)<16384&&fabs(matrix._32)<16384&&matrix._31==(int)matrix._31&&matrix._32==(int)matrix._32;
            double pen=0;
            for(size_t j=0;j<glyphs.ids.size()&&combine;++j){
                double x=(float)glyphs.x+pen+glyphs.offsets[j].advanceOffset;
                if((double)(float)x!=x)combine=false;
                pen+=glyphs.advances[j];
            }
            if(combine&&!hasInk)hasInk=glyphs.bounds(f,matrix,actualMatrix,c.shift,c.textAliased,ink);
            combine=combine&&hasInk;
            if(raster) {
                flush();
                setActual(Matrix3x2F::Identity());
                dc->DrawBitmap(raster->bitmap.Get(),destination,1.f,D2D1_INTERPOLATION_MODE_NEAREST_NEIGHBOR,source);
            } else if(combine){
                if(nativeProfiling)++nativeTimingCounts[19];
                if(group.face.Get()!=glyphs.face.Get()||group.size!=glyphs.size||memcmp(&paint,&group.color,sizeof(paint))||group.mode!=mode)flush();
                bool outside=ink.top>=group.inkBottom||ink.bottom<=group.inkTop;
                bool rowGap=group.disjointRow&&ink.top==group.rowInk.top&&ink.bottom==group.rowInk.bottom
                    &&(ink.left>=group.rowInk.right||ink.right<=group.rowInk.left);
                // Row bounds only prove non-overlap. Draws inside a row's gap
                // still check the original rectangles, avoiding extra flushes.
                if(!outside&&!rowGap)for(auto& previous:group.ink)if(ink.left<previous.right&&ink.right>previous.left&&ink.top<previous.bottom&&ink.bottom>previous.top){flush();break;}
                if(group.ids.size()+glyphs.ids.size()>8192)flush();
                group.face=glyphs.face;group.size=glyphs.size;group.color=paint;group.mode=mode;
                pen=0;
                for(size_t j=0;j<glyphs.ids.size();++j){
                    group.ids.push_back(glyphs.ids[j]);group.advances.push_back(0);
                    float x=(float)((float)glyphs.x+pen+glyphs.offsets[j].advanceOffset)+matrix._31;
                    float y=(float)(glyphs.y+c.shift)-glyphs.offsets[j].ascenderOffset+matrix._32;
                    group.offsets.push_back({x,-y});pen+=glyphs.advances[j];
                }
                if(group.ink.empty()||group.rowInk.top!=ink.top||group.rowInk.bottom!=ink.bottom) {
                    group.rowInk=ink;group.disjointRow=group.ink.empty()||ink.top>=group.inkBottom||ink.bottom<=group.inkTop;
                } else {group.rowInk.left=std::min(group.rowInk.left,ink.left);group.rowInk.right=std::max(group.rowInk.right,ink.right);}
                group.inkTop=std::min(group.inkTop,ink.top);group.inkBottom=std::max(group.inkBottom,ink.bottom);
                group.ink.push_back(ink);
            } else {flush();setActual(matrix);drawGlyphs(f,glyphs,c.shift,c.params[0],c.params[1],c.params[2],c.params[3]);}
            if(mode!=textAA)dc->SetTextAntialiasMode(textAA);
        }
        else if(c.kind==3) {
            flush();
            auto& path=*static_cast<Path*>(c.resource);
            if(pendingClips()&&!fillWithinClip(path,matrix,clipInterior))applyPendingClips();
            setActual(matrix);
            auto paint=f.solid(color(c.params[0],c.params[1],c.params[2],c.params[3]));
            if(c.fillEvenOdd)dc->FillGeometry(rule(&path,1).Get(),paint);
            else dc->FillGeometry(path.geometry.Get(),paint);
            ++statistics[1];
        }
        else if(c.kind==4) {
            flush();
            applyPendingClips();
            setActual(matrix);
            s.saves.push_back({matrix,f.clips.size(),s.spread,s.aspect,s.angle,s.interpolation});
            auto& path=*static_cast<Path*>(c.resource);
            Clip clip{path.geometry,matrix};
            f.clips.push_back(clip);
            Surface::applyClip(f,clip);
        }
        else throw std::runtime_error("Invalid native drawing command");
    }
    // A snapshot, effect, non-batched draw or automatic capacity flush may
    // follow. Expose the exact logical clip stack to all existing APIs.
    flush();materializeClips(f);setActual(matrix);
} CATCH_VOID
// Readback is confined to explicit bitmap lock/export operations. Frame drawing,
// snapshots, filters, and presentation use only GPU resources.
API int d2_read_pixels(Ptr p,uint8_t* out,size_t length) try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    ++statistics[4];
    auto& s=surface(p); if(!out||length<(size_t)s.width*s.height*4) return 0;
    auto image=s.snapshot(); auto& f=*s.frames.back(); ComPtr<ID2D1Bitmap1> staging;
    auto properties=BitmapProperties1((D2D1_BITMAP_OPTIONS)(D2D1_BITMAP_OPTIONS_CPU_READ|D2D1_BITMAP_OPTIONS_CANNOT_DRAW),PixelFormat(DXGI_FORMAT_B8G8R8A8_UNORM,D2D1_ALPHA_MODE_PREMULTIPLIED));
    check(f.dc->CreateBitmap(SizeU(s.width,s.height),nullptr,0,&properties,&staging)); check(staging->CopyFromBitmap(nullptr,image.Get(),nullptr));
    D2D1_MAPPED_RECT mapped{}; check(staging->Map(D2D1_MAP_OPTIONS_READ,&mapped));
    for(UINT32 y=0;y<s.height;++y) memcpy(out+(size_t)y*s.width*4,mapped.bits+(size_t)y*mapped.pitch,(size_t)s.width*4);
    staging->Unmap(); return 1;
} catch(const std::exception& e) { ++statistics[8]; fprintf(stderr,"Direct2D: %s\n",e.what()); return 0; }
API void d2_write_pixels(Ptr p,const uint8_t* data,size_t n) try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    ++statistics[5];
    auto& s=surface(p); if(n!=(size_t)s.width*s.height*4) throw std::runtime_error("Invalid bitmap upload size");
    auto& f=s.frame(); ComPtr<ID2D1Bitmap1> image;
    auto properties=BitmapProperties1(D2D1_BITMAP_OPTIONS_NONE,PixelFormat(DXGI_FORMAT_B8G8R8A8_UNORM,D2D1_ALPHA_MODE_PREMULTIPLIED));
    check(f.dc->CreateBitmap(SizeU(s.width,s.height),data,s.width*4,&properties,&image)); replaceImage(s,image.Get());
} CATCH_VOID
API Ptr d2_decode(const uint8_t* data,size_t n) try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    ++statistics[5];
    if(n>UINT32_MAX) return nullptr;
    ComPtr<IWICStream> stream; check(engine().wic->CreateStream(&stream)); check(stream->InitializeFromMemory(const_cast<BYTE*>(data),(DWORD)n));
    ComPtr<IWICBitmapDecoder> decoder; check(engine().wic->CreateDecoderFromStream(stream.Get(),nullptr,WICDecodeMetadataCacheOnLoad,&decoder));
    ComPtr<IWICBitmapFrameDecode> frame; check(decoder->GetFrame(0,&frame)); UINT w=0,h=0; check(frame->GetSize(&w,&h));
    if(!w||!h||w>16384||h>16384) return nullptr;
    ComPtr<IWICFormatConverter> converter; check(engine().wic->CreateFormatConverter(&converter));
    check(converter->Initialize(frame.Get(),GUID_WICPixelFormat32bppPBGRA,WICBitmapDitherTypeNone,nullptr,0,WICBitmapPaletteTypeCustom));
    auto result=std::make_unique<Surface>(w,h); ComPtr<ID2D1Bitmap1> image;
    check(result->frame().dc->CreateBitmapFromWicBitmap(converter.Get(),nullptr,&image)); replaceImage(*result,image.Get()); return result.release();
} CATCH_PTR
API size_t d2_image_width(Ptr p) { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex); return p?static_cast<Surface*>(p)->width:0; }
API size_t d2_image_height(Ptr p) { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex); return p?static_cast<Surface*>(p)->height:0; }
API Ptr d2_encode(Ptr p,int jpeg,double quality) try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    auto& s=surface(p); std::vector<uint8_t> bytes((size_t)s.width*s.height*4);
    if(!d2_read_pixels(p,bytes.data(),bytes.size())) throw std::runtime_error("The surface could not be read for encoding");
    ComPtr<IStream> stream; check(CreateStreamOnHGlobal(nullptr,TRUE,&stream));
    ComPtr<IWICBitmapEncoder> encoder; check(engine().wic->CreateEncoder(jpeg?GUID_ContainerFormatJpeg:GUID_ContainerFormatPng,nullptr,&encoder));
    check(encoder->Initialize(stream.Get(),WICBitmapEncoderNoCache));
    ComPtr<IWICBitmapFrameEncode> frame; ComPtr<IPropertyBag2> properties; check(encoder->CreateNewFrame(&frame,&properties));
    if(jpeg) { PROPBAG2 option{}; option.pstrName=const_cast<wchar_t*>(L"ImageQuality"); VARIANT value{}; value.vt=VT_R4; value.fltVal=(float)quality; check(properties->Write(1,&option,&value)); }
    check(frame->Initialize(properties.Get())); check(frame->SetSize(s.width,s.height));
    ComPtr<IWICBitmap> bitmap; check(engine().wic->CreateBitmapFromMemory(s.width,s.height,GUID_WICPixelFormat32bppPBGRA,s.width*4,(UINT)bytes.size(),bytes.data(),&bitmap));
    check(frame->WriteSource(bitmap.Get(),nullptr)); check(frame->Commit()); check(encoder->Commit());
    STATSTG stat{}; check(stream->Stat(&stat,STATFLAG_NONAME)); if(stat.cbSize.QuadPart>SIZE_MAX) return nullptr;
    auto result=std::make_unique<Data>(); result->bytes.resize((size_t)stat.cbSize.QuadPart);
    LARGE_INTEGER zero{}; check(stream->Seek(zero,STREAM_SEEK_SET,nullptr)); ULONG read=0;
    check(stream->Read(result->bytes.data(),(ULONG)result->bytes.size(),&read)); result->bytes.resize(read); return result.release();
} CATCH_PTR
struct Window : Object {
    uint64_t generation;
    ComPtr<IDCompositionDevice> compositor;
    ComPtr<IDCompositionTarget> target;
    ComPtr<IDCompositionVisual> visual;
    HWND hwnd; ComPtr<IDXGISwapChain1> chain; UINT32 width=0,height=0; bool lost=false,occluded=false;
    bool profile=GetEnvironmentVariableW(L"FERROUI_D3D_PROFILE",nullptr,0)!=0,frameStarted=false;
    std::chrono::steady_clock::time_point frameStart,lastPresent;
    uint64_t profileFrames=0,profilePixels=0,profileCopies=0,startPixels=0,startCopies=0,cadenceFrames=0;
    double workMilliseconds=0,presentMilliseconds=0,maxWorkMilliseconds=0,cadenceMilliseconds=0;
    uint64_t frameTimings[20]{};
    explicit Window(HWND window):generation(engine().generation),hwnd(window) {}
    void printProfile() {
        if(!profileFrames)return;
        fprintf(stderr,"Direct2D frame profile hwnd=%p frames=%llu work_ms=%.3f max_work_ms=%.3f present_wait_ms=%.3f active_cadence_ms=%.3f kernel_pixels_per_frame=%llu gpu_copies_per_frame=%.1f glyph_atlas_hits=%llu glyph_atlas_bakes=%llu\n",hwnd,
            profileFrames,workMilliseconds/profileFrames,maxWorkMilliseconds,presentMilliseconds/profileFrames,
            cadenceFrames?cadenceMilliseconds/cadenceFrames:0.,profilePixels/profileFrames,(double)profileCopies/profileFrames,glyphAtlasHits,glyphAtlasBakes);
        profileFrames=profilePixels=profileCopies=cadenceFrames=0;workMilliseconds=presentMilliseconds=maxWorkMilliseconds=cadenceMilliseconds=0;
    }
    void recordFrame(std::chrono::steady_clock::time_point submitted) {
        if(!profile||!frameStarted)return;
        auto now=std::chrono::steady_clock::now();
        double work=std::chrono::duration<double,std::milli>(submitted-frameStart).count();
        if(work>16.) {
            fprintf(stderr,"Direct2D slow submission work_ms=%.3f end_draw_ms=%.3f make_frame_ms=%.3f snapshot_ms=%.3f kernel_ms=%.3f end_layer_ms=%.3f path_rule_ms=%.3f clip_ms=%.3f glyph_ms=%.3f path_create_ms=%.3f path_export_ms=%.3f glyph_bounds_ms=%.3f glyph_core_ms=%.3f\n",work,
                (nativeTimings[0]-frameTimings[0])/1e6,(nativeTimings[1]-frameTimings[1])/1e6,
                (nativeTimings[2]-frameTimings[2])/1e6,(nativeTimings[3]-frameTimings[3])/1e6,
                (nativeTimings[4]-frameTimings[4])/1e6,(nativeTimings[5]-frameTimings[5])/1e6,
                (nativeTimings[6]-frameTimings[6])/1e6,(nativeTimings[7]-frameTimings[7])/1e6,
                (nativeTimings[12]-frameTimings[12])/1e6,(nativeTimings[13]-frameTimings[13])/1e6,
                (nativeTimings[17]-frameTimings[17])/1e6,(nativeTimings[18]-frameTimings[18])/1e6);
        }
        workMilliseconds+=work;maxWorkMilliseconds=std::max(maxWorkMilliseconds,work);
        presentMilliseconds+=std::chrono::duration<double,std::milli>(now-submitted).count();
        if(lastPresent.time_since_epoch().count()) {
            double cadence=std::chrono::duration<double,std::milli>(now-lastPresent).count();
            if(cadence<500){cadenceMilliseconds+=cadence;++cadenceFrames;}
        }
        lastPresent=now;profilePixels+=statistics[11].load()-startPixels;profileCopies+=statistics[7].load()-startCopies;
        ++profileFrames;frameStarted=false;if(profileFrames>=60)printProfile();
    }
    void close() {
        if(profile)printProfile();
        if(target)target->SetRoot(nullptr);
        if(visual)visual->SetContent(nullptr);
        if(compositor&&SUCCEEDED(compositor->Commit()))compositor->WaitForCommitCompletion();
        visual.Reset();target.Reset();chain.Reset();compositor.Reset();lost=true;
    }
    ~Window(){close();}
};
API Ptr d2_window(intptr_t hwnd) try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex); engine(); if(!IsWindow((HWND)hwnd)) return nullptr; return new Window((HWND)hwnd); } CATCH_PTR
API void d2_window_frame_begin(Ptr p) try {
    std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    if(!p)return;
    auto& w=*static_cast<Window*>(p);
    auto& e=engine();
    if(!w.lost&&w.generation==e.generation)e.glyphAtlas.begin();
    if(!w.profile)return;
    w.frameStart=std::chrono::steady_clock::now();w.frameStarted=true;w.startPixels=statistics[11];w.startCopies=statistics[7];
    for(unsigned i=0;i<20;++i)w.frameTimings[i]=nativeTimings[i];
} CATCH_VOID
API void d2_window_dispose(Ptr p) {std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);if(p)static_cast<Window*>(p)->close();}
API int d2_window_state(Ptr p) try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    if(!p)return -1;
    auto& w=*static_cast<Window*>(p); if(w.lost||w.generation!=engine().generation||FAILED(engine().d3d->GetDeviceRemovedReason())) {w.close();return -1;}
    return IsWindow(w.hwnd)&&!IsIconic(w.hwnd)?1:0;
} catch(...) {return -1;}
API int d2_present(Ptr window,Ptr source) try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    if(!window)return -1;
    auto& w=*static_cast<Window*>(window); auto& s=surface(source); auto& e=engine();
    e.glyphAtlas.finish();
    if(w.lost||w.generation!=e.generation)return -1;
    if(!IsWindow(w.hwnd)||IsIconic(w.hwnd)) return 0;
    if(!w.chain) {
        DXGI_SWAP_CHAIN_DESC1 desc{};desc.Width=s.width;desc.Height=s.height;desc.Format=DXGI_FORMAT_B8G8R8A8_UNORM;
        desc.SampleDesc.Count=1;desc.BufferUsage=DXGI_USAGE_RENDER_TARGET_OUTPUT;desc.BufferCount=2;desc.SwapEffect=DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL;
        desc.AlphaMode=DXGI_ALPHA_MODE_PREMULTIPLIED;desc.Scaling=DXGI_SCALING_STRETCH;
        check(e.dxgiFactory->CreateSwapChainForComposition(e.d3d.Get(),&desc,nullptr,&w.chain));
        ComPtr<IDXGIDevice> dxgi;check(e.d3d.As(&dxgi));
        check(DCompositionCreateDevice(dxgi.Get(),IID_PPV_ARGS(&w.compositor)));
        check(w.compositor->CreateTargetForHwnd(w.hwnd,TRUE,&w.target));
        check(w.compositor->CreateVisual(&w.visual));check(w.visual->SetContent(w.chain.Get()));
        check(w.target->SetRoot(w.visual.Get()));check(w.compositor->Commit());
        check(e.dxgiFactory->MakeWindowAssociation(w.hwnd,DXGI_MWA_NO_ALT_ENTER));w.width=s.width;w.height=s.height;
    } else if(w.width!=s.width||w.height!=s.height) {
        check(w.chain->ResizeBuffers(0,s.width,s.height,DXGI_FORMAT_UNKNOWN,0));w.width=s.width;w.height=s.height;
    }
    if(w.occluded) { HRESULT hr=w.chain->Present(0,DXGI_PRESENT_TEST); if(hr==DXGI_STATUS_OCCLUDED) return 0;check(hr);w.occluded=false; }
    if(s.generation!=e.generation||s.lost)return -1;
    auto& frame=*s.frames.back();
    if(s.frames.size()==1&&!frame.drawing) {
        // The root EndDraw has submitted all drawing. The retained root and
        // the swap-chain buffer have identical premultiplied BGRA8 storage.
        // Copy directly while preserving the root for the next dirty redraw.
        ComPtr<IDXGISurface> surface;
        check(frame.bitmap->GetSurface(&surface));
        ComPtr<ID3D11Texture2D> root,buffer;
        check(surface.As(&root));
        check(w.chain->GetBuffer(0,IID_PPV_ARGS(&buffer)));
        ComPtr<ID2D1Multithread> interop;
        check(e.factory.As(&interop));
        interop->Enter();
        e.immediate->CopyResource(buffer.Get(),root.Get());
        interop->Leave();
        ++statistics[7];
    } else {
        auto image=s.snapshot();
        ComPtr<IDXGISurface> buffer; check(w.chain->GetBuffer(0,IID_PPV_ARGS(&buffer)));
        ComPtr<ID2D1DeviceContext> dc;check(e.device->CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE,&dc));
        auto properties=BitmapProperties1((D2D1_BITMAP_OPTIONS)(D2D1_BITMAP_OPTIONS_TARGET|D2D1_BITMAP_OPTIONS_CANNOT_DRAW),PixelFormat(DXGI_FORMAT_B8G8R8A8_UNORM,D2D1_ALPHA_MODE_PREMULTIPLIED));
        ComPtr<ID2D1Bitmap1> target; check(dc->CreateBitmapFromDxgiSurface(buffer.Get(),&properties,&target));
        dc->SetUnitMode(D2D1_UNIT_MODE_PIXELS);dc->SetTarget(target.Get());dc->BeginDraw();dc->DrawImage(image.Get(),nullptr,nullptr,D2D1_INTERPOLATION_MODE_NEAREST_NEIGHBOR,D2D1_COMPOSITE_MODE_SOURCE_COPY);check(dc->EndDraw());
        dc->SetTarget(nullptr);
    }
    auto submitted=std::chrono::steady_clock::now();
    // The compositor schedules frames already. Let DWM compose the latest
    // complete image rather than waiting here for another vblank and retaining
    // stale queued frames. This composition swap chain does not enable tearing.
    HRESULT hr=w.chain->Present(0,0); w.occluded=hr==DXGI_STATUS_OCCLUDED;check(hr);++statistics[3];w.recordFrame(submitted);return 1;
} catch(const std::exception& e) { static_cast<Window*>(window)->lost=true;fprintf(stderr,"Direct2D present: %s\n",e.what());return -1; }

API Ptr d2_gpu_layer_snapshot(Ptr p) { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex); return d2_snapshot(p); }
API Ptr d2_gpu_take_layer(Ptr p) try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    Ptr result=d2_snapshot(p);if(!result)return nullptr;
    auto& f=surface(p).frame();
    suspendClips(f);
    f.dc->Clear(ColorF(0,0,0,0));for(auto& clip:f.clips)Surface::applyClip(f,clip);return result;
} CATCH_PTR
static void blurSurface(Surface& s,double sigma,double ax,double ay,double bx,double by) {
    if(sigma<=0) return;
    auto image=s.snapshot();auto& f=s.frame();
    // Two directional passes preserve nonuniform scale, rotation and skew.
    ComPtr<ID2D1Image> output=image;
    for(auto axis:{Point2F((float)ax,(float)ay),Point2F((float)bx,(float)by)}) {
        auto blur=effect(f,CLSID_D2D1DirectionalBlur,output.Get());
        check(blur->SetValue(D2D1_DIRECTIONALBLUR_PROP_STANDARD_DEVIATION,(float)(sigma*std::hypot(axis.x,axis.y))));
        check(blur->SetValue(D2D1_DIRECTIONALBLUR_PROP_ANGLE,(float)(std::atan2(axis.y,axis.x)*180/3.141592653589793)));
        check(blur->SetValue(D2D1_DIRECTIONALBLUR_PROP_BORDER_MODE,D2D1_BORDER_MODE_SOFT));blur->GetOutput(&output);
    }
    replaceImage(s,output.Get());
}
API void d2_gpu_blur(Ptr p,double sigma,double ax,double ay,double bx,double by) try {
    std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    blurSurface(surface(p),sigma,ax,ay,bx,by);
} CATCH_VOID

static void drawBoxShadow(Surface& target,ID2D1Bitmap1* image,float x,float y) {
    auto& frame=target.frame();
    auto size=image->GetPixelSize();
    D2D1_MATRIX_3X2_F previous;
    frame.dc->GetTransform(&previous);
    auto blend=frame.dc->GetPrimitiveBlend();
    auto antialias=frame.dc->GetAntialiasMode();
    frame.dc->SetTransform(Matrix3x2F::Identity());
    frame.dc->SetPrimitiveBlend(D2D1_PRIMITIVE_BLEND_SOURCE_OVER);
    frame.dc->SetAntialiasMode(D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);
    frame.dc->DrawBitmap(image,RectF(x,y,x+size.width,y+size.height),1,D2D1_INTERPOLATION_MODE_NEAREST_NEIGHBOR);
    frame.dc->SetAntialiasMode(antialias);
    frame.dc->SetPrimitiveBlend(blend);
    frame.dc->SetTransform(previous);
    ++statistics[2];
}
API int d2_gpu_try_draw_box_shadow(Ptr p,const uint64_t* fields,float x,float y) try {
    std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    BoxShadowKey key;
    std::copy_n(fields,key.size(),key.begin());
    auto* image=engine().boxShadows.find(key);
    if(!image)return 0;
    drawBoxShadow(surface(p),image,x,y);
    return 1;
} catch(const std::exception& e) {
    ++statistics[8];
    fprintf(stderr,"Direct2D box shadow: %s\n",e.what());
    return -1;
}
API void d2_gpu_draw_box_shadow(Ptr p,const uint64_t* fields,float x,float y,Ptr caster,Ptr mask,
    double sigma,const double* axes,const float* data) try {
    std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    auto& target=surface(p);
    auto& device=engine();
    auto& cache=device.boxShadows;
    BoxShadowKey key;
    std::copy_n(fields,key.size(),key.begin());
    // The bridge lock is released while Rust records the two masks. A
    // different target may have populated this entry in the meantime.
    if(auto* image=cache.find(key)) {
        drawBoxShadow(target,image,x,y);
        return;
    }
    auto& cast=surface(caster);
    auto& coverage=surface(mask);
    if(target.generation!=device.generation||cast.generation!=device.generation||coverage.generation!=device.generation)
        throw std::runtime_error("Stale Direct2D box shadow surface");
    blurSurface(cast,sigma,axes[0],axes[1],axes[2],axes[3]);
    // The kernel result is an immutable shader output, so the parent can
    // consume it directly without copying it back through the caster surface.
    auto image=runKernel(cast,5,data,8,&coverage);
    cache.insert(key,image);
    drawBoxShadow(target,image.Get(),x,y);
} CATCH_VOID
// Private layer-completion operation: the next operation must pop this layer.
// Keep pointwise native effects in the parent's image graph rather than
// snapshotting, filtering into the child, and submitting the child a second time.
API void d2_gpu_defer_layer_filter(Ptr p,int kind,const float* data,size_t n) try {
    std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    auto& s=surface(p);if(s.frames.size()<2)throw std::runtime_error("Color matrix requires an isolated layer");
    auto& f=*s.frames.back();if(f.hasColorMatrix)throw std::runtime_error("Layer filter already completed");
    if(kind==0&&n>=21) {
        memcpy(&f.colorMatrix,data,80);
        f.colorMatrixAlphaMode=data[20]!=0?D2D1_COLORMATRIX_ALPHA_MODE_PREMULTIPLIED:D2D1_COLORMATRIX_ALPHA_MODE_STRAIGHT;
    } else if(kind==1) {
        f.colorMatrix=Matrix5x4F(0,0,0,0.2125f,0,0,0,0.7154f,0,0,0,0.0721f,0,0,0,0,0,0,0,0);
        f.colorMatrixAlphaMode=D2D1_COLORMATRIX_ALPHA_MODE_STRAIGHT;
    } else throw std::runtime_error("Invalid deferred layer filter");
    f.hasColorMatrix=true;
} CATCH_VOID
API void d2_gpu_filter(Ptr p,int kind,const float* data,size_t n,Ptr secondary) try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    auto& s=surface(p);
    if(kind==3||kind==4||kind==5||kind==6||kind==7) {
        auto result=runKernel(s,kind,data,n,secondary?&surface(secondary):nullptr);replaceImage(s,result.Get());return;
    }
    auto input=s.snapshot();auto& f=s.frame();ComPtr<ID2D1Image> output;
    if(kind==0 && n>=21) {
        auto matrix=effect(f,CLSID_D2D1ColorMatrix,input.Get());D2D1_MATRIX_5X4_F values{};memcpy(&values,data,80);
        check(matrix->SetValue(D2D1_COLORMATRIX_PROP_COLOR_MATRIX,values));
        check(matrix->SetValue(D2D1_COLORMATRIX_PROP_ALPHA_MODE,data[20]!=0?D2D1_COLORMATRIX_ALPHA_MODE_PREMULTIPLIED:D2D1_COLORMATRIX_ALPHA_MODE_STRAIGHT));
        check(matrix->SetValue(D2D1_COLORMATRIX_PROP_CLAMP_OUTPUT,TRUE));matrix->GetOutput(&output);
    } else if(kind==1) {
        auto matrix=effect(f,CLSID_D2D1ColorMatrix,input.Get());
        auto values=Matrix5x4F(0,0,0,0.2125f,0,0,0,0.7154f,0,0,0,0.0721f,0,0,0,0,0,0,0,0);
        check(matrix->SetValue(D2D1_COLORMATRIX_PROP_COLOR_MATRIX,values));
        check(matrix->SetValue(D2D1_COLORMATRIX_PROP_ALPHA_MODE,D2D1_COLORMATRIX_ALPHA_MODE_STRAIGHT));matrix->GetOutput(&output);
    } else if((kind==2||kind==5) && n>=8 && secondary) {
        auto original=surface(secondary).snapshot();auto matrix=effect(f,CLSID_D2D1ColorMatrix,input.Get());
        // Constant straight RGB, with the blurred input alpha times the shadow
        // opacity. The color-matrix effect premultiplies that output exactly once.
        auto values=Matrix5x4F(0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,data[7],data[4],data[5],data[6],0);
        check(matrix->SetValue(D2D1_COLORMATRIX_PROP_COLOR_MATRIX,values));
        check(matrix->SetValue(D2D1_COLORMATRIX_PROP_ALPHA_MODE,D2D1_COLORMATRIX_ALPHA_MODE_PREMULTIPLIED));matrix->GetOutput(&output);
        auto move=effect(f,CLSID_D2D12DAffineTransform,output.Get());
        check(move->SetValue(D2D1_2DAFFINETRANSFORM_PROP_TRANSFORM_MATRIX,Matrix3x2F::Translation(data[0],data[1])));move->GetOutput(&output);
        auto compose=effect(f,CLSID_D2D1Composite,kind==2?output.Get():original.Get());
        compose->SetInput(1,kind==2?original.Get():output.Get());
        check(compose->SetValue(D2D1_COMPOSITE_PROP_MODE,kind==2?D2D1_COMPOSITE_MODE_SOURCE_OVER:D2D1_COMPOSITE_MODE_SOURCE_OUT));compose->GetOutput(&output);
        if(kind==5 && data[2]!=0) throw std::runtime_error("Inset shadow complement requires native custom effect");
    } else if(kind==4 && n>=12) {
        auto convolution=effect(f,CLSID_D2D1ConvolveMatrix,input.Get());UINT32 w=(UINT32)data[0],h=(UINT32)data[1];
        if(!w||!h||n<12+(size_t)w*h) throw std::runtime_error("Invalid convolution kernel");
        check(convolution->SetValue(D2D1_CONVOLVEMATRIX_PROP_KERNEL_SIZE_X,w));check(convolution->SetValue(D2D1_CONVOLVEMATRIX_PROP_KERNEL_SIZE_Y,h));
        check(convolution->SetValue(D2D1_CONVOLVEMATRIX_PROP_KERNEL_MATRIX,D2D1_PROPERTY_TYPE_BLOB,(const BYTE*)(data+12),w*h*sizeof(float)));
        check(convolution->SetValue(D2D1_CONVOLVEMATRIX_PROP_DIVISOR,data[2]==0?1.0f:1.0f/data[2]));check(convolution->SetValue(D2D1_CONVOLVEMATRIX_PROP_BIAS,data[3]));
        check(convolution->SetValue(D2D1_CONVOLVEMATRIX_PROP_PRESERVE_ALPHA,(BOOL)(data[5]!=0)));
        check(convolution->SetValue(D2D1_CONVOLVEMATRIX_PROP_BORDER_MODE,data[4]!=0?D2D1_BORDER_MODE_HARD:D2D1_BORDER_MODE_SOFT));convolution->GetOutput(&output);
    } else if(kind==7 && secondary && n>=8 && data[0]==1) {
        auto previous=surface(secondary).snapshot();auto arithmetic=effect(f,CLSID_D2D1ArithmeticComposite,previous.Get());arithmetic->SetInput(1,input.Get());
        check(arithmetic->SetValue(D2D1_ARITHMETICCOMPOSITE_PROP_COEFFICIENTS,Vector4F(data[4],data[5],data[6],data[7])));
        check(arithmetic->SetValue(D2D1_ARITHMETICCOMPOSITE_PROP_CLAMP_OUTPUT,TRUE));arithmetic->GetOutput(&output);
    } else { throw std::runtime_error("Native lighting/pixel filter is not yet implemented"); }
    replaceImage(s,output.Get());
} CATCH_VOID
API void d2_gpu_draw_tile(Ptr p,Ptr source,double x,double y,double w,double h,double alpha,int mode) try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    if(w<=0||h<=0)return;
    auto& s=surface(p);auto image=surface(source).snapshot();auto& f=s.frame();ComPtr<ID2D1BitmapBrush1> brush;
    auto ex=mode==1||mode==3?D2D1_EXTEND_MODE_MIRROR:D2D1_EXTEND_MODE_WRAP;
    auto ey=mode==2||mode==3?D2D1_EXTEND_MODE_MIRROR:D2D1_EXTEND_MODE_WRAP;
    if(!mode)ex=ey=D2D1_EXTEND_MODE_CLAMP;
    auto properties=BitmapBrushProperties1(ex,ey,s.interpolation==1?D2D1_INTERPOLATION_MODE_NEAREST_NEIGHBOR:D2D1_INTERPOLATION_MODE_LINEAR);
    auto paint=BrushProperties((float)alpha,Matrix3x2F::Scale((float)(w/surface(source).width),(float)(h/surface(source).height))*Matrix3x2F::Translation((float)x,(float)y));
    check(f.dc->CreateBitmapBrush(image.Get(),&properties,&paint,&brush));
    if(!mode)f.dc->FillRectangle(RectF((float)x,(float)y,(float)(x+w),(float)(y+h)),brush.Get());
    else {
        D2D1_MATRIX_3X2_F m;f.dc->GetTransform(&m);auto inverse=*Matrix3x2F::ReinterpretBaseType(&m);if(!inverse.Invert())return;
        auto bounds=RectF(0,0,0,0);
        for(auto point:{Point2F(),Point2F((float)s.width,0),Point2F(0,(float)s.height),Point2F((float)s.width,(float)s.height)}) {
            point=inverse.TransformPoint(point);bounds.left=std::min(bounds.left,point.x);bounds.top=std::min(bounds.top,point.y);bounds.right=std::max(bounds.right,point.x);bounds.bottom=std::max(bounds.bottom,point.y);
        }f.dc->FillRectangle(bounds,brush.Get());
    }
} CATCH_VOID
API void d2_gpu_acrylic(Ptr p,const double* colors,Ptr noise,int replace) try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    auto& s=surface(p);d2_begin_layer(p,1,replace?17:0);auto& f=s.frame();
    D2D1_MATRIX_3X2_F m;f.dc->GetTransform(&m);f.dc->SetTransform(Matrix3x2F::Identity());
    auto bounds=RectF(0,0,(float)s.width,(float)s.height);
    for(int i=0;i<2;++i) { ComPtr<ID2D1SolidColorBrush> brush;check(f.dc->CreateSolidColorBrush(color(colors[i*4],colors[i*4+1],colors[i*4+2],colors[i*4+3]),&brush));f.dc->FillRectangle(bounds,brush.Get()); }
    d2_gpu_draw_tile(p,noise,0,0,256,256,0.02,4);f.dc->SetTransform(m);d2_end_layer(p);
} CATCH_VOID
API Ptr d2_path_intercepts(Ptr p,double lower,double upper) try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    auto strip=path();ComPtr<ID2D1GeometrySink> sink;check(strip->geometry->Open(&sink));
    D2D1_RECT_F bounds{};auto geometry=static_cast<Path*>(p)->geometry;check(geometry->GetBounds(nullptr,&bounds));
    ComPtr<ID2D1RectangleGeometry> rect;check(engine().factory->CreateRectangleGeometry(RectF(bounds.left,(float)lower,bounds.right,(float)upper),&rect));
    check(geometry->CombineWithGeometry(rect.Get(),D2D1_COMBINE_MODE_INTERSECT,nullptr,sink.Get()));check(sink->Close());
    struct Intercepts { std::vector<float> values;float left=0,right=0;bool active=false; } values;
    auto emit=[](Ptr info,Command c) {
        auto& v=*(Intercepts*)info;
        auto flush=[&]{if(v.active&&v.right>v.left){v.values.push_back(v.left);v.values.push_back(v.right);}v.active=false;};
        if(c.kind==0) {flush();v.left=v.right=(float)c.p[0];v.active=true;}
        else if(c.kind==1) {v.left=std::min(v.left,(float)c.p[0]);v.right=std::max(v.right,(float)c.p[0]);}
        else if(c.kind==4) flush();
    };
    EmitSink emitter(&values,emit);check(strip->geometry->Simplify(D2D1_GEOMETRY_SIMPLIFICATION_OPTION_LINES,nullptr,0.01f,&emitter));
    if(values.active&&values.right>values.left) {values.values.push_back(values.left);values.values.push_back(values.right);}
    auto result=std::make_unique<Data>();auto data=(const uint8_t*)values.values.data();result->bytes.assign(data,data+values.values.size()*sizeof(float));return result.release();
} CATCH_PTR

struct Texture : Object { ComPtr<ID3D11Texture2D> texture;UINT slice=0; };
API Ptr d2_native_device() try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex); auto d=engine().d3d.Get();d->AddRef();return d; } CATCH_PTR
API void d2_device_luid(uint8_t* out) try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex); memcpy(out,&engine().adapter.AdapterLuid,8); } catch(...) {memset(out,0,8);}
API Ptr d2_texture(Ptr pointer,UINT slice) try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    ++statistics[6];
    if(!pointer)return nullptr;auto result=std::make_unique<Texture>();
    check(static_cast<IUnknown*>(pointer)->QueryInterface(IID_PPV_ARGS(&result->texture)));
    D3D11_TEXTURE2D_DESC desc{};result->texture->GetDesc(&desc);
    if(slice>=desc.ArraySize||desc.MipLevels!=1||desc.SampleDesc.Count!=1||desc.Width>8192||desc.Height>8192||
       (desc.Format!=DXGI_FORMAT_B8G8R8A8_UNORM&&desc.Format!=DXGI_FORMAT_R8G8B8A8_UNORM))return nullptr;
    ComPtr<ID3D11Device> device;result->texture->GetDevice(&device);
    if(device.Get()!=engine().d3d.Get())return nullptr;
    result->slice=slice;return result.release();
} CATCH_PTR
API Ptr d2_open_texture(intptr_t handle,int nt) try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    auto result=std::make_unique<Texture>();
    if(nt) {ComPtr<ID3D11Device1> device;check(engine().d3d.As(&device));check(device->OpenSharedResource1((HANDLE)handle,IID_PPV_ARGS(&result->texture)));}
    else check(engine().d3d->OpenSharedResource((HANDLE)handle,IID_PPV_ARGS(&result->texture)));
    D3D11_TEXTURE2D_DESC desc{};result->texture->GetDesc(&desc);
    if(desc.ArraySize!=1||desc.SampleDesc.Count!=1||desc.MipLevels!=1)return nullptr;
    return result.release();
} CATCH_PTR
API int d2_texture_valid(Ptr p,UINT w,UINT h,int rgba) { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    if(!p)return 0;
    D3D11_TEXTURE2D_DESC desc{};static_cast<Texture*>(p)->texture->GetDesc(&desc);
    return desc.Width==w&&desc.Height==h&&desc.Format==(rgba?DXGI_FORMAT_R8G8B8A8_UNORM:DXGI_FORMAT_B8G8R8A8_UNORM);
}
static int convertFrame(Ptr destination,Ptr source,int rec709,int flipped,int opaque,Ptr callback,void(*completed)(Ptr,int),bool& submitted) {
    auto& s=surface(destination);auto t=static_cast<Texture*>(source);auto& e=engine();
    if(!t)return 0;
    // A texture of another device (one imported before the device was replaced) cannot be the
    // source of a copy on this one.
    ComPtr<ID3D11Device> owner;t->texture->GetDevice(&owner);
    if(owner.Get()!=e.d3d.Get())return 0;
    D3D11_TEXTURE2D_DESC desc{};t->texture->GetDesc(&desc);
    if(desc.Width!=s.width||desc.Height!=s.height)return 0;
    desc.ArraySize=1;desc.MiscFlags=0;desc.BindFlags=D3D11_BIND_SHADER_RESOURCE|D3D11_BIND_RENDER_TARGET;desc.Usage=D3D11_USAGE_DEFAULT;desc.CPUAccessFlags=0;
    ComPtr<ID3D11Texture2D> copy;check(e.d3d->CreateTexture2D(&desc,nullptr,&copy));
    ComPtr<ID3D11Query> query;D3D11_QUERY_DESC q{D3D11_QUERY_EVENT,0};check(e.d3d->CreateQuery(&q,&query));
    e.immediate->CopySubresourceRegion(copy.Get(),0,0,0,0,t->texture.Get(),t->slice,nullptr);
    submitted=true;
    ComPtr<IDXGISurface> dxgi;check(copy.As(&dxgi));auto& f=s.frame();ComPtr<ID2D1Bitmap1> image;
    auto properties=BitmapProperties1(D2D1_BITMAP_OPTIONS_NONE,PixelFormat(desc.Format,opaque?D2D1_ALPHA_MODE_IGNORE:D2D1_ALPHA_MODE_PREMULTIPLIED));
    check(f.dc->CreateBitmapFromDxgiSurface(dxgi.Get(),&properties,&image));ComPtr<ID2D1Image> output=image;
    if(rec709) {
        std::vector<float> curve(1024);
        for(size_t i=0;i<curve.size();++i) {double x=(double)i/(curve.size()-1);double linear=x<0.081?x/4.5:std::pow((x+0.099)/1.099,1/0.45);curve[i]=(float)(linear<=0.0031308?12.92*linear:1.055*std::pow(linear,1/2.4)-0.055);}
        auto transfer=effect(f,CLSID_D2D1TableTransfer,output.Get());
        for(auto property:{D2D1_TABLETRANSFER_PROP_RED_TABLE,D2D1_TABLETRANSFER_PROP_GREEN_TABLE,D2D1_TABLETRANSFER_PROP_BLUE_TABLE})
            check(transfer->SetValue(property,D2D1_PROPERTY_TYPE_BLOB,(const BYTE*)curve.data(),(UINT32)(curve.size()*4)));
        transfer->GetOutput(&output);
    }
    if(flipped) {auto flip=effect(f,CLSID_D2D12DAffineTransform,output.Get());check(flip->SetValue(D2D1_2DAFFINETRANSFORM_PROP_TRANSFORM_MATRIX,Matrix3x2F(1,0,0,-1,0,(float)s.height)));flip->GetOutput(&output);}
    replaceImage(s,output.Get());f.end();e.immediate->End(query.Get());e.immediate->Flush();
    // Retain all COM objects read by this submission until its event query completes.
    struct ReadResources {
        std::shared_ptr<EngineLifetime> engineOwner=retainEngine();
        ComPtr<ID3D11DeviceContext> context;
        ComPtr<ID3D11Device> device;
        ComPtr<ID3D11Query> query;
        ComPtr<ID3D11Texture2D> copy,producer;
    };
    auto read=std::make_shared<ReadResources>();read->context=e.immediate;read->device=e.d3d;
    read->query=query;read->copy=copy;read->producer=t->texture;
    // One thread watches the event queries of all conversions in flight (a video is a
    // conversion a frame). A query that has not completed after the deadline is reported as
    // indeterminate, which quarantines the lease of its frame instead of polling for ever.
    struct Watcher {
        struct Pending { std::shared_ptr<ReadResources> read;Ptr callback;void(*completed)(Ptr,int);std::chrono::steady_clock::time_point deadline; };
        std::mutex mutex;std::condition_variable wake;std::vector<Pending> pending;bool running=false;
        void add(Pending entry) {
            std::lock_guard<std::mutex> lock(mutex);pending.push_back(std::move(entry));
            if(!running){running=true;std::thread([this]{run();}).detach();}
            wake.notify_one();
        }
        void run() {
            apartment();
            for(;;) {
                std::vector<Pending> current;
                {std::unique_lock<std::mutex> lock(mutex);wake.wait(lock,[&]{return !pending.empty();});current.swap(pending);}
                std::vector<Pending> waiting;
                for(auto& entry:current) {
                    BOOL done=FALSE;
                    HRESULT hr=entry.read->context->GetData(entry.read->query.Get(),&done,sizeof(done),D3D11_ASYNC_GETDATA_DONOTFLUSH);
                    int result=-1;
                    if(hr==S_OK&&done){++statistics[9];result=1;}
                    else if(FAILED(entry.read->device->GetDeviceRemovedReason()))result=2;
                    else if(FAILED(hr)||std::chrono::steady_clock::now()>=entry.deadline)result=0;
                    if(result<0){waiting.push_back(std::move(entry));continue;}
                    auto callback=entry.callback;auto completed=entry.completed;
                    // The resources are released before the completion is reported, as the
                    // thread of a single conversion released them when it returned.
                    entry.read.reset();completed(callback,result);
                }
                if(!waiting.empty()) {
                    {std::lock_guard<std::mutex> lock(mutex);for(auto& entry:waiting)pending.push_back(std::move(entry));}
                    std::this_thread::sleep_for(std::chrono::milliseconds(1));
                }
            }
        }
    };
    // Never destroyed: its thread may still wait on it while the process exits.
    static Watcher& watcher=*new Watcher;
    watcher.add({read,callback,completed,std::chrono::steady_clock::now()+std::chrono::seconds(5)});
    return 1;
}
API int d2_convert_frame(Ptr destination,Ptr source,int rec709,int flipped,int opaque,Ptr callback,void(*completed)(Ptr,int)) { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    bool submitted=false;
    try { return convertFrame(destination,source,rec709,flipped,opaque,callback,completed,submitted); }
    catch(const std::exception& e) {
        fprintf(stderr,"Direct2D media: %s\n",e.what());
        // A failed conversion can still have read producer storage. Its lease
        // must be quarantined instead of being freed on submission/failure.
        if(submitted) {completed(callback,0);return -1;}
        return 0;
    }
}
API void d2_statistics(uint64_t* out,size_t count) { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);for(size_t i=0;i<std::min(count,(size_t)12);++i)out[i]=statistics[i].load();}
API void d2_profile(uint64_t* nanoseconds,uint64_t* counts,size_t count) { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);for(size_t i=0;i<std::min(count,(size_t)20);++i){nanoseconds[i]=nativeTimings[i].load();counts[i]=nativeTimingCounts[i].load();} }
API uint64_t d2_device_generation() try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex); engine().recover();return engine().generation; } catch(...) {return 0;}
API int d2_surface_lost(Ptr p) try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);NativeTiming timing(11);auto& s=surface(p);
    return s.lost||s.generation!=engine().generation||FAILED(engine().d3d->GetDeviceRemovedReason());
} catch(...) {return 1;}
struct Fence : Object { ComPtr<ID3D11Fence> fence; };
API int d2_fences_supported() try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    ComPtr<ID3D11Device5> device;ComPtr<ID3D11DeviceContext4> context;
    return SUCCEEDED(engine().d3d.As(&device))&&SUCCEEDED(engine().immediate.As(&context));
} catch(...) {return 0;}
API Ptr d2_open_fence(intptr_t handle) try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    auto result=std::make_unique<Fence>();ComPtr<ID3D11Device5> device;check(engine().d3d.As(&device));
    check(device->OpenSharedFence((HANDLE)handle,IID_PPV_ARGS(&result->fence)));return result.release();
} CATCH_PTR
API int d2_copy_external(Ptr target,Ptr source,int useMutex,uint64_t acquire,uint64_t release,
        Ptr wait,uint64_t waitValue,Ptr signal,uint64_t signalValue,int flipped) try { std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    auto& e=engine();auto texture=static_cast<Texture*>(source);
    ComPtr<IDXGIKeyedMutex> mutex;
    if(useMutex) {
        check(texture->texture.As(&mutex));
        // The producer releases the key when its commands are submitted, which can be a moment
        // after it tells the compositor: wait for it, but never without a bound on the render
        // thread. AcquireSync returns positive WAIT_TIMEOUT/WAIT_ABANDONED values too.
        HRESULT hr=mutex->AcquireSync(acquire,1000);
        if(hr!=S_OK)return 0;
    }
    struct MutexRelease { ComPtr<IDXGIKeyedMutex> mutex;uint64_t key;~MutexRelease(){if(mutex)mutex->ReleaseSync(key);} } unlock{mutex,release};
    ComPtr<ID3D11DeviceContext4> context;
    if(wait||signal) {check(e.immediate.As(&context));if(wait)check(context->Wait(static_cast<Fence*>(wait)->fence.Get(),waitValue));}
    auto callback=[](Ptr,int){};
    int result=d2_convert_frame(target,source,0,flipped,0,nullptr,callback);
    if(result!=1)return 0;
    if(signal)check(context->Signal(static_cast<Fence*>(signal)->fence.Get(),signalValue));
    e.immediate->Flush();return 1;
} catch(const std::exception& e) {fprintf(stderr,"Direct2D external texture: %s\n",e.what());return 0;}

#ifdef FERRO_RUNTIME_SHADERS
struct Shader : Object {
    uint64_t generation=engine().generation;
    // The Rust render-thread cache can outlive the process-wide engine at exit.
    // Keep a public device reference until both shader children have released.
    ComPtr<ID3D11Device> deviceOwner=engine().d3d;
    ComPtr<ID3D11VertexShader> vertex;ComPtr<ID3D11PixelShader> fragment;
};
API Ptr d2_gpu_shader_compile(const char* source,const char* vertex,const char* fragment,uint8_t* message,size_t length) {
    std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    try {
        // The compiler is a system component on the supported Windows baseline.
        // Only the explicit runtime-shaders feature loads it; never ship a DLL copy.
        using Compile=decltype(&D3DCompile);
        static HMODULE library=LoadLibraryExW(L"d3dcompiler_47.dll",nullptr,LOAD_LIBRARY_SEARCH_SYSTEM32);
        auto compile=library?(Compile)GetProcAddress(library,"D3DCompile"):nullptr;
        if(!compile)throw std::runtime_error("System d3dcompiler_47.dll is unavailable");
        auto bytes=[&](const char* entry,const char* profile) {
            ComPtr<ID3DBlob> code,error;
            HRESULT hr=compile(source,strlen(source),"FerroRuntimeShader",nullptr,nullptr,entry,profile,
                D3DCOMPILE_ENABLE_STRICTNESS|D3DCOMPILE_OPTIMIZATION_LEVEL3,0,&code,&error);
            if(FAILED(hr))throw std::runtime_error(error?std::string((const char*)error->GetBufferPointer(),error->GetBufferSize()):"HLSL compilation failed");
            return code;
        };
        auto vs=bytes(vertex,"vs_5_0"),ps=bytes(fragment,"ps_5_0");auto result=std::make_unique<Shader>();
        check(engine().d3d->CreateVertexShader(vs->GetBufferPointer(),vs->GetBufferSize(),nullptr,&result->vertex));
        check(engine().d3d->CreatePixelShader(ps->GetBufferPointer(),ps->GetBufferSize(),nullptr,&result->fragment));return result.release();
    } catch(const std::exception& e) {
        if(length){auto n=std::min(length-1,strlen(e.what()));memcpy(message,e.what(),n);message[n]=0;}return nullptr;
    }
}
API void d2_gpu_shader_apply(Ptr destination,Ptr raw,const uint8_t* uniforms,size_t size,const Ptr* children,size_t count,
        double x,double y,double w,double h) try {
    std::lock_guard<std::recursive_mutex> nativeGuard(nativeMutex);
    auto& s=surface(destination);auto& e=engine();auto shader=static_cast<Shader*>(raw);
    if(!shader)throw std::runtime_error("Runtime shader is null");
    if(shader->generation!=e.generation)throw std::runtime_error("Runtime shader belongs to a removed device");
    if(w<=0||h<=0||w>16384||h>16384||count>16)return;
    double left=std::max(0.0,floor(x)),top=std::max(0.0,floor(y));
    double right=std::min((double)s.width,ceil(x+w)),bottom=std::min((double)s.height,ceil(y+h));
    if(right<=left||bottom<=top)return;
    UINT width=(UINT)(right-left),height=(UINT)(bottom-top);auto input=s.snapshot();Surface cropped(width,height);
    cropped.frame().dc->SetTransform(Matrix3x2F::Translation((float)-left,(float)-top));cropped.frame().dc->DrawImage(input.Get());
    auto content=cropped.snapshot();
    auto makeView=[&](ID2D1Bitmap1* image) {
        ComPtr<IDXGISurface> dxgi;check(image->GetSurface(&dxgi));ComPtr<ID3D11Texture2D> source;check(dxgi.As(&source));
        D3D11_TEXTURE2D_DESC desc{};source->GetDesc(&desc);desc.BindFlags=D3D11_BIND_SHADER_RESOURCE;desc.MiscFlags=0;desc.CPUAccessFlags=0;desc.Usage=D3D11_USAGE_DEFAULT;
        ComPtr<ID3D11Texture2D> texture;check(e.d3d->CreateTexture2D(&desc,nullptr,&texture));e.immediate->CopyResource(texture.Get(),source.Get());
        ComPtr<ID3D11ShaderResourceView> view;check(e.d3d->CreateShaderResourceView(texture.Get(),nullptr,&view));return view;
    };
    ComPtr<ID3D11Multithread> multithread;check(e.immediate.As(&multithread));multithread->Enter();
    struct Unlock{ID3D11Multithread* lock;~Unlock(){lock->Leave();}} unlock{multithread.Get()};
    std::vector<ComPtr<ID3D11ShaderResourceView>> owners;owners.push_back(makeView(content.Get()));
    for(size_t i=0;i<count;++i) {
        if(children[i]){auto image=surface(children[i]).snapshot();owners.push_back(makeView(image.Get()));}
        else {Surface empty(1,1);auto image=empty.snapshot();owners.push_back(makeView(image.Get()));}
    }
    ComPtr<ID3D11Device1> device1;check(e.d3d.As(&device1));ComPtr<ID3D11DeviceContext1> context1;check(e.immediate.As(&context1));
    ComPtr<ID3DDeviceContextState> isolated,previous;D3D_FEATURE_LEVEL supported;
    check(device1->CreateDeviceContextState(0,&e.level,1,D3D11_SDK_VERSION,__uuidof(ID3D11Device),&supported,&isolated));
    context1->SwapDeviceContextState(isolated.Get(),&previous);
    struct Restore{ID3D11DeviceContext1* context;ComPtr<ID3DDeviceContextState> previous;~Restore(){if(previous)context->SwapDeviceContextState(previous.Get(),nullptr);}} restore{context1.Get(),previous};
    auto constantBuffer=[&](const void* data,UINT n) {
        D3D11_BUFFER_DESC desc{};desc.ByteWidth=n;desc.Usage=D3D11_USAGE_IMMUTABLE;desc.BindFlags=D3D11_BIND_CONSTANT_BUFFER;
        D3D11_SUBRESOURCE_DATA initial{data,0,0};ComPtr<ID3D11Buffer> result;check(e.d3d->CreateBuffer(&desc,&initial,&result));return result;
    };
    auto values=constantBuffer(uniforms,(UINT)size);float area[4]={(float)(left-x),(float)(top-y),(float)width,(float)height};auto pass=constantBuffer(area,16);
    ID3D11Buffer* buffers[]={values.Get(),pass.Get()};e.immediate->PSSetConstantBuffers(0,2,buffers);e.immediate->VSSetConstantBuffers(0,2,buffers);
    std::vector<ID3D11ShaderResourceView*> views;for(auto& view:owners)views.push_back(view.Get());
    e.immediate->PSSetShaderResources(0,(UINT)views.size(),views.data());
    D3D11_TEXTURE2D_DESC desc{};desc.Width=width;desc.Height=height;desc.MipLevels=desc.ArraySize=1;desc.SampleDesc.Count=1;
    desc.Format=DXGI_FORMAT_R8G8B8A8_UNORM;desc.BindFlags=D3D11_BIND_RENDER_TARGET|D3D11_BIND_SHADER_RESOURCE;
    ComPtr<ID3D11Texture2D> target;check(e.d3d->CreateTexture2D(&desc,nullptr,&target));ComPtr<ID3D11RenderTargetView> view;
    check(e.d3d->CreateRenderTargetView(target.Get(),nullptr,&view));ID3D11RenderTargetView* targets[]={view.Get()};
    float transparent[4]{};e.immediate->ClearRenderTargetView(view.Get(),transparent);e.immediate->OMSetRenderTargets(1,targets,nullptr);
    D3D11_VIEWPORT viewport{0,0,(float)width,(float)height,0,1};e.immediate->RSSetViewports(1,&viewport);
    D3D11_RASTERIZER_DESC raster{};raster.FillMode=D3D11_FILL_SOLID;raster.CullMode=D3D11_CULL_NONE;raster.DepthClipEnable=TRUE;
    ComPtr<ID3D11RasterizerState> rasterizer;check(e.d3d->CreateRasterizerState(&raster,&rasterizer));e.immediate->RSSetState(rasterizer.Get());
    e.immediate->IASetInputLayout(nullptr);e.immediate->IASetPrimitiveTopology(D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
    e.immediate->VSSetShader(shader->vertex.Get(),nullptr,0);e.immediate->PSSetShader(shader->fragment.Get(),nullptr,0);e.immediate->Draw(3,0);
    e.immediate->OMSetRenderTargets(0,nullptr,nullptr);std::vector<ID3D11ShaderResourceView*> none(views.size());e.immediate->PSSetShaderResources(0,(UINT)none.size(),none.data());
    // Restore the immediate context before invoking Direct2D, which also owns state.
    context1->SwapDeviceContextState(previous.Get(),nullptr);restore.previous.Reset();
    ComPtr<IDXGISurface> dxgi;check(target.As(&dxgi));ComPtr<ID2D1Bitmap1> rendered;
    auto properties=BitmapProperties1(D2D1_BITMAP_OPTIONS_NONE,PixelFormat(desc.Format,D2D1_ALPHA_MODE_PREMULTIPLIED));
    Surface full(s.width,s.height);check(full.frame().dc->CreateBitmapFromDxgiSurface(dxgi.Get(),&properties,&rendered));
    full.frame().dc->DrawImage(rendered.Get(),Point2F((float)left,(float)top));auto result=full.snapshot();replaceImage(s,result.Get());
} CATCH_VOID
#endif
