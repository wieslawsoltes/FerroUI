"""Let ART handle faults in managed code while preserving LLDB stops in native code."""

import lldb


_JIT_REGIONS = {
    "[anon:dalvik-jit-code-cache]",
    "[anon_shmem:dalvik-jit-code-cache]",
    "[anon:dalvik-jit-zygote-cache]",
    "[anon_shmem:dalvik-jit-zygote-cache]",
    "/memfd:jit-cache",
    "/memfd:jit-zygote-cache",
    "/dev/ashmem/dalvik-jit-code-cache",
}
_ART_FILE_ROOTS = ("/apex/", "/system/", "/system_ext/", "/product/", "/vendor/", "/data/app/", "/data/dalvik-cache/")


def _is_art_code(name):
    if not name:
        return False
    deleted = " (deleted)"
    if name.endswith(deleted):
        name = name[: -len(deleted)]
    return name in _JIT_REGIONS or (
        name.startswith(_ART_FILE_ROOTS) and name.endswith((".oat", ".odex"))
    )


class PassThroughArtSignals:
    def __init__(self, _target, _args):
        pass

    def handle_stop(self, context, _stream):
        thread = context.GetThread()
        if thread.GetStopReason() != lldb.eStopReasonSignal or not thread.GetStopReasonDataCount():
            return True

        process = context.GetProcess()
        signals = process.GetUnixSignals()
        number = thread.GetStopReasonDataAtIndex(0)
        if signals.GetSignalAsCString(number) not in ("SIGSEGV", "SIGBUS") or signals.GetShouldSuppress(number):
            return True

        region = lldb.SBMemoryRegionInfo()
        pc = thread.GetFrameAtIndex(0).GetPC()
        result = process.GetMemoryRegionInfo(pc, region)
        return not (result.Success() and region.IsExecutable() and _is_art_code(region.GetName()))
