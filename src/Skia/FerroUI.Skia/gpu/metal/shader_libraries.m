// Shader libraries made when the application was built (docs/porting/gpu-rendering.md, "Pipelines
// made ahead of their first draw").
//
// Skia hands Metal the text of every shader of a pipeline (`newLibraryWithSource:`), and Metal
// compiles it in a helper process: three quarters of the time a pipeline takes to make. A
// library that was compiled from the same text when the application was built
// (`xcrun metal`, `xcrun metallib`) is the same library, loaded in a fraction of that
// (`newLibraryWithData:`). Skia has no place to hand it one, so the device's own
// `newLibraryWithSource:options:error:` is given a front: it asks Rust for the library of this
// very text and goes on to the device's own method when there is none.
//
// The front is put on the class that implements the method for the device this crate draws
// with, once. Any other text compiled on a device of that class (the system's own shaders) is
// not in the table and takes the method it always took.

#import <Foundation/Foundation.h>
#import <Metal/Metal.h>
#import <dispatch/dispatch.h>
#import <objc/runtime.h>
#import <os/lock.h>

// The compiled library of a shader text, or null (Rust, `gpu/metal/shader_libraries.rs`).
// `language_version` and `plain_options` say how the text was to be compiled: a library is
// given only for the way the ones in the table were made.
extern const uint8_t* ferroui_skia_metal_library_for(const uint8_t* source, size_t length, uint64_t language_version,
                                                     bool plain_options, size_t* library_length);

typedef id<MTLLibrary> (*NewLibraryWithSource)(id, SEL, NSString*, MTLCompileOptions*, NSError**)
        __attribute__((ns_returns_retained));

#define FRONTED_CLASSES 4
static os_unfair_lock fronted_lock = OS_UNFAIR_LOCK_INIT;
static Class fronted_class[FRONTED_CLASSES];
static NewLibraryWithSource fronted_original[FRONTED_CLASSES];
static int fronted_count = 0;

static NewLibraryWithSource original_for(id device) {
    NewLibraryWithSource found = NULL;
    os_unfair_lock_lock(&fronted_lock);
    for (Class class = object_getClass(device); class != Nil && found == NULL; class = class_getSuperclass(class)) {
        for (int index = 0; index < fronted_count; index++) {
            if (fronted_class[index] == class) {
                found = fronted_original[index];
                break;
            }
        }
    }
    os_unfair_lock_unlock(&fronted_lock);
    return found;
}

static id<MTLLibrary> library_with_source(id self, SEL _cmd, NSString* source, MTLCompileOptions* options,
                                          NSError** error) __attribute__((ns_returns_retained)) {
    if (source != nil) {
        const char* text = [source UTF8String];
        if (text != NULL) {
            bool plain = options == nil || (options.preprocessorMacros.count == 0 && options.fastMathEnabled);
            uint64_t version = options == nil ? 0 : (uint64_t)options.languageVersion;
            size_t length = 0;
            const uint8_t* bytes = ferroui_skia_metal_library_for((const uint8_t*)text, strlen(text), version, plain, &length);
            if (bytes != NULL && length > 0) {
                // The table lives as long as the process: nothing is copied and nothing freed.
                dispatch_data_t data = dispatch_data_create(bytes, length, dispatch_get_global_queue(QOS_CLASS_UTILITY, 0), ^{});
                NSError* load_error = nil;
                id<MTLLibrary> library = [(id<MTLDevice>)self newLibraryWithData:data error:&load_error];
                if (library != nil) {
                    return library;
                }
            }
        }
    }
    NewLibraryWithSource original = original_for(self);
    if (original == NULL) {
        return nil;
    }
    return original(self, _cmd, source, options, error);
}

// Puts the front on the class that implements `newLibraryWithSource:options:error:` for
// `device` (a retained `id<MTLDevice>`). Whether it is there now.
bool ferroui_skia_metal_libraries_install(void* device_pointer) {
    id device = (__bridge id)device_pointer;
    if (device == nil) {
        return false;
    }
    SEL selector = @selector(newLibraryWithSource:options:error:);
    // The class of the hierarchy that has the method itself.
    Class owner = Nil;
    Method method = NULL;
    for (Class class = object_getClass(device); class != Nil; class = class_getSuperclass(class)) {
        unsigned int count = 0;
        Method* methods = class_copyMethodList(class, &count);
        for (unsigned int index = 0; index < count; index++) {
            if (method_getName(methods[index]) == selector) {
                owner = class;
                method = methods[index];
                break;
            }
        }
        free(methods);
        if (method != NULL) {
            break;
        }
    }
    if (method == NULL) {
        return false;
    }
    bool installed = false;
    os_unfair_lock_lock(&fronted_lock);
    for (int index = 0; index < fronted_count; index++) {
        if (fronted_class[index] == owner) {
            installed = true;
        }
    }
    if (!installed && fronted_count < FRONTED_CLASSES) {
        fronted_class[fronted_count] = owner;
        fronted_original[fronted_count] = (NewLibraryWithSource)method_getImplementation(method);
        fronted_count++;
        method_setImplementation(method, (IMP)library_with_source);
        installed = true;
    }
    os_unfair_lock_unlock(&fronted_lock);
    return installed;
}
