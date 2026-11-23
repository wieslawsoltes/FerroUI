// The thread pool Graphite compiles its pipelines on (`ContextOptions::fExecutor`).
//
// The Skia binding in use has the option as a field and no way to make an executor, so this file
// asks Skia for one. It declares what it uses of `SkExecutor` itself, with the virtual members in
// the order of `include/core/SkExecutor.h`: the headers of Skia are not on the include path of
// this crate (a build that takes Skia's published binaries has none), and the two functions
// below need the factory and the destructor only.

#include <functional>
#include <memory>

class SkExecutor {
public:
    virtual ~SkExecutor();
    static std::unique_ptr<SkExecutor> MakeMultiListFIFOThreadPool(int numWorkLists, int threads, bool allowBorrowing);
    virtual void add(std::function<void(void)> fn, int workList);
    virtual void add(std::function<void(void)> fn) = 0;
    virtual int discardAllPendingWork();
    virtual void borrow();
};

extern "C" {

// An executor with two work lists, as Graphite's pipeline manager uses them: the pipelines a
// recording waits for before those made ahead. `threads` is at least 1. The caller owns it.
SkExecutor* ferroui_skia_pipeline_executor_new(int threads) {
    // Not borrowing: the thread that waits for a pipeline does not run other compiles meanwhile.
    return SkExecutor::MakeMultiListFIFOThreadPool(2, threads < 1 ? 1 : threads, false).release();
}

void ferroui_skia_pipeline_executor_delete(SkExecutor* executor) {
    delete executor;
}

}  // extern "C"
