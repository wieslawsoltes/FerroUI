// Standalone Windows SDK regression harness. Build outside Cargo to exercise
// native synchronization while the application's optimized link is running.
#include "native.cpp"
#include <cassert>
#include <d3d11sdklayers.h>

static void checkGlyphAtlas(Engine& engine) {
    auto font=d2_font("Arial",400,0,5);
    assert(font);
    const UINT32 characters[]={'H','i'};
    uint16_t ids[2]{};
    check(static_cast<Font*>(font)->face->GetGlyphIndices(characters,2,ids));
    assert(ids[0]&&ids[1]);
    const double positions[]={4.,20.,16.,20.};
    auto run=d2_positioned_glyph_run(font,16.,ids,positions,2,nullptr);
    assert(run);
    DrawCommand glyph{};
    glyph.kind=2;glyph.resource=run;
    glyph.matrix[0]=glyph.matrix[3]=1.25;
    glyph.matrix[4]=0.25;glyph.matrix[5]=0.5;
    glyph.params[0]=0.2;glyph.params[1]=0.4;glyph.params[2]=0.6;glyph.params[3]=1.;

    auto render=[&](bool clipped) {
        auto surface=d2_surface(64,40);
        d2_clear(surface,0,0,0.8,0.2,0.1,0.6);
        if(clipped) {
            d2_save(surface);
            d2_clip_rect(surface,12.,0.,20.,40.);
        }
        d2_draw_batch(surface,&glyph,1);
        if(clipped)d2_restore(surface);
        d2_finish(surface);
        std::vector<uint8_t> pixels(64*40*4);
        d2_read_pixels(surface,pixels.data(),pixels.size());
        d2_release(surface);
        return pixels;
    };
    for(int frame=0;frame<2;++frame) {
        engine.glyphAtlas.begin();
        render(false);
        engine.glyphAtlas.finish();
    }
    auto& entries=static_cast<PositionedGlyphRun*>(run)->rasters;
    assert(entries.size()==1);
    auto page=entries.front().page.lock();
    assert(page&&page->ready);
    // An integer device translation keeps the same raster phase. The
    // consuming context still applies its own clip and destination alpha.
    glyph.matrix[4]+=8.;
    const auto direct=render(true);
    const auto previousUse=page->used;
    engine.glyphAtlas.begin();
    const auto cached=render(true);
    engine.glyphAtlas.finish();
    assert(page->used>previousUse);
    assert(cached==direct);
    d2_release(run);d2_release(font);
    engine.glyphAtlas.reset();
    printf("NATIVE_D2D_GLYPH_ATLAS_PASS fractional_dpi=125%% integer_translation=8 clip_and_alpha=exact\n");
}

int main() {
    assert(d2_supported());
    auto& e=engine();
    D3D11_TEXTURE2D_DESC desc{};desc.Width=16;desc.Height=16;desc.ArraySize=2;desc.MipLevels=1;
    desc.Format=DXGI_FORMAT_B8G8R8A8_UNORM;desc.SampleDesc.Count=1;
    desc.BindFlags=D3D11_BIND_SHADER_RESOURCE|D3D11_BIND_RENDER_TARGET;
    ComPtr<ID3D11Texture2D> texture;check(e.d3d->CreateTexture2D(&desc,nullptr,&texture));
    for(UINT slice=0;slice<2;++slice) {
        D3D11_RENDER_TARGET_VIEW_DESC view{};view.Format=desc.Format;view.ViewDimension=D3D11_RTV_DIMENSION_TEXTURE2DARRAY;
        view.Texture2DArray.ArraySize=1;view.Texture2DArray.FirstArraySlice=slice;
        ComPtr<ID3D11RenderTargetView> target;check(e.d3d->CreateRenderTargetView(texture.Get(),&view,&target));
        float color[4]={(float)(slice==0),(float)(slice==1),0,1};e.immediate->ClearRenderTargetView(target.Get(),color);
    }
    Ptr imported=d2_texture(texture.Get(),1);assert(imported);
    assert(!d2_texture(texture.Get(),2));
    assert(d2_texture_valid(imported,16,16,0));assert(!d2_texture_valid(imported,15,16,0));
    auto output=d2_surface(16,16);assert(output);
    std::atomic<int> done{0};
    auto callback=[](Ptr context,int success){static_cast<std::atomic<int>*>(context)->store(success?1:-1);};
    assert(d2_convert_frame(output,imported,0,0,1,&done,callback)==1);
    d2_release(imported);texture.Reset();
    auto deadline=std::chrono::steady_clock::now()+std::chrono::seconds(5);
    while(!done.load()&&std::chrono::steady_clock::now()<deadline)std::this_thread::sleep_for(std::chrono::milliseconds(1));
    assert(done.load()==1);
    uint64_t before[10]{};d2_statistics(before,10);
    // The GPU producer, array-slice import and conversion must not read/map pixels.
    assert(before[4]==0&&before[5]==0);
    std::vector<uint8_t> pixels(16*16*4);d2_read_pixels(output,pixels.data(),pixels.size());
    for(size_t i=0;i<pixels.size();i+=4)assert(pixels[i]==0&&pixels[i+1]==255&&pixels[i+2]==0&&pixels[i+3]==255);
    d2_release(output);
    printf("NATIVE_D3D11_GPU_ARRAY_SLICE_PASS cpu_frame_readbacks=0 uploads=0 completion=event-query\n");
    // Both DXGI handle types must preserve keyed-mutex ownership through a GPU
    // copy, including a resource opened through a distinct COM texture object.
    for(bool nt:{false,true}) {
        desc.ArraySize=1;desc.MiscFlags=D3D11_RESOURCE_MISC_SHARED_KEYEDMUTEX|(nt?D3D11_RESOURCE_MISC_SHARED_NTHANDLE:0);
        ComPtr<ID3D11Texture2D> shared;check(e.d3d->CreateTexture2D(&desc,nullptr,&shared));
        ComPtr<IDXGIKeyedMutex> mutex;check(shared.As(&mutex));assert(mutex->AcquireSync(0,0)==S_OK);
        ComPtr<ID3D11RenderTargetView> view;check(e.d3d->CreateRenderTargetView(shared.Get(),nullptr,&view));
        float blue[4]={0,0,1,1};e.immediate->ClearRenderTargetView(view.Get(),blue);e.immediate->Flush();check(mutex->ReleaseSync(1));
        HANDLE handle=nullptr;
        if(nt) {ComPtr<IDXGIResource1> resource;check(shared.As(&resource));check(resource->CreateSharedHandle(nullptr,DXGI_SHARED_RESOURCE_READ|DXGI_SHARED_RESOURCE_WRITE,nullptr,&handle));}
        else {ComPtr<IDXGIResource> resource;check(shared.As(&resource));check(resource->GetSharedHandle(&handle));}
        auto imported=d2_open_texture((intptr_t)handle,nt);assert(imported);if(nt)CloseHandle(handle);
        auto image=d2_surface(16,16);assert(d2_copy_external(image,imported,1,1,2,nullptr,0,nullptr,0,0)==1);
        assert(mutex->AcquireSync(2,1000)==S_OK);check(mutex->ReleaseSync(0));
        d2_read_pixels(image,pixels.data(),pixels.size());
        for(size_t i=0;i<pixels.size();i+=4)assert(pixels[i]==255&&pixels[i+1]==0&&pixels[i+2]==0&&pixels[i+3]==255);
        d2_release(image);d2_release(imported);
    }
    printf("NATIVE_DXGI_SHARED_HANDLES_PASS handles=NT,global synchronization=keyed-mutex\n");
    assert(d2_fences_supported());
    ComPtr<ID3D11Device5> device5;check(e.d3d.As(&device5));ComPtr<ID3D11DeviceContext4> context4;check(e.immediate.As(&context4));
    ComPtr<ID3D11Fence> fence;check(device5->CreateFence(0,D3D11_FENCE_FLAG_SHARED,IID_PPV_ARGS(&fence)));
    HANDLE fenceHandle=nullptr;check(fence->CreateSharedHandle(nullptr,GENERIC_ALL,nullptr,&fenceHandle));
    auto importedFence=d2_open_fence((intptr_t)fenceHandle);CloseHandle(fenceHandle);assert(importedFence);
    desc.MiscFlags=0;ComPtr<ID3D11Texture2D> source;check(e.d3d->CreateTexture2D(&desc,nullptr,&source));
    ComPtr<ID3D11RenderTargetView> target;check(e.d3d->CreateRenderTargetView(source.Get(),nullptr,&target));
    float red[4]={1,0,0,1};e.immediate->ClearRenderTargetView(target.Get(),red);check(context4->Signal(fence.Get(),1));
    auto importedSource=d2_texture(source.Get(),0);assert(importedSource);auto fenceOutput=d2_surface(16,16);
    assert(d2_copy_external(fenceOutput,importedSource,0,0,0,importedFence,1,importedFence,2,0)==1);
    auto until=std::chrono::steady_clock::now()+std::chrono::seconds(5);
    while(fence->GetCompletedValue()<2&&std::chrono::steady_clock::now()<until)std::this_thread::sleep_for(std::chrono::milliseconds(1));
    assert(fence->GetCompletedValue()==2);d2_read_pixels(fenceOutput,pixels.data(),pixels.size());
    for(size_t i=0;i<pixels.size();i+=4)assert(pixels[i]==0&&pixels[i+1]==0&&pixels[i+2]==255&&pixels[i+3]==255);
    d2_release(fenceOutput);d2_release(importedSource);d2_release(importedFence);
    printf("NATIVE_D3D11_TIMELINE_FENCE_PASS wait=1 signal=2\n");
    auto light=d2_surface(16,16);float lighting[344]{};lighting[0]=0.25f;lighting[1]=0.5f;lighting[2]=0.75f;
    lighting[16]=lighting[21]=1;
    d2_gpu_filter(light,6,lighting,344,nullptr);d2_read_pixels(light,pixels.data(),pixels.size());
    for(size_t i=0;i<pixels.size();i+=4)assert(abs((int)pixels[i]-191)<=1&&abs((int)pixels[i+1]-128)<=1&&abs((int)pixels[i+2]-64)<=1&&pixels[i+3]==255);
    d2_release(light);
    auto gradient=d2_surface(16,16);double colors[]={1,0,0,1,0,0,1,1},stops[]={0,1};
    d2_gpu_gradient_options(gradient,0,1,0);d2_gradient(gradient,colors,stops,2,8.5,8.5,0,0,0,2);
    d2_read_pixels(gradient,pixels.data(),pixels.size());
    assert(pixels[(2*16+8)*4+2]>250&&pixels[(2*16+8)*4]<2);
    assert(pixels[(8*16+14)*4+2]>185&&pixels[(8*16+14)*4]>60);
    d2_release(gradient);
    printf("NATIVE_D3D11_EFFECT_KERNELS_PASS lighting=reference conic=clockwise\n");
    {
        auto target=d2_surface(16,16);d2_clear(target,16,16,1,0,0,1);
        d2_set_transform(target,1,0,0,1,2,3,1,1);
        auto copies=statistics[7].load(),readbacks=statistics[4].load(),work=statistics[11].load();
        d2_gpu_draw_lighting(target,8,7,4,5,lighting,344);
        assert(statistics[7]==copies&&statistics[4]==readbacks&&statistics[11]-work==56);
        D2D1_MATRIX_3X2_F restored{};surface(target).frame().dc->GetTransform(&restored);
        assert(restored._31==2&&restored._32==3);
        d2_finish(target);d2_read_pixels(target,pixels.data(),pixels.size());
        auto at=(5*16+4)*4;assert(abs((int)pixels[at]-191)<=1&&abs((int)pixels[at+1]-128)<=1&&abs((int)pixels[at+2]-64)<=1&&pixels[at+3]==255);
        assert(pixels[0]==0&&pixels[1]==0&&pixels[2]==255&&pixels[3]==255);
        d2_release(target);
    }
    printf("NATIVE_DIRECT_LIGHTING_PASS cropped_work=56 copies=0 readbacks=0 parent_content_and_transform=preserved\n");
#ifdef FERRO_RUNTIME_SHADERS
    const char* program=R"(
        float4 vs(uint index:SV_VertexID):SV_Position {float2 uv=float2((index<<1)&2,index&2);return float4(uv*2-1,0,1);}
        float4 ps(float4 position:SV_Position):SV_Target {return float4(1,0,0,1);}
    )";
    uint8_t error[4096]{};auto pipeline=d2_gpu_shader_compile(program,"vs","ps",error,sizeof(error));
    if(!pipeline)fprintf(stderr,"%s\n",error);assert(pipeline);
    auto shaderOutput=d2_surface(64,64);float uniforms[4]{};
    d2_gpu_shader_apply(shaderOutput,pipeline,(const uint8_t*)uniforms,sizeof(uniforms),nullptr,0,8,12,32,32);
    std::vector<uint8_t> rendered(64*64*4);d2_read_pixels(shaderOutput,rendered.data(),rendered.size());
    fprintf(stderr,"Runtime shader pixel: %u %u %u %u\n",rendered[(20*64+20)*4],rendered[(20*64+20)*4+1],rendered[(20*64+20)*4+2],rendered[(20*64+20)*4+3]);
    assert(rendered[(20*64+20)*4+2]==255);assert(rendered[(20*64+20)*4+3]==255);
    d2_release(shaderOutput);d2_release(pipeline);printf("NATIVE_D3D11_RUNTIME_SHADER_PASS translated_bounds=correct\n");
#endif
    // Exercise swapchain resizing and the minimized-window branch without
    // displaying a helper window over the interactive catalog.
    HWND hwnd=CreateWindowExW(0,L"STATIC",L"Native renderer test",WS_OVERLAPPEDWINDOW,
        0,0,128,128,nullptr,nullptr,GetModuleHandleW(nullptr),nullptr);
    assert(hwnd);
    auto window=d2_window((intptr_t)hwnd);assert(window);
    auto first=d2_surface(32,32);
    d2_finish(first);
    assert(d2_window_state(window)==1);assert(d2_present(window,first)==1);
    auto resized=d2_surface(64,48);assert(d2_present(window,resized)>=0);
    DXGI_SWAP_CHAIN_DESC1 resizedDescription{};
    check(static_cast<Window*>(window)->chain->GetDesc1(&resizedDescription));
    assert(resizedDescription.Width==64&&resizedDescription.Height==48);
    assert(resizedDescription.AlphaMode==DXGI_ALPHA_MODE_PREMULTIPLIED);
    d2_window_dispose(window);assert(d2_window_state(window)==-1);assert(d2_present(window,first)==-1);
    d2_release(window);DestroyWindow(hwnd);
    hwnd=CreateWindowExW(0,L"STATIC",L"Native minimized test",WS_OVERLAPPEDWINDOW|WS_MINIMIZE,
        0,0,128,128,nullptr,nullptr,GetModuleHandleW(nullptr),nullptr);
    assert(hwnd);window=d2_window((intptr_t)hwnd);assert(window);
    assert(d2_window_state(window)==0);assert(d2_present(window,first)==0);
    d2_release(first);d2_release(resized);d2_release(window);DestroyWindow(hwnd);
    printf("NATIVE_DXGI_PRESENTATION_PASS resize=32x32-to-64x48 minimized=skipped hidden-window=true\n");
    for(UINT32 codepoint:{UINT32('A'),UINT32(0x1f600)}) {
        auto font=d2_font(codepoint=='A'?"Segoe UI":"Segoe UI Emoji",400,0,5);assert(font);
        UINT16 glyph=0;check(static_cast<Font*>(font)->face->GetGlyphIndices(&codepoint,1,&glyph));assert(glyph);
        UINT16 glyphs[]={glyph,glyph,glyph};double positions[]={7.25,35,100,52.5,52.5,30.25};
        auto batch=d2_surface(160,70),individual=d2_surface(160,70);
        d2_glyphs(batch,font,28,glyphs,positions,3,0,0,0,1);
        for(int i=0;i<3;++i)d2_glyphs(individual,font,28,glyphs+i,positions+i*2,1,0,0,0,1);
        std::vector<uint8_t> first(160*70*4),second(first.size());
        d2_read_pixels(batch,first.data(),first.size());d2_read_pixels(individual,second.data(),second.size());
        assert(first==second);
        if(codepoint==0x1f600){bool colored=false;for(size_t i=0;i<first.size();i+=4)colored|=first[i]!=first[i+1]||first[i+1]!=first[i+2];assert(colored);}
        d2_release(batch);d2_release(individual);d2_release(font);
    }
    printf("NATIVE_DIRECTWRITE_BATCH_PASS positioned_glyphs=identical color_emoji=preserved\n");
    auto opacity=d2_surface(16,16);d2_begin_layer(opacity,0.25,0);
    d2_clear(opacity,16,16,128./255,64./255,32./255,1);d2_end_layer(opacity);
    d2_read_pixels(opacity,pixels.data(),pixels.size());
    assert(abs((int)pixels[0]-8)<=1&&abs((int)pixels[1]-16)<=1&&abs((int)pixels[2]-32)<=1&&abs((int)pixels[3]-64)<=1);d2_release(opacity);
    auto luminance=d2_surface(16,16);d2_clear(luminance,16,16,1,0,0,0.5);
    d2_gpu_filter(luminance,1,nullptr,0,nullptr);d2_read_pixels(luminance,pixels.data(),pixels.size());
    assert(!pixels[0]&&!pixels[1]&&!pixels[2]&&abs((int)pixels[3]-27)<=1);d2_release(luminance);
    auto halo=d2_surface(16,16),empty=d2_surface(16,16);d2_clear(halo,16,16,1,1,1,0.5);
    float haloParameters[]={0,0,0,0,0,0,1,0.8f};d2_gpu_filter(halo,2,haloParameters,8,empty);
    d2_read_pixels(halo,pixels.data(),pixels.size());
    assert(abs((int)pixels[0]-102)<=1&&!pixels[1]&&!pixels[2]&&abs((int)pixels[3]-102)<=1);
    d2_release(halo);d2_release(empty);
    printf("NATIVE_ALPHA_EFFECTS_PASS opacity=all-channels luminance=alpha-weighted shadow=premultiplied-once\n");
    for(int mode=0;mode<3;++mode) {
        auto eager=d2_surface(16,16),deferred=d2_surface(16,16);
        float matrix[]={.8f,0,0,0,0,.7f,0,0,0,0,.6f,0,0,0,0,.9f,.05f,.03f,.01f,.02f,(float)mode};
        for(auto target:{eager,deferred}) {
            d2_clear(target,16,16,0,0,1,1);d2_begin_layer(target,.75,0);
            d2_clear(target,16,16,.6,.3,.1,.5);
        }
        int kind=mode==2?1:0;
        d2_gpu_filter(eager,kind,matrix,21,nullptr);d2_end_layer(eager);
        auto copies=statistics[7].load();
        d2_gpu_defer_layer_filter(deferred,kind,matrix,21);d2_end_layer(deferred);
        assert(statistics[7]==copies);
        std::vector<uint8_t> reference(16*16*4),actual(reference.size());
        d2_read_pixels(eager,reference.data(),reference.size());d2_read_pixels(deferred,actual.data(),actual.size());
        // The eager path quantizes its intermediate to BGRA8 before opacity.
        // The deferred image graph keeps that intermediate at effect precision.
        for(size_t i=0;i<actual.size();++i)assert(abs((int)actual[i]-reference[i])<=2);
        d2_release(eager);d2_release(deferred);
    }
    printf("NATIVE_DEFERRED_LAYER_FILTER_PASS straight=matched premultiplied=matched luminance=matched copies=0\n");
    {
        auto source=d2_surface(16,16);d2_clear(source,16,16,1,0,0,1);d2_finish(source);
        auto first=surface(source).snapshot(),second=surface(source).snapshot();assert(first.Get()==second.Get());
        d2_clear(source,16,16,0,0,1,1);d2_finish(source);
        auto changed=surface(source).snapshot();assert(changed.Get()!=first.Get());
        auto sampled=d2_surface(16,16);surface(sampled).frame().dc->DrawImage(first.Get());d2_finish(sampled);
        d2_read_pixels(sampled,pixels.data(),pixels.size());assert(pixels[2]==255&&pixels[0]==0);
        d2_clear(sampled,16,16,0,0,0,0);surface(sampled).frame().dc->DrawImage(changed.Get());d2_finish(sampled);
        d2_read_pixels(sampled,pixels.data(),pixels.size());assert(pixels[0]==255&&pixels[2]==0);
        d2_release(sampled);d2_release(source);
    }
    printf("NATIVE_IMMUTABLE_SNAPSHOT_CACHE_PASS reused_until_write previous_content=independent\n");
    checkGlyphAtlas(e);
    ComPtr<ID3D11InfoQueue> diagnostics;
    if(SUCCEEDED(e.d3d.As(&diagnostics))) {
        UINT errors=0,warnings=0;
        for(UINT64 i=0;i<diagnostics->GetNumStoredMessagesAllowedByRetrievalFilter();++i) {
            SIZE_T length=0;check(diagnostics->GetMessage(i,nullptr,&length));std::vector<uint8_t> data(length);
            auto message=(D3D11_MESSAGE*)data.data();check(diagnostics->GetMessage(i,message,&length));
            if(message->Severity<=D3D11_MESSAGE_SEVERITY_WARNING)fprintf(stderr,"D3D DEBUG: %s\n",message->pDescription);
            if(message->Severity<=D3D11_MESSAGE_SEVERITY_ERROR)++errors;
            else if(message->Severity==D3D11_MESSAGE_SEVERITY_WARNING)++warnings;
        }
        assert(errors==0);printf("NATIVE_D3D11_DEBUG_PASS errors=%u warnings=%u\n",errors,warnings);
    }
    auto stale=d2_surface(16,16);auto generation=d2_device_generation();
    HWND recoveryHwnd=CreateWindowExW(WS_EX_NOREDIRECTIONBITMAP,L"STATIC",L"Native recovery test",WS_OVERLAPPEDWINDOW,0,0,64,64,nullptr,nullptr,GetModuleHandleW(nullptr),nullptr);
    assert(recoveryHwnd);auto staleWindow=d2_window((intptr_t)recoveryHwnd);assert(d2_present(staleWindow,stale)>=0);
    {std::lock_guard<std::recursive_mutex> lock(nativeMutex);e.replaceDevice();}
    assert(d2_device_generation()==generation+1);assert(d2_surface_lost(stale));
    assert(d2_window_state(staleWindow)==-1);
    auto replacement=d2_surface(16,16);assert(!d2_surface_lost(replacement));
    d2_clear(replacement,16,16,1,0,0,1);d2_read_pixels(replacement,pixels.data(),pixels.size());
    assert(pixels[2]==255&&pixels[3]==255);
    d2_finish(replacement);
    auto replacementWindow=d2_window((intptr_t)recoveryHwnd);assert(d2_present(replacementWindow,replacement)==1);
    d2_window_dispose(replacementWindow);d2_release(replacementWindow);d2_release(staleWindow);DestroyWindow(recoveryHwnd);
    d2_release(stale);d2_release(replacement);
    printf("NATIVE_D3D11_DEVICE_REPLACEMENT_PASS stale_resources=rejected replacement=hardware\n");
    // Emulate CRT teardown before Rust TLS drops its final cached bitmap/font.
    for(int i=0;i<1000&&engineRoot().value.use_count()!=1;++i)Sleep(1);
    assert(engineRoot().value.use_count()==1);
    auto held=d2_surface(16,16);auto heldSnapshot=d2_snapshot(held);auto heldFont=d2_font("Segoe UI",400,0,5);
    std::weak_ptr<EngineLifetime> lifetime=engineRoot().value;
    engineRoot().release();assert(!lifetime.expired());
    d2_release(held);d2_release(heldSnapshot);assert(!lifetime.expired());
    d2_release(heldFont);assert(lifetime.expired());
    printf("NATIVE_LATE_RESOURCE_RELEASE_PASS factories_and_MTA=retained_until_last_native_object\n");
    return 0;
}
