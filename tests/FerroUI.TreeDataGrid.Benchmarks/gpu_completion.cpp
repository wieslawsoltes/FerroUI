#include <d3d11.h>
#include <dxgi.h>
#include <wrl/client.h>
#include <chrono>
#include <cwchar>
#include <memory>
using Microsoft::WRL::ComPtr;
struct Completion {
    ComPtr<ID3D11Device> device;
    ComPtr<ID3D11DeviceContext> context;
    ComPtr<ID3D11Query> query;
};
// Takes the AddRef-owned device returned by ferroui_direct2d::native_device().
extern "C" void* tdg_completion_create(void* device) {
    auto state=std::make_unique<Completion>();
    state->device.Attach(static_cast<ID3D11Device*>(device));
    state->device->GetImmediateContext(&state->context);
    D3D11_QUERY_DESC desc{D3D11_QUERY_EVENT,0};
    if(FAILED(state->device->CreateQuery(&desc,&state->query)))return nullptr;
    return state.release();
}
extern "C" long tdg_completion_wait(void* value) {
    auto& state=*static_cast<Completion*>(value);
    state.context->End(state.query.Get());state.context->Flush();
    auto deadline=std::chrono::steady_clock::now()+std::chrono::seconds(30);
    for(;;) {
        BOOL done=FALSE;
        HRESULT hr=state.context->GetData(state.query.Get(),&done,sizeof(done),D3D11_ASYNC_GETDATA_DONOTFLUSH);
        if(FAILED(hr))return hr;
        if(hr==S_OK&&done)return S_OK;
        if(std::chrono::steady_clock::now()>deadline)return HRESULT_FROM_WIN32(WAIT_TIMEOUT);
        SwitchToThread();
    }
}
// Writes the description of the adapter the device was created on as UTF-8, without a terminator.
// Returns the number of bytes written, or 0 on failure.
extern "C" int tdg_completion_adapter_description(void* value,char* out,int capacity) {
    auto& state=*static_cast<Completion*>(value);
    ComPtr<IDXGIDevice> dxgi;
    ComPtr<IDXGIAdapter> adapter;
    DXGI_ADAPTER_DESC desc{};
    if(FAILED(state.device.As(&dxgi))||FAILED(dxgi->GetAdapter(&adapter))||FAILED(adapter->GetDesc(&desc)))return 0;
    int length=static_cast<int>(wcsnlen(desc.Description,ARRAYSIZE(desc.Description)));
    return WideCharToMultiByte(CP_UTF8,0,desc.Description,length,out,capacity,nullptr,nullptr);
}
extern "C" void tdg_completion_destroy(void* value) { delete static_cast<Completion*>(value); }
