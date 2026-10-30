// Copyright (c) FerroUI contributors. Licensed under the MIT License.
// Portable ABI smoke check: compile against the current generated native header.
// No AppKit dependency; the real Objective-C++ bridge has its own macOS CI job.
#include <sys/types.h>
#define COM_GUIDS_MATERIALIZE
#include "comimpl.h"
#include "ferro-native.h"
#include <cassert>
#include <cstdio>

class ProbeSettings : public ComObject,
                      public virtual IFrnPlatformSettings,
                      public virtual IFrnAccessibilitySettings
{
    bool* destroyed;

    BEGIN_INTERFACE_MAP()
    INTERFACE_MAP_ENTRY(IFrnPlatformSettings, IID_IFrnPlatformSettings)
    INTERFACE_MAP_ENTRY(IFrnAccessibilitySettings, IID_IFrnAccessibilitySettings)
    END_INTERFACE_MAP()

public:
    FORWARD_IUNKNOWN()
    explicit ProbeSettings(bool* wasDestroyed) : destroyed(wasDestroyed) {}
    ~ProbeSettings() override { *destroyed = true; }
    FrnPlatformThemeVariant GetPlatformTheme() override { return FrnPlatformThemeVariant::Light; }
    unsigned int GetAccentColor() override { return 0; }
    void RegisterColorsChange(IFrnActionCallback*) override {}
    HRESULT GetPreferredLanguage(IFrnString** result) override { *result = nullptr; return S_OK; }
    void RegisterLanguageChange(IFrnActionCallback*) override {}
    bool GetReduceMotion() override { return true; }
    bool GetReduceTransparency() override { return false; }
    void RegisterAccessibilityChange(IFrnActionCallback*) override {}
};

class ProbeCallback : public ComSingleObject<IFrnActionCallback, &IID_IFrnActionCallback>
{
    ComPtr<IFrnActionCallback>* owner;
    bool* destroyed;

public:
    FORWARD_IUNKNOWN()
    ProbeCallback(ComPtr<IFrnActionCallback>* owningField, bool* wasDestroyed)
        : owner(owningField), destroyed(wasDestroyed) {}
    ~ProbeCallback() override { *destroyed = true; }
    void Run() override
    {
        *owner = nullptr;
        assert(!*destroyed && "invocation must retain a callback that unregisters itself");
    }
};

int main()
{
    // Literal UUID bytes are the public IDL/Rust contract. Comparing only two
    // C++ macro expansions would miss an error in GUID materialization itself.
    const GUID platformIID = {0xd1f009cc, 0x9d2d, 0x493b,
        {0x84, 0x5d, 0x90, 0xd2, 0xc1, 0x04, 0xba, 0xae}};
    const GUID accessibilityIID = {0x56b3638d, 0xe693, 0x4784,
        {0x92, 0xba, 0x35, 0x1b, 0xe9, 0xcb, 0xf6, 0x8e}};
    const GUID unknownIID = {0, 0, 0, {0xc0, 0, 0, 0, 0, 0, 0, 0x46}};
    assert(std::memcmp(&platformIID, &IID_IFrnPlatformSettings, sizeof(GUID)) == 0);
    assert(std::memcmp(&accessibilityIID, &IID_IFrnAccessibilitySettings, sizeof(GUID)) == 0);
    assert(std::memcmp(&unknownIID, &IID_IUnknown, sizeof(GUID)) == 0);

    bool destroyed = false;
    IFrnPlatformSettings* original = new ProbeSettings(&destroyed);
    IFrnAccessibilitySettings* extension = nullptr;
    assert(original->QueryInterface(&accessibilityIID, reinterpret_cast<void**>(&extension)) == S_OK);
    assert(extension->GetReduceMotion() && !extension->GetReduceTransparency());

    IUnknown* first = nullptr;
    IUnknown* second = nullptr;
    assert(original->QueryInterface(&unknownIID, reinterpret_cast<void**>(&first)) == S_OK);
    assert(extension->QueryInterface(&unknownIID, reinterpret_cast<void**>(&second)) == S_OK);
    assert(first == second && "both interfaces must have the same COM identity");
    assert(first->Release() == 3);
    assert(second->Release() == 2);
    assert(extension->Release() == 1);
    assert(!destroyed);
    assert(original->Release() == 0);
    assert(destroyed);

    bool callbackDestroyed = false;
    ComPtr<IFrnActionCallback> callback;
    callback.setNoAddRef(new ProbeCallback(std::addressof(callback), &callbackDestroyed));
    {
        // Mirrors the observer's lifetime boundary around a subscriber call.
        auto invocation = callback;
        invocation->Run();
        assert(!callbackDestroyed);
    }
    assert(callbackDestroyed);
    std::puts("Native ABI: UUID bytes, dual-interface QI, IUnknown identity, reference counts and callback disposal passed.");
}
