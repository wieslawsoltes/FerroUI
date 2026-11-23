// The pipelines Graphite makes, reported to Rust, and pipelines made ahead from their keys.
//
// Graphite reports every pipeline it adds to its cache with a key that can be stored
// (`ContextOptions::fPipelineCachingCallback`) and makes a pipeline from such a key before a draw
// needs it (`PrecompileContext::precompile`). The Skia binding in use binds neither, so this file
// does. Like `pipeline_executor.cpp` it declares what it uses of Skia itself, because the headers
// of Skia are not on the include path of this crate: the names and the calling conventions are
// those of `include/core/SkRefCnt.h`, `include/gpu/graphite/Context.h` and
// `include/gpu/graphite/PrecompileContext.h`. `sk_sp` is one pointer and, as Skia is built here
// (`SK_TRIVIAL_ABI` is empty), a class with a destructor of its own: a function that takes one by
// value is given the address of the caller's, which the caller destroys after the call.

#include <cstddef>
#include <cstdint>
#include <memory>
#include <string>

class SkData;

extern "C" {
// In Rust (`pipeline_record.rs`): gives one reference to a `SkData` back.
void ferroui_skia_data_unref(SkData* data);
}

template <typename T> class sk_sp {
public:
    // Takes over one reference.
    explicit sk_sp(T* pointer) : fPtr(pointer) {}
    sk_sp(const sk_sp&) = delete;
    sk_sp(sk_sp&& other) : fPtr(other.fPtr) { other.fPtr = nullptr; }
    ~sk_sp() {
        if (fPtr) {
            ferroui_skia_data_unref(fPtr);
        }
    }
    T* get() const { return fPtr; }

private:
    T* fPtr;
};

namespace skgpu::graphite {

class PrecompileContext {
public:
    ~PrecompileContext();
    bool precompile(sk_sp<SkData> serializedPipelineKey);
};

class Context {
public:
    std::unique_ptr<PrecompileContext> makePrecompileContext();
};

}  // namespace skgpu::graphite

extern "C" {

// In Rust (`pipeline_record.rs`). `data` is the key of the pipeline, for the time of the call,
// or null when the pipeline has no key that can be stored.
void ferroui_skia_pipeline_seen(int op, const char* label, size_t labelLength, uint32_t hash,
                                bool fromPrecompile, SkData* data);

// `ContextOptions::fPipelineCachingCallback`, with the parameters as they are passed: the
// operation is an `enum class` over `int`, and the key a `sk_sp<SkData>` by value, which stays
// the caller's.
void ferroui_skia_pipeline_callback(void* /* context */, int op, const std::string& label, uint32_t hash,
                                    bool fromPrecompile, sk_sp<SkData> data) {
    ferroui_skia_pipeline_seen(op, label.data(), label.size(), hash, fromPrecompile, data.get());
}

// A precompile context of `context` (a `skgpu::graphite::Context`), usable on another thread
// than the context. The caller owns it.
void* ferroui_skia_precompile_context_new(void* context) {
    return static_cast<skgpu::graphite::Context*>(context)->makePrecompileContext().release();
}

// Asks for the pipeline of `key`, one reference the call takes. False when the key is not one of
// this build of Skia or of this device.
bool ferroui_skia_precompile(void* precompileContext, SkData* key) {
    return static_cast<skgpu::graphite::PrecompileContext*>(precompileContext)->precompile(sk_sp<SkData>(key));
}

void ferroui_skia_precompile_context_delete(void* precompileContext) {
    delete static_cast<skgpu::graphite::PrecompileContext*>(precompileContext);
}

}  // extern "C"
