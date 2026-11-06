// Standalone diagnostic for the pinned mozangle build. GetEglDisplaySet is
// inspected read-only to attribute growth; production code uses public EGL
// device lifetime APIs only. Run each mode in a fresh process.
#include "libANGLE/Display.h"
#include <EGL/egl.h>
#include <EGL/eglext.h>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <d3d11.h>
#include <psapi.h>
#include <windows.h>
#include <wrl/client.h>
using Microsoft::WRL::ComPtr;
extern "C" __eglMustCastToProperFunctionPointerType EGLAPIENTRY
EGL_GetProcAddress(const char *);
static PFNEGLGETERRORPROC errorfn = nullptr;
static void need(bool value, const char *step) {
  if (!value) {
    fprintf(stderr, "PROBE_FAILED %s egl_error=%x win_error=%lu\n", step,
            errorfn ? errorfn() : 0, GetLastError());
    exit(1);
  }
}
static void stats(int cycle) {
  struct Summary {
    DWORD cb;
    SIZE_T allocated, committed, reserved, max;
  };
  using Fn = BOOL(WINAPI *)(HANDLE, DWORD, Summary *);
  auto fn =
      (Fn)GetProcAddress(GetModuleHandleW(L"kernel32.dll"), "HeapSummary");
  size_t displays = egl::Display::GetEglDisplaySet().size();
  Summary h{};
  h.cb = sizeof(h);
  need(fn && fn(GetProcessHeap(), 0, &h), "heap summary");
  PROCESS_MEMORY_COUNTERS_EX p{};
  p.cb = sizeof(p);
  need(GetProcessMemoryInfo(GetCurrentProcess(), (PROCESS_MEMORY_COUNTERS *)&p,
                            sizeof(p)),
       "process summary");
  printf("ALLOCATION_PROBE cycle=%d displays=%zu display_size=%zu "
         "heap_allocated=%zu heap_committed=%zu private_bytes=%zu\n",
         cycle, displays, sizeof(egl::Display), h.allocated, h.committed,
         p.PrivateUsage);
  fflush(stdout);
}
int main(int argc, char **argv) {
  errorfn = (PFNEGLGETERRORPROC)EGL_GetProcAddress("eglGetError");
  const char *mode = argc > 1 ? argv[1] : "angle";
  int loops = argc > 2 ? atoi(argv[2]) : 300;
  auto create =
      (PFNEGLCREATEDEVICEANGLEPROC)EGL_GetProcAddress("eglCreateDeviceANGLE");
  auto release =
      (PFNEGLRELEASEDEVICEANGLEPROC)EGL_GetProcAddress("eglReleaseDeviceANGLE");
  auto platform = (PFNEGLGETPLATFORMDISPLAYEXTPROC)EGL_GetProcAddress(
      "eglGetPlatformDisplayEXT");
  need(create && release && platform, "entry points");
  auto eglInitialize =
      (PFNEGLINITIALIZEPROC)EGL_GetProcAddress("eglInitialize");
  auto eglChooseConfig =
      (PFNEGLCHOOSECONFIGPROC)EGL_GetProcAddress("eglChooseConfig");
  auto eglBindAPI = (PFNEGLBINDAPIPROC)EGL_GetProcAddress("eglBindAPI");
  auto eglCreateContext =
      (PFNEGLCREATECONTEXTPROC)EGL_GetProcAddress("eglCreateContext");
  auto eglCreatePbufferSurface =
      (PFNEGLCREATEPBUFFERSURFACEPROC)EGL_GetProcAddress(
          "eglCreatePbufferSurface");
  auto eglMakeCurrent =
      (PFNEGLMAKECURRENTPROC)EGL_GetProcAddress("eglMakeCurrent");
  auto eglDestroyContext =
      (PFNEGLDESTROYCONTEXTPROC)EGL_GetProcAddress("eglDestroyContext");
  auto eglDestroySurface =
      (PFNEGLDESTROYSURFACEPROC)EGL_GetProcAddress("eglDestroySurface");
  auto eglTerminate = (PFNEGLTERMINATEPROC)EGL_GetProcAddress("eglTerminate");
  auto eglReleaseThread =
      (PFNEGLRELEASETHREADPROC)EGL_GetProcAddress("eglReleaseThread");
  ComPtr<ID3D11Device> retained;
  EGLDeviceEXT retained_egl = nullptr;
  stats(0);
  for (int i = 0; i < loops; i++) {
    ComPtr<ID3D11Device> device;
    ComPtr<ID3D11DeviceContext> context;
    if (retained) {
      device = retained;
    } else {
      need(SUCCEEDED(D3D11CreateDevice(
               nullptr, D3D_DRIVER_TYPE_HARDWARE, nullptr,
               D3D11_CREATE_DEVICE_BGRA_SUPPORT |
                   D3D11_CREATE_DEVICE_VIDEO_SUPPORT,
               nullptr, 0, D3D11_SDK_VERSION, &device, nullptr, &context)),
           "D3D device");
    }
    if (strcmp(mode, "d3d") != 0) {
      EGLDeviceEXT ed =
          retained_egl ? retained_egl
                       : create(EGL_D3D11_DEVICE_ANGLE, device.Get(), nullptr);
      need(ed != nullptr, "EGL device");
      EGLDisplay d = platform(EGL_PLATFORM_DEVICE_EXT, ed, nullptr);
      need(d != EGL_NO_DISPLAY, "display");
      need(eglInitialize(d, nullptr, nullptr), "initialize");
      if (strcmp(mode, "display") != 0) {
        EGLint attrs[] = {EGL_SURFACE_TYPE, EGL_PBUFFER_BIT,
                          EGL_RENDERABLE_TYPE, EGL_OPENGL_ES3_BIT, EGL_NONE};
        EGLConfig config;
        EGLint count = 0;
        need(eglChooseConfig(d, attrs, &config, 1, &count) && count == 1,
             "config");
        need(eglBindAPI(EGL_OPENGL_ES_API), "bind api");
        EGLint ca[] = {EGL_CONTEXT_CLIENT_VERSION, 3, EGL_NONE};
        EGLContext c = eglCreateContext(d, config, EGL_NO_CONTEXT, ca);
        need(c != EGL_NO_CONTEXT, "context");
        EGLint sa[] = {EGL_WIDTH, 1, EGL_HEIGHT, 1, EGL_NONE};
        EGLSurface s = eglCreatePbufferSurface(d, config, sa);
        need(s != EGL_NO_SURFACE, "surface");
        need(eglMakeCurrent(d, s, s, c), "make current");
        need(eglMakeCurrent(d, EGL_NO_SURFACE, EGL_NO_SURFACE, EGL_NO_CONTEXT),
             "unbind");
        need(eglDestroyContext(d, c), "destroy context");
        need(eglDestroySurface(d, s), "destroy surface");
      }
      need(eglTerminate(d), "terminate");
      if (strcmp(mode, "reuse") == 0) {
        retained = device;
        retained_egl = ed;
      } else {
        need(release(ed), "release EGL device");
      }
      if (strcmp(mode, "thread") == 0)
        need(eglReleaseThread(), "release thread");
    }
    if (context)
      context->Flush();
    context.Reset();
    device.Reset();
    if ((i + 1) % 25 == 0 || i + 1 == loops)
      stats(i + 1);
  }
  if (retained_egl)
    need(release(retained_egl), "release retained EGL device");
  retained.Reset();
  stats(loops);
  printf("ALLOCATION_PROBE_PASS mode=%s loops=%d\n", mode, loops);
  return 0;
}
